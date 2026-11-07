//! Pinned private configuration plus one native-security secret-store authority.
use super::plan::{HostPlan, invalid};
use crate::{installation::permissions::plain, release::digest};
use humanymous_runtime::{configuration::Configuration, startup::ExternalWitness};
use std::{
    fs,
    io::{self, Read},
    path::Path,
};
use zeroize::Zeroizing;

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
    let mut configuration = humanymous_runtime::service::read_configuration(bytes)?;
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
    if configuration.audit.require_external_witness && !has_witness {
        return Err(invalid("required independent witness provider unavailable"));
    }
    configuration.audit.keys.passphrase =
        secret_string(source, &plan.service_name, &plan.unseal_credential_id)?;
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
    fn plan() -> HostPlan {
        let path = if cfg!(windows) {
            "C:\\owned\\runtime.json"
        } else {
            "/owned/runtime.json"
        };
        serde_json::from_value(serde_json::json!({"schema":"humanymous.runtime-host.v1","service_name":"humanymous-lab-unit","installation":"unit","generation":"generation-1","executable_sha256":"0".repeat(64),"configuration":path,"configuration_sha256":"0".repeat(64),"unseal_credential_id":"audit","observer":null,"diagnostic":"127.0.0.1:19003","ready_response":"ready","startup_ms":1000,"stop":{"drain_ms":1000,"supervisor_reserve_ms":1000,"manager_stop_ms":if cfg!(windows){6000}else{5000}},"mode":"foreground"})).unwrap()
    }
    fn configuration_value() -> serde_json::Value {
        serde_json::json!({"listen":"127.0.0.1:19001","origin":"127.0.0.1:19002","scope":{"installation":"unit","tenant":"tenant","origin":"origin","revision":"revision","authority":"unit.local"},"routes":[{"method":"GET","path":"/protected","required":{"raw_tls":false,"original_h2":false,"raw_header_order":false,"collector":false,"rit":true}}],"audit":{"directory":"audit","node_id":"node","keys":{"directory":"keys","passphrase":""},"allow_new":false,"trusted_head":null,"require_external_witness":false},"owner":{"kind":"unix","uid":1000},"session_capacity":8,"session_ttl_ms":1000,"timeout_ms":1000,"rate_per_second":8,"browser":null})
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
