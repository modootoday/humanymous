//! Pinned private configuration plus one native-security secret-store authority.
use super::plan::{HostPlan, invalid};
use crate::{installation::permissions::plain, release::digest};
use humanymous_authority::startup::authorization::{
    RoleData, ScopeData, SignatureCheckedAuthorization,
};
use humanymous_runtime::{configuration::Configuration, startup::ExternalWitness};
use std::{
    fs,
    io::{self, Read},
    path::Path,
};
use zeroize::Zeroizing;

/// Content-bound preflight DATA only. This owns no native admission, file handle,
/// witness, credential or Ready capability and cannot start a ServiceHost.
pub struct HostAuthorizationData {
    plan: HostPlan,
    configuration: Configuration,
    authorization: SignatureCheckedAuthorization,
}
impl HostAuthorizationData {
    /// Exact parsed BASE plan content, not native ownership proof.
    pub fn plan(&self) -> &HostPlan {
        &self.plan
    }
    /// Validated configuration with an empty unseal passphrase.
    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }
    /// Shared signature-checked DATA; no conversion into native preparation.
    pub fn authorization(&self) -> &SignatureCheckedAuthorization {
        &self.authorization
    }
    /// Move the same preflight DATA/configuration to a future native-admission
    /// composer without cloning it. No native guard or activation is supplied.
    pub fn into_parts(self) -> (HostPlan, Configuration, SignatureCheckedAuthorization) {
        (self.plan, self.configuration, self.authorization)
    }
}

/// Bind exact bounded BASE-plan/configuration bytes to the shared checked DATA.
/// No filesystem, secret-store, listener or native preparation operation occurs.
/// Native issuer/instance/claim/W admission must independently precede unseal.
pub fn bind_authorization_data(
    plan_bytes: &[u8],
    configuration_bytes: &[u8],
    authorization: SignatureCheckedAuthorization,
) -> io::Result<HostAuthorizationData> {
    let (plan, configuration) = bind_documents(
        plan_bytes,
        configuration_bytes,
        authorization.service(),
        authorization.scope(),
        authorization.startup_cap_ms(),
    )?;
    Ok(HostAuthorizationData {
        plan,
        configuration,
        authorization,
    })
}

fn bind_documents(
    plan_bytes: &[u8],
    configuration_bytes: &[u8],
    service: &RoleData,
    scope: &ScopeData,
    startup_cap_ms: u32,
) -> io::Result<(HostPlan, Configuration)> {
    if plan_bytes.len() > 32_768 || configuration_bytes.len() > 65_536 {
        return Err(invalid("host authorization document budget"));
    }
    let plan: HostPlan = serde_json::from_slice(plan_bytes)
        .map_err(|_| invalid("host authorization BASE-plan schema"))?;
    plan.validate()?;
    if plan.observer.is_some() {
        return Err(invalid("legacy observer selector is not a bootstrap lease"));
    }
    if service.name != plan.service_name
        || hex::encode(service.base_plan) != digest(plan_bytes)
        || hex::encode(service.artifact) != plan.executable_sha256
        || u64::from(startup_cap_ms) != plan.startup_ms
    {
        return Err(invalid("host authorization service or BASE-plan binding"));
    }
    let configuration = nonsecret_configuration(configuration_bytes, &plan)?;
    let configured = &configuration.scope;
    if scope.installation != configured.installation
        || scope.tenant != configured.tenant
        || scope.origin != configured.origin
        || scope.revision != configured.revision
        || scope.application_authority != configured.authority
    {
        return Err(invalid("host authorization scope binding"));
    }
    Ok((plan, configuration))
}

/// A producer owns OS identity/store checks; implementations must not prompt or fall back.
pub trait SecretSource {
    /// Return an independently authorized service-scoped secret without logging it.
    fn read(&self, service_name: &str, credential_id: &str) -> io::Result<Zeroizing<Vec<u8>>>;
}

/// Shared native-security is the only implementation of OS credential primitives.
pub struct NativeSecrets;
#[derive(Debug)]
pub(super) struct ProtectedReadFailure;
impl std::fmt::Display for ProtectedReadFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("protected service credential unavailable")
    }
}
impl std::error::Error for ProtectedReadFailure {}
impl SecretSource for NativeSecrets {
    fn read(&self, service_name: &str, credential_id: &str) -> io::Result<Zeroizing<Vec<u8>>> {
        let reference = humanymous_native_security::secret::ServiceSecretReference::new(
            service_name,
            credential_id,
        )?;
        humanymous_native_security::secret::read_service_secret(&reference)
    }
}

