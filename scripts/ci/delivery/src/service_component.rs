//! Standalone hosted service observation; the owner validates cases and cleanup.
use crate::{Result, command, evidence, require, source, write_json};
use serde_json::{Value, json};
use std::path::Path;

const MANIFEST: &str = "scripts/ci/windows-native-runner/Cargo.toml";

fn hosted() -> Result<String> {
    require(
        cfg!(windows)
            && std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
            && std::env::var("RUNNER_ENVIRONMENT").as_deref() == Ok("github-hosted")
            && std::env::var_os("ACT").is_none()
            && std::env::var("GITHUB_REPOSITORY").as_deref() == Ok("modootoday/humanymous"),
        "service component requires the disposable hosted Windows venue",
    )?;
    let run = std::env::var("GITHUB_RUN_ID").map_err(|_| "hosted run absent")?;
    require(
        run.parse::<u64>()
            .is_ok_and(|n| n > 0 && n.to_string() == run),
        "hosted run ID refused",
    )?;
    Ok(run)
}

fn summary(value: &Value, sha: &str, run: &str, case: &str, exit: i32) -> Result<()> {
    require(
        matches!((case, exit), ("normal", 2) | ("after-grant", 1))
            && value["schema"] == "humanymous.service-component-summary/1"
            && value["verified"] == true
            && value["source_sha"] == sha
            && value["run_id"]
                .as_u64()
                .is_some_and(|n| n.to_string() == run)
            && value["actor_case"] == case
            && value["observed_exit_code"] == exit
            && value["qualification"] == "component-only"
            && value["full_runtime_qualified"] == false
            && value["product_qualified"] == false
            && value["native_producer_qualified"] == false
            && value["cleanup_verified"] == true
            && crate::lab_receipts::digest(&value["raw_report_sha256"], 64)
            && crate::lab_receipts::digest(&value["source_manifest_sha256"], 64),
        "owning service summary identity or scope refused",
    )
}

fn invoke(root: &Path, dir: &Path, sha: &str, run: &str, case: &str) -> Result<Value> {
    let raw = dir.join(format!("service-{case}.json"));
    let public = dir.join(format!("service-{case}-summary.json"));
    let binaries = root.join("target/release");
    let raw_text = raw.to_str().ok_or("service report path invalid")?;
    let mut args = vec![
        "run",
        "--locked",
        "--manifest-path",
        MANIFEST,
        "--",
        "standard-user-service",
        "--report",
        raw_text,
        "--binaries",
        binaries.to_str().ok_or("service binary path invalid")?,
    ];
    if case == "after-grant" {
        args.extend(["--actor-case", "after-grant"]);
    }
    // Neither normal exit 2 nor the injected negative exit 1 is accepted here
    // alone. The sole owner must revalidate original logs, cases and cleanup.
    let code = command::json_log(
        root,
        "cargo",
        &args,
        &dir.join(format!("service-{case}-owner.log")),
    )?;
    let observed = code.to_string();
    let validated = command::json_log(
        root,
        "cargo",
        &[
            "run",
            "--locked",
            "--manifest-path",
            MANIFEST,
            "--",
            "summarize-service",
            "--source",
            raw_text,
            "--report",
            public.to_str().ok_or("service summary path invalid")?,
            "--actor-case",
            case,
            "--expected-source-sha",
            sha,
            "--expected-run-id",
            run,
            "--observed-exit-code",
            &observed,
        ],
        &dir.join(format!("service-{case}-validation.log")),
    )?;
    require(validated == 0, "owning service evidence validation failed")?;
    let bytes = std::fs::read(&public).map_err(|_| "service summary missing")?;
    let value = humanymous_platform::testing::json::decode_strict_json(&bytes, 64 * 1024)
        .map_err(|_| "service summary JSON refused")?;
    summary(&value, sha, run, case, code)?;
    require(
        value["raw_report_sha256"] == source::file(&raw)?,
        "service summary original report binding refused",
    )?;
    // Rebuild the public projection; extra owner fields never become log data.
    Ok(
        json!({"actor_case":case,"observed_exit_code":code,"owner_verified":true,
        "cleanup_verified":true,"owner_summary_sha256":source::sha(&bytes),
        "raw_report_sha256":value["raw_report_sha256"],
        "source_manifest_sha256":value["source_manifest_sha256"],
        "service_tests_verified":if case == "normal" {2} else {0},
        "runtime_receipt_verified":if case == "normal" {json!(true)} else {Value::Null},
        "product_qualified":false,"native_producer_qualified":false}),
    )
}

