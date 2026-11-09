//! Fail-closed Unlock A/B/C + G2 candidate-pack transfer probe / applicator helper.
//!
//! Default is probe-only: verify live preimages and candidate digests against a
//! transfer manifest. Apply requires an explicit Root grant and still aborts on
//! any preimage mismatch. Unlock B additionally requires a non-empty
//! `pilot_command` in the grant (Root-named hosted gateway binding). Unlock A
//! requires non-empty path-release destinations **and** the code-defined Cap
//! CLI `--g4-cef-at` pilot; empty destinations / empty entry lists refuse.
//! G2 requires the code-defined Cap CLI `--g2-collector-product-binding` pilot;
//! Cap burn stays Cap-realloc-gated (topology apply ≠ Cap free).
//! G5 requires the code-defined Cap CLI `--g5-witness-transport` pilot;
//! Cap burn stays Cap-realloc-gated (topology apply ≠ Cap free / ≠ authenticated transport).
//! Never invents `pass_product_clean_room` or folklore pilot argv. Cap burn is
//! a separate step after a successful transfer.
use crate::docker::Result;
use crate::orchestration::clean_room_product::{
    CAP_SELECTOR as CR_CAP_SELECTOR, CODE_DEFINED_PILOT_COMMAND as CR_CODE_DEFINED_PILOT,
};
use crate::orchestration::g2_collector_product_binding::{
    CAP_SELECTOR as G2_CAP_SELECTOR, CODE_DEFINED_PILOT_COMMAND as G2_CODE_DEFINED_PILOT,
};
use crate::orchestration::g4_cef_at::{
    CAP_SELECTOR as G4_CEF_CAP_SELECTOR, CODE_DEFINED_PILOT_COMMAND as G4_CODE_DEFINED_PILOT,
};
use crate::orchestration::g5_witness_transport::{
    CAP_SELECTOR as G5_CAP_SELECTOR, CODE_DEFINED_PILOT_COMMAND as G5_CODE_DEFINED_PILOT,
};
use crate::orchestration::g6_origin_audit_binding::{
    CAP_SELECTOR as G6_CAP_SELECTOR, CODE_DEFINED_PILOT_COMMAND as G6_CODE_DEFINED_PILOT,
};
use crate::orchestration::red_blue_product::{
    CAP_SELECTOR as RB_CAP_SELECTOR, CODE_DEFINED_PILOT_COMMAND as RB_CODE_DEFINED_PILOT,
};
use crate::orchestration::clean_room_product_matrix::{
    CAP_SELECTOR as PM_CAP_SELECTOR, CODE_DEFINED_PILOT_COMMAND as PM_CODE_DEFINED_PILOT,
};
use crate::snapshot;
use serde_json::{Map, Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const PROBE_SCHEMA_C: &str = "humanymous.lab-unlock-c-transfer-probe/1";
const GRANT_SCHEMA_C: &str = "humanymous.cursor-unlock-c-transfer-grant/1";
const APPLY_SCHEMA_C: &str = "humanymous.lab-unlock-c-transfer-apply/1";
const PROBE_SCHEMA_B: &str = "humanymous.lab-unlock-b-transfer-probe/1";
const GRANT_SCHEMA_B: &str = "humanymous.cursor-unlock-b-transfer-grant/1";
const APPLY_SCHEMA_B: &str = "humanymous.lab-unlock-b-transfer-apply/1";
const PROBE_SCHEMA_A: &str = "humanymous.lab-unlock-a-transfer-probe/1";
const GRANT_SCHEMA_A: &str = "humanymous.cursor-unlock-a-transfer-grant/1";
const APPLY_SCHEMA_A: &str = "humanymous.lab-unlock-a-transfer-apply/1";
const PROBE_SCHEMA_N06: &str = "humanymous.lab-n06-hosted-venue-transfer-probe/1";
const GRANT_SCHEMA_N06: &str = "humanymous.cursor-n06-hosted-venue-transfer-grant/1";
const APPLY_SCHEMA_N06: &str = "humanymous.lab-n06-hosted-venue-transfer-apply/1";
const PROBE_SCHEMA_G2: &str = "humanymous.lab-g2-collector-product-binding-transfer-probe/1";
const GRANT_SCHEMA_G2: &str = "humanymous.cursor-g2-collector-product-binding-transfer-grant/1";
const APPLY_SCHEMA_G2: &str = "humanymous.lab-g2-collector-product-binding-transfer-apply/1";
const PROBE_SCHEMA_G5: &str = "humanymous.lab-g5-witness-transport-transfer-probe/1";
const GRANT_SCHEMA_G5: &str = "humanymous.cursor-g5-witness-transport-transfer-grant/1";
const APPLY_SCHEMA_G5: &str = "humanymous.lab-g5-witness-transport-transfer-apply/1";
const PROBE_SCHEMA_CR: &str = "humanymous.lab-clean-room-product-transfer-probe/1";
const GRANT_SCHEMA_CR: &str = "humanymous.cursor-clean-room-product-transfer-grant/1";
const APPLY_SCHEMA_CR: &str = "humanymous.lab-clean-room-product-transfer-apply/1";
const PROBE_SCHEMA_G6: &str = "humanymous.lab-g6-origin-audit-binding-transfer-probe/1";
const GRANT_SCHEMA_G6: &str = "humanymous.cursor-g6-origin-audit-binding-transfer-grant/1";
const APPLY_SCHEMA_G6: &str = "humanymous.lab-g6-origin-audit-binding-transfer-apply/1";
const PROBE_SCHEMA_RB: &str = "humanymous.lab-red-blue-product-transfer-probe/1";
const GRANT_SCHEMA_RB: &str = "humanymous.cursor-red-blue-product-transfer-grant/1";
const APPLY_SCHEMA_RB: &str = "humanymous.lab-red-blue-product-transfer-apply/1";
const PROBE_SCHEMA_PM: &str = "humanymous.lab-clean-room-product-matrix-transfer-probe/1";
const GRANT_SCHEMA_PM: &str = "humanymous.cursor-clean-room-product-matrix-transfer-grant/1";
const APPLY_SCHEMA_PM: &str = "humanymous.lab-clean-room-product-matrix-transfer-apply/1";
const UNLOCK_A_APPLIED_REL: &str = ".spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/g4-cef-at/UNLOCK-A-APPLIED.json";
const UNLOCK_B_APPLIED_REL: &str = ".spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/w2-h2-product-binding/UNLOCK-B-APPLIED.json";
const N06_HOSTED_VENUE_APPLIED_REL: &str = ".spec.local/root/evidence/20261109003000-cursor-n06-original-h2-measure/cap-ready-hosted-venue/N06-HOSTED-VENUE-APPLIED.json";
const UNLOCK_G2_APPLIED_REL: &str = ".spec.local/root/evidence/20261109030000-cursor-g2-cap-ready-feeder/cap-ready/g2-collector-product-binding/UNLOCK-G2-APPLIED.json";
const UNLOCK_G5_APPLIED_REL: &str = ".spec.local/root/evidence/20261109031000-cursor-g5-cap-ready-feeder/cap-ready/g5-witness-transport/UNLOCK-G5-APPLIED.json";
const UNLOCK_CR_APPLIED_REL: &str = ".spec.local/root/evidence/20261109040000-cursor-clean-room-product-cap-ready/cap-ready/clean-room-product/UNLOCK-CR-APPLIED.json";
const UNLOCK_G6_APPLIED_REL: &str = ".spec.local/root/evidence/20261109040600-cursor-g6-cap-ready-feeder/cap-ready/g6-origin-audit-binding/UNLOCK-G6-APPLIED.json";
const UNLOCK_RB_APPLIED_REL: &str = ".spec.local/root/evidence/20261110044800-cursor-red-blue-product-cap-ready/cap-ready/red-blue-product/UNLOCK-RB-APPLIED.json";
const UNLOCK_PM_APPLIED_REL: &str = ".spec.local/root/evidence/20261110065000-cursor-clean-room-product-matrix-cap-ready/cap-ready/clean-room-product-matrix/UNLOCK-PM-APPLIED.json";
const N06_HOSTED_VENUE_CAP_SELECTOR: &str = "--w2-h2-hosted-venue";
const N06_HOSTED_VENUE_PILOT: &str =
    "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --w2-h2-hosted-venue";

/// Compatibility alias used by Unlock-C unit tests.
#[cfg(test)]
const PROBE_SCHEMA: &str = PROBE_SCHEMA_C;
#[cfg(test)]
const GRANT_SCHEMA: &str = GRANT_SCHEMA_C;
#[cfg(test)]
const APPLY_SCHEMA: &str = APPLY_SCHEMA_C;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnlockLetter {
    A,
    B,
    C,
    N06,
    G2,
    G5,
    CR,
    G6,
    RB,
    PM,
}

impl UnlockLetter {
    fn from_manifest(manifest: &Value) -> Result<Self> {
        match manifest.get("unlock").and_then(Value::as_str) {
            Some("A") => Ok(Self::A),
            Some("B") => Ok(Self::B),
            Some("N06") => Ok(Self::N06),
            Some("G2") => Ok(Self::G2),
            Some("G5") => Ok(Self::G5),
            Some("CR") => Ok(Self::CR),
            Some("G6") => Ok(Self::G6),
            Some("RB") => Ok(Self::RB),
            Some("PM") => Ok(Self::PM),
            Some("C") | None => Ok(Self::C),
            Some(other) => Err(format!("unsupported manifest.unlock={other}").into()),
        }
    }

    fn probe_schema(self) -> &'static str {
        match self {
            Self::A => PROBE_SCHEMA_A,
            Self::B => PROBE_SCHEMA_B,
            Self::C => PROBE_SCHEMA_C,
            Self::N06 => PROBE_SCHEMA_N06,
            Self::G2 => PROBE_SCHEMA_G2,
            Self::G5 => PROBE_SCHEMA_G5,
            Self::CR => PROBE_SCHEMA_CR,
            Self::G6 => PROBE_SCHEMA_G6,
            Self::RB => PROBE_SCHEMA_RB,
            Self::PM => PROBE_SCHEMA_PM,
        }
    }

    fn apply_schema(self) -> &'static str {
        match self {
            Self::A => APPLY_SCHEMA_A,
            Self::B => APPLY_SCHEMA_B,
            Self::C => APPLY_SCHEMA_C,
            Self::N06 => APPLY_SCHEMA_N06,
            Self::G2 => APPLY_SCHEMA_G2,
            Self::G5 => APPLY_SCHEMA_G5,
            Self::CR => APPLY_SCHEMA_CR,
            Self::G6 => APPLY_SCHEMA_G6,
            Self::RB => APPLY_SCHEMA_RB,
            Self::PM => APPLY_SCHEMA_PM,
        }
    }

    fn grant_schema(self) -> &'static str {
        match self {
            Self::A => GRANT_SCHEMA_A,
            Self::B => GRANT_SCHEMA_B,
            Self::C => GRANT_SCHEMA_C,
            Self::N06 => GRANT_SCHEMA_N06,
            Self::G2 => GRANT_SCHEMA_G2,
            Self::G5 => GRANT_SCHEMA_G5,
            Self::CR => GRANT_SCHEMA_CR,
            Self::G6 => GRANT_SCHEMA_G6,
            Self::RB => GRANT_SCHEMA_RB,
            Self::PM => GRANT_SCHEMA_PM,
        }
    }

    fn ready_key(self) -> &'static str {
        match self {
            Self::A => "unlock_a_preimage_ready",
            Self::B => "unlock_b_preimage_ready",
            Self::C => "unlock_c_preimage_ready",
            Self::N06 => "n06_hosted_venue_preimage_ready",
            Self::G2 => "g2_collector_product_binding_preimage_ready",
            Self::G5 => "g5_witness_transport_preimage_ready",
            Self::CR => "clean_room_product_preimage_ready",
            Self::G6 => "g6_origin_audit_binding_preimage_ready",
            Self::RB => "red_blue_product_preimage_ready",
            Self::PM => "clean_room_product_matrix_preimage_ready",
        }
    }

    fn record_kind_probe(self) -> &'static str {
        match self {
            Self::A => "lab-runner-unlock-a-transfer-probe",
            Self::B => "lab-runner-unlock-b-transfer-probe",
            Self::C => "lab-runner-unlock-c-transfer-probe",
            Self::N06 => "lab-runner-n06-hosted-venue-transfer-probe",
            Self::G2 => "lab-runner-g2-collector-product-binding-transfer-probe",
            Self::G5 => "lab-runner-g5-witness-transport-transfer-probe",
            Self::CR => "lab-runner-clean-room-product-transfer-probe",
            Self::G6 => "lab-runner-g6-origin-audit-binding-transfer-probe",
            Self::RB => "lab-runner-red-blue-product-transfer-probe",
            Self::PM => "lab-runner-clean-room-product-matrix-transfer-probe",
        }
    }

    fn record_kind_apply(self) -> &'static str {
        match self {
            Self::A => "lab-runner-unlock-a-transfer-apply",
            Self::B => "lab-runner-unlock-b-transfer-apply",
            Self::C => "lab-runner-unlock-c-transfer-apply",
            Self::N06 => "lab-runner-n06-hosted-venue-transfer-apply",
            Self::G2 => "lab-runner-g2-collector-product-binding-transfer-apply",
            Self::G5 => "lab-runner-g5-witness-transport-transfer-apply",
            Self::CR => "lab-runner-clean-room-product-transfer-apply",
            Self::G6 => "lab-runner-g6-origin-audit-binding-transfer-apply",
            Self::RB => "lab-runner-red-blue-product-transfer-apply",
            Self::PM => "lab-runner-clean-room-product-matrix-transfer-apply",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::N06 => "N06",
            Self::G2 => "G2",
            Self::G5 => "G5",
            Self::CR => "CR",
            Self::G6 => "G6",
            Self::RB => "RB",
            Self::PM => "PM",
        }
    }
}

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
    let unlock = UnlockLetter::from_manifest(&manifest)?;
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
        .get(unlock.ready_key())
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
            "transfer-apply refused: {}={ready}; require matching grant + preimages; Unlock B also requires non-empty pilot_command; Unlock A requires Root path-release destinations; N06 requires hosted-venue Cap CLI pilot; G2 requires Cap CLI --g2-collector-product-binding pilot; G5 requires Cap CLI --g5-witness-transport pilot; CR requires Cap CLI --clean-room-product pilot; G6 requires Cap CLI --g6-origin-audit-binding pilot; RB requires Cap CLI --red-blue-product --require-full pilot; PM requires Cap CLI --clean-room-product-matrix --require-full pilot; never invent pass_product_clean_room",
            unlock.ready_key()
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
                "unlock_b_preimage_ready": report.get("unlock_b_preimage_ready"),
                "unlock_a_preimage_ready": report.get("unlock_a_preimage_ready"),
                "n06_hosted_venue_preimage_ready": report.get("n06_hosted_venue_preimage_ready"),
                "g2_collector_product_binding_preimage_ready": report
                    .get("g2_collector_product_binding_preimage_ready"),
                "clean_room_product_preimage_ready": report
                    .get("clean_room_product_preimage_ready"),
                "clean_room_product_matrix_preimage_ready": report
                    .get("clean_room_product_matrix_preimage_ready"),
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
    let unlock = UnlockLetter::from_manifest(manifest)?;
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

    let unlock_preimage_ready = mismatches.is_empty()
        && missing_candidates.is_empty()
        && ready_count == entries.len() as u64
        && !entries.is_empty();

    let mut report = json!({
        "schema": unlock.probe_schema(),
        "record_kind": unlock.record_kind_probe(),
        "unlock": unlock.as_str(),
        "manifest": manifest_path.to_string_lossy(),
        "workspace": workspace.to_string_lossy(),
        "applied": false,
        "cap_execute": false,
        "product": 0,
        "pass_product_clean_room": 0,
        "full_product_qualified": false,
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
            "capture_component_is_not_product_binding": true,
            "unlock_b_requires_non_empty_pilot_command": unlock == UnlockLetter::B,
            "unlock_a_requires_root_path_release_destinations": unlock == UnlockLetter::A,
            "n06_requires_hosted_venue_cap_cli_pilot": unlock == UnlockLetter::N06,
            "g2_requires_collector_product_binding_cap_cli_pilot": unlock == UnlockLetter::G2,
            "g2_topology_apply_does_not_invent_cap_free": unlock == UnlockLetter::G2,
            "g5_requires_witness_transport_cap_cli_pilot": unlock == UnlockLetter::G5,
            "g5_topology_apply_does_not_invent_cap_free": unlock == UnlockLetter::G5,
            "g5_topology_apply_does_not_invent_authenticated_transport": unlock == UnlockLetter::G5,
            "cr_requires_clean_room_product_cap_cli_pilot": unlock == UnlockLetter::CR,
            "cr_topology_apply_does_not_invent_cap_free": unlock == UnlockLetter::CR,
            "g6_requires_origin_audit_binding_cap_cli_pilot": unlock == UnlockLetter::G6,
            "g6_topology_apply_does_not_invent_cap_free": unlock == UnlockLetter::G6,
            "rb_requires_red_blue_product_require_full_cap_cli_pilot": unlock == UnlockLetter::RB,
            "rb_topology_apply_does_not_invent_cap_free": unlock == UnlockLetter::RB,
            "pm_requires_clean_room_product_matrix_require_full_cap_cli_pilot": unlock == UnlockLetter::PM,
            "pm_topology_apply_does_not_invent_cap_free": unlock == UnlockLetter::PM,
            "never_invent_pilot_command": true,
            "never_invent_desktop_as_hosted": true,
        },
        "mismatches": mismatches,
        "missing_candidates": missing_candidates,
        "checks": checks,
        "next": if unlock_preimage_ready {
            match unlock {
                UnlockLetter::A => "Root issues Unlock-A GRANT with non-empty path-release destinations + Cap CLI --g4-cef-at pilot_command; transfer-apply then Cap=1 matching g4-cef-at pack (refuse-closed if HUMANYMOUS_CEF_PILOT_BUNDLE absent)",
                UnlockLetter::B => "Root issues Unlock-B GRANT with non-empty pilot_command; transfer-apply then Cap=1 matching w2-h2-product-binding pack",
                UnlockLetter::C => "Root issues transfer grant; transfer-apply then Cap=1 --authority-node-ipc",
                UnlockLetter::N06 => "Root/autonomy GRANT with Cap CLI --w2-h2-hosted-venue; transfer-apply then Cap=1 only when actual_hosted_linux; Desktop Cap=0 fail-closed",
                UnlockLetter::G2 => "Root copies GRANT.TEMPLATE.json → GRANT.json (or issues matching grant); transfer-apply --manifest … --grant GRANT.json; then Cap realloc schema humanymous.cursor-g2-collector-product-binding-cap-realloc/1 → Cap=1 once --g2-collector-product-binding; Cap free stays false until realloc; never invent pass_product_clean_room",
                UnlockLetter::G5 => "Root copies GRANT.TEMPLATE.json → GRANT.json (or issues matching grant); transfer-apply --manifest … --grant GRANT.json; then Cap realloc schema humanymous.cursor-g5-witness-transport-cap-realloc/1 → Cap=1 once --g5-witness-transport; Cap free stays false until realloc; authenticated_witness_transport stays false until Cap measured; never invent pass_product_clean_room",
                UnlockLetter::CR => "Standing autonomy / Root copies GRANT.TEMPLATE.json → GRANT.json; transfer-apply; Cap realloc schema humanymous.cursor-clean-room-product-cap-realloc/1 → Cap=1 once --clean-room-product; binds G2+G5 holds + AUTH/AT refuse; Cap free stays false until realloc; never invent pass_product_clean_room",
                UnlockLetter::G6 => "Standing autonomy / Root copies GRANT.TEMPLATE.json → GRANT.json; transfer-apply; Cap realloc schema humanymous.cursor-g6-origin-audit-binding-cap-realloc/1 → Cap=1 once --g6-origin-audit-binding; binds G5+CR holds; Cap free stays false until realloc; never invent pass_product_clean_room",
                UnlockLetter::RB => "Standing autonomy / Root copies GRANT.TEMPLATE.json → GRANT.json; transfer-apply; Cap realloc schema humanymous.cursor-red-blue-product-cap-realloc/1 → Cap=1 once --red-blue-product --require-full; product/red-blue feeder (not Cap twin); Cap free stays false until realloc; never invent pass_product_clean_room",
                UnlockLetter::PM => "Standing autonomy / Root copies GRANT.TEMPLATE.json → GRANT.json; transfer-apply; Cap realloc schema humanymous.cursor-clean-room-product-matrix-cap-realloc/1 → Cap=1 once --clean-room-product-matrix --require-full; product-matrix feeder (not spent CR Cap twin); Cap free stays false until realloc; never invent pass_product_clean_room",
            }
        } else if unlock == UnlockLetter::A && entries.is_empty() {
            "Unlock A scaffold: wait Root CEF/AT path release (crates/browser/** and/or scripts/ci/cef-pilot/**) + candidate bodies + Cap CLI --g4-cef-at GRANT; empty entries keep unlock_a_preimage_ready=false; Cap day also requires HUMANYMOUS_CEF_PILOT_BUNDLE"
        } else {
            "Abort transfer; refresh candidate-pack or wait Root structural delta"
        },
    });
    if let Some(object) = report.as_object_mut() {
        object.insert(unlock.ready_key().into(), json!(unlock_preimage_ready));
    }
    Ok(report)
}

