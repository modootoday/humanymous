//! Strict accounting for pinned upstream product/developer paired-result shapes.
//! Accounting alone does not authenticate traces, runtime or source identity.
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub expected_runs: usize,
    pub observed_runs: usize,
    pub errors: Vec<String>,
    pub discovery_failures: Vec<String>,
    pub target_reads_expected: usize,
    pub target_reads_observed: usize,
    pub mean_behavior_delta: Option<f64>,
    pub source_bound_qualified: bool,
}

type CaseSpec = (&'static str, Option<&'static str>);
const PRODUCT: &[CaseSpec] = &[
    ("install-plan", Some("humanymous")),
    ("readiness", Some("humanymous")),
    ("unrelated-ui", None),
];
const DEVELOPER: &[CaseSpec] = &[
    ("ci-identity", Some("github-actions-ops")),
    ("ci-accounting", Some("github-actions-ops")),
    ("release-prepare", Some("cut-release")),
    ("release-failed-claim", Some("cut-release")),
    ("unrelated-layout", None),
];

pub fn validate(value: &Value) -> Report {
    validate_profile(value, PRODUCT, "humanymous.product-paired-accounting.v1")
}

pub fn validate_developer(value: &Value) -> Report {
    validate_profile(
        value,
        DEVELOPER,
        "humanymous.developer-paired-accounting.v1",
    )
}

