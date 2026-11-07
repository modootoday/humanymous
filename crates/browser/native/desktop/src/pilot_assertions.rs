//! Independent private-report checks for provisioned hosted CEF process tests.
//! Synthetic tests below validate this protocol only; they never qualify a browser.
use humanymous_browser::navigation::ProbeLedger;
use serde_json::Value;

#[derive(Clone, Copy, Debug)]
pub(super) enum Case {
    Positive,
    Cancelled,
}
#[derive(Debug, serde::Serialize)]
pub(super) struct Summary {
    case: &'static str,
    host_pid: u32,
    observed: usize,
    joined_renderer_probes: usize,
    full_runtime_qualified: bool,
}
fn flag(report: &Value, key: &str, expected: bool) -> Result<(), &'static str> {
    if report.get(key).and_then(Value::as_bool) == Some(expected) {
        Ok(())
    } else {
        Err("missing or mismatched native flag")
    }
}
fn number(report: &Value, key: &str) -> Result<u64, &'static str> {
    report
        .get(key)
        .and_then(Value::as_u64)
        .ok_or("missing or invalid native count")
}
fn joined_canaries(report: &Value) -> Result<usize, &'static str> {
    if number(report, "renderer_file_probe_schema")? != 2 {
        return Err("unsupported file probe schema");
    }
    let probes = report
        .get("renderer_file_probes")
        .and_then(Value::as_array)
        .ok_or("canary observations missing")?;
    if probes.len() > 32 {
        return Err("native observation budget exceeded");
    }
    let mut seen = std::collections::BTreeSet::new();
    let count = probes
        .iter()
        .filter(|probe| {
            let Some(pid) = probe
                .get("pid")
                .and_then(Value::as_u64)
                .filter(|pid| *pid > 0 && u32::try_from(*pid).is_ok())
            else {
                return false;
            };
            seen.insert(pid)
                && probe.get("read_os_error").and_then(Value::as_i64) == Some(5)
                && probe.get("create_os_error").and_then(Value::as_i64) == Some(5)
                && probe.get("os_token_status").and_then(Value::as_str) == Some("observed")
                && probe.get("os_token_error") == Some(&Value::Null)
                && probe.get("os_token").is_some_and(|token| {
                    token.get("pid").and_then(Value::as_u64) == Some(pid)
                        && token.get("role").and_then(Value::as_str) == Some("renderer")
                        && token.get("restricted").and_then(Value::as_bool) == Some(true)
                        && token.get("integrity_rid").and_then(Value::as_u64) == Some(0)
                })
        })
        .count();
    if count == 0 {
        Err("renderer canary lacks a matching restricted OS observation")
    } else {
        Ok(count)
    }
}
fn ordered_receipts(report: &Value) -> Result<(), &'static str> {
    let events = report
        .get("navigation_receipts")
        .and_then(Value::as_array)
        .ok_or("native receipts missing")?;
    if events.len() > 128 {
        return Err("native receipt budget exceeded");
    }
    if events.iter().any(|event| {
        !matches!(
            event.get("phase").and_then(Value::as_str),
            Some(
                "creation_requested"
                    | "request_accepted"
                    | "committed"
                    | "load_end"
                    | "content_observed"
            )
        )
    }) {
        return Err("positive native receipts contain failure or unknown phases");
    }
    let content: Vec<_> = events
        .iter()
        .filter(|event| event.get("phase").and_then(Value::as_str) == Some("content_observed"))
        .collect();
    if content.len() != ProbeLedger::ROUTES.len() {
        return Err("native content denominator mismatch");
    }
    let mut previous = 0;
    for (event, route) in content.into_iter().zip(ProbeLedger::ROUTES) {
        let generation = number(event, "generation")?;
        if generation <= previous
            || event.get("route").and_then(Value::as_str) != Some(route.url())
            || event.get("identity").and_then(Value::as_bool) != Some(true)
            || !(1..=8 * 1024 * 1024).contains(&number(event, "bytes")?)
        {
            return Err("native content identity/order mismatch");
        }
        let index = events
            .iter()
            .position(|candidate| std::ptr::eq(candidate, event))
            .ok_or("native event identity missing")?;
        let commit = events[..index].iter().position(|candidate| {
            candidate.get("generation").and_then(Value::as_u64) == Some(generation)
                && candidate.get("route").and_then(Value::as_str) == Some(route.url())
                && candidate.get("phase").and_then(Value::as_str) == Some("committed")
        });
        let Some(commit) = commit else {
            return Err("native content lacks commit receipt");
        };
        let expected_request = if previous == 0 {
            "creation_requested"
        } else {
            "request_accepted"
        };
        if !events[..commit].iter().any(|candidate| {
            candidate.get("generation").and_then(Value::as_u64) == Some(generation)
                && candidate.get("route").and_then(Value::as_str) == Some(route.url())
                && candidate.get("phase").and_then(Value::as_str) == Some(expected_request)
        }) {
            return Err("native commit lacks matching request receipt");
        }
        if !events[commit + 1..index].iter().any(|candidate| {
            candidate.get("generation").and_then(Value::as_u64) == Some(generation)
                && candidate.get("route").and_then(Value::as_str) == Some(route.url())
                && candidate.get("phase").and_then(Value::as_str) == Some("load_end")
                && candidate.get("code").and_then(Value::as_i64) == Some(200)
        }) {
            return Err("native content lacks successful load receipt");
        }
        previous = generation;
    }
    Ok(())
}
pub(super) fn verify(
    report: &Value,
    pid: u32,
    exit: Option<i32>,
    case: Case,
) -> Result<Summary, &'static str> {
    if pid == 0 || number(report, "host_pid")? != u64::from(pid) {
        return Err("native report belongs to another process");
    }
    if number(report, "navigation_schema")? != 2 {
        return Err("native report revision unsupported");
    }
    flag(report, "internal_probe_requested", true)?;
    flag(report, "before_close", true)?;
    flag(report, "host_closed", true)?;
    flag(report, "target_navigation_enabled", false)?;
    let observed = number(report, "internal_probe_observed")?;
    if observed > ProbeLedger::ROUTES.len() as u64 {
        return Err("native probe count out of bounds");
    }
    let (label, joined) = match case {
        Case::Positive => {
            if exit != Some(0) {
                return Err("positive native process did not exit successfully");
            }
            flag(report, "internal_probe_finished", true)?;
            flag(report, "cancellation_requested", false)?;
            flag(report, "internal_probe_passed", true)?;
            flag(report, "cef_child_created", true)?;
            flag(report, "actual_parent_matched", true)?;
            flag(report, "renderer_restricted_token_observed", true)?;
            if observed != ProbeLedger::ROUTES.len() as u64
                || number(report, "navigation_receipts_dropped")? != 0
                || number(report, "resize_readbacks")? == 0
            {
                return Err("native probe proof incomplete or truncated");
            }
            ordered_receipts(report)?;
            ("positive", joined_canaries(report)?)
        }
        Case::Cancelled => {
            if exit != Some(78) {
                return Err("canceled native process exit classification mismatch");
            }
            flag(report, "cancellation_requested", true)?;
            flag(report, "internal_probe_passed", false)?;
            if observed >= ProbeLedger::ROUTES.len() as u64 {
                return Err("canceled probe was not incomplete");
            }
            ("cancelled", 0)
        }
    };
    Ok(Summary {
        case: label,
        host_pid: pid,
        observed: observed as usize,
        joined_renderer_probes: joined,
        full_runtime_qualified: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture() -> Value {
        let mut events = Vec::new();
        for (index, route) in ProbeLedger::ROUTES.into_iter().enumerate() {
            let request = if index == 0 {
                "creation_requested"
            } else {
                "request_accepted"
            };
            events.push(json!({"generation":index+1,"route":route.url(),"phase":request}));
            for phase in ["committed", "load_end", "content_observed"] {
                events.push(json!({"generation":index+1,"route":route.url(),"phase":phase,"code":200,"identity":true,"bytes":100}));
            }
        }
        json!({"host_pid":100,"navigation_schema":2,"internal_probe_requested":true,
            "internal_probe_finished":true,"internal_probe_passed":true,"internal_probe_observed":7,
            "navigation_receipts_dropped":0,"before_close":true,"host_closed":true,
            "target_navigation_enabled":false,"cef_child_created":true,"actual_parent_matched":true,
            "renderer_restricted_token_observed":true,"resize_readbacks":1,"cancellation_requested":false,
            "renderer_file_probe_schema":2,
            "navigation_receipts":events,"renderer_file_probes":[{"pid":200,"read_os_error":5,"create_os_error":5,
                "os_token_status":"observed","os_token_error":null,
                "os_token":{"pid":200,"role":"renderer","restricted":true,"integrity_rid":0}}],
            "process_tokens":[{"pid":200,"role":"renderer","restricted":true,"integrity_rid":0}],
            "private_field":"private data must never enter the sanitized summary"})
    }
    #[test]
    fn protocol_fixture_accepts_complete_shape_without_exporting_private_fields() {
        let summary = verify(&fixture(), 100, Some(0), Case::Positive).unwrap();
        let summary = serde_json::to_value(summary).unwrap();
        assert_eq!(summary["observed"], 7);
        assert_eq!(summary["full_runtime_qualified"], false);
        assert!(summary.get("private_field").is_none());
    }
    #[test]
    fn protocol_rejects_stale_missing_truncated_and_unjoined_success_claims() {
        assert!(verify(&fixture(), 101, Some(0), Case::Positive).is_err());
        for key in [
            "host_pid",
            "navigation_schema",
            "navigation_receipts",
            "before_close",
        ] {
            let mut report = fixture();
            report.as_object_mut().unwrap().remove(key);
            assert!(verify(&report, 100, Some(0), Case::Positive).is_err());
        }
        let mut report = fixture();
        report["navigation_receipts_dropped"] = json!(1);
        assert!(verify(&report, 100, Some(0), Case::Positive).is_err());
        let mut report = fixture();
        report["renderer_file_probes"][0]["os_token"]["pid"] = json!(201);
        assert!(verify(&report, 100, Some(0), Case::Positive).is_err());
        let mut report = fixture();
        report["renderer_file_probes"][0]["os_token_status"] = json!("owned_pid_not_observed");
        assert!(verify(&report, 100, Some(0), Case::Positive).is_err());
        let mut report = fixture();
        report["navigation_receipts"][0]["phase"] = json!("load_error");
        assert!(verify(&report, 100, Some(0), Case::Positive).is_err());
        let mut report = fixture();
        report["navigation_receipts"][3]["identity"] = json!(false);
        assert!(verify(&report, 100, Some(0), Case::Positive).is_err());
        let mut report = fixture();
        report["navigation_receipts"]
            .as_array_mut()
            .unwrap()
            .remove(0);
        assert!(verify(&report, 100, Some(0), Case::Positive).is_err());
        let mut report = fixture();
        report["cancellation_requested"] = json!(true);
        assert!(verify(&report, 100, Some(0), Case::Positive).is_err());
    }
    #[test]
    fn canceled_protocol_fixture_requires_real_cleanup_and_explicit_cancellation() {
        let mut report = fixture();
        report["internal_probe_passed"] = json!(false);
        report["internal_probe_observed"] = json!(0);
        report["cancellation_requested"] = json!(true);
        assert!(verify(&report, 100, Some(78), Case::Cancelled).is_ok());
        report["before_close"] = json!(false);
        assert!(verify(&report, 100, Some(78), Case::Cancelled).is_err());
    }
}
