//! Exact committed DATA replicas; this check grants no runtime or OS authority.
use crate::{Result, require, source};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

fn producer_git(root: &Path, args: &[&str], cap: usize, deadline: Instant) -> Result<Vec<u8>> {
    let mut child = Command::new("git")
        .arg("--no-replace-objects")
        .args(args)
        .current_dir(root)
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "DATA Git start failed")?;
    let stdout = child.stdout.take().ok_or("DATA Git stdout missing")?;
    let (send, receive) = mpsc::sync_channel(1);
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout.take(cap as u64 + 1).read_to_end(&mut bytes);
        let _ = send.send(result.map(|_| bytes));
    });
    let mut bytes = None;
    let result =
        loop {
            if Instant::now() >= deadline {
                break Err("DATA Git deadline exceeded".into());
            }
            match receive.try_recv() {
                Ok(Ok(output)) if output.len() <= cap => bytes = Some(output),
                Ok(_) => break Err("DATA Git output bound/read refused".into()),
                Err(mpsc::TryRecvError::Disconnected) if bytes.is_none() => {
                    break Err("DATA Git reader failed".into());
                }
                _ => {}
            }
            match child.try_wait() {
                Ok(Some(status)) if !status.success() => break Err(
                    "DATA producer object unavailable; explicit retained-object admission required"
                        .into(),
                ),
                Ok(Some(_)) if bytes.is_some() => break Ok(bytes.take().unwrap()),
                Err(_) => break Err("DATA Git wait failed".into()),
                _ => {}
            }
            std::thread::sleep(Duration::from_millis(5));
        };
    if result.is_err() {
        let _ = child.kill();
    }
    let waited = child.wait();
    let joined = reader.join();
    require(waited.is_ok() && joined.is_ok(), "DATA Git cleanup failed")?;
    result
}

const PACKAGE: &str = "crates/apps/humanymous-cli";
const COMMIT: &str = "3137c905ac4e52b8b345fa2306af9e4ef4d42b48";
const INPUTS: &[(&str, &str)] = &[
    (
        "qualification/browser/cef/pilot-contract.json",
        "crates/browser/native/desktop/pilot-contract.json",
    ),
    (
        "qualification/fixtures/collector/pilot.json",
        "crates/collector/pilot/fixture.json",
    ),
    (
        "qualification/fixtures/core/scoring/bot-webdriver-cdp-hr9.json",
        "crates/sdk/humanymous-core/tests/scoring/fixtures/frozen/bot-webdriver-cdp-hr9.json",
    ),
    (
        "qualification/fixtures/core/scoring/go-http-client.json",
        "crates/sdk/humanymous-core/tests/scoring/fixtures/frozen/go-http-client.json",
    ),
    (
        "qualification/fixtures/core/scoring/human-chrome-allow.json",
        "crates/sdk/humanymous-core/tests/scoring/fixtures/frozen/human-chrome-allow.json",
    ),
    (
        "qualification/fixtures/core/scoring/no-client-hr10.json",
        "crates/sdk/humanymous-core/tests/scoring/fixtures/frozen/no-client-hr10.json",
    ),
    (
        "qualification/fixtures/core/scoring/selenium-hr1.json",
        "crates/sdk/humanymous-core/tests/scoring/fixtures/frozen/selenium-hr1.json",
    ),
    (
        "qualification/fixtures/core/scoring/ua-tls-h2-hr2.json",
        "crates/sdk/humanymous-core/tests/scoring/fixtures/frozen/ua-tls-h2-hr2.json",
    ),
];

fn read(root: &Path, relative: &str) -> Result<Vec<u8>> {
    require(source::safe_relative(relative), "DATA path refused")?;
    let mut path = root.to_path_buf();
    for part in Path::new(relative).components() {
        path.push(part);
        let metadata = std::fs::symlink_metadata(&path).map_err(|_| "DATA input missing")?;
        require(!metadata.file_type().is_symlink(), "DATA symlink refused")?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            require(
                metadata.file_attributes() & 0x400 == 0,
                "DATA reparse point refused",
            )?;
        }
    }
    let selected = std::fs::symlink_metadata(&path).map_err(|_| "DATA input metadata missing")?;
    require(
        selected.is_file() && selected.len() <= 1_048_576,
        "DATA file type/bound refused before open",
    )?;
    let file = std::fs::File::open(&path).map_err(|_| "DATA input open failed")?;
    let before = file.metadata().map_err(|_| "DATA metadata failed")?;
    require(
        before.is_file() && before.len() <= 1_048_576,
        "DATA file bound refused",
    )?;
    let mut bytes = Vec::new();
    (&file)
        .take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|_| "DATA read failed")?;
    let after = file.metadata().map_err(|_| "DATA metadata failed")?;
    require(
        bytes.len() as u64 == before.len()
            && before.len() == after.len()
            && before
                .modified()
                .ok()
                .zip(after.modified().ok())
                .is_some_and(|(a, b)| a == b),
        "DATA changed or exceeded bound",
    )?;
    require(
        path.canonicalize()
            .map_err(|_| "DATA canonical identity missing")?
            .starts_with(root),
        "DATA root escape",
    )?;
    Ok(bytes)
}

