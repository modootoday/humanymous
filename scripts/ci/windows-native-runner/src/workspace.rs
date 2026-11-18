//! Default-feature whole-workspace component probe, never native product readiness.
use crate::{Options, evidence, fixture, harness};
use serde_json::{Value, json};
use std::{fs, io};

pub fn run(options: &Options) -> io::Result<i32> {
    let identity = evidence::identity(options.expected_sid.as_deref())?;
    fixture::actor_environment(options.expected_sid.as_deref())?;
    let source = match &options.source {
        Some(path) => serde_json::from_slice(&fs::read(path)?)?,
        None => evidence::source(true)?,
    };
    evidence::verify_snapshot(&source)?;
    let mut report = json!({"schema":"humanymous.workspace-probe/1","kind":"workspace","scope":"workspace-default-features","qualification":"implemented-workspace-component","full_runtime_qualified":false,"status":"failed","source_sha":source["source_sha"],"source":source,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"actor":identity,"errors":0,"skips":0});
    // Closed stage names for the public summary; the error text stays private.
    let stage = std::cell::Cell::new("prepare");
    // Closed accounting reason codes, published next to the stage.
    let reason = std::cell::Cell::new(None);
    let result = (|| -> io::Result<()> {
        let manifest = match &options.manifest {
            Some(path) => serde_json::from_slice(&fs::read(path)?)?,
            None => harness::prepare(
                &options.report,
                &options.binaries,
                harness::Scope::Workspace,
                &report["source"],
            )?,
        };
        if manifest["scope"] != "workspace-default-features" {
            return Err(io::Error::other("default full workspace manifest required"));
        }
        report["manifest"] = manifest.clone();
        stage.set("packages");
        let packages = harness::packages::Packages::load(
            &report["source"],
            &options.report,
            options.package_metadata.as_deref(),
        )?;
        stage.set("execute");
        harness::execute(
            &manifest["harnesses"],
            &options.report,
            &mut report["coverage"],
            harness::Scope::Workspace,
            &packages,
        )?;
        stage.set("accounting");
        account(&mut report).map_err(|code| {
            reason.set(Some(code));
            io::Error::other(format!("workspace accounting refused: {code}"))
        })?;
        stage.set("snapshot");
        evidence::verify_snapshot(&report["source"])?;
        Ok(())
    })();
    match result {
        Ok(()) => report["status"] = "passed".into(),
        Err(error) => {
            report["errors"] = 1.into();
            report["error"] = error.to_string().into();
            report["error_stage"] = stage.get().into();
            report["error_kind"] = format!("{:?}", error.kind()).into();
            if let Some(code) = reason.get() {
                report["error_reason"] = code.into();
            }
        }
    }
    evidence::write(&options.report, &report)?;
    Ok(if report["status"] == "passed" { 0 } else { 1 })
}

/// Accounting refusals that are not harness failures: they are aggregate
/// checks after execution, so `coverage.failures` never names them. The code
/// is one of [`ACCOUNTING_REASONS`] and is published; the text stays private.
pub(crate) const ACCOUNTING_REASONS: [&str; 6] = [
    "coverage-absent",
    "declaration-unverified",
    "row-ignored-unaccounted",
    "count-overflow",
    "no-passed-tests",
    "ignored-unaccounted",
];
/// Every ignored test must be a counted subprocess helper or a name its
/// verified pilot/unsupported declaration admitted, per row and in aggregate.
fn account(report: &mut Value) -> Result<(), &'static str> {
    let rows = report["coverage"]["harnesses"]
        .as_array()
        .ok_or("coverage-absent")?;
    let mut totals = [0u64; 4];
    for row in rows {
        let (helpers, declared) =
            evidence::accounting::row_ignored(row).map_err(|_| "declaration-unverified")?;
        let ignored = row["ignored"].as_u64().ok_or("row-ignored-unaccounted")?;
        if helpers.checked_add(declared) != Some(ignored) {
            return Err("row-ignored-unaccounted");
        }
        let passed = row["passed"].as_u64().unwrap_or(0);
        for (total, count) in totals.iter_mut().zip([passed, ignored, helpers, declared]) {
            *total = total.checked_add(count).ok_or("count-overflow")?;
        }
    }
    let [passed, ignored, helpers, declared] = totals;
    let zero = rows.iter().filter(|r| r["discovered"] == 0).count();
    report["passed_tests"] = passed.into();
    report["raw_ignored_tests"] = ignored.into();
    report["ignored_subprocess_helpers"] = helpers.into();
    report["declared_pilot_ignored"] = declared.into();
    report["zero_test_harnesses"] = json!(zero);
    if passed == 0 {
        return Err("no-passed-tests");
    }
    if helpers.checked_add(declared) != Some(ignored) {
        return Err("ignored-unaccounted");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn report(rows: Value) -> Value {
        json!({"coverage":{"harnesses":rows}})
    }
    #[test]
    fn declared_pilot_ignores_are_accounted_beside_helpers() {
        // Shape of the Linux x64 workspace rows that refused before: helper
        // rows plus receipt- and job-executed pilot rows.
        let mut good = report(json!([
            {"passed":8,"ignored":2,"discovered":10,"ignored_subprocess_helpers":2},
            {"passed":7,"ignored":2,"discovered":9,"required_not_run":["unix_other_user_is_denied_by_actual_socket_acl"],"declared_subprocess_helpers":["unix_unprivileged_connect_probe"],"pilot_ids":["authority-unix-socket-acl"],"required_execution_scope":"native-pilots","pilot_receipt_verified":true},
            {"passed":186,"ignored":1,"discovered":187,"required_not_run":["components::dotnet_host::sdk::tests::actual_pinned_sdk_download_verifies"],"declared_subprocess_helpers":[],"pilot_ids":["dotnet-sdk-download"],"required_execution_scope":"job","pilot_receipt_verified":true},
            {"passed":0,"ignored":0,"discovered":0}
        ]));
        assert_eq!(account(&mut good), Ok(()));
        assert_eq!(good["raw_ignored_tests"], 5);
        assert_eq!(good["ignored_subprocess_helpers"], 2);
        assert_eq!(good["declared_pilot_ignored"], 3);
        assert_eq!(good["passed_tests"], 201);
        assert_eq!(good["zero_test_harnesses"], 1);
    }
    #[test]
    fn every_refusal_names_a_closed_reason() {
        let pilot = json!({"passed":1,"ignored":1,"discovered":2,"required_not_run":["a"],"pilot_ids":["p"],"required_execution_scope":"job","pilot_receipt_verified":true});
        let mut unverified = pilot.clone();
        unverified["pilot_receipt_verified"] = json!(false);
        let mut undeclared = pilot.clone();
        undeclared["ignored"] = json!(2);
        let mut unknown = pilot.clone();
        unknown["ignored"] = Value::Null;
        let mut huge = pilot.clone();
        huge["passed"] = json!(u64::MAX);
        let cases = [
            (json!({}), "coverage-absent"),
            (report(json!([unverified])), "declaration-unverified"),
            (report(json!([undeclared])), "row-ignored-unaccounted"),
            (report(json!([unknown])), "row-ignored-unaccounted"),
            (report(json!([huge.clone(), huge])), "count-overflow"),
            (report(json!([{"passed":0,"ignored":0}])), "no-passed-tests"),
        ];
        for (mut value, code) in cases {
            assert_eq!(account(&mut value), Err(code));
            assert!(ACCOUNTING_REASONS.contains(&code));
        }
    }
}
