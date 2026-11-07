//! Disposable actual-service fixture. No production provisioning or erasure API.
use super::{HostExit, HostPlan, NativeSecrets, SecretSource, invalid};
use humanymous_audit::keys::{
    KeyMaterial,
    custody::{Custody, Owner},
};
use humanymous_authority::credentials::Credential;
use humanymous_native_security::{
    process::current_identity,
    secret::{
        ServiceSecretReference,
        pilot::{DisposableServiceContext, ServiceSecretFixture},
    },
    storage,
};
use serde::Deserialize;
use std::{
    fs,
    io::{self, Write},
    net::SocketAddr,
    os::windows::fs::OpenOptionsExt,
    path::PathBuf,
};
use zeroize::Zeroizing;

/// Nonsecret, digest-pinned bootstrap for one exclusively owned disposable service.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PilotPlan {
    /// External evidence must authenticate this marker; it is not attestation.
    pub run_id: u64,
    /// Full external source identity.
    pub source_sha: String,
    /// Fresh nonsecret 128-bit correlation nonce, independent of secrets.
    pub nonce: String,
    /// Installer-owned exact delegated StateDirectory.
    pub parent: PathBuf,
    /// New private child component; existing paths are never repaired.
    pub child: String,
    /// New public-to-installer, nonsecret receipt filename.
    pub receipt: String,
    /// Owned loopback origin. This fixture never contacts third parties.
    pub origin: SocketAddr,
    /// Real runtime listener, independently probed by the parent.
    pub listen: SocketAddr,
    /// Remove only the newly created secret before the production read.
    pub missing_credential: bool,
}
fn component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
impl PilotPlan {
    pub(super) fn validate(&self, plan: &HostPlan) -> io::Result<()> {
        DisposableServiceContext::new(&plan.service_name, self.run_id, &self.source_sha)?;
        if !component(&self.child)
            || !component(&self.receipt)
            || self.nonce.len() != 32
            || !self
                .nonce
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || !self.parent.is_absolute()
            || plan.configuration != self.parent.join(&self.child).join("configuration")
            || plan.configuration_sha256 != "0".repeat(64)
            || plan.observer.is_some()
            || plan.mode != super::HostMode::Native
            || !self.origin.ip().is_loopback()
            || self.origin.port() == 0
            || !self.listen.ip().is_loopback()
            || self.listen.port() == 0
            || self.origin == self.listen
            || self.origin == plan.diagnostic
            || self.listen == plan.diagnostic
        {
            return Err(invalid("invalid disposable runtime bootstrap"));
        }
        Ok(())
    }
}

