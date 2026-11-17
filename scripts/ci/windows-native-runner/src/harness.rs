use crate::evidence;
use humanymous_platform::testing::process::capture_with_stderr;
use serde_json::{Value, json};
use std::{collections::BTreeMap, io, path::Path, process::Command, time::Duration};
pub(crate) mod build_observation;
mod collection;
mod helpers;
pub(crate) mod packages;
mod pilots;

const PACKAGES: [&str; 2] = ["humanymous-runtime", "humanymous-cli"];
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Native,
    Workspace,
}
pub fn prepare(
    evidence_base: &Path,
    binaries: &Path,
    scope: Scope,
    source: &Value,
) -> io::Result<Value> {
    let release = binaries.file_name() == Some(std::ffi::OsStr::new("release"));
    let mut observation = json!({"input_key":build_observation::input_key(source,release,scope==Scope::Native)?,"commands":[],"html_timings_retrieved":false});
    let mut build = Command::new("cargo");
    build.args(["build", "--locked", "--bins", "--message-format=json"]);
    build.arg("--timings");
    build.args(["--features", &crate::service::catalog::FEATURES.join(",")]);
    if release {
        build.arg("--release");
    }
    for package in ["humanymous-runtime", "humanymous-cli"] {
        build.args(["-p", package]);
    }
    let products = if scope == Scope::Native {
        let started = std::time::Instant::now();
        let result =
            capture_with_stderr(&mut build, b"", Duration::from_secs(600), 16 * 1024 * 1024)?;
        let elapsed = started.elapsed();
        std::fs::write(
            evidence_base.with_extension("build-stderr.log"),
            &result.stderr,
        )?;
        std::fs::write(
            evidence_base.with_extension("build-messages.jsonl"),
            &result.stdout,
        )?;
        observation["commands"]
            .as_array_mut()
            .unwrap()
            .push(build_observation::events(
                std::str::from_utf8(&result.stdout).map_err(io::Error::other)?,
                "product-binaries",
                elapsed,
            )?);
        evidence::write(
            &evidence_base.with_extension("build-observation.json"),
            &observation,
        )?;
        Some(result)
    } else {
        None
    };
    if let Some(products) = &products
        && !products.status.success()
    {
        return Err(io::Error::other("native product compilation failed"));
    }
    let mut command = Command::new("cargo");
    command.args([
        "test",
        "--locked",
        "--all-targets",
        "--no-run",
        "--message-format=json",
        "--timings",
    ]);
    if scope == Scope::Native {
        command.args(["--features", &crate::service::catalog::FEATURES.join(",")]);
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
    let started = std::time::Instant::now();
    let output = capture_with_stderr(
        &mut command,
        b"",
        Duration::from_secs(600),
        16 * 1024 * 1024,
    )?;
    let elapsed = started.elapsed();
    std::fs::write(
        evidence_base.with_extension("tests-stderr.log"),
        &output.stderr,
    )?;
    std::fs::write(
        evidence_base.with_extension("tests-messages.jsonl"),
        &output.stdout,
    )?;
    observation["commands"]
        .as_array_mut()
        .unwrap()
        .push(build_observation::events(
            std::str::from_utf8(&output.stdout).map_err(io::Error::other)?,
            "test-discovery",
            elapsed,
        )?);
    evidence::write(
        &evidence_base.with_extension("build-observation.json"),
        &observation,
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
        json!({"products":products,"harnesses":harnesses,"build_observation":observation,"profile":if release{"release"}else{"debug"},"scope":if scope==Scope::Workspace{"workspace-default-features"}else{"native-service-feature"}}),
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
pub(crate) fn execute(
    manifest: &Value,
    evidence_base: &Path,
    progress: &mut Value,
    scope: Scope,
    packages: &packages::Packages,
) -> io::Result<()> {
    let required = manifest
        .as_array()
        .ok_or_else(|| io::Error::other("test manifest array required"))?
        .len();
    let mut ledger = collection::Ledger::new(required);
    let result = execute_inner(
        manifest,
        evidence_base,
        progress,
        scope,
        &mut ledger,
        packages,
    );
    if let Err(error) = result {
        ledger.abort();
        ledger.snapshot(progress);
        let _ = save_progress(evidence_base, progress);
        return Err(error);
    }
    let result = ledger.finish(progress);
    save_progress(evidence_base, progress)?;
    result
}
fn save_progress(base: &Path, progress: &Value) -> io::Result<()> {
    let pending = base.with_extension("coverage.pending.json");
    evidence::write(&pending, progress)?;
    std::fs::rename(pending, base.with_extension("coverage.json"))
}
fn execute_inner(
    manifest: &Value,
    evidence_base: &Path,
    progress: &mut Value,
    scope: Scope,
    ledger: &mut collection::Ledger,
    packages: &packages::Packages,
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
        progress["active_harness"] = json!({"index":index,"target":entry["target"]});
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
        let service_case = if scope == Scope::Native {
            crate::service::catalog::required_case(entry, packages)
        } else {
            None
        };
        let service = service_case.is_some();
        let custody = scope == Scope::Workspace
            && entry["target"] == "audit-key-custody"
            && entry["kind"] == json!(["test"])
            && packages.matches(entry, packages::Role::AuditKeyCustody);
        let captured_helper = if scope == Scope::Workspace {
            helpers::captured(entry, packages)
        } else {
            None
        };
        let required_service = service_case.map_or("", |case| case.test);
        let service_implemented = service_case.is_some_and(|case| case.implemented_for_target());
        let pending = usize::from(service_implemented);
        if service_implemented
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
        let declared = if scope == Scope::Workspace {
            pilots::declared(
                entry,
                packages,
                std::env::consts::OS,
                std::env::consts::ARCH,
            )?
        } else {
            pilots::Declared::default()
        };
        let accounting = if custody {
            custody_account(&output, count, execution.status.success())
        } else if let Some((helper, parent)) = captured_helper {
            captured_helper_account(
                &output,
                count,
                execution.status.success(),
                helper,
                parent,
                &declared.ignored,
            )
        } else if !declared.is_empty() {
            declared_account(
                &output,
                count,
                execution.status.success(),
                &declared.ignored,
            )
        } else {
            account_pending(&output, count, execution.status.success(), pending)
        };
        let receipt =
            std::fs::read_to_string(packages.root().join("target/native-pilots/receipt.json")).ok();
        let pilot_receipt = pilots::verify_receipt(
            &declared,
            packages.source_sha(),
            receipt.as_deref(),
            std::env::consts::OS,
            std::env::consts::ARCH,
        );
        let pilot_receipt_verified = pilot_receipt.is_ok();
        let accounting = accounting.and_then(|passed| pilot_receipt.map(|()| passed));
        rows.push(json!({"index":index,"executable_sha256":entry["sha256"],"package_id":entry["package_id"],"target":entry["target"],"discovered":count,"passed":counts.map(|c|c[0]),"failed":counts.map(|c|c[1]),"ignored":counts.map(|c|c[2]),"required_not_run":if service{json!([required_service])}else{json!([])},"filtered":counts.map(|c|c[4]),"validated":accounting.is_ok(),"exit_code":execution.status.code(),"stdout_sha256":format!("{:x}",sha2::Sha256::digest(output.as_bytes())),"failed_test_names":failed_test_names(&output),"panic_locations":panic_locations(&output,&execution.stderr)}));
        if custody {
            let row = rows.last_mut().unwrap();
            row["ignored_subprocess_helpers"] = json!(if accounting.is_ok() { 2 } else { 0 });
            row["subprocess_helpers"] = json!(["child_lock_probe", "child_interrupted_writer"]);
            row["required_parent_tests"] = json!([
                "competing_process_cannot_load_or_create",
                "process_exit_releases_lock_and_rotation_pending_requires_recovery"
            ]);
        }
        if service && !service_implemented {
            rows.last_mut().unwrap()["unsupported_required_case"] = json!(true);
        }
        if let Some((helper, parent)) = captured_helper {
            let row = rows.last_mut().unwrap();
            row["ignored_subprocess_helpers"] = json!(usize::from(accounting.is_ok()));
            row["subprocess_helpers"] = json!([helper]);
            row["required_parent_tests"] = json!(parent);
        }
        if !declared.is_empty() {
            let row = declared.row(pilot_receipt_verified);
            for (key, value) in row.as_object().unwrap() {
                rows.last_mut().unwrap()[key] = value.clone();
            }
        }
        progress["executed_harnesses"] = json!(rows.len());
        progress["harnesses"] = json!(rows);
        let mut required_missing = false;
        if packages.matches(entry, packages::Role::GatewayNative)
            || packages.matches(entry, packages::Role::AuthorityNative)
        {
            let required: &[&str] = if packages.matches(entry, packages::Role::GatewayNative) {
                &["native_binary_session_collection_origin_restart_and_wal_authentication"]
            } else if packages.matches(entry, packages::Role::AuthorityNative) {
                &[
                    "real_ipc_auth_replay_body_scope_capture_and_restart_epoch",
                    "concurrent_nonce_reservation_and_durable_failure",
                    "normalized_producers_are_bound_by_native_authority_credentials",
                    "sealed_startup_retains_custody_and_reuses_existing_identity",
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
                    required_missing = true;
                }
            }
            rows.last_mut().unwrap()["required_native_tests"] = json!(required);
            rows.last_mut().unwrap()["required_native_tests_verified"] = json!(!required_missing);
            runtime |= packages.matches(entry, packages::Role::GatewayNative)
                && counts.is_some_and(|c| c[0] > 0)
                && execution.status.success()
                && !required_missing;
            authority |= packages.matches(entry, packages::Role::AuthorityNative)
                && counts.is_some_and(|c| c[0] > 0)
                && execution.status.success()
                && !required_missing;
        }
        match accounting {
            Ok(_) => {}
            Err(error) => {
                rows.last_mut().unwrap()["accounting_error"] = json!(error.to_string());
                ledger.completed(index, &entry["target"], Some("coverage-accounting"));
                progress["harnesses"] = json!(rows);
                ledger.snapshot(progress);
                save_progress(evidence_base, progress)?;
                continue;
            }
        };
        if required_missing {
            rows.last_mut().unwrap()["validated"] = json!(false);
        }
        ledger.completed(
            index,
            &entry["target"],
            required_missing.then_some("required-native-assertion"),
        );
        progress["harnesses"] = json!(rows);
        ledger.snapshot(progress);
        save_progress(evidence_base, progress)?;
    }
    if scope == Scope::Native && (!runtime || !authority) {
        ledger.missing_native();
    }
    progress["active_harness"] = Value::Null;
    Ok(())
}
use sha2::Digest;
/// Names of failed tests from libtest's own result lines, bounded. The public
/// projection re-filters them to identifier characters only.
fn failed_test_names(log: &str) -> Vec<&str> {
    log.lines()
        .filter_map(|line| line.strip_prefix("test ")?.strip_suffix(" ... FAILED"))
        .take(16)
        .collect()
}
/// Source locations of test panics from libtest's own "panicked at" lines,
/// bounded. libtest prints a captured test's panic in its stdout section, and
/// an uncaptured one on stderr, so both are read. The public projection admits
/// only repository-relative paths.
fn panic_locations(stdout: &str, stderr: &[u8]) -> Vec<String> {
    let stderr = String::from_utf8_lossy(stderr);
    stdout
        .lines()
        .chain(stderr.lines())
        .filter_map(|line| line.split_once(" panicked at ")?.1.strip_suffix(':'))
        .take(16)
        // Windows prints `crates\x\src\a.rs:1:1`; the public filter accepts
        // only relative `/`-separated source positions.
        .map(|location| location.replace('\\', "/"))
        .collect()
}
fn captured_helper_account(
    log: &str,
    discovered: usize,
    success: bool,
    helper: &str,
    parents: &[&str],
    declared: &[String],
) -> io::Result<usize> {
    let mut expected = vec![helper.to_owned()];
    expected.extend(declared.iter().cloned());
    if (helper == "audit::storage::wal::tests::process_lock_probe"
        && log
            .lines()
            .any(|line| line == "humanymous-wal-lock-refused"))
        || !pilots::ignored_matches(log, &expected)
        || parents.is_empty()
        || parents.iter().any(|parent| {
            !log.lines()
                .any(|line| line == format!("test {parent} ... ok"))
        })
    {
        return Err(io::Error::other(
            "captured subprocess helper/parent inventory differs",
        ));
    }
    account_pending(log, discovered, success, expected.len())
}
/// Ignored names must equal the declared pilot names exactly; the receipt or
/// named job step is checked separately.
fn declared_account(
    log: &str,
    discovered: usize,
    success: bool,
    declared: &[String],
) -> io::Result<usize> {
    if !pilots::ignored_matches(log, declared) {
        return Err(io::Error::other("declared pilot ignored inventory differs"));
    }
    account_pending(log, discovered, success, declared.len())
}
fn account_pending(
    log: &str,
    discovered: usize,
    success: bool,
    required_not_run: usize,
) -> io::Result<usize> {
    let [passed, failed, ignored, measured, filtered] = totals(log)?;
    if !success
        || log
            .lines()
            .any(|line| line.starts_with("test result: ") && !line.starts_with("test result: ok. "))
        || passed.checked_add(required_not_run) != Some(discovered)
        || ignored != required_not_run
        || [failed, measured, filtered].iter().any(|count| *count != 0)
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
/// One well-formed SUCCESS footer. General totals also parses failure summaries
/// for accounting; positive admission must use this helper.
pub(super) fn successful_totals(log: &str) -> io::Result<[usize; 5]> {
    let counts = totals(log)?;
    if !log
        .lines()
        .any(|line| line.starts_with("test result: ok. "))
    {
        return Err(io::Error::other("successful native test footer required"));
    }
    Ok(counts)
}
fn summaries(log: &str) -> io::Result<Vec<[usize; 5]>> {
    log.lines()
        .filter(|l| l.starts_with("test result: "))
        .map(parse_summary)
        .collect()
}
fn parse_summary(line: &str) -> io::Result<[usize; 5]> {
    let fields: Vec<_> = line.split_whitespace().collect();
    if fields.len() != 17
        || !matches!(fields[2], "ok." | "FAILED.")
        || [
            (0, "test"),
            (1, "result:"),
            (4, "passed;"),
            (6, "failed;"),
            (8, "ignored;"),
            (10, "measured;"),
            (12, "filtered"),
            (13, "out;"),
            (14, "finished"),
            (15, "in"),
        ]
        .iter()
        .any(|(index, text)| fields[*index] != *text)
        || !fields[16]
            .strip_suffix('s')
            .and_then(|elapsed| elapsed.parse::<f64>().ok())
            .is_some_and(|elapsed| elapsed.is_finite() && elapsed >= 0.0)
    {
        return Err(io::Error::other("native test summary invalid"));
    }
    let count_fields = [fields[3], fields[5], fields[7], fields[9], fields[11]];
    if count_fields
        .iter()
        .any(|field| !field.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(io::Error::other(
            "native test summary count grammar invalid",
        ));
    }
    let numbers = count_fields.map(str::parse::<usize>);
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
        || log
            .lines()
            .any(|line| line.starts_with("test result: ") && !line.starts_with("test result: ok. "))
        || log.lines().any(|line| {
            matches!(
                line,
                "humanymous-key-lock-refused" | "humanymous-key-pending-synced"
            )
        })
        || discovered < 2
        || ignored.len() != 2
        || ignored.iter().copied().collect::<BTreeSet<_>>() != expected
    {
        return Err(io::Error::other("custody helper inventory differs"));
    }
    for parent in [
        "competing_process_cannot_load_or_create",
        "process_exit_releases_lock_and_rotation_pending_requires_recovery",
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
    account_pending(log, discovered, success, 2)
}
/// Closed public vocabulary of declared pilot ids.
pub(crate) fn pilot_ids() -> std::collections::BTreeSet<String> {
    pilots::known_ids()
}
/// Closed public vocabulary of declared pilot subprocess helpers.
pub(crate) fn pilot_helpers() -> std::collections::BTreeSet<String> {
    pilots::known_helpers()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn panic_locations_use_forward_slashes_from_both_streams() {
        let stdout = "thread 'a' (1) panicked at crates\\x\\src\\a.rs:12:5:\nmessage";
        let stderr = b"thread 'b' panicked at src/b.rs:3:9:\n";
        assert_eq!(
            panic_locations(stdout, stderr),
            ["crates/x/src/a.rs:12:5", "src/b.rs:3:9"]
        );
    }
    #[test]
    fn captured_helpers_require_single_summary_exact_helper_and_parent() {
        let good = "test helper ... ignored\ntest parent ... ok\ntest result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.1s";
        assert_eq!(
            captured_helper_account(good, 2, true, "helper", &["parent"], &[]).unwrap(),
            1
        );
        for bad in [
            good.replace("test parent ... ok\n", ""),
            good.replace("test helper", "test unknown"),
            format!("{good}\n{good}"),
        ] {
            assert!(captured_helper_account(&bad, 2, true, "helper", &["parent"], &[]).is_err());
        }
        assert!(captured_helper_account(good, 2, false, "helper", &["parent"], &[]).is_err());
        assert!(captured_helper_account(good, 3, true, "helper", &["parent"], &[]).is_err());
        let wal = good
            .replace("helper", "audit::storage::wal::tests::process_lock_probe")
            .replace(
                "parent",
                "audit::storage::wal::tests::exclusive_writer_rotation_checkpoint_and_restart",
            );
        let helper = "audit::storage::wal::tests::process_lock_probe";
        let parents =
            &["audit::storage::wal::tests::exclusive_writer_rotation_checkpoint_and_restart"];
        assert_eq!(
            captured_helper_account(&wal, 2, true, helper, parents, &[]).unwrap(),
            1
        );
        assert!(
            captured_helper_account(
                &format!("humanymous-wal-lock-refused\n{wal}"),
                2,
                true,
                helper,
                parents,
                &[]
            )
            .is_err()
        );
    }
    #[test]
    fn declared_pilots_extend_helpers_and_must_match_exactly() {
        let log = "test helper ... ignored\ntest pilot ... ignored\ntest parent ... ok\ntest result: ok. 1 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.1s";
        let pilot = vec!["pilot".to_owned()];
        assert_eq!(
            captured_helper_account(log, 3, true, "helper", &["parent"], &pilot).unwrap(),
            1
        );
        assert!(captured_helper_account(log, 3, true, "helper", &["parent"], &[]).is_err());
        let only = "test pilot ... ignored\ntest parent ... ok\ntest result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.1s";
        assert_eq!(declared_account(only, 2, true, &pilot).unwrap(), 1);
        assert!(declared_account(only, 2, true, &["other".to_owned()]).is_err());
        assert!(declared_account(only, 2, false, &pilot).is_err());
        assert!(
            declared_account(&only.replace("1 ignored", "2 ignored"), 2, true, &pilot).is_err()
        );
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
    fn custody_helpers_require_exact_parents_single_summary_and_zero_unknown_skips() {
        let nested = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.1s";
        let final_summary = "test result: ok. 2 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.1s";
        let good = format!(
            "test competing_process_cannot_load_or_create ... ok\ntest process_exit_releases_lock_and_rotation_pending_requires_recovery ... ok\ntest child_lock_probe ... ignored\ntest child_interrupted_writer ... ignored\n{final_summary}"
        );
        assert_eq!(custody_account(&good, 4, true).unwrap(), 2);
        for bad in [
            good.replace("test child_lock_probe ... ignored\n", ""),
            good.replace("test competing_process_cannot_load_or_create ... ok\n", ""),
            good.replace("test child_interrupted_writer", "test unknown_helper"),
            format!("{nested}\n{good}"),
            format!("humanymous-key-lock-refused\n{good}"),
            format!("humanymous-key-pending-synced\n{good}"),
            format!("{good}\ntest other ... ignored"),
            format!("{good}\n{final_summary}"),
        ] {
            assert!(custody_account(&bad, 4, true).is_err());
        }
        assert!(custody_account(&good, 4, false).is_err());
        assert!(account_pending(&good, 4, true, 0).is_err());
    }
    #[test]
    fn extreme_counts_are_rejected_without_arithmetic_overflow() {
        let log = format!(
            "test result: ok. {} passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.1s",
            usize::MAX
        );
        assert!(account_pending(&log, 1, true, 1).is_err());
        let log = format!(
            "test result: FAILED. 0 passed; {} failed; 0 ignored; 1 measured; 0 filtered out; finished in 0.1s",
            usize::MAX
        );
        assert!(account_pending(&log, 0, true, 0).is_err());
    }
    #[test]
    fn summary_requires_exact_labels_and_finite_elapsed_field() {
        let good = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.1s";
        for bad in [
            good.replace("passed;", "invented;"),
            good.replace("finished in 0.1s", "finished"),
            good.replace("0.1s", "NaNs"),
            format!("{good} extra"),
        ] {
            assert!(totals(&bad).is_err());
        }
        assert!(account_pending(&good.replace("ok.", "FAILED."), 1, true, 0).is_err());
        assert!(totals(&good.replace("1 passed;", "+1 passed;")).is_err());
    }
}
