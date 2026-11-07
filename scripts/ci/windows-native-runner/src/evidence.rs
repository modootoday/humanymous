use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io, path::Path, process::Command};

pub fn digest(path: &Path) -> io::Result<String> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}
pub fn write(path: &Path, value: &Value) -> io::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes)
}
fn sources(path: &Path, files: &mut BTreeMap<String, String>) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(io::Error::other("linked runner source"));
        }
        if kind.is_dir() {
            if entry.file_name() != "target" {
                sources(&path, files)?;
            }
        } else if path
            .extension()
            .is_some_and(|e| e == "rs" || e == "toml" || e == "lock")
        {
            files.insert(path.to_string_lossy().replace('\\', "/"), digest(&path)?);
        }
    }
    Ok(())
}
pub fn source() -> io::Result<Value> {
    let head = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()?;
    if !head.status.success() || !dirty.status.success() {
        return Err(io::Error::other("source query failed"));
    }
    let sha = String::from_utf8(head.stdout)
        .map_err(io::Error::other)?
        .trim()
        .to_owned();
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
        && (!dirty.stdout.is_empty() || std::env::var("GITHUB_SHA").as_deref() != Ok(&sha))
    {
        return Err(io::Error::other(
            "remote source must be clean exact workflow SHA",
        ));
    }
    let mut files = BTreeMap::new();
    for path in ["Cargo.toml", "Cargo.lock", "docs/reviews.json"] {
        files.insert(path.to_owned(), digest(Path::new(path))?);
    }
    for path in [
        "crates/runtime/src",
        "crates/runtime/tests",
        "crates/authority/src",
        "crates/authority/tests",
        "crates/cli/src",
        "crates/native-security/src",
        "crates/test-support/src",
        "crates/audit/src",
        "crates/integrity/src",
        "crates/network/src",
        "crates/signals/src",
        "crates/scoring/src",
        "crates/behavior/src",
        "crates/model/src",
        "scripts/ci/windows-native-runner",
    ] {
        sources(Path::new(path), &mut files)?;
    }
    for entry in std::fs::read_dir("crates")? {
        let manifest = entry?.path().join("Cargo.toml");
        if manifest.is_file() {
            files.insert(
                manifest.to_string_lossy().replace('\\', "/"),
                digest(&manifest)?,
            );
        }
    }
    let github: BTreeMap<_, _> = [
        "GITHUB_RUN_ID",
        "GITHUB_RUN_ATTEMPT",
        "GITHUB_WORKFLOW_SHA",
        "GITHUB_SHA",
        "RUNNER_OS",
        "RUNNER_ARCH",
        "ImageOS",
        "ImageVersion",
    ]
    .into_iter()
    .filter_map(|key| std::env::var(key).ok().map(|value| (key, value)))
    .collect();
    Ok(json!({"source_sha":sha,"dirty":!dirty.stdout.is_empty(),"files":files,"github":github}))
}
pub fn verify_snapshot(snapshot: &Value) -> io::Result<()> {
    let files = snapshot["files"]
        .as_object()
        .ok_or_else(|| io::Error::other("source file manifest missing"))?;
    if files.is_empty() {
        return Err(io::Error::other("empty source manifest"));
    }
    for (path, expected) in files {
        if digest(Path::new(path))? != *expected {
            return Err(io::Error::other("source changed during native execution"));
        }
    }
    Ok(())
}
pub fn identity(expected: Option<&str>) -> io::Result<Value> {
    #[cfg(windows)]
    {
        let identity = humanymous_native_security::process::current_identity()?;
        if !identity.installer_allowed() || expected.is_some_and(|sid| sid != identity.sid()) {
            return Err(io::Error::other("actual standard-user SID/token required"));
        }
        Ok(
            json!({"sid":identity.sid(),"elevated":identity.elevated(),"enabled_administrator":identity.enabled_administrator()}),
        )
    }
    #[cfg(unix)]
    {
        let _ = expected;
        let uid = humanymous_native_security::process::current_uid();
        if uid == 0 {
            return Err(io::Error::other("non-root native actor required"));
        }
        Ok(json!({"uid":uid}))
    }
}