fn apply_transfer(
    manifest: &Value,
    probe: &Value,
    grant_path: Option<&str>,
    workspace: &Path,
    dry_run: bool,
) -> Result<Value> {
    let unlock = UnlockLetter::from_manifest(manifest)?;
    let grant_path = grant_path.ok_or("transfer-apply requires --grant")?;
    let grant = read_json(grant_path)?;
    validate_grant(&grant, manifest, unlock)?;
    let ready = probe
        .get(unlock.ready_key())
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !ready {
        let mut refused = probe.clone();
        if let Some(object) = refused.as_object_mut() {
            object.insert("schema".into(), json!(unlock.apply_schema()));
            object.insert("record_kind".into(), json!(unlock.record_kind_apply()));
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
    let pilot_command = grant
        .get("pilot_command")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

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

    if unlock == UnlockLetter::A && !dry_run {
        if written.is_empty() {
            return Err(
                "Unlock A apply refused: no destinations written; Root CEF/AT path release + candidate bodies required; never invent path release"
                    .into(),
            );
        }
        let Some(pilot) = pilot_command else {
            return Err(
                "Unlock A apply refused: grant.pilot_command must be non-empty Cap CLI --g4-cef-at; never invent pilot_command"
                    .into(),
            );
        };
        let receipt = json!({
            "schema": "humanymous.cursor-unlock-a-applied/1",
            "unlock": "A",
            "applied": true,
            "applied_utc": chrono_like_utc(),
            "grant": grant_path,
            "pilot_command": pilot,
            "destinations_written": written,
            "cap_burn_authorized": true,
            "cap_burn_limit": 1,
            "product": 0,
            "pass_product_clean_room": 0,
            "honesty": {
                "never_invent_pass_product_clean_room": true,
                "detection_freeze": true,
                "a11y_non_motor": true,
                "never_cap_spam_a11y_require_full": true,
                "cef_bundle_required_at_cap_day": true,
            },
        });
        let receipt_path = workspace.join(UNLOCK_A_APPLIED_REL);
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::write(
            &receipt_path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&receipt)
                    .map_err(|error| format!("unlock-a receipt encode: {error}"))?
            ),
        )
        .map_err(|error| format!("write UNLOCK-A-APPLIED: {error}"))?;
        written.push(UNLOCK_A_APPLIED_REL);
    }

    if unlock == UnlockLetter::B && !dry_run {
        let Some(pilot) = pilot_command else {
            return Err(
                "Unlock B apply refused: grant.pilot_command must be non-empty; never invent pilot_command"
                    .into(),
            );
        };
        let receipt = json!({
            "schema": "humanymous.cursor-unlock-b-applied/1",
            "unlock": "B",
            "applied": true,
            "applied_utc": chrono_like_utc(),
            "grant": grant_path,
            "pilot_command": pilot,
            "destinations_written": written,
            "cap_burn_authorized": true,
            "cap_burn_limit": 1,
            "product": 0,
            "pass_product_clean_room": 0,
            "honesty": {
                "never_invent_pass_product_clean_room": true,
                "detection_freeze": true,
                "capture_component_is_not_product_binding": true,
                "n06_join_requires_measured_binding": true,
            },
        });
        let receipt_path = workspace.join(UNLOCK_B_APPLIED_REL);
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::write(
            &receipt_path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&receipt)
                    .map_err(|error| format!("unlock-b receipt encode: {error}"))?
            ),
        )
        .map_err(|error| format!("write UNLOCK-B-APPLIED: {error}"))?;
        written.push(UNLOCK_B_APPLIED_REL);
    }

    if unlock == UnlockLetter::N06 && !dry_run {
        let Some(pilot) = pilot_command else {
            return Err(
                "N06 hosted-venue apply refused: grant.pilot_command must be non-empty Cap CLI --w2-h2-hosted-venue; never invent pilot_command"
                    .into(),
            );
        };
        let receipt = json!({
            "schema": "humanymous.cursor-n06-hosted-venue-applied/1",
            "unlock": "N06",
            "applied": true,
            "applied_utc": chrono_like_utc(),
            "grant": grant_path,
            "pilot_command": pilot,
            "destinations_written": written,
            "cap_burn_authorized": true,
            "cap_burn_limit": 1,
            "requires_actual_hosted_linux": true,
            "product": 0,
            "pass_product_clean_room": 0,
            "honesty": {
                "never_invent_pass_product_clean_room": true,
                "desktop_as_hosted": "forbidden",
                "detection_freeze": true,
                "n06_join_requires_hosted_measured_binding_and_owner_integration": true,
                "desktop_measure_is_not_hosted_product": true,
                "no_github_actions_spoof": true,
            },
        });
        let receipt_path = workspace.join(N06_HOSTED_VENUE_APPLIED_REL);
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::write(
            &receipt_path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&receipt)
                    .map_err(|error| format!("n06 hosted-venue receipt encode: {error}"))?
            ),
        )
        .map_err(|error| format!("write N06-HOSTED-VENUE-APPLIED: {error}"))?;
        written.push(N06_HOSTED_VENUE_APPLIED_REL);
    }

    if unlock == UnlockLetter::G2 && !dry_run {
        let Some(pilot) = pilot_command else {
            return Err(
                "G2 apply refused: grant.pilot_command must be non-empty Cap CLI --g2-collector-product-binding; never invent pilot_command"
                    .into(),
            );
        };
        let receipt = json!({
            "schema": "humanymous.cursor-g2-collector-product-binding-applied/1",
            "unlock": "G2",
            "applied": true,
            "applied_utc": chrono_like_utc(),
            "grant": grant_path,
            "pilot_command": pilot,
            "destinations_written": written,
            "cap_burn_authorized": false,
            "cap_burn_limit": 1,
            "cap_realloc_required_before_burn": true,
            "cap_realloc_schema": "humanymous.cursor-g2-collector-product-binding-cap-realloc/1",
            "product": 0,
            "pass_product_clean_room": 0,
            "honesty": {
                "never_invent_pass_product_clean_room": true,
                "detection_freeze": true,
                "desktop_collector_component_is_not_product": true,
                "topology_apply_does_not_invent_cap_free": true,
                "cap_realloc_only_structural_feeder": true,
            },
        });
        let receipt_path = workspace.join(UNLOCK_G2_APPLIED_REL);
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::write(
            &receipt_path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&receipt)
                    .map_err(|error| format!("unlock-g2 receipt encode: {error}"))?
            ),
        )
        .map_err(|error| format!("write UNLOCK-G2-APPLIED: {error}"))?;
        written.push(UNLOCK_G2_APPLIED_REL);
    }

    if unlock == UnlockLetter::G5 && !dry_run {
        let Some(pilot) = pilot_command else {
            return Err(
                "G5 apply refused: grant.pilot_command must be non-empty Cap CLI --g5-witness-transport; never invent pilot_command"
                    .into(),
            );
        };
        let receipt = json!({
            "schema": "humanymous.cursor-g5-witness-transport-applied/1",
            "unlock": "G5",
            "applied": true,
            "applied_utc": chrono_like_utc(),
            "grant": grant_path,
            "pilot_command": pilot,
            "destinations_written": written,
            "cap_burn_authorized": false,
            "cap_burn_limit": 1,
            "cap_realloc_required_before_burn": true,
            "cap_realloc_schema": "humanymous.cursor-g5-witness-transport-cap-realloc/1",
            "product": 0,
            "pass_product_clean_room": 0,
            "authenticated_witness_transport": false,
            "honesty": {
                "never_invent_pass_product_clean_room": true,
                "detection_freeze": true,
                "custody_witness_component_is_not_authenticated_transport": true,
                "topology_apply_does_not_invent_cap_free": true,
                "topology_apply_does_not_invent_authenticated_transport": true,
                "cap_realloc_only_structural_feeder": true,
            },
        });
        let receipt_path = workspace.join(UNLOCK_G5_APPLIED_REL);
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::write(
            &receipt_path,
            format!(
                "{}
",
                serde_json::to_string_pretty(&receipt)
                    .map_err(|error| format!("unlock-g5 receipt encode: {error}"))?
            ),
        )
        .map_err(|error| format!("write UNLOCK-G5-APPLIED: {error}"))?;
        written.push(UNLOCK_G5_APPLIED_REL);
    }

    if unlock == UnlockLetter::CR && !dry_run {
        let Some(pilot) = pilot_command else {
            return Err(
                "CR apply refused: grant.pilot_command must be non-empty Cap CLI --clean-room-product; never invent pilot_command"
                    .into(),
            );
        };
        let receipt = json!({
            "schema": "humanymous.cursor-clean-room-product-applied/1",
            "unlock": "CR",
            "applied": true,
            "applied_utc": chrono_like_utc(),
            "grant": grant_path,
            "pilot_command": pilot,
            "destinations_written": written,
            "cap_burn_authorized": false,
            "cap_burn_limit": 1,
            "cap_realloc_required_before_burn": true,
            "cap_realloc_schema": "humanymous.cursor-clean-room-product-cap-realloc/1",
            "product": 0,
            "pass_product_clean_room": 0,
            "browser_ready": false,
            "n06_owner_join": false,
            "honesty": {
                "never_invent_pass_product_clean_room": true,
                "detection_freeze": true,
                "g2_g5_component_holds_are_not_product": true,
                "topology_apply_does_not_invent_cap_free": true,
                "cap_realloc_only_structural_feeder": true,
                "auth_at_absent_refuse": true,
            },
        });
        let receipt_path = workspace.join(UNLOCK_CR_APPLIED_REL);
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::write(
            &receipt_path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&receipt)
                    .map_err(|error| format!("unlock-cr receipt encode: {error}"))?
            ),
        )
        .map_err(|error| format!("write UNLOCK-CR-APPLIED: {error}"))?;
        written.push(UNLOCK_CR_APPLIED_REL);
    }

    if unlock == UnlockLetter::G6 && !dry_run {
        let Some(pilot) = pilot_command else {
            return Err(
                "G6 apply refused: grant.pilot_command must be non-empty Cap CLI --g6-origin-audit-binding; never invent pilot_command"
                    .into(),
            );
        };
        let receipt = json!({
            "schema": "humanymous.cursor-g6-origin-audit-binding-applied/1",
            "unlock": "G6",
            "applied": true,
            "applied_utc": chrono_like_utc(),
            "grant": grant_path,
            "pilot_command": pilot,
            "destinations_written": written,
            "cap_burn_authorized": false,
            "cap_burn_limit": 1,
            "cap_realloc_required_before_burn": true,
            "cap_realloc_schema": "humanymous.cursor-g6-origin-audit-binding-cap-realloc/1",
            "product": 0,
            "pass_product_clean_room": 0,
            "origin_audit_product_binding": false,
            "honesty": {
                "never_invent_pass_product_clean_room": true,
                "detection_freeze": true,
                "g5_cr_component_holds_are_not_origin_audit_product": true,
                "topology_apply_does_not_invent_cap_free": true,
                "cap_realloc_only_structural_feeder": true,
                "not_g2_g5_cr_cap_twin_spam": true,
            },
        });
        let receipt_path = workspace.join(UNLOCK_G6_APPLIED_REL);
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::write(
            &receipt_path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&receipt)
                    .map_err(|error| format!("unlock-g6 receipt encode: {error}"))?
            ),
        )
        .map_err(|error| format!("write UNLOCK-G6-APPLIED: {error}"))?;
        written.push(UNLOCK_G6_APPLIED_REL);
    }

    if unlock == UnlockLetter::RB && !dry_run {
        let Some(pilot) = pilot_command else {
            return Err(
                "RB apply refused: grant.pilot_command must be non-empty Cap CLI --red-blue-product --require-full; never invent pilot_command"
                    .into(),
            );
        };
        let receipt = json!({
            "schema": "humanymous.cursor-red-blue-product-applied/1",
            "unlock": "RB",
            "applied": true,
            "applied_utc": chrono_like_utc(),
            "grant": grant_path,
            "pilot_command": pilot,
            "destinations_written": written,
            "cap_burn_authorized": false,
            "cap_burn_limit": 1,
            "cap_realloc_required_before_burn": true,
            "cap_realloc_schema": "humanymous.cursor-red-blue-product-cap-realloc/1",
            "product": 0,
            "pass_product_clean_room": 0,
            "honesty": {
                "never_invent_pass_product_clean_room": true,
                "detection_freeze": true,
                "not_g2_g5_cr_g6_cap_twin_spam": true,
                "topology_apply_does_not_invent_cap_free": true,
                "cap_realloc_only_structural_feeder": true,
                "require_full_product_feeder": true,
            },
        });
        let receipt_path = workspace.join(UNLOCK_RB_APPLIED_REL);
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::write(
            &receipt_path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&receipt)
                    .map_err(|error| format!("unlock-rb receipt encode: {error}"))?
            ),
        )
        .map_err(|error| format!("write UNLOCK-RB-APPLIED: {error}"))?;
        written.push(UNLOCK_RB_APPLIED_REL);
    }

    if unlock == UnlockLetter::PM && !dry_run {
        let Some(pilot) = pilot_command else {
            return Err(
                "PM apply refused: grant.pilot_command must be non-empty Cap CLI --clean-room-product-matrix --require-full; never invent pilot_command"
                    .into(),
            );
        };
        let receipt = json!({
            "schema": "humanymous.cursor-clean-room-product-matrix-applied/1",
            "unlock": "PM",
            "applied": true,
            "applied_utc": chrono_like_utc(),
            "grant": grant_path,
            "pilot_command": pilot,
            "destinations_written": written,
            "cap_burn_authorized": false,
            "cap_burn_limit": 1,
            "cap_realloc_required_before_burn": true,
            "cap_realloc_schema": "humanymous.cursor-clean-room-product-matrix-cap-realloc/1",
            "product": 0,
            "pass_product_clean_room": 0,
            "honesty": {
                "never_invent_pass_product_clean_room": true,
                "detection_freeze": true,
                "not_spent_clean_room_product_cap_twin": true,
                "not_g2_g5_g6_rb_cap_twin_spam": true,
                "topology_apply_does_not_invent_cap_free": true,
                "cap_realloc_only_structural_feeder": true,
                "require_full_product_matrix_feeder": true,
                "binds_e2e_exit0_auth_at_refuse": true,
            },
        });
        let receipt_path = workspace.join(UNLOCK_PM_APPLIED_REL);
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::write(
            &receipt_path,
            format!(
                "{}
",
                serde_json::to_string_pretty(&receipt)
                    .map_err(|error| format!("unlock-pm receipt encode: {error}"))?
            ),
        )
        .map_err(|error| format!("write UNLOCK-PM-APPLIED: {error}"))?;
        written.push(UNLOCK_PM_APPLIED_REL);
    }

    let mut report = Map::new();
    report.insert("schema".into(), json!(unlock.apply_schema()));
    report.insert("record_kind".into(), json!(unlock.record_kind_apply()));
    report.insert("unlock".into(), json!(unlock.as_str()));
    report.insert("applied".into(), json!(!dry_run));
    report.insert("dry_run".into(), json!(dry_run));
    report.insert("grant".into(), json!(grant_path));
    report.insert(unlock.ready_key().into(), json!(true));
    report.insert("cap_execute".into(), json!(false));
    report.insert("product".into(), json!(0));
    report.insert("pass_product_clean_room".into(), json!(0));
    report.insert("full_product_qualified".into(), json!(false));
    if let Some(pilot) = pilot_command {
        report.insert("pilot_command".into(), json!(pilot));
    }
    report.insert(
        "honesty".into(),
        json!({
            "never_invent_pass_product_clean_room": true,
            "authority_cap_green_is_not_product": true,
            "capture_component_is_not_product_binding": true,
            "cap_burn_is_separate_step": true,
            "never_invent_desktop_as_hosted": true,
            "g2_topology_apply_does_not_invent_cap_free": unlock == UnlockLetter::G2,
            "g5_topology_apply_does_not_invent_cap_free": unlock == UnlockLetter::G5,
            "g5_topology_apply_does_not_invent_authenticated_transport": unlock == UnlockLetter::G5,
        }),
    );
    report.insert("planned".into(), json!(planned));
    report.insert("written".into(), json!(written));
    report.insert(
        "next".into(),
        json!(if dry_run {
            "Re-run without --dry-run after Root confirms grant; then Cap=1"
        } else {
            match unlock {
                UnlockLetter::A => {
                    "Cap=1 burn matching cap-ready/g4-cef-at pack using grant pilot_command; Cap day refuse-closed if HUMANYMOUS_CEF_PILOT_BUNDLE absent; matrix honesty retained; product=0"
                }
                UnlockLetter::B => {
                    "Cap=1 burn matching cap-ready/w2-h2-product-binding pack using grant pilot_command; matrix honesty retained; product=0"
                }
                UnlockLetter::C => {
                    "Cap=1 burn matching cap-ready/authority-node-ipc pack; matrix honesty retained"
                }
                UnlockLetter::N06 => {
                    "Cap=1 once only when actual_hosted_linux using --w2-h2-hosted-venue; Desktop Cap=0 fail-closed with exact runner requirement; never invent pass_product_clean_room or desktop_as_hosted"
                }
                UnlockLetter::G2 => {
                    "Cap realloc schema humanymous.cursor-g2-collector-product-binding-cap-realloc/1 citing live Cap CLI+topology; then Cap=1 once --g2-collector-product-binding (jobs=1); Cap free conserved until realloc; never invent pass_product_clean_room"
                }
                UnlockLetter::G5 => {
                    "Cap realloc schema humanymous.cursor-g5-witness-transport-cap-realloc/1 citing live Cap CLI+topology; then Cap=1 once --g5-witness-transport (jobs=1); Cap free conserved until realloc; authenticated_witness_transport stays false until Cap measured; never invent pass_product_clean_room"
                }
                UnlockLetter::CR => {
                    "Cap realloc schema humanymous.cursor-clean-room-product-cap-realloc/1 citing live Cap CLI+topology; then Cap=1 once --clean-room-product (jobs=1); Cap free conserved until realloc; G2+G5 holds bound; AUTH/AT refuse; never invent pass_product_clean_room"
                }
                UnlockLetter::G6 => {
                    "Cap realloc schema humanymous.cursor-g6-origin-audit-binding-cap-realloc/1 citing live Cap CLI+topology; then Cap=1 once --g6-origin-audit-binding (jobs=1); Cap free conserved until realloc; G5+CR holds bound; never invent pass_product_clean_room"
                }
                UnlockLetter::RB => {
                    "Cap realloc schema humanymous.cursor-red-blue-product-cap-realloc/1 citing live Cap CLI+topology; then Cap=1 once --red-blue-product --require-full (jobs=1); Cap free conserved until realloc; product/red-blue feeder not Cap twin; never invent pass_product_clean_room"
                }
                UnlockLetter::PM => {
                    "Cap realloc schema humanymous.cursor-clean-room-product-matrix-cap-realloc/1 citing live Cap CLI+topology; then Cap=1 once --clean-room-product-matrix --require-full (jobs=1); Cap free conserved until realloc; product-matrix feeder not spent CR Cap twin; never invent pass_product_clean_room"
                }
            }
        }),
    );
    Ok(Value::Object(report))
}

