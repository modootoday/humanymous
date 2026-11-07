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
    let mut passed = 0u64;
    let mut ignored = 0u64;
    for row in rows {
        let p = number(row, "passed")?;
        let i = number(row, "ignored")?;
        let helpers = match row.get("ignored_subprocess_helpers") {
            None => 0,
            Some(value) => value
                .as_u64()
                .ok_or_else(|| io::Error::other("invalid helper count"))?,
        };
        if row["validated"] != true
            || row["exit_code"].as_i64() != Some(0)
            || number(row, "failed")? != 0
            || number(row, "filtered")? != 0
            || p.checked_add(i) != Some(number(row, "discovered")?)
            || i != helpers
        {
            return Err(io::Error::other("unvalidated workspace row"));
        }
        passed = passed
            .checked_add(p)
            .ok_or_else(|| io::Error::other("passed count overflow"))?;
        ignored = ignored
            .checked_add(i)
            .ok_or_else(|| io::Error::other("ignored count overflow"))?;
    }
    if passed == 0 || ignored != number(report, "ignored_subprocess_helpers")? {
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
        json!({"errors":0,"skips":0,"ignored_subprocess_helpers":0,"coverage":{"complete":true,"required_harnesses":1,"executed_harnesses":1,"harnesses":[{"discovered":2,"passed":2,"failed":0,"ignored":0,"filtered":0,"exit_code":0,"validated":true}]}})
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
