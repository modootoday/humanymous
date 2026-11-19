//! Windows backend of the `standard-user-dotnet-prebuilt` custody provider
//! (SoT dotnet-protected-host-launch 4.1, owner decision D5(a), issue 75
//! comment 6998447599). The elevated hosted parent is the windows-native-runner;
//! this component only invokes it and checks its typed report.
use crate::{Result, command, read_json, require, source};
use serde_json::{Value, json};
use std::path::Path;

const RUNNER_MANIFEST: &str = "scripts/ci/windows-native-runner/Cargo.toml";
const FIXTURE_SCHEMA: &str = "humanymous.dotnet-n1-fixture/1";

/// Every fact the N1 fixture must report before anything else runs on Windows.
const N1_FACTS: [(&str, bool); 12] = [
    ("profile_absent_before", true),
    ("frame_written", true),
    ("natural_exit", true),
    ("job_empty_observed", true),
    ("stdout_joined", true),
    ("stderr_joined", true),
    ("child_line_exact", true),
    ("stderr_marker_exact", true),
    ("profile_absent_after", true),
    ("account_deleted", true),
    ("scratch_deleted", true),
    ("retained_contained", false),
];

/// The N1 report is accepted only with every fact observed and no failure.
pub fn n1_report_ok(report: &Value) -> bool {
    let facts = &report["facts"];
    report["schema"] == FIXTURE_SCHEMA
        && report["status"] == "passed"
        && report["failures"].as_array().is_some_and(Vec::is_empty)
        && facts["start"] == "resumed-in-job"
        && facts["child_exit_code"] == 0
        && N1_FACTS.iter().all(|(key, required)| {
            let value = &facts[*key];
            if *required {
                *value == true
            } else {
                value.is_null()
            }
        })
}

/// SoT 4.1 N1 fixture: the secondary-logon service delivers the std handles
/// of the NoProfile actor launch. A failure stops the Windows provider; the
/// `DuplicateHandle` fallback is not approved.
pub fn stdio_fixture(root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
    report["stage"] = json!("n1-fixture");
    let raw = dir.join("dotnet-n1-fixture.json");
    let log = dir.join("dotnet-n1-fixture.log");
    let raw_text = raw.to_str().ok_or("non-UTF-8 fixture report path")?;
    let code = command::log(
        root,
        "cargo",
        &[
            "run",
            "--locked",
            "--manifest-path",
            RUNNER_MANIFEST,
            "--",
            "dotnet-stdio-fixture",
            "--report",
            raw_text,
        ],
        &log,
    )?;
    report["commands"]["n1-fixture"] = json!({"exit_code":code,"log_sha256":source::file(&log)?});
    let value = read_json(&raw).unwrap_or(Value::Null);
    report["n1_fixture"] = json!({
        "status": value["status"],
        "facts": value["facts"],
        "failures": value["failures"],
    });
    require(code == 0, "N1 fixture runner failed")?;
    require(
        n1_report_ok(&value),
        "N1 std-handle delivery not observed: Windows provider stops",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn good() -> Value {
        json!({"schema":FIXTURE_SCHEMA,"status":"passed","failures":[],"facts":{
            "start":"resumed-in-job","child_exit_code":0,"profile_absent_before":true,
            "frame_written":true,"natural_exit":true,"job_empty_observed":true,
            "stdout_joined":true,"stderr_joined":true,"child_line_exact":true,
            "stderr_marker_exact":true,"profile_absent_after":true,
            "account_deleted":true,"scratch_deleted":true,"frame_bytes":98304}})
    }
    #[test]
    fn n1_report_requires_every_fact() {
        assert!(n1_report_ok(&good()));
        for (key, _) in N1_FACTS.iter().filter(|(_, required)| *required) {
            let mut value = good();
            value["facts"][*key] = json!(false);
            assert!(!n1_report_ok(&value), "{key}");
            value["facts"].as_object_mut().unwrap().remove(*key);
            assert!(!n1_report_ok(&value), "{key}");
        }
        let mut value = good();
        value["facts"]["retained_contained"] = json!(true);
        assert!(!n1_report_ok(&value));
        for (path, wrong) in [
            ("status", json!("failed")),
            ("schema", json!("other")),
            ("failures", json!(["x"])),
        ] {
            let mut value = good();
            value[path] = wrong;
            assert!(!n1_report_ok(&value), "{path}");
        }
        let mut value = good();
        value["facts"]["child_exit_code"] = json!(3);
        assert!(!n1_report_ok(&value));
        value["facts"]["child_exit_code"] = json!(0);
        value["facts"]["start"] = json!("not-started");
        assert!(!n1_report_ok(&value));
    }
}
