//! Exact owned nginx scenario-group accounting, separate from product readiness.
use crate::{Result, require};
use serde_json::{Value, json};
use std::collections::BTreeSet;
mod runner;
pub use runner::run;
fn contract() -> Result<Value> {
    serde_json::from_str(include_str!(
        "../../../../adapters/nginx/pilot-contract.json"
    ))
    .map_err(|e| e.to_string())
}

pub fn validate(report: &Value, revision: &str, manifest: &str, lock: &str) -> Result<Value> {
    let contract = contract()?;
    let hashes = |value: &str, length: usize| {
        value.len() == length
            && value
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    };
    require(
        hashes(revision, 40) && hashes(manifest, 64) && hashes(lock, 64),
        "canonical nginx source identities required",
    )?;
    require(
        contract["schema"] == "humanymous.nginx.pilot-contract/0.1"
            && report["schema"] == contract["report_schema"]
            && report["status"] == "passed"
            && report["full_product_qualified"] == false,
        "nginx component schema/status/scope mismatch",
    )?;
    require(
        report["source_revision"] == revision
            && report["source_manifest"] == manifest
            && report["cargo_lock_sha256"] == lock,
        "nginx source or lock identity mismatch",
    )?;
    let groups = contract["required_groups"]
        .as_array()
        .ok_or("nginx required groups absent")?;
    let expected: BTreeSet<_> = groups
        .iter()
        .map(|id| {
            id.as_str()
                .ok_or_else(|| "invalid required group".to_owned())
        })
        .collect::<Result<_>>()?;
    let cases = report["cases"].as_array().ok_or("nginx groups absent")?;
    let mut actual = BTreeSet::new();
    for case in cases {
        let id = case["id"].as_str().ok_or("nginx group identity absent")?;
        require(
            case["name"] == id && case["result"] == "passed" && actual.insert(id),
            "duplicate or nonpassing nginx group",
        )?;
    }
    require(
        !expected.is_empty()
            && expected.len() == groups.len()
            && actual == expected
            && cases.len() == groups.len(),
        "nginx required group coverage incomplete",
    )?;
    require(
        report["group_count"].as_u64() == Some(groups.len() as u64)
            && report["passed"].as_u64() == Some(groups.len() as u64)
            && ["failed", "ignored", "not_run"]
                .iter()
                .all(|key| report[key].as_u64() == Some(0)),
        "nginx scenario counters are not exact",
    )?;
    for field in ["module_sha256", "engine_sha256"] {
        require(
            report[field]
                .as_str()
                .is_some_and(|value| hashes(value, 64)),
            "nginx module/engine hash absent",
        )?;
    }
    Ok(
        json!({"schema":"humanymous.nginx.component-summary/1","status":"passed","full_product_qualified":false,
        "source_revision":revision,"source_manifest":manifest,"cargo_lock_sha256":lock,
        "module_sha256":report["module_sha256"],"engine_sha256":report["engine_sha256"],
        "scenario_groups":groups.len(),"passed":groups.len(),"failed":0,"ignored":0,"not_run":0,
        "denominator":contract["denominator"]}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn report() -> Value {
        let contract: Value = serde_json::from_str(include_str!(
            "../../../../adapters/nginx/pilot-contract.json"
        ))
        .unwrap();
        json!({"schema":contract["report_schema"],"status":"passed","full_product_qualified":false,"source_revision":"a".repeat(40),"source_manifest":"b".repeat(64),"cargo_lock_sha256":"c".repeat(64),"module_sha256":"d".repeat(64),"engine_sha256":"e".repeat(64),"cases":contract["required_groups"].as_array().unwrap().iter().map(|id|json!({"id":id,"name":id,"result":"passed","detail":"private-canary"})).collect::<Vec<_>>(),"group_count":35,"passed":35,"failed":0,"ignored":0,"not_run":0})
    }
    #[test]
    fn exact_owned_scenarios_are_not_harness_or_full_product_success() {
        let summary =
            validate(&report(), &"a".repeat(40), &"b".repeat(64), &"c".repeat(64)).unwrap();
        assert_eq!(summary["scenario_groups"], 35);
        assert_eq!(summary["full_product_qualified"], false);
        assert!(!summary.to_string().contains("private-canary"));
    }
    #[test]
    fn stale_missing_duplicate_and_ignored_results_fail() {
        for mutation in 0..4 {
            let mut report = report();
            match mutation {
                0 => report["source_revision"] = json!("f".repeat(40)),
                1 => {
                    report["cases"].as_array_mut().unwrap().pop();
                }
                2 => report["cases"][1] = report["cases"][0].clone(),
                _ => report["ignored"] = json!(1),
            }
            assert!(validate(&report, &"a".repeat(40), &"b".repeat(64), &"c".repeat(64)).is_err());
        }
    }
}
