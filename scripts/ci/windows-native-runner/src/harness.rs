use crate::evidence;
use humanymous_test_support::process::{capture, capture_with_stderr};
use serde_json::{Value, json};
use std::{collections::BTreeMap, io, path::Path, process::Command, time::Duration};

const PACKAGES: [&str; 4] = [
    "humanymous-runtime",
    "humanymous-authority",
    "humanymous-cli",
    "humanymous-native-security",
];
pub fn prepare(evidence_base: &Path, binaries: &Path) -> io::Result<Value> {
    let release = binaries.file_name() == Some(std::ffi::OsStr::new("release"));
    let mut build = Command::new("cargo");
    build.args(["build", "--locked", "--bins", "--message-format=json"]);
    if release {
        build.arg("--release");
    }
    for package in [
        "humanymous-runtime",
        "humanymous-authority",
        "humanymous-cli",
    ] {
        build.args(["-p", package]);
    }
    let products =
        capture_with_stderr(&mut build, b"", Duration::from_secs(600), 16 * 1024 * 1024)?;
    std::fs::write(
        evidence_base.with_extension("build-stderr.log"),
        &products.stderr,
    )?;
    std::fs::write(
        evidence_base.with_extension("build-messages.jsonl"),
        &products.stdout,
    )?;
    if !products.status.success() {
        return Err(io::Error::other("native product compilation failed"));
    }
    let mut command = Command::new("cargo");
    command.args([
        "test",
        "--locked",
        "--all-targets",
        "--no-run",
        "--message-format=json",
    ]);
    if release {
        command.arg("--release");
    }
    for package in PACKAGES {
        command.args(["-p", package]);
    }
    let output = capture_with_stderr(
        &mut command,
        b"",
        Duration::from_secs(600),
        16 * 1024 * 1024,
    )?;
    std::fs::write(
        evidence_base.with_extension("tests-stderr.log"),
        &output.stderr,
    )?;
    std::fs::write(
        evidence_base.with_extension("tests-messages.jsonl"),
        &output.stdout,
    )?;
    if !output.status.success() {
        return Err(io::Error::other("native pilot compilation failed"));
    }
    let products = artifacts(
        std::str::from_utf8(&products.stdout).map_err(io::Error::other)?,
        false,
    )?;
    let harnesses = artifacts(
        std::str::from_utf8(&output.stdout).map_err(io::Error::other)?,
        true,
    )?;
    Ok(
        json!({"products":products,"harnesses":harnesses,"profile":if release{"release"}else{"debug"}}),
    )
}
fn artifacts(messages: &str, test: bool) -> io::Result<Value> {
    let mut entries = BTreeMap::new();
    for line in messages.lines() {
        let row: Value = serde_json::from_str(line)?;
        if row["reason"] != "compiler-artifact" || row["profile"]["test"] != test {
            continue;
        }
        if let Some(exe) = row["executable"].as_str() {
            let path = Path::new(exe).canonicalize()?;
            entries.insert(path.clone(),json!({"executable":path,"sha256":evidence::digest(&path)?,"package_id":row["package_id"],"target":row["target"]["name"],"kind":row["target"]["kind"]}));
        }
    }
    if entries.is_empty() {
        return Err(io::Error::other("empty native test manifest"));
    }
    Ok(Value::Array(entries.into_values().collect()))
}
pub fn execute(manifest: &Value) -> io::Result<Value> {
    let entries = manifest
        .as_array()
        .ok_or_else(|| io::Error::other("test manifest array required"))?;
    let mut rows = Vec::new();
    let mut runtime = false;
    let mut authority = false;
    for entry in entries {
        let path = Path::new(
            entry["executable"]
                .as_str()
                .ok_or_else(|| io::Error::other("executable missing"))?,
        );
        if evidence::digest(path)? != entry["sha256"] {
            return Err(io::Error::other("test executable identity mismatch"));
        }
        let list = capture(
            Command::new(path).args(["--list", "--format", "terse"]),
            b"",
            Duration::from_secs(30),
            1024 * 1024,
        )?;
        if !list.status.success() {
            return Err(io::Error::other("test enumeration failed"));
        }
        let listed = String::from_utf8(list.stdout).map_err(io::Error::other)?;
        let count = listed
            .lines()
            .filter(|line| line.ends_with(": test"))
            .count();
        let execution = capture(
            Command::new(path).args(["--format", "pretty"]),
            b"",
            Duration::from_secs(300),
            2 * 1024 * 1024,
        )?;
        let output = String::from_utf8(execution.stdout).map_err(io::Error::other)?;
        let passed = account(&output, count, execution.status.success())?;
        let package = entry["package_id"].as_str().unwrap_or_default();
        if entry["target"] == "native_process" {
            let required: &[&str] = if package.contains("humanymous-runtime") {
                &["native_binary_session_collection_origin_restart_and_wal_authentication"]
            } else if package.contains("humanymous-authority") {
                &[
                    "real_ipc_auth_replay_body_scope_capture_and_restart_epoch",
                    "concurrent_nonce_reservation_and_durable_failure",
                ]
            } else {
                &[]
            };
            for name in required {
                if !listed.lines().any(|line| line == format!("{name}: test"))
                    || !output
                        .lines()
                        .any(|line| line == format!("test {name} ... ok"))
                {
                    return Err(io::Error::other("required actual-process assertion absent"));
                }
            }
            runtime |= package.contains("humanymous-runtime") && passed > 0;
            authority |= package.contains("humanymous-authority") && passed > 0;
        }
        rows.push(json!({"executable_sha256":entry["sha256"],"package_id":entry["package_id"],"target":entry["target"],"discovered":count,"passed":passed,"ignored":0,"filtered":0,"exit_code":execution.status.code(),"stdout_sha256":format!("{:x}",sha2::Sha256::digest(output.as_bytes()))}));
    }
    if !runtime || !authority {
        return Err(io::Error::other(
            "actual runtime/authority native process harness required",
        ));
    }
    Ok(
        json!({"required_harnesses":entries.len(),"executed_harnesses":rows.len(),"harnesses":rows,"scope":"four-package-native-process-component"}),
    )
}
use sha2::Digest;
fn account(log: &str, discovered: usize, success: bool) -> io::Result<usize> {
    let summaries: Vec<_> = log
        .lines()
        .filter(|l| l.starts_with("test result: "))
        .collect();
    if !success || summaries.len() != 1 {
        return Err(io::Error::other("native test exit/summary failed"));
    }
    let fields: Vec<_> = summaries[0].split_whitespace().collect();
    if fields.len() < 15 || fields[2] != "ok." {
        return Err(io::Error::other("native test summary invalid"));
    }
    let numbers = [fields[3], fields[5], fields[7], fields[9], fields[11]].map(str::parse::<usize>);
    let [
        Ok(passed),
        Ok(failed),
        Ok(ignored),
        Ok(measured),
        Ok(filtered),
    ] = numbers
    else {
        return Err(io::Error::other("native test summary counts invalid"));
    };
    if passed != discovered || failed + ignored + measured + filtered != 0 {
        return Err(io::Error::other("native manifest tests incomplete/ignored"));
    }
    Ok(passed)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn summary_rejects_skips_missing_tests_and_bad_exit() {
        let good = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.1s";
        assert_eq!(account(good, 1, true).unwrap(), 1);
        assert!(account(good, 2, true).is_err());
        assert!(account(good, 1, false).is_err());
        assert!(account(&good.replace("0 ignored", "1 ignored"), 1, true).is_err());
        assert!(account(&format!("{good}\n{good}"), 1, true).is_err());
    }
}
