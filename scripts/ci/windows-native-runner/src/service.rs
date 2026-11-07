//! Opt-in disposable native manager pilot; never executes on a local workstation.
use crate::{Options, evidence, harness};
use humanymous_test_support::process::capture_with_stderr;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, io, path::Path, process::Command, time::Duration};

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
        None => evidence::source()?,
    };
    evidence::verify_snapshot(&source)?;
    let mut report = json!({"schema":1,"kind":"native","qualification":"component-only","status":"blocked","source_sha":source["source_sha"],"source":source,"actor":actor,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"documentation_review_sha256":evidence::digest(Path::new("docs/reviews.json"))?,"documentation_examples_executed":false,"full_runtime_qualified":false,"errors":0,"skips":0,"cases":[],"missing_required_cases":["startup","configuration-invalid","doctor","install-dry-run-no-mutation","local-only-admin","restart-replay","storage-failure-closed","documentation-examples"]});
    let result = (|| -> io::Result<()> {
        let manifest = match &options.manifest {
            Some(path) => serde_json::from_slice(&fs::read(path)?)?,
            None => harness::prepare(&options.report, &options.binaries)?,
        };
        let entry = manifest["harnesses"]
            .as_array()
            .and_then(|rows| {
                rows.iter().find(|r| {
                    r["target"] == "service_process"
                        && r["package_id"]
                            .as_str()
                            .is_some_and(|id| id.contains("humanymous-cli"))
                })
            })
            .ok_or_else(|| io::Error::other("CLI native service harness absent"))?;
        let executable = Path::new(
            entry["executable"]
                .as_str()
                .ok_or_else(|| io::Error::other("service executable absent"))?,
        );
        if evidence::digest(executable)? != entry["sha256"] {
            return Err(io::Error::other("service executable digest mismatch"));
        }
        let output = capture_with_stderr(
            Command::new(executable)
                .args([
                    "--ignored",
                    "--exact",
                    "native_service_install_upgrade_rollback_and_remove",
                    "--nocapture",
                ])
                .env("HUMANYMOUS_DISPOSABLE_SERVICE_ACTOR", "1"),
            b"",
            Duration::from_secs(180),
            1024 * 1024,
        )?;
        fs::write(
            options.report.with_extension("service-stdout.log"),
            &output.stdout,
        )?;
        fs::write(
            options.report.with_extension("service-stderr.log"),
            &output.stderr,
        )?;
        let text = std::str::from_utf8(&output.stdout).map_err(io::Error::other)?;
        let totals = harness::totals(text)?;
        report["manifest"] = manifest.clone();
        report["service_execution"] = json!({"test":"native_service_install_upgrade_rollback_and_remove","exit_code":output.status.code(),"passed":totals[0],"failed":totals[1],"ignored":totals[2],"measured":totals[3],"nonselected_tests":totals[4],"stdout_sha256":format!("{:x}",Sha256::digest(&output.stdout))});
        if !output.status.success() || totals[0..4] != [1, 0, 0, 0] {
            return Err(io::Error::other(
                "required actual native service lifecycle did not pass",
            ));
        }
        report["cases"] = json!([{"id":"native-service-lifecycle","status":"passed","evidence":"actual signed fixture install/repeat/upgrade/rollback/remove through native service manager"}]);
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