/// Owns only resources created by this fixture inside the actual service process.
pub(super) struct Session {
    metadata: PilotPlan,
    service: String,
    installer: String,
    generation: String,
    executable: String,
    root: Option<PathBuf>,
    credential: Option<ServiceSecretFixture>,
    removed_credential: bool,
    prepared: bool,
}
impl Session {
    pub(super) fn new(metadata: PilotPlan, plan: &HostPlan) -> Self {
        Self {
            metadata,
            service: plan.service_name.clone(),
            installer: plan.installer_owner_sid.clone().unwrap_or_default(),
            generation: plan.generation.clone(),
            executable: plan.executable_sha256.clone(),
            root: None,
            credential: None,
            removed_credential: false,
            prepared: false,
        }
    }
    pub(super) fn prepare(&mut self, plan: &mut HostPlan) -> io::Result<()> {
        let context = DisposableServiceContext::new(
            &self.service,
            self.metadata.run_id,
            &self.metadata.source_sha,
        )?;
        let root = storage::create_service_private_directory(
            &self.metadata.parent,
            &self.metadata.child,
            &self.installer,
            &self.service,
        )?;
        self.root = Some(root.clone());
        let keys = storage::create_service_private_subdirectory(&root, "keys", &self.service)?;
        let audit = storage::create_service_private_subdirectory(&root, "audit", &self.service)?;
        let id_entropy = Credential::generate().map_err(|_| invalid("fixture entropy"))?;
        plan.unseal_credential_id =
            format!("pilot-{}", &id_entropy.expose_for_provisioning()[..32]);
        let secret = Credential::generate().map_err(|_| invalid("fixture entropy"))?;
        let reference = ServiceSecretReference::new(&self.service, &plan.unseal_credential_id)?;
        self.credential = Some(ServiceSecretFixture::create(
            &reference,
            Zeroizing::new(secret.expose_for_provisioning().as_bytes().to_vec()),
            &context,
        )?);
        let protected = NativeSecrets.read(&self.service, &plan.unseal_credential_id)?;
        if protected.as_slice() != secret.expose_for_provisioning().as_bytes() {
            return Err(invalid("fixture protected read mismatch"));
        }
        {
            let sid = current_identity()?.sid().to_owned();
            let mut custody = Custody::acquire(&keys, Owner::WindowsSid(sid))?;
            let seed = Credential::generate().map_err(|_| invalid("fixture entropy"))?;
            let mut bytes = Zeroizing::new([0; 32]);
            hex::decode_to_slice(seed.expose_for_provisioning(), &mut bytes[..])
                .map_err(|_| invalid("fixture seed encoding"))?;
            let hmac = Credential::generate().map_err(|_| invalid("fixture entropy"))?;
            let _ = custody.load_or_create(&protected, || {
                KeyMaterial::new(
                    *bytes,
                    hmac.expose_for_provisioning().as_bytes().to_vec(),
                    None,
                    None,
                )
            })?;
        }
        // Only nonsecret configuration crosses the filesystem boundary.
        let configuration=serde_json::to_vec(&serde_json::json!({
            "listen":self.metadata.listen,"origin":self.metadata.origin,
            "scope":{"installation":plan.installation,"tenant":"pilot","origin":"owned-loopback","revision":"pilot-v1","authority":"pilot.local"},
            "routes":[{"method":"GET","path":"/protected","required":{"raw_tls":false,"original_h2":false,"raw_header_order":false,"collector":false,"rit":true}}],
            "audit":{"directory":audit,"node_id":"pilot-node","keys":{"directory":keys,"passphrase":""},"allow_new":true,"trusted_head":null,"require_external_witness":false},
            "owner":{"kind":"windows","sid":current_identity()?.sid(),"pid":std::process::id()},
            "session_capacity":8,"session_ttl_ms":1000,"timeout_ms":1000,"rate_per_second":8,"browser":null
        })).map_err(|_|invalid("fixture configuration encoding"))?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&plan.configuration)?;
        file.write_all(&configuration)?;
        file.sync_all()?;
        plan.configuration_sha256 = crate::release::digest(&configuration);
        if self.metadata.missing_credential {
            self.credential
                .take()
                .ok_or_else(|| invalid("fixture credential unavailable"))?
                .finish()?;
            self.removed_credential = true;
            if NativeSecrets
                .read(&self.service, &plan.unseal_credential_id)
                .is_ok()
            {
                return Err(invalid("removed fixture credential remained readable"));
            }
        }
        self.prepared = true;
        Ok(())
    }
    fn parent(&self) -> io::Result<fs::File> {
        crate::installation::permissions::plain(&self.metadata.parent)?;
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(0x02200000)
            .open(&self.metadata.parent)?;
        let service = storage::resolve_service_sid(&self.service)?;
        if current_identity()?.sid() != service {
            return Err(invalid("actual fixture service required"));
        }
        storage::verify_service_access(
            &file,
            &self.installer,
            &service,
            storage::ServiceRole::StateDirectory,
        )?;
        Ok(file)
    }
    pub(super) fn finish(
        &mut self,
        result: io::Result<HostExit>,
        ready: bool,
        readiness_manifest: Option<serde_json::Value>,
    ) -> io::Result<HostExit> {
        // Runtime and its custody/WAL guards have returned and dropped before this call.
        let credential_cleanup = match self.credential.take() {
            Some(credential) => credential.finish().is_ok(),
            None => self.removed_credential,
        };
        let private_cleanup = match self.root.take() {
            Some(root) => self
                .parent()
                .and_then(|_| {
                    let file = fs::OpenOptions::new()
                        .read(true)
                        .custom_flags(0x02200000)
                        .open(&root)?;
                    storage::verify_windows_privacy(
                        &file,
                        current_identity()?.sid(),
                        storage::WindowsPrivacy::ProtectedDirectory,
                    )?;
                    drop(file);
                    fs::remove_dir_all(&root)?;
                    match fs::symlink_metadata(root) {
                        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                        _ => Err(invalid("fixture state cleanup unproven")),
                    }
                })
                .is_ok(),
            None => false,
        };
        let missing = self.metadata.missing_credential
            && self.prepared
            && self.removed_credential
            && result.as_ref().err().is_some_and(|error| {
                error
                    .get_ref()
                    .is_some_and(|cause| cause.is::<super::input::ProtectedReadFailure>())
            })
            && !ready;
        let expected = if self.metadata.missing_credential {
            missing
        } else {
            ready && result.is_ok()
        };
        let failure = result
            .as_ref()
            .err()
            .map(|error| super::describe_failure(error, &self.generation, &self.executable));
        let receipt = serde_json::json!({"schema":"humanymous.runtime-service-pilot.receipt.v1","run_id":self.metadata.run_id,"source_sha":self.metadata.source_sha,"nonce":self.metadata.nonce,"service":self.service,"generation":self.generation,"executable_sha256":self.executable,"service_pid":std::process::id(),"prepared":self.prepared,"readiness":ready,"readiness_manifest":readiness_manifest,"missing_credential_fail_closed":missing,"exit":result.as_ref().ok(),"failure":failure,"credential_cleanup":credential_cleanup,"private_state_cleanup":private_cleanup,"error_code":if expected&&credential_cleanup&&private_cleanup{"none"}else{"fixture-phase-failed"},"product_qualified":false});
        let _parent = self.parent()?;
        let bytes =
            serde_json::to_vec(&receipt).map_err(|_| invalid("fixture receipt encoding"))?;
        if bytes.len() > 8192 {
            return Err(invalid("fixture receipt budget"));
        }
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(
            self.metadata
                .parent
                .join(format!("{}-{}", self.metadata.receipt, std::process::id())),
        )?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        if !expected || !credential_cleanup || !private_cleanup {
            return Err(invalid("fixture phases or cleanup unproven"));
        }
        result
    }
}
