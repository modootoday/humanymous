use super::{LabError, isolation};
use humanymous_runtime::gateway::configuration::Configuration;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::SocketAddr,
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

/// Source-bound process observations, not a remotely signed target attestation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessReceipt {
    /// Exact installed runtime executable bytes.
    pub runtime_sha256: String,
    /// Exact supervisor executable bytes.
    pub supervisor_sha256: String,
    /// Trusted private-pipe installation scope.
    pub installation: String,
    /// Trusted private-pipe configuration revision.
    pub revision: String,
    /// Narrow execution boundary, never full product qualification.
    pub binding: String,
}

fn digest(path: &Path) -> Result<String, LabError> {
    let file = std::fs::File::open(path).map_err(|_| LabError("binary open"))?;
    if file
        .metadata()
        .map_err(|_| LabError("binary metadata"))?
        .len()
        > 268_435_456
    {
        return Err(LabError("binary size"));
    }
    let mut reader = file.take(268_435_457);
    let mut hash = Sha256::new();
    let mut buf = [0; 65536];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|_| LabError("binary digest"))?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub(super) fn ready_address(config: &Configuration, value: &Value) -> Result<SocketAddr, LabError> {
    if value["profile"] != humanymous_runtime::gateway::service::standalone_profile(config) {
        return Err(LabError("readiness profile mismatch"));
    }
    let address: SocketAddr = value["listen"]
        .as_str()
        .ok_or(LabError("readiness address"))?
        .parse()
        .map_err(|_| LabError("readiness address"))?;
    if !address.ip().is_loopback()
        || address.port() == 0
        || address.ip() != config.listen.ip()
        || config.listen.port() != 0 && config.listen.port() != address.port()
        || address == config.origin
    {
        return Err(LabError("readiness listener mismatch"));
    }
    Ok(address)
}