pub(super) fn bounded(path: &Path, limit: u64) -> io::Result<Zeroizing<Vec<u8>>> {
    plain(path)?;
    let mut file = opened(path)?;
    private_input(&file, path)?;
    read_bounded(&mut file, limit)
}

fn opened(path: &Path) -> io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000);
    }
    options.open(path)
}

fn private_input(file: &fs::File, path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        let _ = path;
        let identity = humanymous_native_security::process::current_identity()?;
        humanymous_native_security::storage::verify_windows_privacy(
            file,
            identity.sid(),
            humanymous_native_security::storage::WindowsPrivacy::File,
        )?;
    }
    #[cfg(not(windows))]
    {
        let _ = file;
        crate::installation::permissions::private_file(path)?;
    }
    Ok(())
}

fn read_bounded(file: &mut fs::File, limit: u64) -> io::Result<Zeroizing<Vec<u8>>> {
    if !file.metadata()?.is_file() {
        return Err(invalid("regular runtime input required"));
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(invalid("host input budget"));
    }
    Ok(bytes)
}

/// Verify the exact non-secret host-plan before any native secret-store operation.
pub fn read_plan(path: &Path, expected_sha256: &str) -> io::Result<HostPlan> {
    if !super::plan::hash(expected_sha256) {
        return Err(invalid("host plan digest encoding"));
    }
    plain(path)?;
    let mut file = opened(path)?;
    let bytes = read_bounded(&mut file, 32_768)?;
    if digest(&bytes) != expected_sha256 {
        return Err(invalid("host plan digest mismatch"));
    }
    let plan: HostPlan = serde_json::from_slice(&bytes).map_err(|_| invalid("host plan schema"))?;
    plan.validate()?;
    #[cfg(windows)]
    {
        use humanymous_native_security::{process::current_identity, storage};
        let service = storage::resolve_service_sid(&plan.service_name)?;
        if current_identity()?.sid() != service {
            return Err(invalid("actual launch service principal required"));
        }
        storage::verify_service_access(
            &file,
            plan.installer_owner_sid
                .as_deref()
                .ok_or_else(|| invalid("installer SID required"))?,
            &service,
            storage::ServiceRole::Executable,
        )?;
    }
    #[cfg(not(windows))]
    private_input(&file, path)?;
    Ok(plan)
}

pub(super) fn secret_string(
    source: &impl SecretSource,
    service_name: &str,
    id: &str,
) -> io::Result<Zeroizing<String>> {
    let mut bytes = source
        .read(service_name, id)
        .map_err(|_| io::Error::other(ProtectedReadFailure))?;
    if bytes.is_empty() || bytes.len() > 4096 {
        return Err(invalid("protected secret budget"));
    }
    // Move the one owned allocation; invalid UTF-8 must also be zeroized on error.
    match String::from_utf8(std::mem::take(&mut *bytes)) {
        Ok(secret) => Ok(Zeroizing::new(secret)),
        Err(error) => {
            let _ = Zeroizing::new(error.into_bytes());
            Err(invalid("protected secret encoding"))
        }
    }
}

pub(super) fn configuration(
    plan: &HostPlan,
    source: &impl SecretSource,
    witness: Option<&ExternalWitness>,
) -> io::Result<Configuration> {
    let mut configuration = nonsecret_configuration(bytes, plan)?;
    if configuration.audit.require_external_witness && !has_witness {
        return Err(invalid("required independent witness provider unavailable"));
    }
    configuration.audit.keys.passphrase =
        secret_string(source, &plan.service_name, &plan.unseal_credential_id)?;
    Ok(configuration)
}

fn nonsecret_configuration(bytes: &[u8], plan: &HostPlan) -> io::Result<Configuration> {
    if bytes.len() > 65_536 {
        return Err(invalid("non-secret configuration budget"));
    }
    let bytes = bounded(&plan.configuration, 65_536)?;
    prepare_configuration(bytes.as_slice(), plan, source, witness.is_some())
}