fn chrono_like_utc() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

/// Fail-closed refuse of documented Cap-spam / folklore Cap shapes for Unlock A.
///
/// Citations: cap-ready/g4-cef-at/COMMANDS.md §3 and harness.json
/// `forbidden_reburns`. Only the code-defined Cap CLI `--g4-cef-at` is accepted
/// as lab-runner Unlock A pilot; a11y/`--require-full` and foreign Cap
/// selectors stay refuse-closed.
fn refuse_forbidden_unlock_a_pilot_command(pilot: &str) -> Result<()> {
    let normalized = pilot.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = normalized.to_ascii_lowercase();
    if lower.contains(G4_CEF_CAP_SELECTOR)
        && (lower.contains("humanymous-lab-runner") || lower.contains("lab-runner"))
    {
        if lower.contains("--require-full")
            || [
                "--browser-product",
                "--collector",
                "--mount-probe",
                "--authority-node-ipc",
                "--w2-h2-product-binding",
                "--w2-h2-hosted-venue",
                "--go-product",
            ]
            .iter()
            .any(|flag| lower.contains(flag))
        {
            return Err(
                "Unlock A grant.pilot_command refused: --g4-cef-at Cap requires its own fresh profile; refuse foreign Cap selectors / --require-full; never invent pass_product_clean_room"
                    .into(),
            );
        }
        return Ok(());
    }
    if lower.contains("humanymous-lab-runner") || lower.contains("lab-runner") {
        let feeder = [
            "--browser-product",
            "--collector",
            "--mount-probe",
            "--authority-node-ipc",
            "--w2-h2-product-binding",
            "--w2-h2-hosted-venue",
            "--go-product",
        ]
        .iter()
        .any(|flag| lower.contains(flag));
        let require_full = lower.contains("--require-full");
        if feeder || require_full {
            return Err(format!(
                "Unlock A grant.pilot_command refused: lab-runner feeder / --require-full Cap spam is not G4 CEF/AT; use code-defined {G4_CODE_DEFINED_PILOT}"
            )
            .into());
        }
    }
    if !lower.contains(G4_CEF_CAP_SELECTOR) {
        return Err(format!(
            "Unlock A grant.pilot_command refused: must embed Cap CLI {G4_CEF_CAP_SELECTOR}; code-defined pilot is {G4_CODE_DEFINED_PILOT}; never invent CEF folklore argv"
        )
        .into());
    }
    Ok(())
}

