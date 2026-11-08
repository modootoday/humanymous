//! Fail-closed Unlock C candidate-pack transfer probe / applicator helper.
//!
//! Default is probe-only: verify live preimages and candidate digests against a
//! transfer manifest. Apply requires an explicit Root grant and still aborts on
//! any preimage mismatch. Never invents `pass_product_clean_room`. Cap burn is
//! a separate step after a successful transfer.
use crate::docker::Result;
use crate::snapshot;
use serde_json::{json, Map, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const PROBE_SCHEMA: &str = "humanymous.lab-unlock-c-transfer-probe/1";
const GRANT_SCHEMA: &str = "humanymous.cursor-unlock-c-transfer-grant/1";
const APPLY_SCHEMA: &str = "humanymous.lab-unlock-c-transfer-apply/1";

/// Probe Unlock C transfer readiness, or apply after Root grant.
pub fn run(args: &[String]) -> Result<i32> {
    let mut manifest_path = None;
    let mut workspace = None;
    let mut emit_path = None;
    let mut grant_path = None;
    let mut apply = false;
    let mut dry_run = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--manifest" => {
                index += 1;
                manifest_path = Some(args.get(index).ok_or("--manifest path missing")?.clone());
            }
            "--workspace" => {
                index += 1;
                workspace = Some(args.get(index).ok_or("--workspace path missing")?.clone());
            }
            "--emit" => {
                index += 1;
                emit_path = Some(args.get(index).ok_or("--emit path missing")?.clone());
            }
            "--grant" => {
                index += 1;
                grant_path = Some(args.get(index).ok_or("--grant path missing")?.clone());
            }
            "--apply" => apply = true,
            "--dry-run" => dry_run = true,
            _ => {
                return Err(
                    "usage: transfer-probe --manifest PATH [--workspace PATH] [--emit PATH] | transfer-apply --manifest PATH --grant PATH [--workspace PATH] [--dry-run] [--emit PATH]"
                        .into(),
                );
            }
        }
        index += 1;
    }
    let manifest_path = manifest_path.ok_or("--manifest required")?;
    let workspace = workspace
        .map(PathBuf::from)
        .unwrap_or_else(|| snapshot::root().unwrap_or_else(|_| PathBuf::from(".")));
    let manifest = read_json(&manifest_path)?;
    let probe = probe_manifest(&manifest, &workspace, Path::new(&manifest_path))?;
    let report = if apply {
        apply_transfer(
            &manifest,
            &probe,
            grant_path.as_deref(),
            &workspace,
            dry_run,
        )?
    } else {
        probe
    };
    emit_report(&report, emit_path.as_deref())?;
    let ready = report
        .get("unlock_c_preimage_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    // Probe always exits 0 with honesty fields; apply refuses (exit 1) when not ready/granted.
    if apply
        && !report
            .get("applied")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        && !dry_run
    {
        return Err(format!(
            "transfer-apply refused: unlock_c_preimage_ready={ready}; require matching grant + preimages; never invent pass_product_clean_room"
        )
        .into());
    }
    Ok(0)
}

fn read_json(path: &str) -> Result<Value> {
    let raw = fs::read_to_string(path).map_err(|error| format!("read {path}: {error}"))?;
    Ok(serde_json::from_str(&raw).map_err(|error| format!("json {path}: {error}"))?)
}

fn emit_report(report: &Value, emit_path: Option<&str>) -> Result<()> {
    let rendered =
        serde_json::to_string_pretty(report).map_err(|error| format!("encode failed: {error}"))?;
    if let Some(path) = emit_path {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).map_err(|error| format!("emit mkdir: {error}"))?;
        }
        fs::write(path, format!("{rendered}\n"))
            .map_err(|error| format!("emit write failed: {error}"))?;
        println!(
            "{}",
            json!({
                "schema": report.get("schema").cloned().unwrap_or(Value::Null),
                "emit": path,
                "unlock_c_preimage_ready": report.get("unlock_c_preimage_ready"),
                "applied": report.get("applied").cloned().unwrap_or(json!(false)),
                "cap_execute": false,
                "pass_product_clean_room": 0,
                "product": 0,
            })
        );
    } else {
        println!("{rendered}");
    }
    Ok(())
}