fn prepare_configuration(
    bytes: &[u8],
    plan: &HostPlan,
    source: &impl SecretSource,
    has_witness: bool,
) -> io::Result<Configuration> {
    if digest(bytes) != plan.configuration_sha256 {
        return Err(invalid("non-secret runtime configuration digest mismatch"));
    }
    let configuration = humanymous_runtime::service::read_configuration(bytes)?;
    if !configuration.audit.keys.passphrase.is_empty() {
        return Err(invalid(
            "plaintext unseal secret in configuration is forbidden",
        ));
    }
    if configuration.scope.installation != plan.installation
        || configuration.listen == plan.diagnostic
        || configuration.origin == plan.diagnostic
    {
        return Err(invalid("runtime host scope or diagnostic collision"));
    }
    Ok(configuration)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    struct Source {
        calls: Cell<usize>,
        value: Vec<u8>,
    }
    impl SecretSource for Source {
        fn read(&self, _: &str, _: &str) -> io::Result<Zeroizing<Vec<u8>>> {
            self.calls.set(self.calls.get() + 1);
            Ok(Zeroizing::new(self.value.clone()))
        }
    }
    fn plan_value() -> serde_json::Value {
        let path = if cfg!(windows) {
            "C:\\owned\\runtime.json"
        } else {
            "/owned/runtime.json"
        };
        serde_json::json!({"schema":"humanymous.runtime-host.v1","service_name":"humanymous-lab-unit","installer_owner_sid":"S-1-5-21-1000","installation":"unit","generation":"generation-1","executable_sha256":"0".repeat(64),"configuration":path,"configuration_sha256":"0".repeat(64),"unseal_credential_id":"audit","observer":null,"diagnostic":"127.0.0.1:19003","ready_response":"ready","startup_ms":1000,"stop":{"drain_ms":1000,"supervisor_reserve_ms":1000,"manager_stop_ms":if cfg!(windows){6000}else{5000}},"mode":"foreground"})
    }
    fn plan() -> HostPlan {
        serde_json::from_value(plan_value()).unwrap()
    }
    fn configuration_value() -> serde_json::Value {
        serde_json::json!({"listen":"127.0.0.1:19001","origin":"127.0.0.1:19002","scope":{"installation":"unit","tenant":"tenant","origin":"origin","revision":"revision","authority":"unit.local"},"routes":[{"method":"GET","path":"/protected","required":{"raw_tls":false,"original_h2":false,"raw_header_order":false,"collector":false,"rit":true}}],"audit":{"directory":"audit","node_id":"node","keys":{"directory":"keys","passphrase":""},"allow_new":false,"trusted_head":null,"require_external_witness":false},"owner":{"kind":"unix","uid":1000},"session_capacity":8,"session_ttl_ms":1000,"timeout_ms":1000,"rate_per_second":8,"browser":null})
    }
    #[test]
    fn pure_host_projection_binds_exact_documents_and_all_scope_fields() {
        // Synthetic DATA projections exercise CLI comparisons, not signatures or
        // native authority. The public entry requires the shared checked type.
        let config_bytes = serde_json::to_vec(&configuration_value()).unwrap();
        let mut value = plan_value();
        value["configuration_sha256"] = serde_json::json!(digest(&config_bytes));
        let plan_bytes = serde_json::to_vec(&value).unwrap();
        let service = RoleData {
            role: humanymous_authority::startup::authorization::AudienceRole::Service,
            name: "humanymous-lab-unit".into(),
            sid: vec![1, 0, 0, 0, 0, 0, 0, 5],
            pid: 101,
            creation: 1,
            epoch: [1; 16],
            base_plan: hex::decode(digest(&plan_bytes))
                .unwrap()
                .try_into()
                .unwrap(),
            artifact: [0; 32],
        };
        let scope = ScopeData {
            installation: "unit".into(),
            tenant: "tenant".into(),
            origin: "origin".into(),
            revision: "revision".into(),
            application_authority: "unit.local".into(),
        };
        let (_, config) =
            bind_documents(&plan_bytes, &config_bytes, &service, &scope, 1000).unwrap();
        assert!(config.audit.keys.passphrase.is_empty());
        let mut changed = plan_bytes.clone();
        changed.push(b' ');
        assert!(bind_documents(&changed, &config_bytes, &service, &scope, 1000).is_err());
        let mut changed = config_bytes.clone();
        changed.push(b' ');
        assert!(bind_documents(&plan_bytes, &changed, &service, &scope, 1000).is_err());
        assert!(bind_documents(&plan_bytes, &config_bytes, &service, &scope, 999).is_err());
        for field in 0..5 {
            let mut changed = scope.clone();
            match field {
                0 => changed.installation.push('x'),
                1 => changed.tenant.push('x'),
                2 => changed.origin.push('x'),
                3 => changed.revision.push('x'),
                _ => changed.application_authority.push('x'),
            }
            assert!(bind_documents(&plan_bytes, &config_bytes, &service, &changed, 1000).is_err());
        }
        for field in 0..3 {
            let mut changed = service.clone();
            match field {
                0 => changed.name.push('x'),
                1 => changed.artifact[0] ^= 1,
                _ => changed.base_plan[0] ^= 1,
            }
            assert!(bind_documents(&plan_bytes, &config_bytes, &changed, &scope, 1000).is_err());
        }
        assert!(
            bind_documents(&vec![b' '; 32_769], &config_bytes, &service, &scope, 1000).is_err()
        );
        assert!(bind_documents(&plan_bytes, &vec![b' '; 65_537], &service, &scope, 1000).is_err());
        let peer = if cfg!(windows) {
            serde_json::json!({"kind":"windows","sid":"S-1-5-21-1000","pid":101})
        } else {
            serde_json::json!({"kind":"unix","uid":1000})
        };
        value["observer"] = serde_json::json!({"endpoint":"synthetic","peer":peer,"credential_id":"legacy-observer"});
        let legacy = serde_json::to_vec(&value).unwrap();
        let mut changed = service;
        changed.base_plan = hex::decode(digest(&legacy)).unwrap().try_into().unwrap();
        let error = bind_documents(&legacy, &config_bytes, &changed, &scope, 1000)
            .err()
            .unwrap();
        assert!(error.to_string().contains("legacy observer"));
    }
    #[test]
    fn pure_input_rejects_plaintext_and_required_witness_before_secret_read() {
        let source = Source {
            calls: Cell::new(0),
            value: b"synthetic-unseal".to_vec(),
        };
        let mut plan = plan();
        let mut value = configuration_value();
        value["audit"]["keys"]["passphrase"] = serde_json::json!("never-echo-this");
        let bytes = serde_json::to_vec(&value).unwrap();
        plan.configuration_sha256 = digest(&bytes);
        let error = prepare_configuration(&bytes, &plan, &source, false)
            .err()
            .unwrap();
        assert!(!error.to_string().contains("never-echo-this"));
        assert_eq!(source.calls.get(), 0);
        value["audit"]["keys"]["passphrase"] = serde_json::json!("");
        value["audit"]["require_external_witness"] = serde_json::json!(true);
        let bytes = serde_json::to_vec(&value).unwrap();
        plan.configuration_sha256 = digest(&bytes);
        assert!(prepare_configuration(&bytes, &plan, &source, false).is_err());
        assert_eq!(source.calls.get(), 0);
    }
    #[test]
    fn pure_input_injects_only_bounded_utf8_from_protected_producer() {
        let mut plan = plan();
        let value = serde_json::to_vec(&configuration_value()).unwrap();
        plan.configuration_sha256 = digest(&value);
        let source = Source {
            calls: Cell::new(0),
            value: b"synthetic-unseal".to_vec(),
        };
        let config = prepare_configuration(&value, &plan, &source, false).unwrap();
        assert_eq!(config.audit.keys.passphrase.as_str(), "synthetic-unseal");
        assert_eq!(source.calls.get(), 1);
        let mut changed = value.clone();
        changed.push(b' ');
        assert!(prepare_configuration(&changed, &plan, &source, false).is_err());
        assert_eq!(source.calls.get(), 1);
        for bytes in [vec![], vec![0xff], vec![b'x'; 4097]] {
            let source = Source {
                calls: Cell::new(0),
                value: bytes,
            };
            assert!(prepare_configuration(&value, &plan, &source, false).is_err());
        }
        struct Unavailable;
        impl SecretSource for Unavailable {
            fn read(&self, _: &str, _: &str) -> io::Result<Zeroizing<Vec<u8>>> {
                Err(io::Error::other("private provider payload must not escape"))
            }
        }
        let error = prepare_configuration(&value, &plan, &Unavailable, false)
            .err()
            .unwrap();
        assert!(error.get_ref().unwrap().is::<ProtectedReadFailure>());
        assert!(!error.to_string().contains("payload"));
    }
}