/// Fail-closed refuse of documented non-product-binding Cap shapes for Unlock B.
///
/// Citations: cap-ready/w2-h2-product-binding/COMMANDS.md §2 (capture run
/// 42479169280 forbidden as product-binding) and harness.json
/// `forbidden_reburns`. Does not invent an allowlist of good argv — only
/// rejects known capture/feeder reburns so Root-named GRANT.pilot_command
/// cannot accidentally Cap-spam component receipts.
fn refuse_forbidden_unlock_b_pilot_command(pilot: &str) -> Result<()> {
    let normalized = pilot.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = normalized.to_ascii_lowercase();
    if lower.contains("native-component") && lower.contains("capture") {
        return Err(
            "Unlock B grant.pilot_command refused: native-component capture is pass_component_hosted only (run 42479169280); not hosted gateway H2 product-binding; never invent product_binding from observe_h2"
                .into(),
        );
    }
    // Code-defined Cap CLI is the only lab-runner Unlock B shape (mirrors Unlock C).
    if lower.contains("--w2-h2-product-binding")
        && (lower.contains("humanymous-lab-runner") || lower.contains("lab-runner"))
    {
        if lower.contains("--require-full")
            || [
                "--browser-product",
                "--collector",
                "--mount-probe",
                "--authority-node-ipc",
                "--g4-cef-at",
                "--w2-h2-hosted-venue",
                "--go-product",
            ]
            .iter()
            .any(|flag| lower.contains(flag))
        {
            return Err(
                "Unlock B grant.pilot_command refused: --w2-h2-product-binding Cap requires its own fresh profile; refuse foreign Cap selectors / --require-full; never invent pass_product_clean_room"
                    .into(),
            );
        }
        return Ok(());
    }
    if lower.contains("humanymous-lab-runner") || lower.contains("lab-runner") {
        let feeder = [
            "--browser-product",
            "--collector",
            "--mount-probe",
            "--authority-node-ipc",
            "--g4-cef-at",
            "--w2-h2-hosted-venue",
            "--go-product",
        ]
        .iter()
        .any(|flag| lower.contains(flag));
        let require_full = lower.contains("--require-full");
        if feeder || require_full {
            return Err(
                "Unlock B grant.pilot_command refused: lab-runner feeder / --require-full Cap spam is not W2 H2 product-binding; use code-defined --w2-h2-product-binding Cap CLI"
                    .into(),
            );
        }
    }
    Ok(())
}