/// Owns the only process/address the network client may contact. No attach API.
pub struct OwnedRuntime {
    pub(super) child: Option<Child>,
    pub(super) config: Configuration,
    pub(super) address: SocketAddr,
    receipt: ProcessReceipt,
}
impl OwnedRuntime {
    /// Start the fixed sibling runtime in an already isolated network namespace.
    /// Custody must already exist; this never provisions or repairs target keys.
    pub fn spawn(value: &Value) -> Result<Self, LabError> {
        isolation::check()?;
        let mut config: Configuration =
            serde_json::from_value(value.clone()).map_err(|_| LabError("runtime configuration"))?;
        config
            .validate()
            .map_err(|_| LabError("runtime configuration"))?;
        let supervisor = std::env::current_exe()
            .map_err(|_| LabError("supervisor image"))?
            .canonicalize()
            .map_err(|_| LabError("supervisor path"))?;
        let executable = supervisor
            .parent()
            .ok_or(LabError("supervisor directory"))?
            .join("humanymous-runtime");
        if !std::fs::symlink_metadata(&executable)
            .map_err(|_| LabError("fixed runtime image missing"))?
            .file_type()
            .is_file()
        {
            return Err(LabError("runtime must be a regular installed image"));
        }
        let receipt = ProcessReceipt {
            runtime_sha256: digest(&executable)?,
            supervisor_sha256: digest(&supervisor)?,
            installation: config.scope.installation.clone(),
            revision: config.scope.revision.clone(),
            binding: "supervisor-owned-child/private-pipe/same-loopback-only-namespace".into(),
        };
        let bytes = zeroize::Zeroizing::new(
            serde_json::to_vec(value).map_err(|_| LabError("startup serialization"))?,
        );
        if bytes.len() > 65536 {
            return Err(LabError("startup budget"));
        }
        // The retained client needs canonical scope, not a second passphrase copy.
        config.audit.keys.passphrase = Default::default();
        let child = Command::new(&executable)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| LabError("runtime spawn"))?;
        let mut owned = Self {
            child: Some(child),
            address: config.listen,
            config,
            receipt,
        };
        let child = owned.child.as_mut().ok_or(LabError("runtime missing"))?;
        let mut input = child
            .stdin
            .take()
            .ok_or(LabError("private configuration pipe"))?;
        let output = child
            .stdout
            .take()
            .ok_or(LabError("private readiness pipe"))?;
        let (tx, rx) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("owned-runtime-readiness".into())
            .spawn(move || {
                let result = (|| {
                    input
                        .write_all(&bytes)
                        .map_err(|_| LabError("private configuration write"))?;
                    drop(input);
                    let mut line = Vec::new();
                    BufReader::new(output)
                        .take(16385)
                        .read_until(b'\n', &mut line)
                        .map_err(|_| LabError("readiness read"))?;
                    if line.len() > 16384 || !line.ends_with(b"\n") {
                        return Err(LabError("readiness framing"));
                    }
                    serde_json::from_slice::<Value>(&line).map_err(|_| LabError("readiness JSON"))
                })();
                let _ = tx.send(result);
            })
            .map_err(|_| LabError("readiness reader unavailable"))?;
        let ready = rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| LabError("readiness deadline"))??;
        owned.address = ready_address(&owned.config, &ready)?;
        owned.check_live()?;
        #[cfg(target_os = "linux")]
        if digest(Path::new(&format!(
            "/proc/{}/exe",
            owned
                .child
                .as_ref()
                .ok_or(LabError("runtime missing"))?
                .id()
        )))? != owned.receipt.runtime_sha256
        {
            return Err(LabError("child image mismatch"));
        }
        Ok(owned)
    }
    /// Observed source/process scope; contains no credentials.
    pub fn receipt(&self) -> &ProcessReceipt {
        &self.receipt
    }
    /// Explicit bounded child termination; Drop also performs it on every error.
    pub fn stop(&mut self) -> Result<(), LabError> {
        if let Some(child) = self.child.as_mut() {
            if child
                .try_wait()
                .map_err(|_| LabError("child status"))?
                .is_none()
                && child.kill().is_err()
                && child
                    .try_wait()
                    .map_err(|_| LabError("child status"))?
                    .is_none()
            {
                return Err(LabError("child stop"));
            }
            child.wait().map_err(|_| LabError("child reap"))?;
        }
        self.child = None;
        Ok(())
    }
    pub(super) fn check_live(&mut self) -> Result<(), LabError> {
        isolation::check()?;
        let child = self.child.as_mut().ok_or(LabError("runtime stopped"))?;
        if child
            .try_wait()
            .map_err(|_| LabError("child status"))?
            .is_some()
        {
            return Err(LabError("runtime exited"));
        }
        isolation::child_namespace(child.id())
    }
}
impl Drop for OwnedRuntime {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readiness_is_exact_scope_not_just_a_loopback_string() {
        let config = super::super::config();
        let profile = humanymous_runtime::gateway::service::standalone_profile(&config);
        let mut value = serde_json::json!({"listen":"127.0.0.1:9000","profile":profile});
        assert!(ready_address(&config, &value).is_ok());
        value["profile"] = config.readiness();
        assert!(ready_address(&config, &value).is_err());
        value["profile"] = profile.clone();
        value["profile"]["audit_external_witness"] = serde_json::json!(true);
        assert!(ready_address(&config, &value).is_err());
        value["profile"] = profile.clone();
        value["profile"]["unexpected_capability"] = serde_json::json!(false);
        assert!(ready_address(&config, &value).is_err());
        value["profile"] = profile.clone();
        value["profile"]["revision"] = serde_json::json!("foreign");
        assert!(ready_address(&config, &value).is_err());
        value["profile"] = profile;
        for address in [
            "0.0.0.0:9000",
            "192.0.2.1:9000",
            "127.0.0.1:0",
            "127.0.0.1:8081",
        ] {
            value["listen"] = serde_json::json!(address);
            assert!(ready_address(&config, &value).is_err());
        }
    }
}
