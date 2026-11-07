//! Default-feature whole-workspace component probe, never native product readiness.
use crate::{Options, evidence, fixture, harness};
use serde_json::json;
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
        let packages = harness::packages::Packages::load(&report["source"], &options.report)?;
        harness::execute(
            &manifest["harnesses"],
            &options.report,
            &mut report["coverage"],
            harness::Scope::Workspace,
            &packages,
        )?;
        let rows = report["coverage"]["harnesses"]
            .as_array()
            .ok_or_else(|| io::Error::other("workspace coverage absent"))?;
        let passed = rows
            .iter()
            .filter_map(|r| r["passed"].as_u64())
            .sum::<u64>();
        let ignored = rows
            .iter()
            .filter_map(|r| r["ignored"].as_u64())
            .sum::<u64>();
        let helpers = rows
            .iter()
            .filter_map(|r| r["ignored_subprocess_helpers"].as_u64())
            .sum::<u64>();
        let zero = rows.iter().filter(|r| r["discovered"] == 0).count();
        if passed == 0 || ignored != helpers {
            return Err(io::Error::other(
                "empty/incomplete required workspace tests",
            ));
        }
        report["passed_tests"] = passed.into();
        report["raw_ignored_tests"] = ignored.into();
        report["ignored_subprocess_helpers"] = helpers.into();
        report["zero_test_harnesses"] = json!(zero);
        evidence::verify_snapshot(&report["source"])?;
        Ok(())
    })();
    match result {
        Ok(()) => report["status"] = "passed".into(),
        Err(error) => {
            report["errors"] = 1.into();
            report["error"] = error.to_string().into();
        }
    }
    evidence::write(&options.report, &report)?;
    Ok(if report["status"] == "passed" { 0 } else { 1 })
}