fn refuse_forbidden_g6_origin_audit_binding_pilot_command(pilot: &str) -> Result<()> {
    let lower = pilot.to_ascii_lowercase();
    if lower.contains(G6_CAP_SELECTOR)
        && (lower.contains("--require-full")
            || lower.contains("--g2-collector-product-binding")
            || lower.contains("--g5-witness-transport")
            || lower.contains("--clean-room-product")
            || lower.contains("--browser-product")
            || lower.contains("--collector")
            || lower.contains("--authority-node-ipc")
            || lower.contains("--g4-cef-at")
            || lower.contains("--w2-h2"))
    {
        return Err(
            "G6 grant.pilot_command refused: --g6-origin-audit-binding Cap requires its own fresh profile; refuse foreign Cap selectors / G2/G5/CR twin/--require-full Cap spam; never invent pass_product_clean_room"
                .into(),
        );
    }
    if (lower.contains("--require-full") && !lower.contains(G6_CAP_SELECTOR))
        || lower.contains("--browser-product")
        || lower.contains("--collector")
        || lower.contains("observe_h2")
    {
        return Err(format!(
            "G6 grant.pilot_command refused: lab-runner feeder / --require-full Cap spam is not G6 origin-audit Cap; use code-defined {G6_CODE_DEFINED_PILOT}"
        )
        .into());
    }
    if !lower.contains(G6_CAP_SELECTOR) {
        return Err(format!(
            "G6 grant.pilot_command refused: must embed Cap CLI {G6_CAP_SELECTOR}; code-defined pilot is {G6_CODE_DEFINED_PILOT}; never invent pass_product_clean_room"
        )
        .into());
    }
    Ok(())
}
fn refuse_forbidden_clean_room_product_pilot_command(pilot: &str) -> Result<()> {
    let lower = pilot.to_ascii_lowercase();
    if lower.contains(CR_CAP_SELECTOR)
        && (lower.contains("--require-full")
            || lower.contains("--g2-collector-product-binding")
            || lower.contains("--g5-witness-transport")
            || lower.contains("--g6-origin-audit-binding")
            || lower.contains("--browser-product")
            || lower.contains("--collector")
            || lower.contains("--authority-node-ipc")
            || lower.contains("--g4-cef-at")
            || lower.contains("--w2-h2"))
    {
        return Err(
            "CR grant.pilot_command refused: --clean-room-product Cap requires its own fresh profile; refuse foreign Cap selectors / --require-full Cap spam; never invent pass_product_clean_room"
                .into(),
        );
    }
    if (lower.contains("--require-full") && !lower.contains(CR_CAP_SELECTOR))
        || lower.contains("--browser-product")
        || lower.contains("--collector")
        || lower.contains("observe_h2")
    {
        return Err(format!(
            "CR grant.pilot_command refused: lab-runner feeder / --require-full Cap spam is not clean-room product Cap; use code-defined {CR_CODE_DEFINED_PILOT}"
        )
        .into());
    }
    if !lower.contains(CR_CAP_SELECTOR) {
        return Err(format!(
            "CR grant.pilot_command refused: must embed Cap CLI {CR_CAP_SELECTOR}; code-defined pilot is {CR_CODE_DEFINED_PILOT}; never invent pass_product_clean_room"
        )
        .into());
    }
    Ok(())
}
fn refuse_forbidden_g5_witness_transport_pilot_command(pilot: &str) -> Result<()> {
    let normalized = pilot.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = normalized.to_ascii_lowercase();
    if lower.contains(G5_CAP_SELECTOR)
        && (lower.contains("humanymous-lab-runner") || lower.contains("lab-runner"))
    {
        if lower.contains("--require-full")
            || [
                "--browser-product",
                "--collector",
                "--mount-probe",
                "--authority-node-ipc",
                "--w2-h2-product-binding",
                "--w2-h2-hosted-venue",
                "--g4-cef-at",
                "--go-product",
                "--g2-collector-product-binding",
                "--g2-collector-product-binding-readiness",
                "--g5-witness-transport-honesty",
            ]
            .iter()
            .any(|flag| lower.contains(flag))
        {
            return Err(
                "G5 grant.pilot_command refused: --g5-witness-transport Cap requires its own fresh profile; refuse foreign Cap selectors / honesty/--require-full Cap spam; never invent authenticated witness transport product"
                    .into(),
            );
        }
        // Exact Cap selector must not be only the honesty twin substring falsely accepted.
        // "--g5-witness-transport-honesty" contains CAP_SELECTOR as prefix; blocked above.
        return Ok(());
    }
    if lower.contains("humanymous-lab-runner") || lower.contains("lab-runner") {
        let feeder = [
            "--browser-product",
            "--collector",
            "--mount-probe",
            "--authority-node-ipc",
            "--w2-h2-product-binding",
            "--w2-h2-hosted-venue",
            "--g4-cef-at",
            "--go-product",
            "--g2-collector-product-binding",
            "--g5-witness-transport-honesty",
        ]
        .iter()
        .any(|flag| lower.contains(flag));
        let require_full = lower.contains("--require-full");
        if feeder || require_full {
            return Err(format!(
                "G5 grant.pilot_command refused: lab-runner feeder / --require-full Cap spam is not G5 witness transport; use code-defined {G5_CODE_DEFINED_PILOT}"
            )
            .into());
        }
    }
    if !lower.contains(G5_CAP_SELECTOR) || lower.contains("--g5-witness-transport-honesty") {
        return Err(format!(
            "G5 grant.pilot_command refused: must embed Cap CLI {G5_CAP_SELECTOR} (not honesty twin); code-defined pilot is {G5_CODE_DEFINED_PILOT}; never invent custody component as authenticated transport"
        )
        .into());
    }
    Ok(())
}

