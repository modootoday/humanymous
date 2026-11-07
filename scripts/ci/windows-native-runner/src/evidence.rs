use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io, path::Path, process::Command};
mod accounting;
mod coverage;
mod failure;
mod lifecycle;
pub(crate) mod recovery;
pub(crate) use failure::record as record_initial_failure;

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
pub fn source(workspace: bool) -> io::Result<Value> {
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
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "docs/reviews.json",
    ] {
        files.insert(path.to_owned(), digest(Path::new(path))?);
    }
    let directories = &[
        "crates/runtime/src",
        "crates/runtime/tests",
        "crates/authority/src",
        "crates/authority/tests",
        "crates/cli/src",
        "crates/native-security/src",
        "crates/test-support/src",
        "crates/audit/src",
        "crates/sdk/humanymous-core/src",
        "crates/network/src",
        "scripts/ci/windows-native-runner",
    ];
    if workspace {
        let metadata = Command::new("cargo")
            .args(["metadata", "--locked", "--no-deps", "--format-version", "1"])
            .output()?;
        if !metadata.status.success() {
            return Err(io::Error::other("workspace source inventory unavailable"));
        }
        let metadata: Value = serde_json::from_slice(&metadata.stdout)?;
        let members = metadata["workspace_members"]
            .as_array()
            .ok_or_else(|| io::Error::other("workspace members absent"))?;
        let packages = metadata["packages"]
            .as_array()
            .ok_or_else(|| io::Error::other("workspace packages absent"))?;
        let root = std::env::current_dir()?;
        for package in packages.iter().filter(|p| members.contains(&p["id"])) {
            let manifest = Path::new(
                package["manifest_path"]
                    .as_str()
                    .ok_or_else(|| io::Error::other("workspace manifest absent"))?,
            );
            let directory = manifest
                .parent()
                .and_then(|p| p.strip_prefix(&root).ok())
                .ok_or_else(|| io::Error::other("workspace member outside checkout"))?;
            sources(directory, &mut files)?;
        }
        sources(Path::new("scripts/ci/windows-native-runner"), &mut files)?;
    } else {
        for path in directories {
            sources(Path::new(path), &mut files)?;
        }
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
    if let Some(os) = snapshot["github"]["RUNNER_OS"].as_str() {
        let expected = match os.to_ascii_lowercase().as_str() {
            "windows" => "windows",
            "linux" => "linux",
            "macos" => "macos",
            _ => return Err(io::Error::other("unsupported observed native OS")),
        };
        if expected != std::env::consts::OS {
            return Err(io::Error::other(
                "native executable OS differs from required runner tuple",
            ));
        }
    }
    if let Some(arch) = snapshot["github"]["RUNNER_ARCH"].as_str() {
        let expected = match arch.to_ascii_uppercase().as_str() {
            "X64" => "x86_64",
            "ARM64" => "aarch64",
            _ => {
                return Err(io::Error::other(
                    "unsupported observed native runner architecture",
                ));
            }
        };
        if expected != std::env::consts::ARCH {
            return Err(io::Error::other(
                "native executable architecture differs from required runner tuple",
            ));
        }
    }
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
        std::env::var_os("ACT").as_deref(),
    )
}
pub(super) fn host_allowed(
    actions: Option<&str>,
    environment: Option<&str>,
    act: Option<&std::ffi::OsStr>,
) -> bool {
    actions == Some("true") && environment == Some("github-hosted") && act.is_none()
}
/// Publish only typed counts/commitments, excluding actual SID, paths and traces.
pub fn summary(input: &Path, output: &Path) -> io::Result<()> {
    let report: Value = serde_json::from_slice(&fs::read(input)?)?;
    let workspace =
        report["schema"] == "humanymous.workspace-probe/1" && report["kind"] == "workspace";
    let native = report["schema"] == 1 && report["kind"] == "native";
    if (!workspace && !native) || report["full_runtime_qualified"] != false {
        return Err(io::Error::other("scoped native input report required"));
    }
    let rows=report["coverage"]["harnesses"].as_array().map(|rows|rows.iter().map(|row|json!({"target":row["target"],"executable_sha256":row["executable_sha256"],"discovered":row["discovered"],"passed":row["passed"],"failed":row["failed"],"validated":row["validated"],"ignored":row["ignored"],"ignored_subprocess_helpers":row["ignored_subprocess_helpers"],"subprocess_helpers":row["subprocess_helpers"],"required_parent_tests":row["required_parent_tests"],"required_not_run":row["required_not_run"],"filtered":row["filtered"],"exit_code":row["exit_code"],"stdout_sha256":row["stdout_sha256"]})).collect::<Vec<_>>()).unwrap_or_default();
    let mut rows = rows;
    for (index, row) in rows.iter_mut().enumerate() {
        coverage::row(&report["coverage"]["harnesses"][index], row);
        row["unsupported_required_case"] =
            json!(report["coverage"]["harnesses"][index]["unsupported_required_case"].as_bool());
    }
    let passed = accounting::observed_total(&rows, "passed")?;
    let ignored = accounting::observed_total(&rows, "ignored")?;
    let zero = rows.iter().filter(|r| r["discovered"] == 0).count();
    let failed = accounting::observed_total(&rows, "failed")?;
    let summary = json!({"schema":"humanymous.native-runner-summary/1","scope":"four-package-native-process-component","status":report["status"],"qualification":"component-only","full_runtime_qualified":false,"source_sha":report["source_sha"],"validated_worktree_dirty":report["source"]["dirty"],"raw_report_sha256":digest(input)?,"os":report["os"],"arch":report["arch"],"errors":report["errors"],"skips":report["skips"],"missing_required_cases":report["missing_required_cases"],"unsupported":report["unsupported"],"documentation_examples_executed":report["documentation_examples_executed"],"harnesses":rows,"passed_tests":passed,"raw_ignored_tests":ignored,"zero_test_harnesses":zero,"own_source_git_bytes":report["source"]["own_source_git_bytes"]});
    let mut summary = summary;
    let unknown_counts = rows
        .iter()
        .filter(|row| {
            ["passed", "failed", "ignored", "filtered"]
                .iter()
                .any(|key| !row[*key].is_u64())
        })
        .count();
    summary["unknown_count_harnesses"] = json!(unknown_counts);
    summary["counts_complete"] =
        json!(unknown_counts == 0 && report["coverage"]["execution_complete"] == true);
    summary["failed_tests"] = json!(failed);
    summary["required_harnesses"] = report["coverage"]["required_harnesses"].clone();
    summary["executed_harnesses"] = report["coverage"]["executed_harnesses"].clone();
    summary["coverage_complete"] = report["coverage"]["complete"].clone();
    summary["failures"] = coverage::failures(&report["coverage"]["failures"]);
    summary["execution_complete"] = json!(report["coverage"]["execution_complete"].as_bool());
    summary["fatal_stop"] = json!(report["coverage"]["fatal_stop"].as_bool());
    summary["recovered_after_missing_final_report"] =
        json!(report["coverage"]["recovered_after_missing_final_report"].as_bool());
    if report["initialization_failure"]["reported"] == true {
        summary["initialization_failure"] = json!({"reported":true,"os_error_code":report["initialization_failure"]["os_error_code"].as_i64()});
    }
    if workspace {
        if report["status"] == "passed" {
            accounting::workspace_pass(&report)?;
        }
        summary["schema"] = "humanymous.workspace-probe-summary/1".into();
        summary["scope"] = "workspace-default-features".into();
        summary["qualification"] = "implemented-workspace-component".into();
        summary["ignored_subprocess_helpers"] = report["ignored_subprocess_helpers"].clone();
    }
    summary["runner_provenance"] = report["source"]["github"].clone();
    lifecycle::project(&report, &mut summary);
    if report["manifest"].get("build_observation").is_some() {
        summary["build_observation"] =
            crate::harness::build_observation::project(&report["manifest"]["build_observation"]);
    }
    summary["source_manifest_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&report["source"]["files"])?)
    ));
    write(output, &summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mutation_guard_rejects_any_act_marker_and_malformed_host_labels() {
        use std::ffi::OsStr;
        assert!(host_allowed(Some("true"), Some("github-hosted"), None));
        for marker in ["", "true", "false", "0", "invalid"] {
            assert!(!host_allowed(
                Some("true"),
                Some("github-hosted"),
                Some(OsStr::new(marker))
            ));
        }
        for (actions, environment) in [
            (None, Some("github-hosted")),
            (Some("TRUE"), Some("github-hosted")),
            (Some("true "), Some("github-hosted")),
            (Some("true"), None),
            (Some("true"), Some("self-hosted")),
            (Some("true"), Some("github-hosted ")),
        ] {
            assert!(!host_allowed(actions, environment, None));
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            let malformed = std::ffi::OsString::from_wide(&[0xd800]);
            assert!(!host_allowed(
                Some("true"),
                Some("github-hosted"),
                Some(&malformed)
            ));
        }
    }
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
        let mut workspace: Value = serde_json::from_slice(&fs::read(&raw).unwrap()).unwrap();
        workspace["schema"] = "humanymous.workspace-probe/1".into();
        workspace["kind"] = "workspace".into();
        workspace["status"] = "passed".into();
        workspace["skips"] = 0.into();
        write(&raw, &workspace).unwrap();
        assert!(summary(&raw, &public).is_err());
        workspace["coverage"]["complete"] = true.into();
        workspace["coverage"]["required_harnesses"] = 1.into();
        workspace["coverage"]["executed_harnesses"] = 1.into();
        workspace["ignored_subprocess_helpers"] = 0.into();
        workspace["coverage"]["harnesses"][0]["ignored"] = 0.into();
        workspace["coverage"]["harnesses"][0]["validated"] = true.into();
        workspace["coverage"]["harnesses"][0]["failed"] = 0.into();
        workspace["coverage"]["harnesses"][0]["exit_code"] = 0.into();
        workspace["coverage"]["harnesses"][0]["discovered"] = 2.into();
        write(&raw, &workspace).unwrap();
        summary(&raw, &public).unwrap();
        let published: Value = serde_json::from_slice(&fs::read(&public).unwrap()).unwrap();
        assert_eq!(published["schema"], "humanymous.workspace-probe-summary/1");
        assert_eq!(published["full_runtime_qualified"], false);
    }
}