fn records(manifest: &Value) -> Result<Vec<&Value>> {
    require(
        manifest["schema"] == "humanymous.cli-qualification-data-provenance/1"
            && manifest["replica_count"].as_u64() == Some(8),
        "DATA provenance schema refused",
    )?;
    let rows = manifest["records"]
        .as_array()
        .ok_or("DATA records missing")?;
    require(rows.len() == INPUTS.len(), "DATA count refused")?;
    let mut seen = BTreeSet::new();
    for row in rows {
        let destination = row["destination"]
            .as_str()
            .ok_or("DATA destination missing")?;
        let producer = row["producer_source_path"]
            .as_str()
            .ok_or("DATA producer missing")?;
        require(
            INPUTS.contains(&(destination, producer)) && seen.insert(destination),
            "DATA ownership or duplicate refused",
        )?;
        require(
            row["producer_source_commit"] == COMMIT && row["license"] == "Apache-2.0",
            "DATA provenance refused",
        )?;
        require(
            row["bytes"]
                .as_u64()
                .is_some_and(|n| n > 0 && n <= 1_048_576),
            "DATA size refused",
        )?;
        for (key, length) in [("producer_git_blob", 40), ("sha256", 64)] {
            require(
                row[key].as_str().is_some_and(|v| {
                    v.len() == length
                        && v.bytes()
                            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                }),
                "DATA digest refused",
            )?;
        }
    }
    Ok(rows.iter().collect())
}

pub fn check(root: &Path) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(30);
    require(
        root.is_absolute()
            && root
                .canonicalize()
                .map_err(|_| "explicit DATA root missing")?
                == root,
        "explicit canonical DATA root required",
    )?;
    let manifest_bytes = read(
        root,
        &format!("{PACKAGE}/qualification/data-provenance.json"),
    )?;
    let manifest =
        humanymous_platform::testing::json::decode_strict_json(&manifest_bytes, 1_048_576)
            .map_err(|_| "DATA provenance JSON refused")?;
    for row in records(&manifest)? {
        let destination = row["destination"]
            .as_str()
            .ok_or("DATA destination missing")?;
        let producer = row["producer_source_path"]
            .as_str()
            .ok_or("DATA producer missing")?;
        let bytes = read(root, &format!("{PACKAGE}/{destination}"))?;
        require(
            row["bytes"].as_u64() == Some(bytes.len() as u64)
                && row["sha256"] == source::sha(&bytes),
            "DATA replica hash mismatch",
        )?;
        let object = format!("{COMMIT}:{producer}");
        let blob = String::from_utf8(producer_git(
            root,
            &["rev-parse", "--verify", &object],
            128,
            deadline,
        )?)
        .map_err(|_| "DATA Git object encoding refused")?;
        require(
            row["producer_git_blob"] == blob.trim(),
            "DATA producer blob mismatch",
        )?;
        let original = producer_git(
            root,
            &["cat-file", "blob", blob.trim()],
            bytes.len(),
            deadline,
        )?;
        require(original == bytes, "DATA producer byte mismatch")?;
    }
    Ok(
        json!({"schema":"humanymous.cli-qualification-data-check/1", "replicas":8,
        "source_commit":COMMIT,"manifest_sha256":source::sha(&manifest_bytes),
        "git_byte_equality":true,"hostile_race_resistant":false,"runtime_qualified":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn manifest() -> Value {
        json!({"schema":"humanymous.cli-qualification-data-provenance/1","replica_count":8,
            "records":INPUTS.iter().map(|(dest, producer)| json!({"destination":dest,
                "producer_source_path":producer,"producer_source_commit":COMMIT,"license":"Apache-2.0",
                "bytes":1,"producer_git_blob":"a".repeat(40),"sha256":"b".repeat(64)})).collect::<Vec<_>>()})
    }
    #[test]
    fn finite_ownership_rejects_missing_duplicate_and_retargeted_inputs() {
        let good = manifest();
        assert!(records(&good).is_ok());
        let mut missing = good.clone();
        missing["records"].as_array_mut().unwrap().pop();
        assert!(records(&missing).is_err());
        let mut duplicate = good.clone();
        duplicate["records"][1] = duplicate["records"][0].clone();
        assert!(records(&duplicate).is_err());
        for path in ["../escape", "/absolute", "qualification/unknown.json"] {
            let mut bad = good.clone();
            bad["records"][0]["destination"] = json!(path);
            assert!(records(&bad).is_err());
        }
    }
    #[test]
    fn provenance_types_hashes_license_and_source_are_required() {
        for (key, value) in [
            ("bytes", json!(0)),
            ("bytes", json!("1")),
            ("bytes", json!(1_048_577)),
            ("sha256", json!("b".repeat(63))),
            ("producer_git_blob", json!("z".repeat(40))),
            ("license", json!("unknown")),
            ("producer_source_commit", json!("a".repeat(40))),
        ] {
            let mut bad = manifest();
            bad["records"][0][key] = value;
            assert!(records(&bad).is_err());
        }
    }
}