fn refuse_forbidden_g2_collector_product_binding_pilot_command(pilot: &str) -> Result<()> {
    let normalized = pilot.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = normalized.to_ascii_lowercase();
    if lower.contains(G2_CAP_SELECTOR)
        && (lower.contains("humanymous-lab-runner") || lower.contains("lab-runner"))
    {
        if lower.contains("--require-full")
            || [
                "--browser-product",
                "--collector",
                "--mount-probe",
                "--authority-node-ipc",
                "--w2-h2-product-binding",
                "--w2-h2-hosted-venue",
                "--g4-cef-at",
                "--go-product",
                "--g2-collector-product-binding-readiness",
                "--g5-witness-transport",
                "--g5-witness-transport-honesty",
            ]
            .iter()
            .any(|flag| lower.contains(flag))
        {
            return Err(
                "G2 grant.pilot_command refused: --g2-collector-product-binding Cap requires its own fresh profile; refuse foreign Cap selectors / Desktop collector --require-full / Cap=0 readiness reburn; never invent pass_product_clean_room"
                    .into(),
            );
        }
        return Ok(());
    }
    if lower.contains("humanymous-lab-runner") || lower.contains("lab-runner") {
        let feeder = [
            "--browser-product",
            "--collector",
            "--mount-probe",
            "--authority-node-ipc",
            "--w2-h2-product-binding",
            "--w2-h2-hosted-venue",
            "--g4-cef-at",
            "--go-product",
            "--g2-collector-product-binding-readiness",
        ]
        .iter()
        .any(|flag| lower.contains(flag));
        let require_full = lower.contains("--require-full");
        if feeder || require_full {
            return Err(format!(
                "G2 grant.pilot_command refused: lab-runner feeder / --require-full Cap spam is not G2 collector product-binding; use code-defined {G2_CODE_DEFINED_PILOT}"
            )
            .into());
        }
    }
    if !lower.contains(G2_CAP_SELECTOR) {
        return Err(format!(
            "G2 grant.pilot_command refused: must embed Cap CLI {G2_CAP_SELECTOR}; code-defined pilot is {G2_CODE_DEFINED_PILOT}; never invent Desktop collector component as G2 product"
        )
        .into());
    }
    Ok(())
}

fn refuse_forbidden_red_blue_product_pilot_command(pilot: &str) -> Result<()> {
    let lower = pilot.to_ascii_lowercase();
    if lower.contains(RB_CAP_SELECTOR)
        && (lower.contains("humanymous-lab-runner") || lower.contains("lab-runner"))
    {
        if [
            "--g2-collector-product-binding",
            "--g5-witness-transport",
            "--clean-room-product",
            "--clean-room-product-matrix",
            "--g6-origin-audit-binding",
            "--browser-product",
            "--collector",
            "--mount-probe",
            "--authority-node-ipc",
            "--w2-h2-product-binding",
            "--w2-h2-hosted-venue",
            "--g4-cef-at",
            "--go-product",
        ]
        .iter()
        .any(|flag| lower.contains(flag))
        {
            return Err(
                "RB grant.pilot_command refused: --red-blue-product Cap requires its own fresh profile; refuse G2/G5/CR/G6/PM Cap twin / foreign Cap selectors; never invent pass_product_clean_room"
                    .into(),
            );
        }
        if !lower.contains("--require-full") {
            return Err(
                "RB grant.pilot_command refused: product/red-blue feeder requires --require-full; Cap=0 honesty / Cap=1 feeder; never invent pass_product_clean_room"
                    .into(),
            );
        }
        return Ok(());
    }
    if !lower.contains(RB_CAP_SELECTOR) {
        return Err(format!(
            "RB grant.pilot_command refused: must embed Cap CLI {RB_CAP_SELECTOR}; code-defined pilot is {RB_CODE_DEFINED_PILOT}; never invent pass_product_clean_room"
        )
        .into());
    }
    Ok(())
}

fn refuse_forbidden_clean_room_product_matrix_pilot_command(pilot: &str) -> Result<()> {
    let lower = pilot.to_ascii_lowercase();
    if lower.contains(PM_CAP_SELECTOR)
        && (lower.contains("humanymous-lab-runner") || lower.contains("lab-runner"))
    {
        // Strip PM selector first: `--clean-room-product` is a substring of
        // `--clean-room-product-matrix` and must not self-refuse.
        let without_pm = lower.replace(PM_CAP_SELECTOR, "");
        let foreign = [
            "--g2-collector-product-binding",
            "--g5-witness-transport",
            "--clean-room-product",
            "--g6-origin-audit-binding",
            "--red-blue-product",
            "--browser-product",
            "--collector",
            "--mount-probe",
            "--authority-node-ipc",
            "--w2-h2-product-binding",
            "--w2-h2-hosted-venue",
            "--g4-cef-at",
            "--go-product",
        ]
        .iter()
        .any(|flag| without_pm.contains(flag));
        if foreign {
            return Err(
                "PM grant.pilot_command refused: --clean-room-product-matrix Cap requires its own fresh profile; refuse spent CR Cap twin / G2/G5/G6/RB Cap twin / foreign Cap selectors; never invent pass_product_clean_room"
                    .into(),
            );
        }
        if !lower.contains("--require-full") {
            return Err(
                "PM grant.pilot_command refused: product-matrix feeder requires --require-full; Cap=0 honesty / Cap=1 feeder; never invent pass_product_clean_room"
                    .into(),
            );
        }
        return Ok(());
    }
    if !lower.contains(PM_CAP_SELECTOR) {
        return Err(format!(
            "PM grant.pilot_command refused: must embed Cap CLI {PM_CAP_SELECTOR}; code-defined pilot is {PM_CODE_DEFINED_PILOT}; never invent pass_product_clean_room"
        )
        .into());
    }
    Ok(())
}

fn refuse_forbidden_n06_hosted_venue_pilot_command(pilot: &str) -> Result<()> {
    let normalized = pilot.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = normalized.to_ascii_lowercase();
    if lower.contains(N06_HOSTED_VENUE_CAP_SELECTOR)
        && (lower.contains("humanymous-lab-runner") || lower.contains("lab-runner"))
    {
        if lower.contains("--require-full")
            || [
                "--browser-product",
                "--collector",
                "--mount-probe",
                "--authority-node-ipc",
                "--w2-h2-product-binding",
                "--g4-cef-at",
                "--go-product",
            ]
            .iter()
            .any(|flag| lower.contains(flag))
        {
            return Err(
                "N06 hosted-venue grant.pilot_command refused: --w2-h2-hosted-venue Cap requires its own fresh profile; refuse foreign Cap selectors / Desktop measure reburn; never invent pass_product_clean_room"
                    .into(),
            );
        }
        return Ok(());
    }
    if lower.contains("native-component") && lower.contains("capture") {
        return Err(
            "N06 hosted-venue grant.pilot_command refused: capture observe_h2 is not hosted venue Cap; never invent product from observe_h2"
                .into(),
        );
    }
    if !lower.contains(N06_HOSTED_VENUE_CAP_SELECTOR) {
        return Err(format!(
            "N06 hosted-venue grant.pilot_command refused: must embed Cap CLI {N06_HOSTED_VENUE_CAP_SELECTOR}; code-defined pilot is {N06_HOSTED_VENUE_PILOT}; never invent desktop_as_hosted"
        )
        .into());
    }
    Ok(())
}

