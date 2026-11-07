//! Strict accounting for the sole actual WSGI harness, independent of exit code.
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) const TESTS: [&str; 8] = [
    "actual_allow_preserves_binary_and_two_cookies",
    "authority_response_deadline_closes_without_origin",
    "dependency_down_and_invalid_routes_never_reach_origin",
    "disconnected_client_cannot_reach_origin_after_allow",
    "http::tests::complete_without_connection_close",
    "http::tests::rejects_incomplete_or_conflicting_framing",
    "incomplete_bodies_never_reach_available_authority_or_origin",
    "production_authority_nonallow_is_audited_before_origin",
];

pub(super) fn discovery(raw: &str) -> Result<(), String> {
    let names: Vec<_> = raw
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .collect();
    if names != TESTS || !raw.lines().any(|line| line == "8 tests, 0 benchmarks") {
        return Err("required WSGI test discovery differs".into());
    }
    Ok(())
}

pub(super) fn validate(raw: &str) -> Result<(), String> {
    if raw.matches("test result:").count() != 1
        || !raw
            .contains("test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;")
    {
        return Err("required WSGI test counts did not pass exactly".into());
    }
    let mut rows = BTreeSet::new();
    let mut cases = 0;
    for line in raw.lines().filter(|line| line.contains('{')) {
        let row: Value = serde_json::from_str(&line[line.find('{').unwrap()..])
            .map_err(|_| "invalid WSGI scenario receipt")?;
        let framework = row["framework"].as_str().ok_or("missing framework")?;
        if !["flask", "django"].contains(&framework) || row["wholeProductQualified"] != false {
            return Err("unexpected WSGI framework or qualification".into());
        }
        let group = row["group"].as_str().ok_or("missing scenario group")?;
        let (expected_cases, audit, effects, control) = match group {
            "synthetic-engine-allow-and-invalid" => {
                if row["negativeCases"] != 4 || row["productionBrowserEvidence"] != false {
                    return Err("missing synthetic evidence boundary".into());
                }
                (5, Some(1), 1, None)
            }
            "authority-response-timeout" => (1, None, 0, None),
            "dependency-down-and-routes" => (3, None, 0, None),
            "incomplete-body" => (2, Some(0), 0, None),
            "production-authority-nonallow" => (1, Some(1), 0, None),
            "disconnect-before-handoff" => {
                let disconnected = row["disconnected"]
                    .as_bool()
                    .ok_or("missing matched control")?;
                if row["productionBrowserEvidence"] != false {
                    return Err("missing disconnect evidence boundary".into());
                }
                (1, Some(1), u64::from(!disconnected), Some(disconnected))
            }
            _ => return Err("unexpected WSGI scenario group".into()),
        };
        if row["cases"] != expected_cases
            || row["originEffects"] != effects
            || audit.is_some_and(|count| row["auditRecords"] != count)
            || !rows.insert((framework.to_owned(), group.to_owned(), control))
        {
            return Err("duplicate or inconsistent WSGI scenario row".into());
        }
        cases += expected_cases;
    }
    if rows.len() != 14 || cases != 28 {
        return Err("required WSGI rows or actual cases are missing".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_exit_summary_cannot_replace_missing_or_duplicate_scenarios() {
        let summary = "test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;";
        let fixture: Value =
            serde_json::from_str(include_str!("wsgi_receipt_actual.json")).unwrap();
        assert_eq!(
            fixture["sourceSha"],
            "7842b6a875f1a77b8e88d2d9f31808d2089cebce"
        );
        let rows = fixture["rows"].as_array().unwrap();
        let transcript = |rows: &[Value]| {
            format!(
                "{}\n{summary}",
                rows.iter()
                    .map(Value::to_string)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };
        assert!(validate(&transcript(rows)).is_ok());
        let mut duplicate = rows.clone();
        duplicate.push(rows[0].clone());
        assert!(validate(&transcript(&duplicate)).is_err());
        assert!(validate(&transcript(&rows[..rows.len() - 1])).is_err());
        for (group, field) in [
            ("incomplete-body", "auditRecords"),
            ("production-authority-nonallow", "originEffects"),
            ("synthetic-engine-allow-and-invalid", "negativeCases"),
        ] {
            let mut changed = rows.clone();
            let row = changed
                .iter_mut()
                .find(|row| row["group"] == group)
                .unwrap();
            row[field] = serde_json::json!(99);
            assert!(validate(&transcript(&changed)).is_err());
        }
        assert!(validate(summary).is_err());
        assert!(validate(&format!("{summary}\n{summary}")).is_err());
        assert!(
            validate("test result: ok. 6 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out;")
                .is_err()
        );
    }

    #[test]
    fn discovery_rejects_an_empty_or_duplicate_test_list() {
        let list = format!(
            "{}\n8 tests, 0 benchmarks",
            TESTS.map(|name| format!("{name}: test")).join("\n")
        );
        assert!(discovery(&list).is_ok());
        assert!(discovery("").is_err());
        assert!(discovery(&list.replace(TESTS[0], TESTS[1])).is_err());
    }
}