fn pack_dir(manifest_path: &Path) -> PathBuf {
    manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn probe_manifest(manifest: &Value, workspace: &Path, manifest_path: &Path) -> Result<Value> {
    let entries = manifest
        .get("entries")
        .and_then(Value::as_array)
        .ok_or("manifest.entries array required")?;
    let pack = pack_dir(manifest_path);
    let mut checks = Vec::new();
    let mut mismatches = Vec::new();
    let mut missing_candidates = Vec::new();
    let mut ready_count = 0u64;
    for entry in entries {
        let destination = entry
            .get("destination")
            .and_then(Value::as_str)
            .ok_or("entry.destination required")?;
        let expected_preimage = entry.get("preimageSha256").and_then(Value::as_str);
        let candidate_relative = entry
            .get("candidateRelative")
            .and_then(Value::as_str)
            .ok_or("entry.candidateRelative required")?;
        let expected_candidate = entry
            .get("candidateSha256")
            .and_then(Value::as_str)
            .ok_or("entry.candidateSha256 required")?;

        let live_path = workspace.join(destination);
        let live_state = match (expected_preimage, fs::read(&live_path)) {
            (None, Err(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                json!({
                    "ok": true,
                    "kind": "absent_as_expected",
                    "liveSha256": Value::Null,
                })
            }
            (None, Ok(bytes)) => {
                let live = snapshot::digest(&bytes);
                mismatches.push(json!({
                    "destination": destination,
                    "reason": "expected_absent_but_live_present",
                    "liveSha256": live,
                }));
                json!({
                    "ok": false,
                    "kind": "unexpected_live",
                    "liveSha256": live,
                })
            }
            (Some(expected), Ok(bytes)) => {
                let live = snapshot::digest(&bytes);
                if live == expected {
                    json!({
                        "ok": true,
                        "kind": "preimage_match",
                        "liveSha256": live,
                    })
                } else {
                    mismatches.push(json!({
                        "destination": destination,
                        "reason": "preimage_mismatch",
                        "expectedSha256": expected,
                        "liveSha256": live,
                    }));
                    json!({
                        "ok": false,
                        "kind": "preimage_mismatch",
                        "expectedSha256": expected,
                        "liveSha256": live,
                    })
                }
            }
            (Some(expected), Err(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                mismatches.push(json!({
                    "destination": destination,
                    "reason": "live_missing",
                    "expectedSha256": expected,
                }));
                json!({
                    "ok": false,
                    "kind": "live_missing",
                    "expectedSha256": expected,
                    "liveSha256": Value::Null,
                })
            }
            (_, Err(error)) => {
                return Err(format!("live read {destination}: {error}").into());
            }
        };

        let candidate_path = pack.join(candidate_relative);
        let candidate_state = match fs::read(&candidate_path) {
            Ok(bytes) => {
                let actual = snapshot::digest(&bytes);
                if actual == expected_candidate {
                    json!({
                        "ok": true,
                        "kind": "candidate_match",
                        "candidateSha256": actual,
                        "bytes": bytes.len(),
                    })
                } else {
                    mismatches.push(json!({
                        "destination": destination,
                        "reason": "candidate_digest_mismatch",
                        "expectedSha256": expected_candidate,
                        "candidateSha256": actual,
                    }));
                    json!({
                        "ok": false,
                        "kind": "candidate_digest_mismatch",
                        "expectedSha256": expected_candidate,
                        "candidateSha256": actual,
                    })
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing_candidates.push(candidate_relative);
                json!({
                    "ok": false,
                    "kind": "candidate_missing",
                    "candidateRelative": candidate_relative,
                })
            }
            Err(error) => {
                return Err(format!("candidate read {candidate_relative}: {error}").into());
            }
        };

        let entry_ok = live_state["ok"] == true && candidate_state["ok"] == true;
        if entry_ok {
            ready_count += 1;
        }
        checks.push(json!({
            "destination": destination,
            "ok": entry_ok,
            "live": live_state,
            "candidate": candidate_state,
        }));
    }

    let unlock_c_preimage_ready = mismatches.is_empty()
        && missing_candidates.is_empty()
        && ready_count == entries.len() as u64
        && !entries.is_empty();

    Ok(json!({
        "schema": PROBE_SCHEMA,
        "record_kind": "lab-runner-unlock-c-transfer-probe",
        "manifest": manifest_path.to_string_lossy(),
        "workspace": workspace.to_string_lossy(),
        "applied": false,
        "cap_execute": false,
        "product": 0,
        "pass_product_clean_room": 0,
        "full_product_qualified": false,
        "unlock_c_preimage_ready": unlock_c_preimage_ready,
        "summary": {
            "entries": entries.len(),
            "ready": ready_count,
            "mismatches": mismatches.len(),
            "missing_candidates": missing_candidates.len(),
        },
        "honesty": {
            "never_invent_pass_product_clean_room": true,
            "probe_does_not_apply": true,
            "apply_requires_root_grant": true,
            "authority_cap_green_is_not_product": true,
        },
        "mismatches": mismatches,
        "missing_candidates": missing_candidates,
        "checks": checks,
        "next": if unlock_c_preimage_ready {
            "Root issues transfer grant; transfer-apply then Cap=1 --authority-node-ipc"
        } else {
            "Abort transfer; refresh candidate-pack or wait Root structural delta"
        },
    }))
}

fn apply_transfer(
    manifest: &Value,
    probe: &Value,
    grant_path: Option<&str>,
    workspace: &Path,
    dry_run: bool,
) -> Result<Value> {
    let grant_path = grant_path.ok_or("transfer-apply requires --grant")?;
    let grant = read_json(grant_path)?;
    validate_grant(&grant, manifest)?;
    let ready = probe
        .get("unlock_c_preimage_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !ready {
        let mut refused = probe.clone();
        if let Some(object) = refused.as_object_mut() {
            object.insert("schema".into(), json!(APPLY_SCHEMA));
            object.insert(
                "record_kind".into(),
                json!("lab-runner-unlock-c-transfer-apply"),
            );
            object.insert("applied".into(), json!(false));
            object.insert("dry_run".into(), json!(dry_run));
            object.insert("grant".into(), json!(grant_path));
            object.insert(
                "refuse_reason".into(),
                json!("preimage_or_candidate_not_ready"),
            );
        }
        return Ok(refused);
    }

    let entries = manifest
        .get("entries")
        .and_then(Value::as_array)
        .ok_or("manifest.entries array required")?;
    let pack = pack_dir(Path::new(
        probe
            .get("manifest")
            .and_then(Value::as_str)
            .ok_or("probe.manifest missing")?,
    ));
    let allowed = grant
        .get("destinations")
        .and_then(Value::as_array)
        .ok_or("grant.destinations array required")?;
    let allowed: Vec<&str> = allowed.iter().filter_map(Value::as_str).collect();

    let mut planned = Vec::new();
    let mut written = Vec::new();
    for entry in entries {
        let destination = entry
            .get("destination")
            .and_then(Value::as_str)
            .ok_or("entry.destination required")?;
        if !allowed.contains(&destination) {
            return Err(format!(
                "destination {destination} absent from grant.destinations; fail-closed"
            )
            .into());
        }
        let candidate_relative = entry
            .get("candidateRelative")
            .and_then(Value::as_str)
            .ok_or("entry.candidateRelative required")?;
        let expected_candidate = entry
            .get("candidateSha256")
            .and_then(Value::as_str)
            .ok_or("entry.candidateSha256 required")?;
        let candidate_path = pack.join(candidate_relative);
        let bytes = fs::read(&candidate_path)
            .map_err(|error| format!("candidate read {candidate_relative}: {error}"))?;
        let actual = snapshot::digest(&bytes);
        if actual != expected_candidate {
            return Err(format!("candidate digest drifted for {destination} before apply").into());
        }
        let live_path = workspace.join(destination);
        planned.push(json!({
            "destination": destination,
            "bytes": bytes.len(),
            "candidateSha256": actual,
        }));
        if !dry_run {
            if let Some(parent) = live_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
            }
            fs::write(&live_path, &bytes)
                .map_err(|error| format!("write {destination}: {error}"))?;
            written.push(destination);
        }
    }

    let mut report = Map::new();
    report.insert("schema".into(), json!(APPLY_SCHEMA));
    report.insert(
        "record_kind".into(),
        json!("lab-runner-unlock-c-transfer-apply"),
    );
    report.insert("applied".into(), json!(!dry_run));
    report.insert("dry_run".into(), json!(dry_run));
    report.insert("grant".into(), json!(grant_path));
    report.insert("unlock_c_preimage_ready".into(), json!(true));
    report.insert("cap_execute".into(), json!(false));
    report.insert("product".into(), json!(0));
    report.insert("pass_product_clean_room".into(), json!(0));
    report.insert("full_product_qualified".into(), json!(false));
    report.insert(
        "honesty".into(),
        json!({
            "never_invent_pass_product_clean_room": true,
            "authority_cap_green_is_not_product": true,
            "cap_burn_is_separate_step": true,
        }),
    );
    report.insert("planned".into(), json!(planned));
    report.insert("written".into(), json!(written));
    report.insert(
        "next".into(),
        json!(if dry_run {
            "Re-run without --dry-run after Root confirms grant; then Cap=1"
        } else {
            "Cap=1 burn matching cap-ready/authority-node-ipc pack; matrix honesty retained"
        }),
    );
    Ok(Value::Object(report))
}

fn validate_grant(grant: &Value, manifest: &Value) -> Result<()> {
    if grant.get("schema").and_then(Value::as_str) != Some(GRANT_SCHEMA) {
        return Err(format!("grant.schema must be {GRANT_SCHEMA}").into());
    }
    if grant.get("root_authorized").and_then(Value::as_bool) != Some(true) {
        return Err("grant.root_authorized must be true".into());
    }
    if grant.get("allow_apply").and_then(Value::as_bool) != Some(true) {
        return Err("grant.allow_apply must be true".into());
    }
    if grant.get("unlock").and_then(Value::as_str) != Some("C") {
        return Err("grant.unlock must be C".into());
    }
    let destinations = grant
        .get("destinations")
        .and_then(Value::as_array)
        .ok_or("grant.destinations array required")?;
    if destinations.is_empty() {
        return Err("grant.destinations must be non-empty".into());
    }
    let entries = manifest
        .get("entries")
        .and_then(Value::as_array)
        .ok_or("manifest.entries array required")?;
    for entry in entries {
        let destination = entry
            .get("destination")
            .and_then(Value::as_str)
            .ok_or("entry.destination required")?;
        if !destinations
            .iter()
            .any(|path| path.as_str() == Some(destination))
        {
            return Err(format!("grant.destinations incomplete: missing {destination}").into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("hm-transfer-{label}-{nanos}"));
        fs::create_dir_all(&path).expect("temp");
        path
    }

    fn write(path: &Path, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(path, bytes).expect("write");
    }

    fn sample_pack(root: &Path) -> (PathBuf, Value) {
        let live = b"live-integration-body\n";
        let candidate = b"candidate-integration-body\n";
        let unchanged = b"unchanged-compose\n";
        write(&root.join("workspace/deployments/lab/INTEGRATION.md"), live);
        write(
            &root.join("workspace/deployments/lab/compose.authority.yaml"),
            unchanged,
        );
        write(
            &root.join("pack/candidate/deployments/lab/INTEGRATION.md"),
            candidate,
        );
        write(
            &root.join("pack/candidate/deployments/lab/compose.authority.yaml"),
            unchanged,
        );
        write(
            &root.join("pack/candidate/deployments/lab/authority/CAP-PILOT.md"),
            b"cap pilot notes\n",
        );
        let manifest = json!({
            "schema": "humanymous.cursor-unlock-c-authority-cap-candidate/1",
            "applied": false,
            "entries": [
                {
                    "destination": "deployments/lab/INTEGRATION.md",
                    "preimageSha256": snapshot::digest(live),
                    "candidateRelative": "candidate/deployments/lab/INTEGRATION.md",
                    "candidateSha256": snapshot::digest(candidate),
                },
                {
                    "destination": "deployments/lab/compose.authority.yaml",
                    "preimageSha256": snapshot::digest(unchanged),
                    "candidateRelative": "candidate/deployments/lab/compose.authority.yaml",
                    "candidateSha256": snapshot::digest(unchanged),
                },
                {
                    "destination": "deployments/lab/authority/CAP-PILOT.md",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/deployments/lab/authority/CAP-PILOT.md",
                    "candidateSha256": snapshot::digest(b"cap pilot notes\n"),
                }
            ]
        });
        let manifest_path = root.join("pack/transfer-manifest.json");
        write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap().as_bytes(),
        );
        (manifest_path, manifest)
    }

    #[test]
    fn probe_reports_ready_when_preimages_and_candidates_match() {
        let root = temp_dir("ready");
        let (manifest_path, manifest) = sample_pack(&root);
        let probe =
            probe_manifest(&manifest, &root.join("workspace"), &manifest_path).expect("probe");
        assert_eq!(probe["schema"], PROBE_SCHEMA);
        assert_eq!(probe["unlock_c_preimage_ready"], true);
        assert_eq!(probe["applied"], false);
        assert_eq!(probe["cap_execute"], false);
        assert_eq!(probe["pass_product_clean_room"], 0);
        assert_eq!(probe["product"], 0);
        assert_eq!(
            probe["honesty"]["never_invent_pass_product_clean_room"],
            true
        );
        assert_eq!(probe["summary"]["mismatches"], 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn probe_fail_closed_on_preimage_mismatch() {
        let root = temp_dir("mismatch");
        let (manifest_path, manifest) = sample_pack(&root);
        write(
            &root.join("workspace/deployments/lab/INTEGRATION.md"),
            b"drifted-live-body\n",
        );
        let probe =
            probe_manifest(&manifest, &root.join("workspace"), &manifest_path).expect("probe");
        assert_eq!(probe["unlock_c_preimage_ready"], false);
        assert_eq!(probe["pass_product_clean_room"], 0);
        assert!(probe["mismatches"]
            .as_array()
            .expect("mismatches")
            .iter()
            .any(|item| item["reason"] == "preimage_mismatch"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn apply_refuses_without_matching_grant() {
        let root = temp_dir("nogrant");
        let (manifest_path, manifest) = sample_pack(&root);
        let probe =
            probe_manifest(&manifest, &root.join("workspace"), &manifest_path).expect("probe");
        let err = apply_transfer(&manifest, &probe, None, &root.join("workspace"), false)
            .expect_err("grant required");
        assert!(err.to_string().contains("--grant"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn apply_dry_run_with_grant_does_not_write_or_claim_product() {
        let root = temp_dir("dry");
        let (manifest_path, manifest) = sample_pack(&root);
        let probe =
            probe_manifest(&manifest, &root.join("workspace"), &manifest_path).expect("probe");
        let grant = json!({
            "schema": GRANT_SCHEMA,
            "root_authorized": true,
            "allow_apply": true,
            "unlock": "C",
            "destinations": [
                "deployments/lab/INTEGRATION.md",
                "deployments/lab/compose.authority.yaml",
                "deployments/lab/authority/CAP-PILOT.md"
            ]
        });
        let grant_path = root.join("grant.json");
        write(
            &grant_path,
            serde_json::to_string_pretty(&grant).unwrap().as_bytes(),
        );
        let report = apply_transfer(
            &manifest,
            &probe,
            Some(grant_path.to_str().unwrap()),
            &root.join("workspace"),
            true,
        )
        .expect("dry-run");
        assert_eq!(report["schema"], APPLY_SCHEMA);
        assert_eq!(report["applied"], false);
        assert_eq!(report["dry_run"], true);
        assert_eq!(report["pass_product_clean_room"], 0);
        assert_eq!(report["product"], 0);
        assert_eq!(report["cap_execute"], false);
        assert!(!root
            .join("workspace/deployments/lab/authority/CAP-PILOT.md")
            .exists());
        let live = fs::read_to_string(root.join("workspace/deployments/lab/INTEGRATION.md"))
            .expect("live");
        assert_eq!(live, "live-integration-body\n");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn apply_writes_only_with_grant_and_keeps_product_zero() {
        let root = temp_dir("apply");
        let (manifest_path, manifest) = sample_pack(&root);
        let probe =
            probe_manifest(&manifest, &root.join("workspace"), &manifest_path).expect("probe");
        let grant = json!({
            "schema": GRANT_SCHEMA,
            "root_authorized": true,
            "allow_apply": true,
            "unlock": "C",
            "destinations": [
                "deployments/lab/INTEGRATION.md",
                "deployments/lab/compose.authority.yaml",
                "deployments/lab/authority/CAP-PILOT.md"
            ]
        });
        let grant_path = root.join("grant.json");
        write(
            &grant_path,
            serde_json::to_string_pretty(&grant).unwrap().as_bytes(),
        );
        let report = apply_transfer(
            &manifest,
            &probe,
            Some(grant_path.to_str().unwrap()),
            &root.join("workspace"),
            false,
        )
        .expect("apply");
        assert_eq!(report["applied"], true);
        assert_eq!(report["pass_product_clean_room"], 0);
        assert_eq!(report["product"], 0);
        let live = fs::read_to_string(root.join("workspace/deployments/lab/INTEGRATION.md"))
            .expect("live");
        assert_eq!(live, "candidate-integration-body\n");
        assert!(root
            .join("workspace/deployments/lab/authority/CAP-PILOT.md")
            .exists());
        let _ = fs::remove_dir_all(root);
    }
}
