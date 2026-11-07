use crate::evidence;
use humanymous_test_support::process::capture_with_stderr;
use serde_json::{Value, json};
use std::{collections::BTreeMap, io, path::Path, process::Command, time::Duration};

const PACKAGES: [&str; 4] = [
    "humanymous-runtime",
    "humanymous-authority",
    "humanymous-cli",
    "humanymous-native-security",
];
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Native,
    Workspace,
}
pub fn prepare(evidence_base: &Path, binaries: &Path, scope: Scope) -> io::Result<Value> {
    let release = binaries.file_name() == Some(std::ffi::OsStr::new("release"));
    let mut build = Command::new("cargo");
    build.args(["build", "--locked", "--bins", "--message-format=json"]);
    build.args(["--features", "humanymous-cli/service-pilot"]);
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
    let products = if scope == Scope::Native {
        Some(capture_with_stderr(
            &mut build,
            b"",
            Duration::from_secs(600),
            16 * 1024 * 1024,
        )?)
    } else {
        None
    };
    if let Some(products) = &products {
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
    }
    let mut command = Command::new("cargo");
    command.args([
        "test",
        "--locked",
        "--all-targets",
        "--no-run",
        "--message-format=json",
    ]);
    if scope == Scope::Native {
        command.args(["--features", "humanymous-cli/service-pilot"]);
    }
    if release {
        command.arg("--release");
    }
    if scope == Scope::Workspace {
        command.arg("--workspace");
    } else {
        for package in PACKAGES {
            command.args(["-p", package]);
        }
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
    let products = match products {
        Some(products) => artifacts(
            std::str::from_utf8(&products.stdout).map_err(io::Error::other)?,
            false,
        )?,
        None => json!([]),
    };
    let harnesses = artifacts(
        std::str::from_utf8(&output.stdout).map_err(io::Error::other)?,
        true,
    )?;
    Ok(
        json!({"products":products,"harnesses":harnesses,"profile":if release{"release"}else{"debug"},"scope":if scope==Scope::Workspace{"workspace-default-features"}else{"native-service-feature"}}),
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
pub fn execute(
    manifest: &Value,
    evidence_base: &Path,
    progress: &mut Value,
    scope: Scope,
) -> io::Result<()> {
    let entries = manifest
        .as_array()
        .ok_or_else(|| io::Error::other("test manifest array required"))?;
    let mut rows = Vec::new();
    *progress = json!({"required_harnesses":entries.len(),"executed_harnesses":0,"harnesses":[],"complete":false,"scope":"four-package-native-process-component"});
    if scope == Scope::Workspace {
        progress["scope"] = "workspace-default-features".into();
    }
    let mut runtime = false;
    let mut authority = false;
    for (index, entry) in entries.iter().enumerate() {
        let path = Path::new(
            entry["executable"]
                .as_str()
                .ok_or_else(|| io::Error::other("executable missing"))?,
        );
        if evidence::digest(path)? != entry["sha256"] {
            return Err(io::Error::other("test executable identity mismatch"));
        }
        let list = capture_with_stderr(
            Command::new(path).args(["--list", "--format", "terse"]),
            b"",
            Duration::from_secs(30),
            1024 * 1024,
        )?;
        std::fs::write(
            evidence_base.with_extension(format!("harness-{index}-list-stdout.log")),
            &list.stdout,
        )?;
        std::fs::write(
            evidence_base.with_extension(format!("harness-{index}-list-stderr.log")),
            &list.stderr,
        )?;
        if !list.status.success() {
            return Err(io::Error::other("test enumeration failed"));
        }
        let listed = String::from_utf8(list.stdout).map_err(io::Error::other)?;
        let count = listed
            .lines()
            .filter(|line| line.ends_with(": test"))
            .count();
        let execution = capture_with_stderr(
            Command::new(path).args(["--format", "pretty"]),
            b"",
            Duration::from_secs(300),
            2 * 1024 * 1024,
        )?;
        std::fs::write(
            evidence_base.with_extension(format!("harness-{index}-run-stdout.log")),
            &execution.stdout,
        )?;
        std::fs::write(
            evidence_base.with_extension(format!("harness-{index}-run-stderr.log")),
            &execution.stderr,
        )?;
        let output = String::from_utf8(execution.stdout).map_err(io::Error::other)?;
        let package = entry["package_id"].as_str().unwrap_or_default();
        let service = scope == Scope::Native
            && entry["target"] == "service_process"
            && package.contains("humanymous-cli");
        let custody = scope == Scope::Workspace
            && entry["target"] == "key_custody"
            && entry["kind"] == json!(["test"])
            && package.contains("#humanymous-audit@");
        let captured_helper = if scope == Scope::Workspace
            && entry["kind"] == json!(["test"])
            && package.contains("#humanymous-audit@")
        {
            match entry["target"].as_str() {
                Some("durable_witness") => Some((
                    "child_witness_probe",
                    "witness_process_lifetime_and_interrupted_recovery",
                )),
                Some("key_backups") => Some((
                    "child_retire_without_pin_acknowledgement",
                    "crash_reconcile_preserves_consumed_authorization",
                )),
                _ => None,
            }
        } else {
            None
        };
        let required_service = "native_service_install_upgrade_rollback_and_remove";
        let pending = usize::from(service);
        if service
            && !output
                .lines()
                .any(|line| line.starts_with(&format!("test {required_service} ... ignored")))
        {
            return Err(io::Error::other(
                "required privileged service inventory changed",
            ));
        }
        // Retain the observed final counts even when strict summary accounting
        // rejects extra child output. `validated` still records that rejection.
        let counts = summaries(&output).ok().and_then(|s| s.last().copied());
        let accounting = if custody {
            custody_account(&output, count, execution.status.success())
        } else if let Some((helper, parent)) = captured_helper {
            captured_helper_account(&output, count, execution.status.success(), helper, parent)
        } else {
            account_pending(&output, count, execution.status.success(), pending)
        };
        rows.push(json!({"index":index,"executable_sha256":entry["sha256"],"package_id":entry["package_id"],"target":entry["target"],"discovered":count,"passed":counts.map(|c|c[0]),"failed":counts.map(|c|c[1]),"ignored":counts.map(|c|c[2]),"required_not_run":if service{json!([required_service])}else{json!([])},"filtered":counts.map(|c|c[4]),"validated":accounting.is_ok(),"exit_code":execution.status.code(),"stdout_sha256":format!("{:x}",sha2::Sha256::digest(output.as_bytes()))}));
        if custody {
            let row = rows.last_mut().unwrap();
            row["ignored_subprocess_helpers"] = json!(if accounting.is_ok() { 2 } else { 0 });
            row["subprocess_helpers"] = json!(["child_lock_probe", "child_interrupted_writer"]);
            row["required_parent_tests"] = json!([
                "competing_process_cannot_load_or_create",
                "process_exit_releases_lock_and_rotation_pending_requires_recovery"
            ]);
        }
        if let Some((helper, parent)) = captured_helper {
            let row = rows.last_mut().unwrap();
            row["ignored_subprocess_helpers"] = json!(usize::from(accounting.is_ok()));
            row["subprocess_helpers"] = json!([helper]);
            row["required_parent_tests"] = json!([parent]);
        }
        progress["executed_harnesses"] = json!(rows.len());
        progress["harnesses"] = json!(rows);
        let passed = accounting.map_err(|error|io::Error::other(format!("harness {index} target={} exit={:?} discovered={count}: {error}; inspect report-stem.harness-{index}-run-stdout.log",entry["target"],execution.status.code())))?;
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
    }
    if scope == Scope::Native && (!runtime || !authority) {
        return Err(io::Error::other(
            "actual runtime/authority native process harness required",
        ));
    }
    progress["complete"] = true.into();
    Ok(())
}
use sha2::Digest;
fn captured_helper_account(
    log: &str,
    discovered: usize,
    success: bool,
    helper: &str,
    parent: &str,
) -> io::Result<usize> {
    let ignored: Vec<_> = log
        .lines()
        .filter_map(|line| {
            line.strip_prefix("test ")
                .and_then(|s| s.split_once(" ... ignored").map(|(name, _)| name))
        })
        .collect();
    if ignored != [helper]
        || !log
            .lines()
            .any(|line| line == format!("test {parent} ... ok"))
    {
        return Err(io::Error::other(
            "captured subprocess helper/parent inventory differs",
        ));
    }
    account_pending(log, discovered, success, 1)
}
fn account_pending(
    log: &str,
    discovered: usize,
    success: bool,
    required_not_run: usize,
) -> io::Result<usize> {
    let [passed, failed, ignored, measured, filtered] = totals(log)?;
    if !success
        || passed + required_not_run != discovered
        || ignored != required_not_run
        || failed + measured + filtered != 0
    {
        return Err(io::Error::other("native manifest tests incomplete/ignored"));
    }
    Ok(passed)
}
pub(super) fn totals(log: &str) -> io::Result<[usize; 5]> {
    let summaries = summaries(log)?;
    if summaries.len() != 1 {
        return Err(io::Error::other("native test exit/summary failed"));
    }
    Ok(summaries[0])
}
fn summaries(log: &str) -> io::Result<Vec<[usize; 5]>> {
    log.lines()
        .filter(|l| l.starts_with("test result: "))
        .map(parse_summary)
        .collect()
}
fn parse_summary(line: &str) -> io::Result<[usize; 5]> {
    let fields: Vec<_> = line.split_whitespace().collect();
    if fields.len() < 15 || !matches!(fields[2], "ok." | "FAILED.") {
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
    Ok([passed, failed, ignored, measured, filtered])
}
fn custody_account(log: &str, discovered: usize, success: bool) -> io::Result<usize> {
    use std::collections::BTreeSet;
    let expected = BTreeSet::from(["child_lock_probe", "child_interrupted_writer"]);
    let ignored: Vec<_> = log
        .lines()
        .filter_map(|line| {
            line.strip_prefix("test ")
                .and_then(|s| s.split_once(" ... ignored").map(|(name, _)| name))
        })
        .collect();
    if !success
        || discovered < 2
        || ignored.len() != 2
        || ignored.iter().copied().collect::<BTreeSet<_>>() != expected
    {
        return Err(io::Error::other("custody helper inventory differs"));
    }
    for parent in [
        "competing_process_cannot_load_or_create",
        "process_exit_releases_lock_and_rotation_pending_requires_recovery",
        "child_lock_probe",
    ] {
        if !log
            .lines()
            .any(|line| line == format!("test {parent} ... ok"))
        {
            return Err(io::Error::other(
                "custody actual subprocess/parent proof absent",
            ));
        }
    }
    let counts = summaries(log)?;
    if counts != vec![[1, 0, 0, 0, discovered - 1], [discovered - 2, 0, 2, 0, 0]] {
        return Err(io::Error::other("custody nested/final summary differs"));
    }
    Ok(discovered - 2)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captured_helpers_require_single_summary_exact_helper_and_parent() {
        let good = "test helper ... ignored\ntest parent ... ok\ntest result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.1s";
        assert_eq!(
            captured_helper_account(good, 2, true, "helper", "parent").unwrap(),
            1
        );
        for bad in [
            good.replace("test parent ... ok\n", ""),
            good.replace("test helper", "test unknown"),
            format!("{good}\n{good}"),
        ] {
            assert!(captured_helper_account(&bad, 2, true, "helper", "parent").is_err());
        }
        assert!(captured_helper_account(good, 2, false, "helper", "parent").is_err());
        assert!(captured_helper_account(good, 3, true, "helper", "parent").is_err());
    }
    #[test]
    fn summary_rejects_skips_missing_tests_and_bad_exit() {
        let good = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.1s";
        assert_eq!(account_pending(good, 1, true, 0).unwrap(), 1);
        assert!(account_pending(good, 2, true, 0).is_err());
        assert!(account_pending(good, 1, false, 0).is_err());
        assert!(account_pending(&good.replace("0 ignored", "1 ignored"), 1, true, 0).is_err());
        assert!(account_pending(&format!("{good}\n{good}"), 1, true, 0).is_err());
    }
    #[test]
    fn custody_helpers_require_exact_parents_nested_proof_and_zero_unknown_skips() {
        let nested = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.1s";
        let final_summary = "test result: ok. 2 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.1s";
        let good = format!(
            "test child_lock_probe ... ok\n{nested}\ntest competing_process_cannot_load_or_create ... ok\ntest process_exit_releases_lock_and_rotation_pending_requires_recovery ... ok\ntest child_lock_probe ... ignored\ntest child_interrupted_writer ... ignored\n{final_summary}"
        );
        assert_eq!(custody_account(&good, 4, true).unwrap(), 2);
        for bad in [
            good.replace("test child_lock_probe ... ok\n", ""),
            good.replace("test competing_process_cannot_load_or_create ... ok\n", ""),
            good.replace("3 filtered out", "2 filtered out"),
            format!("{good}\ntest other ... ignored"),
            format!("{good}\n{final_summary}"),
        ] {
            assert!(custody_account(&bad, 4, true).is_err());
        }
        assert!(custody_account(&good, 4, false).is_err());
        assert!(account_pending(&good, 4, true, 0).is_err());
    }
}
