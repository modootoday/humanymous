//! Pure publication checks: producer flags never replace observed denominators.
use serde_json::Value;
use std::io;

fn number(value: &Value, field: &str) -> io::Result<u64> {
    value[field]
        .as_u64()
        .ok_or_else(|| io::Error::other(format!("missing unsigned count: {field}")))
}

pub(super) fn observed_total(rows: &[Value], field: &str) -> io::Result<u64> {
    rows.iter()
        .filter_map(|row| row[field].as_u64())
        .try_fold(0u64, |sum, count| {
            sum.checked_add(count)
                .ok_or_else(|| io::Error::other("observed count overflow"))
        })
}

fn names(row: &Value, field: &str) -> io::Result<u64> {
    match row.get(field) {
        None | Some(Value::Null) => Ok(0),
        Some(Value::Array(list)) => Ok(list.len() as u64),
        Some(_) => Err(io::Error::other(format!(
            "invalid declared name list: {field}"
        ))),
    }
}

/// Ignored tests that one workspace row accounts for, as
/// `(subprocess_helpers, declared_pilot_names)`. Captured and custody helpers
/// carry their own count. Declared pilot names (receipt- or job-executed
/// tests, declared pilot helpers, owner-declared unsupported tests) count only
/// when the row names its declaration and the receipt/job check held; the
/// exact ignored-name match itself is part of the row's `validated` verdict.
pub(crate) fn row_ignored(row: &Value) -> io::Result<(u64, u64)> {
    let helpers = match row.get("ignored_subprocess_helpers") {
        None => 0,
        Some(value) => value
            .as_u64()
            .ok_or_else(|| io::Error::other("invalid helper count"))?,
    };
    let mut declared = 0u64;
    for field in [
        "required_not_run",
        "declared_subprocess_helpers",
        "unsupported_not_run",
    ] {
        declared = declared
            .checked_add(names(row, field)?)
            .ok_or_else(|| io::Error::other("declared count overflow"))?;
    }
    let named = |field: &str| row[field].as_array().is_some_and(|ids| !ids.is_empty());
    let scope = row["required_execution_scope"].as_str();
    let bound = match scope {
        Some("native-pilots" | "job") => named("pilot_ids"),
        Some("unsupported") => named("unsupported_ids"),
        _ => false,
    };
    if declared != 0 && (!bound || row["pilot_receipt_verified"] != true) {
        return Err(io::Error::other(
            "declared ignored names without verified pilot",
        ));
    }
    Ok((helpers, declared))
}