fn validate_grant(grant: &Value, manifest: &Value, unlock: UnlockLetter) -> Result<()> {
    let expected_schema = unlock.grant_schema();
    if grant.get("schema").and_then(Value::as_str) != Some(expected_schema) {
        return Err(format!("grant.schema must be {expected_schema}").into());
    }
    if grant.get("root_authorized").and_then(Value::as_bool) != Some(true) {
        return Err("grant.root_authorized must be true".into());
    }
    if grant.get("allow_apply").and_then(Value::as_bool) != Some(true) {
        return Err("grant.allow_apply must be true".into());
    }
    if grant.get("unlock").and_then(Value::as_str) != Some(unlock.as_str()) {
        return Err(format!("grant.unlock must be {}", unlock.as_str()).into());
    }
    if unlock == UnlockLetter::B {
        let pilot = grant
            .get("pilot_command")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if pilot.is_empty() {
            return Err(
                "Unlock B grant.pilot_command must be a non-empty Root-named Cap argv; never invent pilot_command; template alone is not a live grant"
                    .into(),
            );
        }
        refuse_forbidden_unlock_b_pilot_command(pilot)?;
    }
    if unlock == UnlockLetter::N06 {
        let pilot = grant
            .get("pilot_command")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if pilot.is_empty() {
            return Err(
                "N06 hosted-venue grant.pilot_command must be a non-empty Cap CLI --w2-h2-hosted-venue argv; never invent pilot_command; template alone is not a live grant"
                    .into(),
            );
        }
        refuse_forbidden_n06_hosted_venue_pilot_command(pilot)?;
        if grant
            .get("requires_actual_hosted_linux")
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err(
                "N06 hosted-venue grant.requires_actual_hosted_linux must be true; Desktop Cap cannot invent hosted venue"
                    .into(),
            );
        }
    }
    if unlock == UnlockLetter::G2 {
        let pilot = grant
            .get("pilot_command")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if pilot.is_empty() {
            return Err(
                "G2 grant.pilot_command must be a non-empty Cap CLI --g2-collector-product-binding argv; never invent pilot_command; template alone is not a live grant"
                    .into(),
            );
        }
        refuse_forbidden_g2_collector_product_binding_pilot_command(pilot)?;
    }
    if unlock == UnlockLetter::G5 {
        let pilot = grant
            .get("pilot_command")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if pilot.is_empty() {
            return Err(
                "G5 grant.pilot_command must be a non-empty Cap CLI --g5-witness-transport argv; never invent pilot_command; template alone is not a live grant"
                    .into(),
            );
        }
        refuse_forbidden_g5_witness_transport_pilot_command(pilot)?;
    }
    if unlock == UnlockLetter::CR {
        let pilot = grant
            .get("pilot_command")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if pilot.is_empty() {
            return Err(
                "CR grant.pilot_command must be a non-empty Cap CLI --clean-room-product argv; never invent pilot_command; template alone is not a live grant"
                    .into(),
            );
        }
        refuse_forbidden_clean_room_product_pilot_command(pilot)?;
    }
    if unlock == UnlockLetter::G6 {
        let pilot = grant
            .get("pilot_command")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if pilot.is_empty() {
            return Err(
                "G6 grant.pilot_command must be a non-empty Cap CLI --g6-origin-audit-binding argv; never invent pilot_command; template alone is not a live grant"
                    .into(),
            );
        }
        refuse_forbidden_g6_origin_audit_binding_pilot_command(pilot)?;
    }
    if unlock == UnlockLetter::RB {
        let pilot = grant
            .get("pilot_command")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(
                "RB grant.pilot_command must be a non-empty Cap CLI --red-blue-product --require-full argv; never invent pilot_command; template alone is not a live grant"
            )?;
        refuse_forbidden_red_blue_product_pilot_command(pilot)?;
    }

    if unlock == UnlockLetter::PM {
        let pilot = grant
            .get("pilot_command")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(
                "PM grant.pilot_command must be a non-empty Cap CLI --clean-room-product-matrix --require-full argv; never invent pilot_command; template alone is not a live grant"
            )?;
        refuse_forbidden_clean_room_product_matrix_pilot_command(pilot)?;
    }

    if unlock == UnlockLetter::A {
        let release = grant
            .get("path_release_required")
            .and_then(Value::as_array)
            .map(|items| !items.is_empty())
            .unwrap_or(false);
        if !release {
            return Err(
                "Unlock A grant.path_release_required must name Root CEF/AT paths (crates/browser/** and/or scripts/ci/cef-pilot/**); never invent path release; template alone is not a live grant"
                    .into(),
            );
        }
        let pilot = grant
            .get("pilot_command")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if pilot.is_empty() {
            return Err(
                "Unlock A grant.pilot_command must be a non-empty Cap CLI --g4-cef-at argv; never invent pilot_command; template alone is not a live grant"
                    .into(),
            );
        }
        refuse_forbidden_unlock_a_pilot_command(pilot)?;
    }
    let destinations = grant
        .get("destinations")
        .and_then(Value::as_array)
        .ok_or("grant.destinations array required")?;
    if destinations.is_empty() {
        return Err(if unlock == UnlockLetter::A {
            "Unlock A grant.destinations must be non-empty after Root path release; empty scaffold is not executable"
                .into()
        } else {
            "grant.destinations must be non-empty".into()
        });
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
        assert!(
            probe["mismatches"]
                .as_array()
                .expect("mismatches")
                .iter()
                .any(|item| item["reason"] == "preimage_mismatch")
        );
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
        assert!(
            !root
                .join("workspace/deployments/lab/authority/CAP-PILOT.md")
                .exists()
        );
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
        assert!(
            root.join("workspace/deployments/lab/authority/CAP-PILOT.md")
                .exists()
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unlock_b_grant_refuses_capture_component_pilot_command() {
        let err = refuse_forbidden_unlock_b_pilot_command(
            "cargo run --locked -p humanymous-delivery -- native-component capture",
        )
        .expect_err("capture refused");
        assert!(err.to_string().contains("native-component capture"));
        let err = refuse_forbidden_unlock_b_pilot_command(
            "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --collector --require-full",
        )
        .expect_err("feeder require-full refused");
        assert!(err.to_string().contains("feeder"));
        refuse_forbidden_unlock_b_pilot_command(
            "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --w2-h2-product-binding",
        )
        .expect("code-defined Cap CLI pilot accepted");
        refuse_forbidden_unlock_b_pilot_command(
            "root-named-hosted-gateway-h2-product-binding-pilot",
        )
        .expect("non-feeder Root argv still accepted without inventing allowlist");
    }

    #[test]
    fn unlock_b_grant_refuses_null_pilot_command() {
        let root = temp_dir("b-null-pilot");
        let contract = b"{\"pilot_command\":null}\n";
        let notes = b"h2 binding notes\n";
        write(
            &root.join("pack/candidate/deployments/lab/gateway/h2-product-binding-contract.json"),
            contract,
        );
        write(
            &root.join("pack/candidate/deployments/lab/gateway/H2-PRODUCT-BINDING.md"),
            notes,
        );
        let manifest = json!({
            "schema": "humanymous.cursor-unlock-b-h2-binding-candidate/1",
            "unlock": "B",
            "entries": [
                {
                    "destination": "deployments/lab/gateway/h2-product-binding-contract.json",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/deployments/lab/gateway/h2-product-binding-contract.json",
                    "candidateSha256": snapshot::digest(contract),
                },
                {
                    "destination": "deployments/lab/gateway/H2-PRODUCT-BINDING.md",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/deployments/lab/gateway/H2-PRODUCT-BINDING.md",
                    "candidateSha256": snapshot::digest(notes),
                }
            ]
        });
        let manifest_path = root.join("pack/transfer-manifest.json");
        write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap().as_bytes(),
        );
        let probe =
            probe_manifest(&manifest, &root.join("workspace"), &manifest_path).expect("probe");
        assert_eq!(probe["unlock_b_preimage_ready"], true);
        let grant = json!({
            "schema": GRANT_SCHEMA_B,
            "root_authorized": true,
            "allow_apply": true,
            "unlock": "B",
            "pilot_command": null,
            "destinations": [
                "deployments/lab/gateway/h2-product-binding-contract.json",
                "deployments/lab/gateway/H2-PRODUCT-BINDING.md"
            ]
        });
        let grant_path = root.join("grant.json");
        write(
            &grant_path,
            serde_json::to_string_pretty(&grant).unwrap().as_bytes(),
        );
        let err = apply_transfer(
            &manifest,
            &probe,
            Some(grant_path.to_str().unwrap()),
            &root.join("workspace"),
            false,
        )
        .expect_err("null pilot refused");
        assert!(err.to_string().contains("pilot_command"));
        assert!(!root.join("workspace").join(UNLOCK_B_APPLIED_REL).exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unlock_a_empty_entries_not_ready() {
        let root = temp_dir("a-empty");
        let manifest = json!({
            "schema": "humanymous.cursor-unlock-a-g4-cef-at-candidate/1",
            "unlock": "A",
            "entries": []
        });
        let manifest_path = root.join("pack/transfer-manifest.json");
        write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap().as_bytes(),
        );
        let probe =
            probe_manifest(&manifest, &root.join("workspace"), &manifest_path).expect("probe");
        assert_eq!(probe["schema"], PROBE_SCHEMA_A);
        assert_eq!(probe["unlock_a_preimage_ready"], false);
        assert_eq!(probe["pass_product_clean_room"], 0);
        assert_eq!(
            probe["honesty"]["unlock_a_requires_root_path_release_destinations"],
            true
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unlock_a_grant_refuses_empty_destinations() {
        let root = temp_dir("a-empty-dest");
        let body = b"cef pilot notes\n";
        write(
            &root.join("pack/candidate/scripts/ci/cef-pilot/NOTES.md"),
            body,
        );
        let manifest = json!({
            "schema": "humanymous.cursor-unlock-a-g4-cef-at-candidate/1",
            "unlock": "A",
            "entries": [
                {
                    "destination": "scripts/ci/cef-pilot/NOTES.md",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/scripts/ci/cef-pilot/NOTES.md",
                    "candidateSha256": snapshot::digest(body),
                }
            ]
        });
        let manifest_path = root.join("pack/transfer-manifest.json");
        write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap().as_bytes(),
        );
        let probe =
            probe_manifest(&manifest, &root.join("workspace"), &manifest_path).expect("probe");
        assert_eq!(probe["unlock_a_preimage_ready"], true);
        let grant = json!({
            "schema": GRANT_SCHEMA_A,
            "root_authorized": true,
            "allow_apply": true,
            "unlock": "A",
            "path_release_required": ["scripts/ci/cef-pilot/**"],
            "pilot_command": G4_CODE_DEFINED_PILOT,
            "destinations": []
        });
        let grant_path = root.join("grant.json");
        write(
            &grant_path,
            serde_json::to_string_pretty(&grant).unwrap().as_bytes(),
        );
        let err = apply_transfer(
            &manifest,
            &probe,
            Some(grant_path.to_str().unwrap()),
            &root.join("workspace"),
            false,
        )
        .expect_err("empty destinations refused");
        assert!(err.to_string().contains("destinations"));
        assert!(!root.join("workspace").join(UNLOCK_A_APPLIED_REL).exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unlock_a_grant_refuses_missing_pilot_command() {
        let root = temp_dir("a-no-pilot");
        let body = b"cef pilot notes\n";
        write(
            &root.join("pack/candidate/scripts/ci/cef-pilot/NOTES.md"),
            body,
        );
        let manifest = json!({
            "schema": "humanymous.cursor-unlock-a-g4-cef-at-candidate/1",
            "unlock": "A",
            "entries": [
                {
                    "destination": "scripts/ci/cef-pilot/NOTES.md",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/scripts/ci/cef-pilot/NOTES.md",
                    "candidateSha256": snapshot::digest(body),
                }
            ]
        });
        let manifest_path = root.join("pack/transfer-manifest.json");
        write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap().as_bytes(),
        );
        let probe =
            probe_manifest(&manifest, &root.join("workspace"), &manifest_path).expect("probe");
        let grant = json!({
            "schema": GRANT_SCHEMA_A,
            "root_authorized": true,
            "allow_apply": true,
            "unlock": "A",
            "path_release_required": ["scripts/ci/cef-pilot/**"],
            "destinations": ["scripts/ci/cef-pilot/NOTES.md"]
        });
        let grant_path = root.join("grant.json");
        write(
            &grant_path,
            serde_json::to_string_pretty(&grant).unwrap().as_bytes(),
        );
        let err = apply_transfer(
            &manifest,
            &probe,
            Some(grant_path.to_str().unwrap()),
            &root.join("workspace"),
            false,
        )
        .expect_err("missing pilot refused");
        assert!(err.to_string().contains("pilot_command"));
        assert!(err.to_string().contains(G4_CEF_CAP_SELECTOR));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unlock_a_grant_refuses_a11y_require_full_spam() {
        let err = refuse_forbidden_unlock_a_pilot_command(
            "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --browser-product --require-full",
        )
        .expect_err("a11y spam refused");
        assert!(err.to_string().contains("Cap spam") || err.to_string().contains("g4-cef-at"));
        let ok = refuse_forbidden_unlock_a_pilot_command(G4_CODE_DEFINED_PILOT);
        assert!(ok.is_ok());
    }

    fn sample_g2_pack(root: &Path) -> (PathBuf, Value) {
        let compose = b"services: {}\n";
        let dockerfile = b"FROM scratch\n";
        let dockerignore = b"*\n";
        let contract = b"{\"pilot_command\":null}\n";
        let scaffold = b"{\"status\":\"scaffold\"}\n";
        let pilot_sh = b"#!/bin/sh\nexit 0\n";
        write(
            &root.join("pack/candidate/deployments/lab/compose.g2-collector-product-binding.yaml"),
            compose,
        );
        write(
            &root.join("pack/candidate/build/rust/g2-collector-product-binding-lab.Dockerfile"),
            dockerfile,
        );
        write(
            &root.join(
                "pack/candidate/build/rust/g2-collector-product-binding-lab.Dockerfile.dockerignore",
            ),
            dockerignore,
        );
        write(
            &root.join(
                "pack/candidate/deployments/lab/collector/g2-collector-product-binding-contract.json",
            ),
            contract,
        );
        write(
            &root.join(
                "pack/candidate/deployments/lab/collector/g2-collector-product-binding-measure-receipt.scaffold.json",
            ),
            scaffold,
        );
        write(
            &root.join(
                "pack/candidate/deployments/lab/collector/g2-collector-product-binding-cap-pilot.sh",
            ),
            pilot_sh,
        );
        let manifest = json!({
            "schema": "humanymous.cursor-g2-collector-product-binding-candidate/1",
            "unlock": "G2",
            "entries": [
                {
                    "destination": "deployments/lab/compose.g2-collector-product-binding.yaml",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/deployments/lab/compose.g2-collector-product-binding.yaml",
                    "candidateSha256": snapshot::digest(compose),
                },
                {
                    "destination": "build/rust/g2-collector-product-binding-lab.Dockerfile",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/build/rust/g2-collector-product-binding-lab.Dockerfile",
                    "candidateSha256": snapshot::digest(dockerfile),
                },
                {
                    "destination": "build/rust/g2-collector-product-binding-lab.Dockerfile.dockerignore",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/build/rust/g2-collector-product-binding-lab.Dockerfile.dockerignore",
                    "candidateSha256": snapshot::digest(dockerignore),
                },
                {
                    "destination": "deployments/lab/collector/g2-collector-product-binding-contract.json",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/deployments/lab/collector/g2-collector-product-binding-contract.json",
                    "candidateSha256": snapshot::digest(contract),
                },
                {
                    "destination": "deployments/lab/collector/g2-collector-product-binding-measure-receipt.scaffold.json",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/deployments/lab/collector/g2-collector-product-binding-measure-receipt.scaffold.json",
                    "candidateSha256": snapshot::digest(scaffold),
                },
                {
                    "destination": "deployments/lab/collector/g2-collector-product-binding-cap-pilot.sh",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/deployments/lab/collector/g2-collector-product-binding-cap-pilot.sh",
                    "candidateSha256": snapshot::digest(pilot_sh),
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
    fn g2_probe_reports_ready_when_candidates_match_and_live_absent() {
        let root = temp_dir("g2-ready");
        let (manifest_path, manifest) = sample_g2_pack(&root);
        let probe =
            probe_manifest(&manifest, &root.join("workspace"), &manifest_path).expect("probe");
        assert_eq!(probe["schema"], PROBE_SCHEMA_G2);
        assert_eq!(probe["g2_collector_product_binding_preimage_ready"], true);
        assert_eq!(probe["applied"], false);
        assert_eq!(probe["cap_execute"], false);
        assert_eq!(probe["pass_product_clean_room"], 0);
        assert_eq!(probe["product"], 0);
        assert_eq!(
            probe["honesty"]["g2_requires_collector_product_binding_cap_cli_pilot"],
            true
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn g2_grant_refuses_collector_require_full_spam() {
        let err = refuse_forbidden_g2_collector_product_binding_pilot_command(
            "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --collector --require-full",
        )
        .expect_err("collector spam refused");
        assert!(
            err.to_string().contains("feeder")
                || err.to_string().contains("g2-collector-product-binding")
        );
        refuse_forbidden_g2_collector_product_binding_pilot_command(G2_CODE_DEFINED_PILOT)
            .expect("code-defined Cap CLI pilot accepted");
    }

    #[test]
    fn g2_apply_dry_run_with_grant_keeps_product_zero_and_no_writes() {
        let root = temp_dir("g2-dry");
        let (manifest_path, manifest) = sample_g2_pack(&root);
        let probe =
            probe_manifest(&manifest, &root.join("workspace"), &manifest_path).expect("probe");
        let grant = json!({
            "schema": GRANT_SCHEMA_G2,
            "root_authorized": true,
            "allow_apply": true,
            "unlock": "G2",
            "pilot_command": G2_CODE_DEFINED_PILOT,
            "destinations": [
                "deployments/lab/compose.g2-collector-product-binding.yaml",
                "build/rust/g2-collector-product-binding-lab.Dockerfile",
                "build/rust/g2-collector-product-binding-lab.Dockerfile.dockerignore",
                "deployments/lab/collector/g2-collector-product-binding-contract.json",
                "deployments/lab/collector/g2-collector-product-binding-measure-receipt.scaffold.json",
                "deployments/lab/collector/g2-collector-product-binding-cap-pilot.sh"
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
        assert_eq!(report["schema"], APPLY_SCHEMA_G2);
        assert_eq!(report["applied"], false);
        assert_eq!(report["dry_run"], true);
        assert_eq!(report["pass_product_clean_room"], 0);
        assert_eq!(report["product"], 0);
        assert_eq!(report["cap_execute"], false);
        assert!(
            !root
                .join("workspace/deployments/lab/compose.g2-collector-product-binding.yaml")
                .exists()
        );
        assert!(!root.join("workspace").join(UNLOCK_G2_APPLIED_REL).exists());
        let _ = fs::remove_dir_all(root);
    }

    fn sample_g5_pack(root: &Path) -> (PathBuf, Value) {
        let compose = b"services: {}\n";
        let dockerfile = b"FROM scratch\n";
        let dockerignore = b"*\n";
        let contract = b"{\"pilot_command\":null}\n";
        let scaffold = b"{\"status\":\"scaffold\"}\n";
        let pilot_sh = b"#!/bin/sh\nexit 0\n";
        write(
            &root.join("pack/candidate/deployments/lab/compose.g5-witness-transport.yaml"),
            compose,
        );
        write(
            &root.join("pack/candidate/build/rust/g5-witness-transport-lab.Dockerfile"),
            dockerfile,
        );
        write(
            &root
                .join("pack/candidate/build/rust/g5-witness-transport-lab.Dockerfile.dockerignore"),
            dockerignore,
        );
        write(
            &root.join("pack/candidate/deployments/lab/witness/g5-witness-transport-contract.json"),
            contract,
        );
        write(
            &root.join(
                "pack/candidate/deployments/lab/witness/g5-witness-transport-measure-receipt.scaffold.json",
            ),
            scaffold,
        );
        write(
            &root.join("pack/candidate/deployments/lab/witness/g5-witness-transport-cap-pilot.sh"),
            pilot_sh,
        );
        let manifest = json!({
            "schema": "humanymous.cursor-g5-witness-transport-candidate/1",
            "unlock": "G5",
            "entries": [
                {
                    "destination": "deployments/lab/compose.g5-witness-transport.yaml",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/deployments/lab/compose.g5-witness-transport.yaml",
                    "candidateSha256": snapshot::digest(compose),
                },
                {
                    "destination": "build/rust/g5-witness-transport-lab.Dockerfile",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/build/rust/g5-witness-transport-lab.Dockerfile",
                    "candidateSha256": snapshot::digest(dockerfile),
                },
                {
                    "destination": "build/rust/g5-witness-transport-lab.Dockerfile.dockerignore",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/build/rust/g5-witness-transport-lab.Dockerfile.dockerignore",
                    "candidateSha256": snapshot::digest(dockerignore),
                },
                {
                    "destination": "deployments/lab/witness/g5-witness-transport-contract.json",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/deployments/lab/witness/g5-witness-transport-contract.json",
                    "candidateSha256": snapshot::digest(contract),
                },
                {
                    "destination": "deployments/lab/witness/g5-witness-transport-measure-receipt.scaffold.json",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/deployments/lab/witness/g5-witness-transport-measure-receipt.scaffold.json",
                    "candidateSha256": snapshot::digest(scaffold),
                },
                {
                    "destination": "deployments/lab/witness/g5-witness-transport-cap-pilot.sh",
                    "preimageSha256": Value::Null,
                    "candidateRelative": "candidate/deployments/lab/witness/g5-witness-transport-cap-pilot.sh",
                    "candidateSha256": snapshot::digest(pilot_sh),
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
    fn g5_probe_reports_ready_when_candidates_match_and_live_absent() {
        let root = temp_dir("g5-ready");
        let (manifest_path, manifest) = sample_g5_pack(&root);
        let probe =
            probe_manifest(&manifest, &root.join("workspace"), &manifest_path).expect("probe");
        assert_eq!(probe["schema"], PROBE_SCHEMA_G5);
        assert_eq!(probe["g5_witness_transport_preimage_ready"], true);
        assert_eq!(probe["applied"], false);
        assert_eq!(probe["cap_execute"], false);
        assert_eq!(probe["pass_product_clean_room"], 0);
        assert_eq!(probe["product"], 0);
        assert_eq!(
            probe["honesty"]["g5_requires_witness_transport_cap_cli_pilot"],
            true
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn g5_grant_refuses_honesty_require_full_spam() {
        let err = refuse_forbidden_g5_witness_transport_pilot_command(
            "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --g5-witness-transport-honesty",
        )
        .expect_err("honesty twin refused");
        assert!(
            err.to_string().contains("feeder")
                || err.to_string().contains("g5-witness-transport")
                || err.to_string().contains("honesty")
        );
        refuse_forbidden_g5_witness_transport_pilot_command(G5_CODE_DEFINED_PILOT)
            .expect("code-defined Cap CLI pilot accepted");
    }

    #[test]
    fn g5_apply_dry_run_with_grant_keeps_product_zero_and_no_writes() {
        let root = temp_dir("g5-dry");
        let (manifest_path, manifest) = sample_g5_pack(&root);
        let probe =
            probe_manifest(&manifest, &root.join("workspace"), &manifest_path).expect("probe");
        let grant = json!({
            "schema": GRANT_SCHEMA_G5,
            "root_authorized": true,
            "allow_apply": true,
            "unlock": "G5",
            "pilot_command": G5_CODE_DEFINED_PILOT,
            "destinations": [
                "deployments/lab/compose.g5-witness-transport.yaml",
                "build/rust/g5-witness-transport-lab.Dockerfile",
                "build/rust/g5-witness-transport-lab.Dockerfile.dockerignore",
                "deployments/lab/witness/g5-witness-transport-contract.json",
                "deployments/lab/witness/g5-witness-transport-measure-receipt.scaffold.json",
                "deployments/lab/witness/g5-witness-transport-cap-pilot.sh"
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
        assert_eq!(report["schema"], APPLY_SCHEMA_G5);
        assert_eq!(report["applied"], false);
        assert_eq!(report["dry_run"], true);
        assert_eq!(report["pass_product_clean_room"], 0);
        assert_eq!(report["product"], 0);
        assert_eq!(report["cap_execute"], false);
        assert!(
            !root
                .join("workspace/deployments/lab/compose.g5-witness-transport.yaml")
                .exists()
        );
        assert!(!root.join("workspace").join(UNLOCK_G5_APPLIED_REL).exists());
        let _ = fs::remove_dir_all(root);
    }
}
