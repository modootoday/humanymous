//! Merge readiness, separate from release readiness (issue #16, option 1).
//!
//! `ready` stays the release gate: it needs every validation job to succeed,
//! including full-product pilots. `merge-ready` needs the non-pilot jobs to
//! succeed and accepts the Docker and browser pilots only when they show no
//! regression: zero failed assertions and no error other than the honest
//! component-only block. A missing or unreadable signal fails closed.
use crate::{Result, read_json, require, source};
use serde_json::Value;
use std::path::Path;

/// Jobs that must succeed outright for a merge.
pub const SUCCESS: &[&str] = &[
    "policy",
    "native",
    "msrv",
    "quality",
    "nginx",
    "images",
    "packages",
    "node",
    "advisories",
    "documentation",
];
/// Pilots that may fail only with the honest component-only block.
pub const PILOTS: &[&str] = &["docker", "browser"];
/// Job output carrying a pilot's merge signal.
pub const OUTPUT: &str = "merge-signal";
const NO_REGRESSION: &str = "no-regression";

/// Classify a pilot's public summary for merging. Writes the signal to
/// `GITHUB_OUTPUT` when present and always succeeds, so the pilot's own
/// failure stays visible as the job result.
pub fn signal(root: &Path, kind: &str) -> Result<()> {
    require(PILOTS.contains(&kind), "merge signal is only for pilots")?;
    let path = root
        .join("target/delivery-evidence")
        .join(format!("{kind}-{}.json", std::env::consts::OS));
    let verdict = read_json(&path)
        .and_then(|summary| classify(&summary, &source::head(root)?))
        .map(|()| NO_REGRESSION.to_owned())
        .unwrap_or_else(|reason| format!("regression: {reason}"));
    println!("{kind} merge signal: {verdict}");
    if let Ok(output) = std::env::var("GITHUB_OUTPUT") {
        let line = format!(
            "signal={}\n",
            if verdict == NO_REGRESSION {
                NO_REGRESSION
            } else {
                "regression"
            }
        );
        std::fs::OpenOptions::new()
            .append(true)
            .open(output)
            .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// A summary shows no regression when it is bound to this source, every case
/// passed or is blocked, no component reports a failed assertion, and the only
/// reason it is not qualified is the component-only block.
pub fn classify(summary: &Value, head: &str) -> Result<()> {
    require(
        summary["source_sha"] == head,
        "summary is not for this source",
    )?;
    require(
        summary["source_changed_during_execution"] == false,
        "source changed during execution",
    )?;
    require(
        summary["errors"].as_u64() == Some(0),
        "pilot reported execution errors",
    )?;
    require(
        (summary["status"] == "blocked" && summary["qualification"] == "component-only")
            || (summary["status"] == "passed" && summary["qualification"] == "full-product"),
        "pilot status is neither passed nor the component-only block",
    )?;
    let cases = summary["cases"].as_array().ok_or("pilot cases absent")?;
    require(
        !cases.is_empty()
            && cases
                .iter()
                .all(|case| case["status"] == "passed" || case["status"] == "blocked"),
        "a pilot case failed",
    )?;
    let components = summary["components"]
        .as_array()
        .ok_or("pilot components absent")?;
    require(
        components.iter().all(|component| {
            component["status"] != "failed"
                && component["assertions"]["failed"]
                    .as_u64()
                    .is_none_or(|failed| failed == 0)
        }),
        "a component reported failed assertions",
    )
}

/// Decide merge readiness from the `needs` context of the aggregate job.
pub fn check(raw: &str) -> Result<()> {
    let needs: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let jobs = needs
        .as_object()
        .ok_or("merge aggregate must be an object")?;
    require(
        jobs.len() == SUCCESS.len() + PILOTS.len(),
        "merge aggregate coverage changed",
    )?;
    for id in SUCCESS {
        require(
            jobs.get(*id).is_some_and(|job| job["result"] == "success"),
            format!("{id} did not succeed; failed/skipped/cancelled never authorizes merge"),
        )?;
    }
    for id in PILOTS {
        let job = jobs.get(*id).ok_or(format!("{id} missing"))?;
        require(
            job["result"] == "success"
                || (job["result"] == "failure" && job["outputs"][OUTPUT] == NO_REGRESSION),
            format!("{id} shows a regression or did not finish"),
        )?;
    }
    println!("merge-ready: every merge check passed; release readiness stays with ready");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn summary() -> Value {
        json!({"source_sha":"abc","source_changed_during_execution":false,"errors":0,
            "status":"blocked","qualification":"component-only",
            "cases":[{"id":"gate","status":"passed"},{"id":"swarm","status":"blocked"}],
            "components":[{"status":"blocked","assertions":{"total":72,"passed":72,"failed":0,"other":0}}]})
    }

    #[test]
    fn component_only_block_without_failures_is_no_regression() {
        classify(&summary(), "abc").unwrap();
        let mut full = summary();
        full["status"] = "passed".into();
        full["qualification"] = "full-product".into();
        classify(&full, "abc").unwrap();
    }

    #[test]
    fn regressions_and_stale_or_partial_summaries_fail_closed() {
        type Mutation = fn(&mut Value);
        let cases: [(&str, Mutation); 9] = [
            ("stale", |s| s["source_sha"] = "old".into()),
            ("changed", |s| {
                s["source_changed_during_execution"] = true.into()
            }),
            ("errors", |s| s["errors"] = 1.into()),
            ("failed status", |s| s["status"] = "failed".into()),
            ("odd qualification", |s| {
                s["qualification"] = "unsupported".into()
            }),
            ("failed case", |s| s["cases"][0]["status"] = "failed".into()),
            ("no cases", |s| s["cases"] = json!([])),
            ("failed assertion", |s| {
                s["components"][0]["assertions"]["failed"] = 2.into()
            }),
            ("failed component", |s| {
                s["components"][0]["status"] = "failed".into()
            }),
        ];
        for (name, mutate) in cases {
            let mut s = summary();
            mutate(&mut s);
            assert!(classify(&s, "abc").is_err(), "{name} accepted");
        }
    }

    fn needs() -> Value {
        let mut needs = json!({});
        for id in SUCCESS {
            needs[*id] = json!({"result":"success","outputs":{}});
        }
        for id in PILOTS {
            needs[*id] = json!({"result":"failure","outputs":{OUTPUT:NO_REGRESSION}});
        }
        needs
    }

    #[test]
    fn merge_needs_successes_and_honest_pilot_blocks() {
        check(&needs().to_string()).unwrap();
        let mut green = needs();
        green["docker"] = json!({"result":"success","outputs":{}});
        check(&green.to_string()).unwrap();
        type Mutation = fn(&mut Value);
        let cases: [(&str, Mutation); 7] = [
            ("failed job", |n| n["node"]["result"] = "failure".into()),
            ("skipped job", |n| n["images"]["result"] = "skipped".into()),
            ("missing job", |n| {
                n.as_object_mut().unwrap().remove("advisories");
            }),
            ("extra job", |n| n["ready"] = json!({"result":"success"})),
            ("regression", |n| {
                n["browser"]["outputs"][OUTPUT] = "regression".into()
            }),
            ("no signal", |n| n["docker"]["outputs"] = json!({})),
            ("cancelled pilot", |n| {
                n["docker"]["result"] = "cancelled".into()
            }),
        ];
        for (name, mutate) in cases {
            let mut n = needs();
            mutate(&mut n);
            assert!(check(&n.to_string()).is_err(), "{name} accepted");
        }
    }
}
