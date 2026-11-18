//! Documented adopter step 3 ("provision sealed custody on the hmn-data
//! volume"), which no shipped Rust command performs yet, plus the attested
//! installation directories and an offline audit read-back. It mounts only
//! the manifest volume and has no network.
//!
//! One private control line in, one report line out:
//! - `{"action":"provision","configuration":TEXT}` creates the custody and
//!   empty audit directories named by the configuration, sealed custody under
//!   its passphrase, and both installation directories holding TEXT verbatim.
//! - `{"action":"change"}` rewrites the attested installation's configuration
//!   with a different revision (a changed target).
//! - `{"action":"verify","configuration":TEXT}` authenticates the persisted
//!   history offline with the same custody and reports installation key facts
//!   (presence, mode, length; never the key bytes).
use super::{FOREIGN, INSTALLATION, STATE, emit, read_line};
use crate::origin_audit::gate::{private_directory, provision};
use humanymous_runtime::{
    audit::{
        keys::custody::Owner,
        log::{SealedLog, SealedLogOptions, UnlockCredential},
    },
    gateway::{
        attestation::{CONFIGURATION_FILE, KEY_FILE},
        configuration::Configuration,
    },
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, BufReader, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
};
use zeroize::Zeroizing;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Parse and validate with the runtime's own type; both custody paths must
/// live on the manifest volume.
fn configuration(text: &str) -> Result<Configuration> {
    let parsed: Configuration = serde_json::from_str(text)?;
    parsed
        .validate()
        .map_err(|_| "configuration refused by the runtime's validate()")?;
    for directory in [&parsed.audit.directory, &parsed.audit.keys.directory] {
        if !directory.starts_with(STATE) || directory.as_path() == Path::new(STATE) {
            return Err("custody paths must be on the manifest volume".into());
        }
    }
    Ok(parsed)
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn provision_all(text: &str) -> Result<Value> {
    let parsed = configuration(text)?;
    let state = fs::metadata(STATE)?;
    // The image's empty-volume layout must hand the volume to the runtime UID.
    let volume = json!({"uid":state.uid(),"mode":format!("{:o}",state.mode() & 0o7777)});
    private_directory(&parsed.audit.keys.directory)?;
    provision(
        &parsed.audit.keys.directory,
        &parsed.audit.directory,
        parsed.audit.keys.passphrase.as_str(),
    )?;
    for directory in [INSTALLATION, FOREIGN] {
        private_directory(Path::new(directory))?;
        write_private(
            &Path::new(directory).join(CONFIGURATION_FILE),
            text.as_bytes(),
        )?;
    }
    Ok(
        json!({"role":"dep_custody","action":"provision","volume":volume,
        "sealed":parsed.audit.keys.directory.join("keys.sealed").is_file(),
        "audit_directory_empty":fs::read_dir(&parsed.audit.directory)?.next().is_none(),
        "configuration_sha256":sha256(text.as_bytes())}),
    )
}

fn change() -> Result<Value> {
    let path = Path::new(INSTALLATION).join(CONFIGURATION_FILE);
    let mut document: Value = serde_json::from_slice(&fs::read(&path)?)?;
    document["scope"]["revision"] = json!("changed-after-enrollment");
    let bytes = serde_json::to_vec(&document)?;
    configuration(std::str::from_utf8(&bytes)?)?;
    write_private(&path, &bytes)?;
    Ok(json!({"role":"dep_custody","action":"change","configuration_sha256":sha256(&bytes)}))
}

fn key_facts(directory: &str) -> Value {
    let key = Path::new(directory).join(KEY_FILE);
    match fs::symlink_metadata(&key) {
        Ok(metadata) => json!({"present":metadata.is_file(),
            "mode":format!("{:o}",metadata.mode() & 0o7777),"length":metadata.len(),
            "uid":metadata.uid()}),
        Err(_) => json!({"present":false}),
    }
}

fn verify(text: &str) -> Result<Value> {
    let parsed = configuration(text)?;
    let options = SealedLogOptions {
        key_directory: parsed.audit.keys.directory.clone(),
        wal_directory: parsed.audit.directory.clone(),
        owner: Owner::CurrentUser,
        node_id: parsed.audit.node_id.clone(),
        checkpoint_every: 1,
        allow_new_history: false,
        trusted_head: None,
        require_external_witness: false,
    };
    let mut unlock = Zeroizing::new(parsed.audit.keys.passphrase.as_str().to_owned());
    let history = match SealedLog::open(&options, UnlockCredential::Utf8(&mut unlock), None) {
        Ok(log) => {
            let records = log.records().unwrap_or_default();
            let checkpoints = log.checkpoints().unwrap_or_default();
            let contiguous = records
                .iter()
                .enumerate()
                .all(|(index, record)| record.seq == index as u64 + 1);
            let summary: Vec<_> = records
                .iter()
                .map(|r| json!({"seq":r.seq,"event_type":r.event_type,"verdict":r.verdict,"enforcement_action":r.enforcement_action}))
                .collect();
            json!({"verified":true,"sequence":log.sequence(),"contiguous":contiguous,
                "checkpoints":checkpoints.len(),"head":checkpoints.last(),"records":summary})
        }
        Err(error) => json!({"verified":false,"stage":format!("{:?}",error.stage())}),
    };
    Ok(
        json!({"role":"dep_custody","action":"verify","history":history,
        "installation_key":key_facts(INSTALLATION),"foreign_key":key_facts(FOREIGN)}),
    )
}

pub fn run() {
    let report = (|| -> Result<Value> {
        let control = read_line(&mut BufReader::new(io::stdin()), 65_536)?;
        let text = || {
            control["configuration"]
                .as_str()
                .ok_or("configuration text")
        };
        match control["action"].as_str() {
            Some("provision") => provision_all(text()?),
            Some("change") => change(),
            Some("verify") => verify(text()?),
            _ => Err("unknown custody action".into()),
        }
    })()
    .unwrap_or_else(|error| json!({"role":"dep_custody","error":error.to_string()}));
    let failed = report.get("error").is_some();
    emit(&report).expect("custody report output");
    if failed {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(audit: &str, keys: &str) -> String {
        json!({"listen":"127.0.0.1:8444","origin":"127.0.0.1:8080",
          "scope":{"installation":"example-installation","tenant":"t","origin":"o","revision":"r1","authority":"example.com"},
          "routes":[{"method":"POST","path":"/login","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}}],
          "audit":{"directory":audit,"node_id":"n","keys":{"directory":keys,"passphrase":"p"},"allow_new":true,"trusted_head":null},
          "owner":{"kind":"unix","uid":65532},"session_capacity":16,"session_ttl_ms":60000,"timeout_ms":5000,"rate_per_second":1000})
        .to_string()
    }

    #[test]
    fn custody_paths_must_stay_on_the_manifest_volume() {
        assert!(
            configuration(&document(
                "/var/lib/humanymous/audit",
                "/var/lib/humanymous/keys"
            ))
            .is_ok()
        );
        for (audit, keys) in [
            ("/tmp/audit", "/var/lib/humanymous/keys"),
            ("/var/lib/humanymous/audit", "/var/lib/humanymous"),
            ("/var/lib/humanymousx/audit", "/var/lib/humanymous/keys"),
        ] {
            assert!(
                configuration(&document(audit, keys)).is_err(),
                "{audit} {keys}"
            );
        }
        assert!(configuration("{\"listen\":1}").is_err());
    }
}