pub(super) fn workspace_pass(report: &Value) -> io::Result<()> {
    let coverage = &report["coverage"];
    let rows = coverage["harnesses"]
        .as_array()
        .ok_or_else(|| io::Error::other("workspace rows absent"))?;
    let required = number(coverage, "required_harnesses")?;
    if number(report, "errors")? != 0
        || number(report, "skips")? != 0
        || coverage["complete"] != true
        || required == 0
        || required != number(coverage, "executed_harnesses")?
        || required != rows.len() as u64
    {
        return Err(io::Error::other("incomplete workspace publication"));
    }
    let overflow = || io::Error::other("count overflow");
    let [
        mut passed,
        mut ignored,
        mut helper_total,
        mut declared_total,
    ] = [0u64; 4];
    for row in rows {
        let p = number(row, "passed")?;
        let i = number(row, "ignored")?;
        let (helpers, declared) = row_ignored(row)?;
        if row["validated"] != true
            || row["exit_code"].as_i64() != Some(0)
            || number(row, "failed")? != 0
            || number(row, "filtered")? != 0
            || p.checked_add(i) != Some(number(row, "discovered")?)
            || helpers.checked_add(declared) != Some(i)
        {
            return Err(io::Error::other("unvalidated workspace row"));
        }
        passed = passed.checked_add(p).ok_or_else(overflow)?;
        ignored = ignored.checked_add(i).ok_or_else(overflow)?;
        helper_total = helper_total.checked_add(helpers).ok_or_else(overflow)?;
        declared_total = declared_total.checked_add(declared).ok_or_else(overflow)?;
    }
    // Each published aggregate equals its own observed row total.
    if passed == 0
        || helper_total != number(report, "ignored_subprocess_helpers")?
        || declared_total != number(report, "declared_pilot_ignored")?
        || helper_total.checked_add(declared_total) != Some(ignored)
    {
        return Err(io::Error::other("workspace aggregate differs"));
    }
    if let Some(actor) = report.get("standard_user_lifecycle")
        && ([
            "account_deleted",
            "scratch_deleted",
            "job_kill_on_close",
            "job_empty_observed",
        ]
        .iter()
        .any(|field| actor[*field] != true)
            || actor["child_exit_code"].as_i64() != Some(0)
            || actor["child_launch_attempted"] != true
            || actor["actor_case"] != "normal"
            || actor["scm_delegation_requested"] != false)
    {
        return Err(io::Error::other("workspace actor cleanup incomplete"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn report() -> Value {
        json!({"errors":0,"skips":0,"ignored_subprocess_helpers":0,"declared_pilot_ignored":0,"coverage":{"complete":true,"required_harnesses":1,"executed_harnesses":1,"harnesses":[{"discovered":2,"passed":2,"failed":0,"ignored":0,"filtered":0,"exit_code":0,"validated":true}]}})
    }
    #[test]
    fn overflowing_observed_totals_are_errors_before_publication() {
        let rows = vec![json!({"passed":u64::MAX}), json!({"passed":1})];
        assert!(observed_total(&rows, "passed").is_err());
        let mut bad = report();
        bad["coverage"]["harnesses"][0]["passed"] = json!(u64::MAX);
        bad["coverage"]["harnesses"][0]["ignored"] = json!(1);
        bad["coverage"]["harnesses"][0]["ignored_subprocess_helpers"] = json!(1);
        assert!(workspace_pass(&bad).is_err());
    }
    #[test]
    fn rejects_null_missing_negative_nonzero_and_inconsistent_counts() {
        assert!(workspace_pass(&report()).is_ok());
        for field in [
            "discovered",
            "passed",
            "failed",
            "ignored",
            "filtered",
            "exit_code",
        ] {
            for value in [Value::Null, json!(-1), json!("0")] {
                let mut bad = report();
                bad["coverage"]["harnesses"][0][field] = value;
                assert!(workspace_pass(&bad).is_err(), "{field}");
            }
        }
        for (field, value) in [
            ("exit_code", 1),
            ("failed", 1),
            ("filtered", 1),
            ("discovered", 3),
            ("ignored", 1),
        ] {
            let mut bad = report();
            bad["coverage"]["harnesses"][0][field] = json!(value);
            assert!(workspace_pass(&bad).is_err(), "{field}");
        }
        let mut missing = report();
        missing["coverage"]["harnesses"][0]
            .as_object_mut()
            .unwrap()
            .remove("failed");
        assert!(workspace_pass(&missing).is_err());
    }
    #[test]
    fn preserves_zero_harnesses_and_counts_helpers_separately() {
        let mut good = report();
        let row = &mut good["coverage"]["harnesses"][0];
        row["ignored"] = json!(1);
        row["ignored_subprocess_helpers"] = json!(1);
        row["discovered"] = json!(3);
        good["ignored_subprocess_helpers"] = json!(1);
        good["coverage"]["harnesses"].as_array_mut().unwrap().push(json!({"discovered":0,"passed":0,"failed":0,"ignored":0,"filtered":0,"exit_code":0,"validated":true}));
        good["coverage"]["required_harnesses"] = json!(2);
        good["coverage"]["executed_harnesses"] = json!(2);
        assert!(workspace_pass(&good).is_ok());
        good["ignored_subprocess_helpers"] = json!(0);
        assert!(workspace_pass(&good).is_err());
    }
    fn pilot_report() -> Value {
        let mut good = report();
        good["coverage"]["harnesses"][0] = json!({"discovered":4,"passed":2,"failed":0,"ignored":2,"filtered":0,"exit_code":0,"validated":true,"required_not_run":["actual_pinned_sdk_download_verifies"],"declared_subprocess_helpers":["unix_unprivileged_connect_probe"],"unsupported_not_run":[],"pilot_ids":["dotnet-sdk-download"],"unsupported_ids":[],"required_execution_scope":"job","pilot_receipt_verified":true});
        good["declared_pilot_ignored"] = json!(2);
        good
    }
    #[test]
    fn declared_pilot_names_are_accounted_only_with_a_verified_declaration() {
        assert!(workspace_pass(&pilot_report()).is_ok());
        // The aggregate must carry the declared names separately from helpers.
        let mut bad = pilot_report();
        bad["declared_pilot_ignored"] = json!(0);
        assert!(workspace_pass(&bad).is_err());
        bad["ignored_subprocess_helpers"] = json!(2);
        assert!(workspace_pass(&bad).is_err());
        let mut missing = pilot_report();
        missing
            .as_object_mut()
            .unwrap()
            .remove("declared_pilot_ignored");
        assert!(workspace_pass(&missing).is_err());
        // One extra ignored test beyond the declared names is unaccounted.
        let mut extra = pilot_report();
        extra["coverage"]["harnesses"][0]["ignored"] = json!(3);
        extra["coverage"]["harnesses"][0]["discovered"] = json!(5);
        extra["declared_pilot_ignored"] = json!(3);
        assert!(workspace_pass(&extra).is_err());
        for (field, value) in [
            ("pilot_receipt_verified", json!(false)),
            ("pilot_receipt_verified", Value::Null),
            ("pilot_ids", json!([])),
            ("required_execution_scope", Value::Null),
            ("required_execution_scope", json!("unsupported")),
            (
                "required_not_run",
                json!("actual_pinned_sdk_download_verifies"),
            ),
        ] {
            let mut bad = pilot_report();
            bad["coverage"]["harnesses"][0][field] = value;
            assert!(workspace_pass(&bad).is_err(), "{field}");
        }
        let mut unsupported = pilot_report();
        let row = &mut unsupported["coverage"]["harnesses"][0];
        row["required_execution_scope"] = json!("unsupported");
        row["pilot_ids"] = json!([]);
        row["unsupported_ids"] = json!(["dotnet-package-macos-unsupported"]);
        row["required_not_run"] = json!([]);
        row["declared_subprocess_helpers"] = json!([]);
        row["unsupported_not_run"] = json!(["a", "b"]);
        assert!(workspace_pass(&unsupported).is_ok());
        unsupported["coverage"]["harnesses"][0]["unsupported_ids"] = json!([]);
        assert!(workspace_pass(&unsupported).is_err());
    }
    #[test]
    fn row_ignored_separates_helpers_from_declared_names() {
        let row = &pilot_report()["coverage"]["harnesses"][0];
        assert_eq!(row_ignored(row).unwrap(), (0, 2));
        let mut captured = row.clone();
        captured["ignored_subprocess_helpers"] = json!(1);
        assert_eq!(row_ignored(&captured).unwrap(), (1, 2));
        let plain = json!({"ignored":0,"required_not_run":[]});
        assert_eq!(row_ignored(&plain).unwrap(), (0, 0));
    }
    #[test]
    fn passed_child_cannot_hide_missing_or_failed_actor_cleanup() {
        let mut good = report();
        good["standard_user_lifecycle"] = json!({"account_deleted":true,"scratch_deleted":true,"job_kill_on_close":true,"job_empty_observed":true,"child_exit_code":0,"child_launch_attempted":true,"actor_case":"normal","scm_delegation_requested":false});
        assert!(workspace_pass(&good).is_ok());
        for field in [
            "account_deleted",
            "scratch_deleted",
            "job_kill_on_close",
            "job_empty_observed",
        ] {
            for value in [Value::Null, json!(false)] {
                let mut bad = good.clone();
                bad["standard_user_lifecycle"][field] = value;
                assert!(workspace_pass(&bad).is_err(), "{field}");
            }
        }
    }
}
