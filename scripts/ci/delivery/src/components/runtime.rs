//! Explicit owner-required Runtime harnesses, independent of generic actor stops.
use crate::{Result, command, require, source};
use serde_json::{Value, json};
use std::path::Path;

fn count(text: &str) -> Result<u64> {
    let tests = text.lines().filter(|line| line.ends_with(": test")).count() as u64;
    let unique: std::collections::BTreeSet<_> = text
        .lines()
        .filter(|line| line.ends_with(": test"))
        .collect();
    require(
        tests > 0
            && unique.len() as u64 == tests
            && text.lines().any(|line| {
                line == format!(
                    "{tests} {}, 0 benchmarks",
                    if tests == 1 { "test" } else { "tests" }
                )
            }),
        "runtime discovery count missing or inconsistent",
    )?;
    Ok(tests)
}
fn execution(text: &str, expected: u64) -> bool {
    let summaries: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("test result:"))
        .collect();
    summaries.len() == 1
        && summaries[0].starts_with(&format!(
            "test result: ok. {expected} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;"
        ))
}
fn invoke(root: &Path, dir: &Path, report: &mut Value, label: &str, args: &[&str]) -> Result<bool> {
    report["stage"] = json!(label);
    let log = dir.join(format!("{label}.log"));
    let code = command::json_log(root, "cargo", args, &log)?;
    report["commands"][label] = json!({"exit_code":code,"stdout_sha256":source::file(&log)?,"stderr_sha256":source::file(&log.with_extension("stderr.log"))?});
    Ok(code == 0)
}
pub(super) fn run(root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
    let mut failed = false;
    for (name, target, expected) in [
        ("runtime-lib", "--lib", if cfg!(unix) { 45 } else { 42 }),
        ("runtime-process", "--test", 1),
    ] {
        let mut base = vec![
            "test",
            "--locked",
            "-j1",
            "-p",
            "humanymous-runtime",
            "--all-features",
            target,
        ];
        if target == "--test" {
            base.push("native_process");
        }
        let list_label = format!("{name}-list");
        let mut discovery = base.clone();
        discovery.extend(["--", "--list"]);
        let observed: Result<u64> = (|| {
            require(
                invoke(root, dir, report, &list_label, &discovery)?,
                "runtime discovery command failed",
            )?;
            let text = std::fs::read_to_string(dir.join(format!("{list_label}.log")))
                .map_err(|e| e.to_string())?;
            if target == "--test" {
                require(text.lines().any(|line|line=="native_binary_session_collection_origin_restart_and_wal_authentication: test"),"required Runtime native process case absent")?;
            }
            count(&text)
        })();
        report["test_counts"][name] = json!({"expected":expected,"discovered":observed.as_ref().ok(),"passed":null,"validated":false});
        // Both owner harnesses still run independently if one discovery/execution fails.
        let mut actual = base;
        actual.extend(["--", "--test-threads=1"]);
        let outcome: Result<bool> = (|| {
            let success = invoke(root, dir, report, name, &actual)?;
            let text = std::fs::read_to_string(dir.join(format!("{name}.log")))
                .map_err(|e| e.to_string())?;
            Ok(success
                && observed.as_ref().is_ok_and(|count| *count == expected)
                && execution(&text, expected))
        })();
        if outcome == Ok(true) {
            report["test_counts"][name]["passed"] = json!(expected);
            report["test_counts"][name]["validated"] = json!(true);
        } else {
            failed = true;
        }
    }
    require(!failed, "required independent Runtime harness incomplete")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_and_execution_cannot_claim_ignored_or_empty_success() {
        assert_eq!(count("owned_case: test\n1 test, 0 benchmarks").unwrap(), 1);
        assert!(count("0 tests, 0 benchmarks").is_err());
        assert!(count("owned_case: test\n2 tests, 0 benchmarks").is_err());
        assert!(count("owned_case: test\nowned_case: test\n2 tests, 0 benchmarks").is_err());
        let log = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;";
        assert!(execution(log, 1));
        assert!(!execution(log, 2));
        assert!(!execution(&log.replace("0 ignored", "1 ignored"), 1));
        assert!(!execution(&format!("{log}\n{log}"), 1));
    }
}
