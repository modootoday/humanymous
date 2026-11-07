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
    let mut git_bytes = BTreeMap::new();
    for (path, worktree) in &files {
        if [
            "crates/native-security/",
            "crates/test-support/",
            "scripts/ci/windows-native-runner/",
        ]
        .iter()
        .any(|prefix| path.starts_with(prefix))
        {
            let object = Command::new("git")
                .args(["show", &format!("HEAD:{path}")])
                .stderr(std::process::Stdio::null())
                .output()?;
            let canonical = object
                .status
                .success()
                .then(|| format!("{:x}", Sha256::digest(&object.stdout)));
            let lf_only = !fs::read(path)?.windows(2).any(|pair| pair == b"\r\n");
            git_bytes.insert(path.clone(),json!({"validated_worktree_sha256":worktree,"canonical_git_sha256":canonical,"matches_git":canonical.as_ref()==Some(worktree),"lf_only":lf_only}));
        }
    }
    Ok(
        json!({"source_sha":sha,"dirty":!dirty.stdout.is_empty(),"files":files,"github":github,"own_source_git_bytes":git_bytes}),
    )
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
pub fn disposable_host() -> bool {
    host_allowed(
        std::env::var("GITHUB_ACTIONS").ok().as_deref(),
        std::env::var("RUNNER_ENVIRONMENT").ok().as_deref(),
    )
}
pub(super) fn host_allowed(actions: Option<&str>, environment: Option<&str>) -> bool {
    actions == Some("true") && environment == Some("github-hosted")
}
/// Publish only typed counts/commitments, excluding actual SID, paths and traces.
pub fn summary(input: &Path, output: &Path) -> io::Result<()> {
    let report: Value = serde_json::from_slice(&fs::read(input)?)?;
    if report["schema"] != 1
        || report["kind"] != "native"
        || report["full_runtime_qualified"] != false
    {
        return Err(io::Error::other("scoped native input report required"));
    }
    let rows=report["coverage"]["harnesses"].as_array().map(|rows|rows.iter().map(|row|json!({"target":row["target"],"executable_sha256":row["executable_sha256"],"discovered":row["discovered"],"passed":row["passed"],"failed":row["failed"],"validated":row["validated"],"ignored":row["ignored"],"required_not_run":row["required_not_run"],"filtered":row["filtered"],"exit_code":row["exit_code"],"stdout_sha256":row["stdout_sha256"]})).collect::<Vec<_>>()).unwrap_or_default();
    let passed = rows
        .iter()
        .filter_map(|r| r["passed"].as_u64())
        .sum::<u64>();
    let ignored = rows
        .iter()
        .filter_map(|r| r["ignored"].as_u64())
        .sum::<u64>();
    let zero = rows.iter().filter(|r| r["discovered"] == 0).count();
    let failed = rows
        .iter()
        .filter_map(|r| r["failed"].as_u64())
        .sum::<u64>();
    let summary = json!({"schema":"humanymous.native-runner-summary/1","scope":"four-package-native-process-component","status":report["status"],"qualification":"component-only","full_runtime_qualified":false,"source_sha":report["source_sha"],"validated_worktree_dirty":report["source"]["dirty"],"raw_report_sha256":digest(input)?,"os":report["os"],"arch":report["arch"],"errors":report["errors"],"skips":report["skips"],"missing_required_cases":report["missing_required_cases"],"unsupported":report["unsupported"],"documentation_examples_executed":report["documentation_examples_executed"],"harnesses":rows,"passed_tests":passed,"raw_ignored_tests":ignored,"zero_test_harnesses":zero,"own_source_git_bytes":report["source"]["own_source_git_bytes"]});
    let mut summary = summary;
    summary["failed_tests"] = json!(failed);
    summary["required_harnesses"] = report["coverage"]["required_harnesses"].clone();
    summary["executed_harnesses"] = report["coverage"]["executed_harnesses"].clone();
    summary["coverage_complete"] = report["coverage"]["complete"].clone();
    write(output, &summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_summary_excludes_identity_credentials_and_raw_traces() {
        let directory = crate::fixture::Directory::new().unwrap();
        let raw = directory.path().join("raw.json");
        let public = directory.path().join("public.json");
        write(&raw,&json!({"schema":1,"kind":"native","status":"blocked","full_runtime_qualified":false,"source_sha":"a".repeat(40),"source":{"dirty":true},"actor":{"sid":"private-identity"},"credential":"private-credential","trace":"private-trace","errors":0,"skips":1,"coverage":{"harnesses":[{"target":"native_process","passed":2,"discovered":3,"ignored":1,"filtered":0}]}})).unwrap();
        summary(&raw, &public).unwrap();
        let bytes = fs::read(&public).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        for secret in ["private-identity", "private-credential", "private-trace"] {
            assert!(!text.contains(secret));
        }
        assert!(!bytes.windows(2).any(|b| b == b"\r\n"));
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["raw_report_sha256"], digest(&raw).unwrap());
        assert_eq!(value["passed_tests"], 2);
        assert_eq!(value["raw_ignored_tests"], 1);
        assert_eq!(value["status"], "blocked");
    }
}
