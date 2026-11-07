//! Strict accounting for the pinned upstream product paired-result shape.
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
    pub mean_behavior_delta: Option<f64>,
    pub source_bound_qualified: bool,
}

pub fn validate(value: &Value) -> Report {
    let mut report = Report {
        schema: "humanymous.product-paired-accounting.v1",
        expected_runs: 12,
        observed_runs: 0,
        errors: Vec::new(),
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
    let expected = ["install-plan", "readiness", "unrelated-ui"];
    let mut seen = BTreeSet::new();
    let mut deltas = Vec::new();
    for case in cases {
        let name = case["name"].as_str().unwrap_or("");
        if !expected.contains(&name) || !seen.insert(name) {
            report
                .errors
                .push(format!("unknown/duplicate case: {name}"));
        }
        if case["runsPerCase"] != 2 {
            report.errors.push(format!("{name}: wrong runsPerCase"));
        }
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
                        }
                    }
                    _ => report
                        .errors
                        .push(format!("{label}: missing/duplicate/ungraded behavior")),
                }
                let opened = run["skillsOpened"].as_array();
                let product_opened =
                    opened.is_some_and(|items| items.iter().any(|v| v == "humanymous"));
                if opened.is_some_and(|items| items.iter().any(|v| v != "humanymous")) {
                    report
                        .errors
                        .push(format!("{label}: unexpected skill contamination"));
                }
                if opened.is_none()
                    || (name == "unrelated-ui" && product_opened)
                    || (*arm == "without" && product_opened)
                    || (*arm == "with" && name != "unrelated-ui" && !product_opened)
                {
                    report
                        .errors
                        .push(format!("{label}: invalid discovery observation"));
                }
            }
        }
        deltas.push((sums[0] - sums[1]) / 2.0);
    }
    if seen != BTreeSet::from(expected) {
        report.errors.push("case denominator mismatch".into());
    }
    if report.observed_runs != report.expected_runs {
        report.errors.push("run denominator mismatch".into());
    }
    if report.errors.is_empty() {
        report.mean_behavior_delta = Some(deltas.iter().sum::<f64>() / 3.0);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    fn complete() -> Value {
        let cases: Vec<_> = ["install-plan", "readiness", "unrelated-ui"].into_iter().map(|name| {
            let run = |arm| serde_json::json!({"score":1,"failedBeforeAnswer":false,
                "skippedPaidGraders":false,"contaminated":false,
                "skillsOpened":if arm == "with" && name != "unrelated-ui" {vec!["humanymous"]} else {vec![]},
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
    }
}
