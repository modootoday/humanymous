//! Opt-in disposable native manager pilot; never executes on a local workstation.
pub(crate) mod catalog;
mod execution;
pub(crate) mod receipt;
use crate::{Options, evidence, harness};
use serde_json::json;
use std::{fs, io, path::Path};

pub fn run(options: &Options) -> io::Result<i32> {
    if !evidence::disposable_host() {
        return Err(io::Error::other(
            "explicit disposable GitHub-hosted service actor required",
        ));
    }
    let actor = evidence::identity(options.expected_sid.as_deref())?;
    crate::fixture::actor_environment(options.expected_sid.as_deref())?;
    let source = match &options.source {
        Some(path) => serde_json::from_slice(&fs::read(path)?)?,
        None => evidence::source(false)?,
    };
    evidence::verify_snapshot(&source)?;
    let mut report = json!({"schema":1,"kind":"native","qualification":"component-only","status":"blocked","source_sha":source["source_sha"],"source":source,"actor":actor,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"documentation_review_sha256":evidence::digest(Path::new("docs/reviews.json"))?,"documentation_examples_executed":false,"full_runtime_qualified":false,"errors":0,"skips":0,"cases":[],"missing_required_cases":["startup","configuration-invalid","doctor","install-dry-run-no-mutation","local-only-admin","restart-replay","storage-failure-closed","documentation-examples"]});
    let result = (|| -> io::Result<()> {
        let manifest = match &options.manifest {
            Some(path) => serde_json::from_slice(&fs::read(path)?)?,
            None => harness::prepare(
                &options.report,
                &options.binaries,
                harness::Scope::Native,
                &report["source"],
            )?,
        };
        report["manifest"] = manifest.clone();
        let rows = manifest["harnesses"]
            .as_array()
            .ok_or_else(|| io::Error::other("service manifest absent"))?;
        for case in &catalog::CASES {
            if !case.implemented_for_target() {
                report[case.field] =
                    json!({"test":case.test,"status":"unsupported","validated":false});
                report["missing_required_cases"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!(case.id));
                continue;
            }
            let entry = rows
                .iter()
                .find(|row| {
                    catalog::required_case(row).is_some_and(|found| found.test == case.test)
                })
                .ok_or_else(|| io::Error::other("required native service harness absent"))?;
            let result = execution::run(entry, case, &options.report, &report["source"]);
            report[case.field] = match result {
                Ok(row) => row,
                Err(error) => {
                    report[case.field] =
                        json!({"test":case.test,"validated":false,"status":"failed"});
                    return Err(error);
                }
            };
            if report[case.field]["validated"] != true {
                return Err(io::Error::other(
                    "required actual native service pilot did not pass",
                ));
            }
            report["cases"]
                .as_array_mut()
                .unwrap()
                .push(json!({"id":case.id,"status":"passed"}));
        }
        evidence::verify_snapshot(&report["source"])?;
        Ok(())
    })();
    if let Err(error) = result {
        report["status"] = "failed".into();
        report["errors"] = 1.into();
        report["error"] = error.to_string().into();
    }
    evidence::write(&options.report, &report)?;
    Ok(if report["errors"] == 0 { 2 } else { 1 })
}
