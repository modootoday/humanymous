//! Explicit owner-required Runtime harnesses, independent of generic actor stops.
use crate::{Result, command, require, source};
use serde_json::{Value, json};
use std::path::Path;

const CONTRACT: &str =
    "crates/sdk/humanymous-runtime/qualification/gateway/qualification-contract.json";
fn contract(root: &Path) -> Result<Value> {
    crate::read_json(&root.join(CONTRACT))
}
fn inventory(
    contract: &Value,
    target: &str,
    os: &str,
) -> Result<std::collections::BTreeSet<String>> {
    require(
        contract["schema"] == "humanymous.runtime-qualification/0.2"
            && contract["package"] == "humanymous-runtime"
            && contract["context"] == "gateway"
            && contract["library"]["test_filter"] == "gateway::"
            && contract["all_features"] == true
            && contract["locked"] == true
            && contract["required_ignored"].as_u64() == Some(0)
            && contract["filtered_policy"]
                == "Only complete-inventory names outside gateway:: may be filtered by the explicit library context profile; all selected gateway names required. Integration target has zero filtered cases."
            && contract["both_targets_must_be_attempted"] == true
            && contract["full_product_qualified"] == false,
        "Runtime owner contract mismatch",
    )?;
    require(
        matches!(os, "linux" | "macos" | "windows"),
        "Runtime contract OS unsupported",
    )?;
    let section = &contract[target];
    require(
        section["target"]
            == if target == "library" {
                "lib"
            } else {
                "gateway-native-process"
            },
        "Runtime target contract mismatch",
    )?;
    let mut names = section[if target == "library" {
        "common_test_names"
    } else {
        "test_names"
    }]
    .as_array()
    .ok_or("Runtime owner names absent")?
    .clone();
    if target == "library" && os != "windows" {
        names.extend_from_slice(
            section["unix_extra_test_names"]
                .as_array()
                .ok_or("Runtime Unix names absent")?,
        );
    }
    let mut unique = std::collections::BTreeSet::new();
    for name in &names {
        let name = name.as_str().ok_or("Runtime owner name invalid")?;
        require(
            !name.is_empty()
                && name
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b':'))
                && unique.insert(name.to_owned()),
            "Runtime owner name invalid or duplicate",
        )?;
    }
    require(
        !unique.is_empty() && section["expected_counts"][os].as_u64() == Some(unique.len() as u64),
        "Runtime owner inventory count mismatch",
    )?;
    Ok(unique)
}