fn validate_profile(value: &Value, cases_expected: &[CaseSpec], schema: &'static str) -> Report {
    let mut report = Report {
        schema,
        expected_runs: cases_expected.len() * 4,
        observed_runs: 0,
        errors: Vec::new(),
        discovery_failures: Vec::new(),
        target_reads_expected: cases_expected
            .iter()
            .filter(|case| case.1.is_some())
            .count()
            * 2,
        target_reads_observed: 0,
        mean_behavior_delta: None,
        source_bound_qualified: false,
    };
    for (valid, message) in [
        (value["schemaVersion"] == 1, "unsupported schema"),
        (value["runtime"] == "codex", "unexpected runtime"),
        (
            value["partial"] == false,
            "partial or missing completion flag",
        ),
        (
            value["suite"]["ablation"] == "with-without",
            "missing paired ablation",
        ),
        (
            value["suite"]["judgeVotes"] == 3,
            "expected three judge votes",
        ),
    ] {
        if !valid {
            report.errors.push(message.into());
        }
    }
    if !value["partialReason"].is_null() {
        report.errors.push("partial reason present".into());
    }
    let Some(cases) = value["cases"].as_array() else {
        report.errors.push("missing cases".into());
        return report;
    };
    let expected = cases_expected
        .iter()
        .map(|case| case.0)
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut deltas = Vec::new();
    for case in cases {
        let name = case["name"].as_str().unwrap_or("");
        if !expected.contains(name) || !seen.insert(name) {
            report
                .errors
                .push(format!("unknown/duplicate case: {name}"));
        }
        if case["runsPerCase"] != 2 {
            report.errors.push(format!("{name}: wrong runsPerCase"));
        }
        if case["arms"].as_object().is_none_or(|arms| {
            arms.len() != 2 || !arms.contains_key("with") || !arms.contains_key("without")
        }) {
            report.errors.push(format!("{name}: unexpected arm set"));
        }
        let expected_skill = cases_expected
            .iter()
            .find(|case| case.0 == name)
            .and_then(|case| case.1);
        let mut sums = [0.0; 2];
        for (index, arm) in ["with", "without"].iter().enumerate() {
            let Some(runs) = case["arms"][arm].as_array() else {
                report.errors.push(format!("{name}/{arm}: missing arm"));
                continue;
            };
            if runs.len() != 2 {
                report.errors.push(format!("{name}/{arm}: wrong run count"));
            }
            for (run_index, run) in runs.iter().enumerate() {
                report.observed_runs += 1;
                let label = format!("{name}/{arm}/{run_index}");
                if !run["error"].is_null()
                    || run["failedBeforeAnswer"] != false
                    || run["skippedPaidGraders"] != false
                    || run["contaminated"] != false
                {
                    report
                        .errors
                        .push(format!("{label}: failed, ungraded or contaminated"));
                }
                match run["score"].as_f64() {
                    Some(score) if (0.0..=1.0).contains(&score) => {}
                    _ => report
                        .errors
                        .push(format!("{label}: missing/invalid score")),
                }
                let behavior = run["graders"].as_array().map(|graders| {
                    graders
                        .iter()
                        .filter(|g| g["name"] == "behavior")
                        .collect::<Vec<_>>()
                });
                match behavior.as_deref() {
                    Some([grade])
                        if grade["passed"].is_boolean()
                            && grade["scored"] == true
                            && grade["weight"] == 1 =>
                    {
                        if grade["passed"] == true {
                            sums[index] += 1.0;
                        }
                        if grade["judgeVotes"].as_array().is_none_or(|votes| {
                            votes.len() != 3 || votes.iter().any(|vote| !vote.is_boolean())
                        }) {
                            report
                                .errors
                                .push(format!("{label}: incomplete judge votes"));
                        } else if let Some(votes) = grade["judgeVotes"].as_array() {
                            let majority = votes.iter().filter(|vote| **vote == true).count() >= 2;
                            if majority != (grade["passed"] == true) {
                                report.errors.push(format!(
                                    "{label}: verdict disagrees with judge majority"
                                ));
                            }
                        }
                    }
                    _ => report
                        .errors
                        .push(format!("{label}: missing/duplicate/ungraded behavior")),
                }
                let opened = run["skillsOpened"].as_array();
                let target_opened = opened.is_some_and(|items| {
                    expected_skill.is_some_and(|skill| items.iter().any(|v| v == skill))
                });
                let product_profile = schema == "humanymous.product-paired-accounting.v1";
                if opened.is_some_and(|items| {
                    items.iter().any(|v| {
                        !v.is_string()
                            || if product_profile {
                                v != "humanymous"
                            } else {
                                v == "humanymous"
                            }
                    })
                }) {
                    report
                        .errors
                        .push(format!("{label}: unexpected skill contamination"));
                }
                if opened.is_none()
                    || (*arm == "without" && opened.is_some_and(|items| !items.is_empty()))
                {
                    report
                        .errors
                        .push(format!("{label}: invalid discovery observation"));
                }
                if *arm == "with" {
                    if target_opened {
                        report.target_reads_observed += 1;
                    }
                    if expected_skill.is_some() && !target_opened {
                        report
                            .discovery_failures
                            .push(format!("{label}: target skill not read"));
                    }
                    if opened.is_some_and(|items| {
                        items
                            .iter()
                            .any(|v| expected_skill.is_none_or(|skill| v != skill))
                    }) {
                        report
                            .discovery_failures
                            .push(format!("{label}: out-of-scope skill activation"));
                    }
                }
            }
        }
        deltas.push((sums[0] - sums[1]) / 2.0);
    }
    if seen != expected {
        report.errors.push("case denominator mismatch".into());
    }
    if report.observed_runs != report.expected_runs {
        report.errors.push("run denominator mismatch".into());
    }
    if report.errors.is_empty() {
        report.mean_behavior_delta = Some(deltas.iter().sum::<f64>() / cases_expected.len() as f64);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    fn complete() -> Value {
        complete_profile(PRODUCT)
    }
    fn complete_profile(profile: &[CaseSpec]) -> Value {
        let cases: Vec<_> = profile.iter().map(|&(name,skill)| {
            let run = |arm| serde_json::json!({"score":1,"failedBeforeAnswer":false,
                "skippedPaidGraders":false,"contaminated":false,
                "skillsOpened":if arm == "with" {skill.into_iter().collect::<Vec<_>>()} else {vec![]},
                "graders":[{"name":"behavior","passed":true,"scored":true,"weight":1,"judgeVotes":[true,true,true]}]});
            serde_json::json!({"name":name,"runsPerCase":2,"arms":{
                "with":[run("with"),run("with")],"without":[run("without"),run("without")]}})
        }).collect();
        serde_json::json!({"schemaVersion":1,"runtime":"codex","partial":false,
            "suite":{"ablation":"with-without","judgeVotes":3},"cases":cases})
    }
    #[test]
    fn full_denominator_is_accounted_but_never_self_qualifies_source() {
        let report = validate(&complete());
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(report.mean_behavior_delta, Some(0.0));
        assert!(!report.source_bound_qualified);
    }
    #[test]
    fn developer_suite_requires_its_own_twenty_runs_and_targeted_reads() {
        let mut data = complete_profile(DEVELOPER);
        let report = validate_developer(&data);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(report.expected_runs, 20);
        assert_eq!(report.observed_runs, 20);
        assert_eq!(report.mean_behavior_delta, Some(0.0));
        assert!(validate(&data).mean_behavior_delta.is_none());
        assert!(
            validate_developer(&complete())
                .mean_behavior_delta
                .is_none()
        );
        data["cases"][0]["arms"]["with"][0]["skillsOpened"] = serde_json::json!(["cut-release"]);
        let report = validate_developer(&data);
        assert!(report.errors.is_empty());
        assert!(!report.discovery_failures.is_empty());
        assert_eq!(report.mean_behavior_delta, Some(0.0));
        assert_eq!(report.target_reads_observed, 7);
        data = complete_profile(DEVELOPER);
        data["cases"].as_array_mut().unwrap().pop();
        assert!(validate_developer(&data).mean_behavior_delta.is_none());
    }
    #[test]
    fn observed_nonactivation_is_reported_without_dropping_behavior_runs() {
        let mut data = complete();
        data["cases"][0]["arms"]["with"][0]["skillsOpened"] = serde_json::json!([]);
        let report = validate(&data);
        assert!(report.errors.is_empty());
        assert_eq!(report.observed_runs, 12);
        assert_eq!(report.target_reads_expected, 4);
        assert_eq!(report.target_reads_observed, 3);
        assert!(!report.discovery_failures.is_empty());
        assert_eq!(report.mean_behavior_delta, Some(0.0));
        data = complete();
        data["cases"][2]["arms"]["with"][0]["skillsOpened"] = serde_json::json!(["humanymous"]);
        let report = validate(&data);
        assert!(report.errors.is_empty());
        assert!(!report.discovery_failures.is_empty());
        assert_eq!(report.mean_behavior_delta, Some(0.0));
    }
    #[test]
    fn partial_missing_null_and_contaminated_results_refuse_delta() {
        for pointer in [
            "/partial",
            "/cases/0/arms/with/0/failedBeforeAnswer",
            "/cases/0/arms/without/0/contaminated",
        ] {
            let mut data = complete();
            *data.pointer_mut(pointer).unwrap() = Value::Bool(true);
            assert!(validate(&data).mean_behavior_delta.is_none());
        }
        let mut data = complete();
        data["cases"][0]["arms"]["with"][0]["graders"][0]["passed"] = Value::Null;
        assert!(validate(&data).mean_behavior_delta.is_none());
        data = complete();
        data["cases"].as_array_mut().unwrap().pop();
        assert!(validate(&data).mean_behavior_delta.is_none());
        data = complete();
        data["cases"][0]["arms"]
            .as_object_mut()
            .unwrap()
            .remove("without");
        assert!(validate(&data).mean_behavior_delta.is_none());
        data = complete();
        data["cases"][1]["name"] = "install-plan".into();
        assert!(validate(&data).mean_behavior_delta.is_none());
        data = complete();
        data["cases"][0]["arms"]["with"][0]["skillsOpened"] =
            serde_json::json!(["humanymous", "implement-sot-slice"]);
        assert!(validate(&data).mean_behavior_delta.is_none());
        data = complete();
        data["cases"][0]["arms"]["extra"] = serde_json::json!([]);
        assert!(validate(&data).mean_behavior_delta.is_none());
        data = complete();
        data["cases"][0]["arms"]["with"][0]["graders"][0]["judgeVotes"] =
            serde_json::json!([false, false, false]);
        assert!(validate(&data).mean_behavior_delta.is_none());
    }
}
