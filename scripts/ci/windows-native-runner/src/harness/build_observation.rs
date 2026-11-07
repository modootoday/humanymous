//! Cargo artifact-event observations; never a harness count or cache-reuse proof.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{io, time::Duration};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn environment_hash(name: &str) -> Value {
    match std::env::var(name) {
        Ok(value) => json!({"present":true,"sha256":hash(value.as_bytes())}),
        Err(std::env::VarError::NotPresent) => json!({"present":false,"sha256":null}),
        Err(std::env::VarError::NotUnicode(_)) => json!({"present":true,"sha256":null}),
    }
}
pub(super) fn input_key(source: &Value, release: bool, native: bool) -> io::Result<Value> {
    let inputs = json!({"source_sha":source["source_sha"],"source_files_sha256":hash(&serde_json::to_vec(&source["files"])?),"root_lock_sha256":source["files"]["Cargo.lock"],"runner_lock_sha256":source["files"]["scripts/ci/windows-native-runner/Cargo.lock"],"declared_toolchain_sha256":source["files"]["rust-toolchain.toml"],"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"host_abi":if cfg!(target_env="msvc"){"msvc"}else if cfg!(target_env="gnu"){"gnu"}else if cfg!(target_env="musl"){"musl"}else{"unspecified"},"profile":if release{"release"}else{"debug"},"features":if native{json!(crate::service::catalog::FEATURES)}else{json!([])},"rustflags":environment_hash("RUSTFLAGS"),"encoded_rustflags":environment_hash("CARGO_ENCODED_RUSTFLAGS"),"toolchain_override":environment_hash("RUSTUP_TOOLCHAIN"),"target_override":environment_hash("CARGO_BUILD_TARGET")});
    Ok(
        json!({"sha256":hash(&serde_json::to_vec(&inputs)?),"inputs":inputs,"actual_rustc_version":"not-observed","input_key_complete":false,"cache_reuse_proven":false}),
    )
}
pub(super) fn events(messages: &str, phase: &str, elapsed: Duration) -> io::Result<Value> {
    let (mut count, mut fresh, mut rebuilt, mut unknown) = (0u64, 0u64, 0u64, 0u64);
    for line in messages.lines() {
        let row: Value = serde_json::from_str(line)?;
        if row["reason"] != "compiler-artifact" {
            continue;
        }
        count += 1;
        match row["fresh"].as_bool() {
            Some(true) => fresh += 1,
            Some(false) => rebuilt += 1,
            None => unknown += 1,
        }
    }
    Ok(
        json!({"phase":phase,"compiler_artifact_events":count,"fresh":fresh,"rebuilt":rebuilt,"unknown":unknown,"cargo_child_elapsed_ms":u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)}),
    )
}
fn digest(value: &Value) -> Option<&str> {
    value
        .as_str()
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
}
/// Project only counters, known labels and digests; never raw Cargo paths or flags.
pub(crate) fn project(raw: &Value) -> Value {
    let input = &raw["input_key"]["inputs"];
    let mut fingerprints = serde_json::Map::new();
    for key in [
        "rustflags",
        "encoded_rustflags",
        "toolchain_override",
        "target_override",
    ] {
        fingerprints.insert(key.to_owned(),json!({"present":input[key]["present"].as_bool(),"sha256":digest(&input[key]["sha256"])}));
    }
    let rows=raw["commands"].as_array().map(|rows|rows.iter().map(|row|json!({"phase":match row["phase"].as_str(){Some("product-binaries")=>Some("product-binaries"),Some("test-discovery")=>Some("test-discovery"),_=>None},"compiler_artifact_events":row["compiler_artifact_events"].as_u64(),"fresh":row["fresh"].as_u64(),"rebuilt":row["rebuilt"].as_u64(),"unknown":row["unknown"].as_u64(),"cargo_child_elapsed_ms":row["cargo_child_elapsed_ms"].as_u64()})).collect::<Vec<_>>()).unwrap_or_default();
    let mut result = json!({"input_key_sha256":digest(&raw["input_key"]["sha256"]),"source_files_sha256":digest(&input["source_files_sha256"]),"root_lock_sha256":digest(&input["root_lock_sha256"]),"runner_lock_sha256":digest(&input["runner_lock_sha256"]),"declared_toolchain_sha256":digest(&input["declared_toolchain_sha256"]),"profile":match input["profile"].as_str(){Some("debug")=>Some("debug"),Some("release")=>Some("release"),_=>None},"fingerprints":fingerprints,"commands":rows,"input_key_complete":false,"cache_reuse_proven":false,"html_timings_requested":true,"html_timings_retrieved":false});
    result["features"] = if input["features"] == json!([]) {
        json!([])
    } else if input["features"] == json!(crate::service::catalog::FEATURES) {
        json!(crate::service::catalog::FEATURES)
    } else {
        Value::Null
    };
    result["host_abi"] = match input["host_abi"].as_str() {
        Some("msvc") => json!("msvc"),
        Some("gnu") => json!("gnu"),
        Some("musl") => json!("musl"),
        Some("unspecified") => json!("unspecified"),
        _ => Value::Null,
    };
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fresh_counts_use_artifact_events_without_dropping_unknowns() {
        let messages = "{\"reason\":\"compiler-artifact\",\"fresh\":true}\n{\"reason\":\"compiler-artifact\",\"fresh\":false}\n{\"reason\":\"compiler-artifact\"}\n{\"reason\":\"build-finished\",\"success\":true}";
        let row = events(messages, "test-discovery", Duration::from_millis(7)).unwrap();
        assert_eq!(row["compiler_artifact_events"], 3);
        assert_eq!(row["fresh"], 1);
        assert_eq!(row["rebuilt"], 1);
        assert_eq!(row["unknown"], 1);
        assert_eq!(row["cargo_child_elapsed_ms"], 7);
        assert!(events("not JSON", "test-discovery", Duration::ZERO).is_err());
    }
    #[test]
    fn projection_excludes_private_paths_flags_and_untrusted_labels() {
        let raw = json!({"input_key":{"sha256":"a".repeat(64),"inputs":{"rustflags":"PRIVATE-FLAGS"}},"commands":[{"phase":"PRIVATE-PATH","fresh":1,"rebuilt":0,"unknown":0,"compiler_artifact_events":1,"cargo_child_elapsed_ms":3,"executable":"PRIVATE-EXE"}]});
        let output = project(&raw);
        assert!(!output.to_string().contains("PRIVATE"));
        assert_eq!(output["commands"][0]["fresh"], 1);
    }
}