pub(super) fn count(text: &str) -> Result<u64> {
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
pub(super) fn execution(text: &str, expected: u64, filtered: u64) -> bool {
    let summaries: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("test result:"))
        .collect();
    summaries.len() == 1
        && summaries[0].starts_with(&format!(
            "test result: ok. {expected} passed; 0 failed; 0 ignored; 0 measured; {filtered} filtered out;"
        ))
}
fn context_inventory(
    text: &str,
    names: &std::collections::BTreeSet<String>,
    library: bool,
) -> Result<u64> {
    let total = count(text)?;
    let actual: std::collections::BTreeSet<String> = text
        .lines()
        .filter_map(|line| line.strip_suffix(": test").map(str::to_owned))
        .collect();
    let selected: std::collections::BTreeSet<_> = actual
        .iter()
        .filter(|name| !library || name.starts_with("gateway::"))
        .cloned()
        .collect();
    require(
        selected == *names,
        "Runtime discovered names differ from reviewed owner inventory",
    )?;
    let outside: Vec<_> = actual.difference(&selected).collect();
    require(
        outside.iter().all(|name| {
            [
                "authority::",
                "audit::",
                "capture::",
                "network::",
                "native_security::",
            ]
            .iter()
            .any(|prefix| name.starts_with(*prefix))
        }),
        "unknown Runtime context cannot be silently filtered",
    )?;
    require(
        total == (selected.len() + outside.len()) as u64,
        "Runtime context inventory accounting mismatch",
    )?;
    Ok(outside.len() as u64)
}
pub(super) fn invoke(
    root: &Path,
    dir: &Path,
    report: &mut Value,
    label: &str,
    args: &[&str],
) -> Result<bool> {
    report["stage"] = json!(label);
    let log = dir.join(format!("{label}.log"));
    let code = command::json_log(root, "cargo", args, &log)?;
    report["commands"][label] = json!({"exit_code":code,"stdout_sha256":source::file(&log)?,"stderr_sha256":source::file(&log.with_extension("stderr.log"))?});
    Ok(code == 0)
}
pub(super) fn run(root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
    let contract = contract(root)?;
    report["owner_contract_sha256"] = json!(source::file(&root.join(CONTRACT))?);
    report["qualification_scope"] =
        json!("gateway context only; whole Runtime/workspace remains separate and mandatory");
    let mut failed = false;
    for (name, target, section) in [
        ("runtime-lib", "--lib", "library"),
        ("runtime-process", "--test", "integration"),
    ] {
        let names = inventory(&contract, section, std::env::consts::OS)?;
        let expected = names.len() as u64;
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
            base.push("gateway-native-process");
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
            context_inventory(&text, &names, section == "library")
        })();
        report["test_counts"][name] = json!({"expected":expected,"discovered":observed.as_ref().ok().map(|_| expected),"outside_context_filtered":observed.as_ref().ok(),"inventory_scope":if section == "library" {"complete library discovery; explicit gateway context execution"} else {"complete integration target"},"passed":null,"validated":false});
        // Both owner harnesses still run independently if one discovery/execution fails.
        let mut actual = base;
        actual.push("--");
        if section == "library" {
            actual.push("gateway::");
        }
        actual.push("--test-threads=1");
        let outcome: Result<bool> = (|| {
            let success = invoke(root, dir, report, name, &actual)?;
            let text = std::fs::read_to_string(dir.join(format!("{name}.log")))
                .map_err(|e| e.to_string())?;
            Ok(success
                && observed
                    .as_ref()
                    .is_ok_and(|filtered| execution(&text, expected, *filtered)))
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
    fn owner_inventory_is_one_source_for_each_platform() {
        let fixture = || json!({"schema":"humanymous.runtime-qualification/0.2","package":"humanymous-runtime","context":"gateway","all_features":true,"locked":true,"required_ignored":0,"both_targets_must_be_attempted":true,"full_product_qualified":false,"filtered_policy":"Only complete-inventory names outside gateway:: may be filtered by the explicit library context profile; all selected gateway names required. Integration target has zero filtered cases.","library":{"target":"lib","test_filter":"gateway::","common_test_names":["gateway::common"],"unix_extra_test_names":["gateway::unix"],"expected_counts":{"linux":2,"macos":2,"windows":1}},"integration":{"target":"gateway-native-process","test_names":["native_case"],"expected_counts":{"linux":1,"macos":1,"windows":1}}});
        let mut owner = fixture();
        for os in ["linux", "macos", "windows"] {
            assert!(!inventory(&owner, "library", os).unwrap().is_empty());
            assert_eq!(inventory(&owner, "integration", os).unwrap().len(), 1);
        }
        owner["library"]["expected_counts"]["windows"] = json!(0);
        assert!(inventory(&owner, "library", "windows").is_err());
        let mut owner = fixture();
        let duplicate = owner["library"]["common_test_names"][0].clone();
        owner["library"]["common_test_names"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(inventory(&owner, "library", "linux").is_err());
    }
    #[test]
    fn discovery_and_execution_cannot_claim_ignored_or_empty_success() {
        assert_eq!(count("owned_case: test\n1 test, 0 benchmarks").unwrap(), 1);
        assert!(count("0 tests, 0 benchmarks").is_err());
        assert!(count("owned_case: test\n2 tests, 0 benchmarks").is_err());
        assert!(count("owned_case: test\nowned_case: test\n2 tests, 0 benchmarks").is_err());
        let log = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;";
        assert!(execution(log, 1, 0));
        assert!(!execution(log, 2, 0));
        assert!(!execution(&log.replace("0 ignored", "1 ignored"), 1, 0));
        assert!(!execution(&format!("{log}\n{log}"), 1, 0));
        assert!(!execution(log, 1, 1));
        assert!(execution(&log.replace("0 filtered", "1 filtered"), 1, 1));
    }
    #[test]
    fn gateway_filter_requires_complete_inventory_and_exact_selected_names() {
        let names = std::collections::BTreeSet::from(["gateway::owned_case".to_owned()]);
        let log = "gateway::owned_case: test\naudit::other_case: test\n2 tests, 0 benchmarks";
        assert_eq!(context_inventory(log, &names, true).unwrap(), 1);
        assert!(context_inventory(log, &names, false).is_err());
        assert!(context_inventory(&log.replace("audit::", "unknown::"), &names, true).is_err());
        assert!(
            context_inventory(&log.replace("owned_case", "unreviewed_case"), &names, true).is_err()
        );
        assert!(context_inventory(&log.replace("2 tests", "3 tests"), &names, true).is_err());
    }
}