pub fn run(root: &Path) -> Result<()> {
    let run = hosted()?;
    let before = source::snapshot(root)?;
    require(
        before["dirty"] == false,
        "service component requires clean candidate bytes",
    )?;
    let sha = before["source_sha"]
        .as_str()
        .ok_or("service candidate SHA absent")?;
    require(
        std::env::var("GITHUB_SHA").as_deref() == Ok(sha),
        "hosted service source mismatch",
    )?;
    let dir = evidence::bundle(root)?;
    write_json(&dir.join("source-before.json"), &before)?;
    let mut report = json!({"schema":"humanymous.hosted-service-component/1",
        "source_sha":sha,"run_id":run,"status":"failed","qualification":"component-only",
        "full_product_qualified":false,"release_ready":false,"cases":[],
        "original_clock_qualified":false,"native_producer_qualified":false});
    let result: Result<()> = (|| {
        for case in ["normal", "after-grant"] {
            let observed = invoke(root, &dir, sha, &run, case)?;
            report["cases"]
                .as_array_mut()
                .ok_or("service case list absent")?
                .push(observed);
        }
        Ok(())
    })();
    let after = source::snapshot(root)?;
    write_json(&dir.join("source-after.json"), &after)?;
    let unchanged = before["source_sha"] == after["source_sha"]
        && before["manifest_sha256"] == after["manifest_sha256"]
        && after["dirty"] == false;
    report["source_unchanged"] = json!(unchanged);
    report["source_manifest_sha256"] = before["manifest_sha256"].clone();
    if result.is_ok() && unchanged {
        report["status"] = json!("passed");
    }
    write_json(&dir.join("service-component.json"), &report)?;
    report["private_report_sha256"] = json!(source::file(&dir.join("service-component.json"))?);
    evidence::register(
        &dir,
        "Hosted service component and injected no-launch cleanup",
    )?;
    evidence::emit_public_summary(
        "Hosted service component; full product remains unqualified",
        &report,
    )?;
    result?;
    require(unchanged, "service component source changed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supplied_owner_summary_cannot_cross_source_scope_or_cleanup_boundaries() {
        let sha = "a".repeat(40);
        let good = json!({"schema":"humanymous.service-component-summary/1","verified":true,
            "source_sha":sha,"run_id":7,"actor_case":"normal","observed_exit_code":2,
            "qualification":"component-only","full_runtime_qualified":false,"product_qualified":false,
            "native_producer_qualified":false,"cleanup_verified":true,
            "raw_report_sha256":"b".repeat(64),"source_manifest_sha256":"c".repeat(64)});
        assert!(summary(&good, &sha, "7", "normal", 2).is_ok());
        for key in [
            "verified",
            "cleanup_verified",
            "source_sha",
            "run_id",
            "actor_case",
            "observed_exit_code",
            "raw_report_sha256",
            "source_manifest_sha256",
        ] {
            let mut bad = good.clone();
            bad[key] = Value::Null;
            assert!(summary(&bad, &sha, "7", "normal", 2).is_err());
        }
        for key in [
            "full_runtime_qualified",
            "product_qualified",
            "native_producer_qualified",
        ] {
            let mut bad = good.clone();
            bad[key] = json!(true);
            assert!(summary(&bad, &sha, "7", "normal", 2).is_err());
        }
        assert!(summary(&good, &sha, "7", "after-grant", 1).is_err());
        assert!(summary(&good, &sha, "8", "normal", 2).is_err());
    }
}
