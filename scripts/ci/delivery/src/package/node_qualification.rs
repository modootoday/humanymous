//! Same-job producer of target/node-native/qualification.json.
//!
//! Every field is derived from two actual executions on this exact source: the
//! passed node-host component bundle (Docker Node/Bun host groups against the
//! real authority executable) and the production package build/inspection this
//! command runs. Nothing is copied from fixtures or earlier runs; a missing or
//! mismatched input refuses the receipt. `node_receipt` remains the verifier.
use crate::components::node_host;
use crate::{Result, command, evidence, read_json, require, source, write_json};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const RUNTIMES: [&str; 2] = ["node", "bun"];
const PRODUCTION: &str = "production-application-http";

fn hex64(value: &Value) -> bool {
    value
        .as_str()
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
}
/// Tuple fields come from the addon identity the host image actually loaded.
fn tuple_fields(tuple: &str) -> Result<(&'static str, &'static str, &'static str)> {
    match tuple {
        "linux-x64-gnu" => Ok(("linux", "x64", "gnu")),
        _ => Err(format!("no qualification mapping for native tuple {tuple}")),
    }
}
/// Newest node-host component bundle: an older pass can never mask a newer run.
fn component_bundle(root: &Path) -> Result<(PathBuf, Value)> {
    let parent = root.join(".spec.local/root/evidence");
    let mut bundles = Vec::new();
    for entry in std::fs::read_dir(&parent).map_err(|e| format!("{}: {e}", parent.display()))? {
        let directory = entry.map_err(|e| e.to_string())?.path();
        let report = directory.join("component.json");
        if !report.is_file() {
            continue;
        }
        let value = read_json(&report)?;
        if value["component"] == "node-host" {
            bundles.push((directory, value));
        }
    }
    bundles.sort_by(|left, right| left.0.cmp(&right.0));
    bundles
        .pop()
        .ok_or_else(|| "same-job node-host component bundle missing".into())
}
/// Read a stage log only when its bytes are the ones the component recorded.
fn stage_log(directory: &Path, report: &Value, stage: &str) -> Result<String> {
    let path = directory.join(format!("{stage}.log"));
    let recorded = &report["commands"][stage];
    require(
        recorded["exit_code"] == 0 && recorded["stdout_sha256"] == source::file(&path)?,
        format!("node-host stage log missing, failed or changed: {stage}"),
    )?;
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}
/// Verify the component report and re-derive every value it claims from its logs.
fn host_evidence(root: &Path, snapshot: &Value, directory: &Path, report: &Value) -> Result<Value> {
    require(
        report["schema"] == "humanymous.hosted-component/1"
            && report["status"] == "passed"
            && report["source_changed"] == false
            && report["full_product_qualified"] == false
            && report["container_cleanup_observed"] == true
            && report["runner_os"] == "linux"
            && report["runner_arch"] == "x86_64"
            && report["source_sha"] == snapshot["source_sha"]
            && report["source_manifest_sha256"] == snapshot["manifest_sha256"]
            && report["cargo_lock_sha256"] == source::file(&root.join("Cargo.lock"))?
            && report["toolchain_sha256"] == source::file(&root.join("rust-toolchain.toml"))?,
        "node-host component did not pass on this exact clean source",
    )?;
    let sha = report["source_sha"]
        .as_str()
        .ok_or("node-host source absent")?;
    let manifest = report["source_manifest_sha256"]
        .as_str()
        .ok_or("node-host manifest absent")?;
    let inspected: Value =
        serde_json::from_str(&stage_log(directory, report, "node-image-inspect")?)
            .map_err(|e| e.to_string())?;
    let image_id = node_host::image(&inspected, sha, manifest)?;
    require(
        report["image_id"] == image_id.as_str(),
        "node-host tested image identity differs from its inspection",
    )?;
    let tuple = node_host::addon_tuple(&inspected)?;
    let authority_path = node_host::authority_path(&inspected)?;
    let authority = &report["authority_artifact"];
    require(
        authority["path"] == authority_path && hex64(&authority["sha256"]),
        "tested authority executable digest missing",
    )?;
    stage_log(directory, report, "node-authority-copy")?;
    let tests = stage_log(directory, report, "node-host-tests")?;
    let counts = node_host::counts(&tests)?;
    require(
        counts == report["test_counts"] && counts["failed"] == 0 && counts["ignored"] == 0,
        "node-host test counts differ from the recorded harness output",
    )?;
    let mut versions = serde_json::Map::new();
    for runtime in RUNTIMES {
        let observed = stage_log(directory, report, &format!("{runtime}-version"))?;
        let observed = observed.trim();
        require(
            !observed.is_empty() && report["runtime_versions"][runtime] == observed,
            format!("{runtime} observed host version differs from its probe"),
        )?;
        versions.insert(runtime.into(), json!(observed));
    }
    Ok(json!({
        "image_id": image_id,
        "tuple": tuple,
        "authority_sha256": authority["sha256"],
        "versions": versions,
        "counts": counts,
        "rows": node_host::rows(&tests),
    }))
}
/// One host run per runtime, counted only from rows the owning test printed.
fn host_run(runtime: &str, host: &Value) -> Result<Value> {
    let tuple = host["tuple"].as_str().ok_or("host tuple absent")?;
    let (os, arch, libc) = tuple_fields(tuple)?;
    let rows: Vec<&Value> = host["rows"]
        .as_array()
        .ok_or("host rows absent")?
        .iter()
        .filter(|row| row["runtime"] == runtime)
        .collect();
    let production: Vec<&&Value> = rows
        .iter()
        .filter(|row| row["authority"] == PRODUCTION)
        .collect();
    let mut verdicts = json!({"ALLOW":0,"CHALLENGE":0,"DENY":0,"error":0});
    let mut origin_effects = 0;
    for row in &production {
        require(
            row["status"] == "passed",
            "production authority row did not pass",
        )?;
        let verdict = row["verdict"]
            .as_str()
            .filter(|verdict| ["CHALLENGE", "DENY"].contains(verdict))
            .ok_or("production authority row lacks an observed non-ALLOW verdict")?;
        verdicts[verdict] = json!(verdicts[verdict].as_u64().unwrap_or(0) + 1);
        origin_effects += row["originEffects"]
            .as_u64()
            .ok_or("production origin effect count invalid")?;
    }
    require(
        production.len() == 2,
        format!("{runtime} needs actual Express and Hono production authority rows"),
    )?;
    let mut profiles = serde_json::Map::new();
    for row in &rows {
        let profile = row["authority"].as_str().ok_or("row profile absent")?;
        let cases = match profile {
            PRODUCTION => 1,
            "real-engine-synthetic-evidence" => {
                row["allowCases"]
                    .as_u64()
                    .ok_or("synthetic ALLOW invalid")?
                    + row["negativeCases"]
                        .as_u64()
                        .ok_or("synthetic negatives invalid")?
            }
            "unavailable-negative" => row["cases"].as_u64().ok_or("unavailable cases invalid")?,
            _ => return Err("unknown host row profile".into()),
        };
        let total = profiles.entry(profile).or_insert(json!(0));
        *total = json!(total.as_u64().unwrap_or(0) + cases);
    }
    Ok(json!({
        "runtime": runtime,
        "version": host["versions"][runtime],
        "os": os,
        "arch": arch,
        "libc": libc,
        "target": tuple,
        "status": "passed",
        "mock": false,
        "evidence_profile": PRODUCTION,
        "test_count": production.len(),
        "failed": host["counts"]["failed"],
        "ignored": host["counts"]["ignored"],
        "verdict_counts": verdicts,
        "origin_effect_count": origin_effects,
        "authority_artifact_sha256": host["authority_sha256"],
        "image_id": host["image_id"],
        "component_case_counts": profiles,
        "production_browser_evidence": false,
    }))
}
/// Build and inspect the production package; inspection loads both generated
/// loaders under host Node and Bun and requires the production feature receipt.
fn package_evidence(root: &Path, tuple: &str) -> Result<Value> {
    let folder = root.join("adapters/node");
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    command::status(&folder, npm, &["ci", "--ignore-scripts"])?;
    let output = command::text(
        root,
        "cargo",
        &[
            "run",
            "--locked",
            "-p",
            "humanymous-node-adapter",
            "--bin",
            "humanymous-node-package",
            "--",
            "build",
        ],
    )?;
    print!("{output}");
    let loads: Vec<Value> = output
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|row| row["adapter"] == "humanymous.node")
        .collect();
    let observation = |mode: &str| {
        loads
            .iter()
            .filter(|row| row["expressObservation"] == mode)
            .count()
    };
    require(
        loads.len() == 4
            && loads.iter().all(|row| {
                row["implementation"] == "rust-node-api" && row["hostPilotEnabled"] == false
            })
            && observation("node-raw") == 2
            && observation("fetch-normalized") == 2,
        "production loaders did not load under both host Node and Bun",
    )?;
    let node = command::text(root, "node", &["--version"])?
        .trim()
        .to_owned();
    let bun = command::text(root, "bun", &["--version"])?
        .trim()
        .to_owned();
    require(
        node.starts_with("v24.") && bun == "1.3.14",
        "host production import runtimes are not Node 24 and Bun 1.3.14",
    )?;
    let provenance = read_json(&folder.join("native/provenance.json"))?;
    let binary = format!("humanymous-node.{tuple}.node");
    let binary_sha256 = provenance["files"]
        .as_array()
        .ok_or("native provenance files absent")?
        .iter()
        .find(|file| file["path"] == binary.as_str())
        .map(|file| file["sha256"].clone())
        .filter(hex64)
        .ok_or("production native binary for the tested tuple is absent")?;
    require(
        provenance["generator"] == "@napi-rs/cli"
            && provenance["version"] == "3.10.12"
            && provenance["package_manifest_sha256"] == source::file(&folder.join("package.json"))?,
        "native provenance does not bind this package manifest",
    )?;
    Ok(json!({
        "loader_loads": loads.len(),
        "host_pilot_enabled": false,
        "node_version": node,
        "bun_version": bun,
        "native_binary": binary,
        "native_binary_sha256": binary_sha256,
    }))
}
fn hosted_source(root: &Path) -> Result<Value> {
    require(
        cfg!(all(target_os = "linux", target_arch = "x86_64"))
            && std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
            && std::env::var_os("ACT").is_none()
            && std::env::var("RUNNER_ENVIRONMENT").as_deref() == Ok("github-hosted"),
        "Node qualification requires the standard hosted linux-x64 runner",
    )?;
    let snapshot = source::snapshot(root)?;
    require(
        snapshot["dirty"] == false
            && snapshot["source_sha"] == std::env::var("GITHUB_SHA").unwrap_or_default(),
        "Node qualification requires clean exact workflow source",
    )?;
    Ok(snapshot)
}
pub fn produce(root: &Path) -> Result<()> {
    let snapshot = hosted_source(root)?;
    let (directory, report) = component_bundle(root)?;
    let host = host_evidence(root, &snapshot, &directory, &report)?;
    let tuple = host["tuple"]
        .as_str()
        .ok_or("host tuple absent")?
        .to_owned();
    let package = package_evidence(root, &tuple)?;
    let after = source::snapshot(root)?;
    require(
        after["dirty"] == false
            && after["source_sha"] == snapshot["source_sha"]
            && after["manifest_sha256"] == snapshot["manifest_sha256"],
        "source changed while producing Node qualification",
    )?;
    let runs = RUNTIMES
        .iter()
        .map(|runtime| host_run(runtime, &host))
        .collect::<Result<Vec<_>>>()?;
    let (os, arch, libc) = tuple_fields(&tuple)?;
    // Derived, not asserted: every run passed with zero failed/ignored cases,
    // the tested image was cleaned up and both production loaders loaded.
    let qualified = runs.iter().all(|run| {
        run["status"] == "passed"
            && run["mock"] == false
            && run["failed"] == 0
            && run["ignored"] == 0
            && run["test_count"].as_u64().is_some_and(|count| count > 0)
            && run["os"] == os
            && run["arch"] == arch
    }) && report["container_cleanup_observed"] == true
        && package["loader_loads"] == 4;
    require(
        qualified,
        "Node/Bun host evidence does not qualify the tuple",
    )?;
    let folder = root.join("adapters/node");
    let manifest = read_json(&folder.join("package.json"))?;
    let stamp = directory
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.get(..14))
        .ok_or("component evidence identifier absent")?;
    let receipt = json!({
        "schema": "humanymous.node-native-qualification/1",
        "source_sha": snapshot["source_sha"],
        "package_manifest_sha256": source::file(&folder.join("package.json"))?,
        "package_version": manifest["version"],
        "provenance_sha256": source::file(&folder.join("native/provenance.json"))?,
        "full_native_qualified": qualified,
        "qualification_scope": "production-application-http non-ALLOW authority enforcement before origin plus synthetic-evidence engine compatibility for the listed tuple; no production browser ALLOW evidence and no full-product qualification",
        "production_browser_evidence": false,
        "full_product_qualified": false,
        "host_runs": runs,
        "production_import": package,
        "authority_docker": {
            "status": "passed",
            "source_sha": report["source_sha"],
            "image_digests": [host["image_id"]],
            "container_cleanup_observed": report["container_cleanup_observed"],
            "component_evidence": format!("evidence:root/{stamp}"),
            "component_report_sha256": source::file(&directory.join("component.json"))?,
        },
        "advertised_tuples": [{"target": tuple, "os": os, "arch": arch, "libc": libc, "qualified": qualified}],
    });
    write_json(
        &root.join("target/node-native/qualification.json"),
        &receipt,
    )?;
    super::node_receipt(root)?;
    evidence::emit_public_summary("Node native qualification receipt", &receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn host(rows: Vec<Value>) -> Value {
        json!({"tuple":"linux-x64-gnu","image_id":format!("sha256:{}","a".repeat(64)),"authority_sha256":"b".repeat(64),"versions":{"node":"v24.0.0","bun":"1.3.14"},"counts":{"failed":0,"ignored":0},"rows":rows})
    }
    fn production(runtime: &str, framework: &str, verdict: &str) -> Value {
        json!({"runtime":runtime,"framework":framework,"authority":PRODUCTION,"verdict":verdict,"originEffects":0,"status":"passed"})
    }
    #[test]
    fn runs_count_only_observed_production_verdicts() {
        let rows = vec![
            production("node", "express", "DENY"),
            production("node", "hono", "CHALLENGE"),
            json!({"runtime":"node","framework":"hono","authority":"real-engine-synthetic-evidence","allowCases":7,"negativeCases":3,"originEffects":7}),
            production("bun", "express", "DENY"),
        ];
        let run = host_run("node", &host(rows.clone())).unwrap();
        assert_eq!(
            run["verdict_counts"],
            json!({"ALLOW":0,"CHALLENGE":1,"DENY":1,"error":0})
        );
        assert_eq!(run["test_count"], 2);
        assert_eq!(run["origin_effect_count"], 0);
        assert_eq!(run["os"], "linux");
        assert_eq!(run["arch"], "x64");
        assert_eq!(
            run["component_case_counts"]["real-engine-synthetic-evidence"],
            10
        );
        // Bun has only one production row: the receipt refuses rather than pads.
        assert!(host_run("bun", &host(rows)).is_err());
    }
    #[test]
    fn missing_or_allow_verdicts_and_failed_rows_cannot_qualify() {
        for bad in [
            json!({"runtime":"node","framework":"hono","authority":PRODUCTION,"originEffects":0,"status":"passed"}),
            production("node", "hono", "ALLOW"),
            production("node", "hono", "error"),
            json!({"runtime":"node","framework":"hono","authority":PRODUCTION,"verdict":"DENY","originEffects":0,"status":"failed"}),
        ] {
            let rows = vec![production("node", "express", "DENY"), bad];
            assert!(host_run("node", &host(rows)).is_err());
        }
    }
    #[test]
    fn unknown_tuples_are_never_mapped() {
        assert!(tuple_fields("linux-x64-gnu").is_ok());
        for tuple in ["linux-arm64-gnu", "win32-x64-msvc", "linux-x64-musl", ""] {
            assert!(tuple_fields(tuple).is_err());
        }
    }
    #[test]
    fn stage_logs_must_match_their_recorded_digest() {
        let directory = std::env::temp_dir().join(format!(
            "humanymous-node-qualification-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("probe.log"), b"v24.0.0\n").unwrap();
        let digest = source::file(&directory.join("probe.log")).unwrap();
        let report = json!({"commands":{"probe":{"exit_code":0,"stdout_sha256":digest}}});
        assert_eq!(
            stage_log(&directory, &report, "probe").unwrap(),
            "v24.0.0\n"
        );
        let failed = json!({"commands":{"probe":{"exit_code":1,"stdout_sha256":digest}}});
        assert!(stage_log(&directory, &failed, "probe").is_err());
        std::fs::write(directory.join("probe.log"), b"v24.9.9\n").unwrap();
        assert!(stage_log(&directory, &report, "probe").is_err());
        assert!(stage_log(&directory, &report, "absent").is_err());
        std::fs::remove_dir_all(&directory).unwrap();
    }
}
