//! Cap=0 orchestrator: fail-closed chain of measured applies.
//!
//! Order (when LIVE files present):
//! 1. `handoff-apply --kind at`
//! 2. `handoff-apply --kind browser-ready-join`
//! 3. `handoff-apply --kind auth` (N06)
//! 4. `cap-realloc-apply` (NEW non-twin Cap-free-restore GRANT + LIVE TRANSFER)
//!
//! Refuses closed if any step is missing / TEMPLATE / mismatch.
//! Never invents BrowserReady / AUTH / AT / Cap free / pass_product / Cap burn.
use crate::cap_realloc_apply;
use crate::docker::Result;
use crate::handoff_apply;
use crate::orchestration::cap0_product_honesty::{
    INTERACTIVE_AT_HANDOFF_CANDIDATES, UNLOCK_A_CLOSER_REL, cap_free_after_unlock_a,
};
use crate::orchestration::h2_product_binding::N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES;
use crate::snapshot;
use serde_json::{Value, json};
use std::{fs, path::Path};

const SCHEMA: &str = "humanymous.lab-cap0-chain/1";
const PILOT_COMMAND: &str =
    "cargo run --locked -p humanymous-lab-runner -- cap0-chain [--dry-run] [--workspace PATH] [--emit PATH] [--grant PATH] [--manifest PATH]";

/// Cap-free-restore LIVE GRANT candidates (never `*.TEMPLATE.json`).
const CAP_FREE_GRANT_CANDIDATES: &[&str] = &[
    ".spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/GRANT.json",
    ".spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/cap-ready/cap-free-restore/candidate-pack/GRANT.json",
    ".spec.local/root/grants/cap-free-restore/GRANT.json",
    "deployments/lab/product/GRANT.json",
];

/// Cap-free-restore LIVE TRANSFER manifest candidates (never TEMPLATE path).
const CAP_FREE_MANIFEST_CANDIDATES: &[&str] = &[
    ".spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/transfer-manifest.json",
    ".spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/cap-ready/cap-free-restore/candidate-pack/transfer-manifest.json",
    ".spec.local/root/grants/cap-free-restore/transfer-manifest.json",
    "deployments/lab/product/transfer-manifest.json",
];

const TEMPLATE_GRANT_MARKERS: &[&str] = &[
    ".spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/GRANT.TEMPLATE.json",
    ".spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/cap-ready/cap-free-restore/candidate-pack/GRANT.TEMPLATE.json",
];

/// Cap=0 fail-closed product-path chain.
///
/// Usage:
/// `cap0-chain [--workspace PATH] [--dry-run] [--emit PATH] [--grant PATH] [--manifest PATH]`
pub fn run(args: &[String]) -> Result<i32> {
    let mut workspace = None;
    let mut emit_path = None;
    let mut grant_override = None;
    let mut manifest_override = None;
    let mut dry_run = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
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
                grant_override = Some(args.get(index).ok_or("--grant path missing")?.clone());
            }
            "--manifest" => {
                index += 1;
                manifest_override =
                    Some(args.get(index).ok_or("--manifest path missing")?.clone());
            }
            "--dry-run" => dry_run = true,
            _ => {
                return Err(format!(
                    "usage: cap0-chain [--workspace PATH] [--dry-run] [--emit PATH] [--grant PATH] [--manifest PATH]; never invent Cap free / BrowserReady / AUTH / AT"
                )
                .into());
            }
        }
        index += 1;
    }

    let root = match workspace {
        Some(path) => Path::new(&path).canonicalize().map_err(|error| {
            format!("cap0-chain refused: workspace canonicalize {path}: {error}")
        })?,
        None => snapshot::root()?,
    };

    let report = run_chain(
        &root,
        dry_run,
        grant_override.as_deref(),
        manifest_override.as_deref(),
    )?;
    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("cap0-chain encode failed: {error}"))?;
    if let Some(path) = emit_path {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).map_err(|error| format!("emit mkdir: {error}"))?;
        }
        fs::write(&path, format!("{rendered}\n"))
            .map_err(|error| format!("emit write failed: {error}"))?;
    }
    println!("{rendered}");

    let status = report
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("refuse_closed");
    if status.starts_with("refuse_closed") || status == "refused" {
        return Err(format!(
            "cap0-chain refused: status={status}; require LIVE AT → browser-ready-join → AUTH → Cap-free GRANT+TRANSFER; never invent Cap free / BrowserReady / AUTH / AT / pass_product"
        )
        .into());
    }
    Ok(0)
}

fn is_template_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            let lowered = name.to_ascii_lowercase();
            lowered.contains(".template.") || lowered.ends_with(".template.json")
        })
}

fn first_live_file(root: &Path, candidates: &[&str]) -> Option<String> {
    candidates.iter().find_map(|relative| {
        let path = root.join(relative);
        if path.is_file() && !is_template_path(&path) {
            Some((*relative).to_string())
        } else {
            None
        }
    })
}

fn template_grant_present(root: &Path) -> Option<String> {
    TEMPLATE_GRANT_MARKERS
        .iter()
        .find(|relative| root.join(relative).is_file())
        .map(|relative| (*relative).to_string())
}

fn resolve_grant_manifest(
    root: &Path,
    grant_override: Option<&str>,
    manifest_override: Option<&str>,
) -> (Option<String>, Option<String>, Option<String>) {
    let grant = grant_override
        .map(|path| path.to_string())
        .or_else(|| first_live_file(root, CAP_FREE_GRANT_CANDIDATES));
    let manifest = manifest_override
        .map(|path| path.to_string())
        .or_else(|| first_live_file(root, CAP_FREE_MANIFEST_CANDIDATES));
    let template_only = if grant.is_none() {
        template_grant_present(root)
    } else {
        None
    };
    (grant, manifest, template_only)
}

fn step_ok(report: &Value) -> bool {
    let status = report
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("refuse_closed");
    if status.starts_with("refuse_closed")
        || status == "refused"
        || status == "skipped_prior_refuse"
    {
        return false;
    }
    matches!(
        status,
        "measured_apply_bound"
            | "validated_dry_run"
            | "cap_free_restored"
            | "already_free"
            | "prerequisites_bound_await_honesty_measure"
            | "at_bound_cef_missing_await_cap_day"
    ) || report.get("applied").and_then(Value::as_bool) == Some(true)
        || (report.get("dry_run").and_then(Value::as_bool) == Some(true)
            && report.get("would_set_cap_free").and_then(Value::as_bool) == Some(true))
}

fn honesty_block() -> Value {
    json!({
        "never_invent_pass_product_clean_room": true,
        "never_invent_browser_ready": true,
        "never_invent_cap_free": true,
        "never_forge_auth_at": true,
        "never_apply_template_grant": true,
        "refuse_spent_twin_reburn_g2_g5_cr_g6_rb_pm": true,
        "detection_freeze": true,
        "cap0_only": true,
        "cap_burn_0_while_cap_free_false": true,
        "no_cap1_inside_chain": true
    })
}

fn run_chain(
    root: &Path,
    dry_run: bool,
    grant_override: Option<&str>,
    manifest_override: Option<&str>,
) -> Result<Value> {
    let cap_free_before = cap_free_after_unlock_a(root)?;
    let at_present = first_live_file(root, INTERACTIVE_AT_HANDOFF_CANDIDATES);
    let auth_present = first_live_file(root, N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES);
    let (grant_path, manifest_path, template_grant) =
        resolve_grant_manifest(root, grant_override, manifest_override);

    let mut steps = Vec::new();
    let mut stopped_at = None;

    // 1) AT
    let at_report = handoff_apply::apply_at_handoff(root, dry_run)?;
    let at_ok = step_ok(&at_report);
    steps.push(json!({
        "step": 1,
        "kind": "at",
        "ok": at_ok,
        "report": at_report
    }));
    if !at_ok {
        stopped_at = Some("at");
    }

    // 2) BrowserReady-join (requires AT LIVE)
    let join_report = if stopped_at.is_none() {
        handoff_apply::apply_browser_ready_join(root, dry_run)?
    } else {
        json!({
            "schema": "humanymous.lab-browser-ready-join-apply/1",
            "status": "skipped_prior_refuse",
            "applied": false,
            "browser_ready": false,
            "product": 0,
            "pass_product_clean_room": 0
        })
    };
    let join_ok = stopped_at.is_none() && step_ok(&join_report);
    steps.push(json!({
        "step": 2,
        "kind": "browser-ready-join",
        "ok": join_ok,
        "report": join_report
    }));
    if stopped_at.is_none() && !join_ok {
        stopped_at = Some("browser-ready-join");
    }

    // 3) AUTH / N06
    let auth_report = if stopped_at.is_none() {
        handoff_apply::apply_auth_handoff(root, dry_run)?
    } else {
        json!({
            "schema": "humanymous.lab-auth-handoff-apply/1",
            "status": "skipped_prior_refuse",
            "applied": false,
            "n06_owner_join": false,
            "product": 0,
            "pass_product_clean_room": 0
        })
    };
    let auth_ok = stopped_at.is_none() && step_ok(&auth_report);
    steps.push(json!({
        "step": 3,
        "kind": "auth",
        "ok": auth_ok,
        "report": auth_report
    }));
    if stopped_at.is_none() && !auth_ok {
        stopped_at = Some("auth");
    }

    // 4) Cap-realloc / Cap-free-restore
    let realloc_report = if stopped_at.is_some() {
        json!({
            "schema": "humanymous.lab-cap-realloc-apply/1",
            "status": "skipped_prior_refuse",
            "applied": false,
            "cap_free": false,
            "product": 0,
            "pass_product_clean_room": 0
        })
    } else if let (Some(grant), Some(manifest)) = (grant_path.as_ref(), manifest_path.as_ref()) {
        if is_template_path(Path::new(grant)) {
            json!({
                "schema": "humanymous.lab-cap-realloc-apply/1",
                "status": "refuse_closed_grant_template",
                "applied": false,
                "cap_free": false,
                "product": 0,
                "pass_product_clean_room": 0,
                "grant": grant,
                "note": "GRANT path is TEMPLATE; Root must copy to GRANT.json; never invent Cap free"
            })
        } else {
            let grant_abs = if Path::new(grant).is_absolute() {
                Path::new(grant).to_path_buf()
            } else {
                root.join(grant)
            };
            let manifest_abs = if Path::new(manifest).is_absolute() {
                Path::new(manifest).to_path_buf()
            } else {
                root.join(manifest)
            };
            match cap_realloc_apply::apply_cap_free_restore(
                root,
                &grant_abs,
                &manifest_abs,
                dry_run,
            ) {
                Ok(report) => report,
                Err(error) => json!({
                    "schema": "humanymous.lab-cap-realloc-apply/1",
                    "status": "refuse_closed_cap_realloc",
                    "applied": false,
                    "cap_free": false,
                    "product": 0,
                    "pass_product_clean_room": 0,
                    "grant": grant,
                    "manifest": manifest,
                    "error": error.to_string(),
                    "note": "cap-realloc-apply refused; Cap free stays false; never invent Cap free"
                }),
            }
        }
    } else if template_grant.is_some() {
        json!({
            "schema": "humanymous.lab-cap-realloc-apply/1",
            "status": "refuse_closed_grant_template",
            "applied": false,
            "cap_free": false,
            "product": 0,
            "pass_product_clean_room": 0,
            "template_grant": template_grant,
            "manifest": manifest_path,
            "note": "Only GRANT.TEMPLATE.json present; Root must author LIVE GRANT.json + matching TRANSFER; never invent Cap free"
        })
    } else {
        json!({
            "schema": "humanymous.lab-cap-realloc-apply/1",
            "status": "refuse_closed_grant_or_manifest_absent",
            "applied": false,
            "cap_free": false,
            "product": 0,
            "pass_product_clean_room": 0,
            "grant_candidates": CAP_FREE_GRANT_CANDIDATES,
            "manifest_candidates": CAP_FREE_MANIFEST_CANDIDATES,
            "note": "LIVE Cap-free-restore GRANT.json and/or transfer-manifest.json absent; Cap free stays false; never invent Cap free"
        })
    };
    let realloc_ok = stopped_at.is_none() && step_ok(&realloc_report);
    steps.push(json!({
        "step": 4,
        "kind": "cap-realloc-apply",
        "ok": realloc_ok,
        "report": realloc_report
    }));
    if stopped_at.is_none() && !realloc_ok {
        stopped_at = Some("cap-realloc-apply");
    }

    let all_ok = at_ok && join_ok && auth_ok && realloc_ok;
    let cap_free_after = cap_free_after_unlock_a(root)?;
    let status = if all_ok {
        if dry_run {
            "validated_dry_run_full_chain".to_string()
        } else {
            "measured_full_chain_bound".to_string()
        }
    } else if let Some(step) = stopped_at {
        format!("refuse_closed_at_step_{step}")
    } else {
        "refuse_closed".to_string()
    };

    Ok(json!({
        "schema": SCHEMA,
        "record_kind": "lab-cap0-chain",
        "cap": 0,
        "cap_burn": 0,
        "cap_free": cap_free_after,
        "cap_free_before": cap_free_before,
        "product": 0,
        "pass_product_clean_room": 0,
        "browser_ready": false,
        "n06_owner_join": false,
        "applied": all_ok && !dry_run,
        "dry_run": dry_run,
        "status": status,
        "stopped_at": stopped_at,
        "chain_order": ["at", "browser-ready-join", "auth", "cap-realloc-apply"],
        "inspect": {
            "at_handoff_present": at_present.is_some(),
            "at_handoff_path": at_present,
            "auth_handoff_present": auth_present.is_some(),
            "auth_handoff_path": auth_present,
            "cap_free_grant_present": grant_path.is_some(),
            "cap_free_grant_path": grant_path,
            "cap_free_manifest_present": manifest_path.is_some(),
            "cap_free_manifest_path": manifest_path,
            "template_grant_only": template_grant,
            "unlock_a_closer": UNLOCK_A_CLOSER_REL
        },
        "steps": steps,
        "pilot_command": PILOT_COMMAND,
        "next": if all_ok {
            "Cap free restored (or already free); Cap1 serial once --clean-room-product / matrix / red-blue-validate; never Cap twin 73-80 reburn"
        } else {
            "Root place LIVE AT-HANDOFF + AUTH-HANDOFF + Cap-free-restore GRANT.json (not TEMPLATE) + LIVE-matching TRANSFER; re-run cap0-chain; Cap1 HOLD; Cap burn=0"
        },
        "honesty": honesty_block()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::cap0_product_honesty::UNLOCK_A_CLOSER_SCHEMA;
    use crate::orchestration::h2_product_binding::N06_OWNER_JOIN_AUTH_HANDOFF_SCHEMA;
    use std::time::{SystemTime, UNIX_EPOCH};

    const INTERACTIVE_AT_HANDOFF_SCHEMA: &str = "humanymous.root-interactive-at-windows-tuple/1";

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("hmn-cap0-chain-{label}-{nanos}"));
        fs::create_dir_all(&root).expect("tmpdir");
        root
    }

    fn write_unlock_a(root: &Path) {
        let path = root.join(UNLOCK_A_CLOSER_REL);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &path,
            serde_json::to_vec_pretty(&json!({
                "schema": UNLOCK_A_CLOSER_SCHEMA,
                "cap_exit": 0,
                "cap_free": false,
                "product": 0,
                "pass_product_clean_room": 0
            }))
            .expect("encode"),
        )
        .expect("write");
    }

    #[test]
    fn absent_live_files_refuse_closed_at_first_step() {
        let root = temp_root("absent");
        write_unlock_a(&root);
        let report = run_chain(&root, true, None, None).expect("report");
        assert_eq!(report["schema"], SCHEMA);
        assert_eq!(report["product"], 0);
        assert_eq!(report["browser_ready"], false);
        assert_eq!(report["cap_free"], false);
        assert_eq!(report["cap_burn"], 0);
        assert_eq!(report["status"], "refuse_closed_at_step_at");
        assert_eq!(report["stopped_at"], "at");
        assert_eq!(report["steps"][0]["ok"], false);
        assert_eq!(report["steps"][1]["report"]["status"], "skipped_prior_refuse");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn template_grant_refuses_cap_realloc_step_after_handoffs() {
        let root = temp_root("template-grant");
        write_unlock_a(&root);

        let at_rel = INTERACTIVE_AT_HANDOFF_CANDIDATES[0];
        let at_path = root.join(at_rel);
        fs::create_dir_all(at_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &at_path,
            serde_json::to_vec_pretty(&json!({
                "schema": INTERACTIVE_AT_HANDOFF_SCHEMA,
                "browser_ready_authorized": true,
                "product": 0,
                "pass_product_clean_room": 0,
                "browser_ready": false
            }))
            .expect("encode"),
        )
        .expect("write at");

        let auth_rel = N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES[0];
        let auth_path = root.join(auth_rel);
        fs::create_dir_all(auth_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &auth_path,
            serde_json::to_vec_pretty(&json!({
                "schema": N06_OWNER_JOIN_AUTH_HANDOFF_SCHEMA,
                "n06_owner_join_authorized": true,
                "n06_owner_join": false,
                "product": 0,
                "pass_product_clean_room": 0
            }))
            .expect("encode"),
        )
        .expect("write auth");

        // Plant TEMPLATE grant marker only (no LIVE GRANT.json).
        let template = root.join(TEMPLATE_GRANT_MARKERS[0]);
        fs::create_dir_all(template.parent().expect("parent")).expect("mkdir");
        fs::write(
            &template,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.root-cap-free-restore-grant/1",
                "note": "TEMPLATE only"
            }))
            .expect("encode"),
        )
        .expect("write template");

        let report = run_chain(&root, true, None, None).expect("report");
        assert_eq!(report["steps"][0]["ok"], true);
        assert_eq!(report["steps"][1]["ok"], true);
        assert_eq!(report["steps"][2]["ok"], true);
        assert_eq!(report["steps"][3]["ok"], false);
        assert_eq!(
            report["steps"][3]["report"]["status"],
            "refuse_closed_grant_template"
        );
        assert_eq!(report["status"], "refuse_closed_at_step_cap-realloc-apply");
        assert_eq!(report["cap_free"], false);
        assert_eq!(report["product"], 0);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn full_live_chain_dry_run_validates_without_inventing_product() {
        let root = temp_root("full-live");
        write_unlock_a(&root);

        let at_rel = INTERACTIVE_AT_HANDOFF_CANDIDATES[0];
        let at_path = root.join(at_rel);
        fs::create_dir_all(at_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &at_path,
            serde_json::to_vec_pretty(&json!({
                "schema": INTERACTIVE_AT_HANDOFF_SCHEMA,
                "browser_ready_authorized": true,
                "product": 0,
                "pass_product_clean_room": 0,
                "browser_ready": false
            }))
            .expect("encode"),
        )
        .expect("write at");

        let auth_rel = N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES[0];
        let auth_path = root.join(auth_rel);
        fs::create_dir_all(auth_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &auth_path,
            serde_json::to_vec_pretty(&json!({
                "schema": N06_OWNER_JOIN_AUTH_HANDOFF_SCHEMA,
                "n06_owner_join_authorized": true,
                "n06_owner_join": false,
                "product": 0,
                "pass_product_clean_room": 0
            }))
            .expect("encode"),
        )
        .expect("write auth");

        let dest = "deployments/lab/product/cap-free-restore-contract.json";
        let dest_path = root.join(dest);
        fs::create_dir_all(dest_path.parent().expect("parent")).expect("mkdir");
        let live_bytes = b"fixture-cap-free-restore-chain";
        fs::write(&dest_path, live_bytes).expect("write dest");
        let live_sha = snapshot::digest(live_bytes);

        let grant_rel = CAP_FREE_GRANT_CANDIDATES[0];
        let grant_path = root.join(grant_rel);
        fs::create_dir_all(grant_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &grant_path,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.root-cap-free-restore-grant/1",
                "root_authorized": true,
                "allow_apply": true,
                "authorize_cap_free_restore": true,
                "new_non_twin": true,
                "feeder": "product-cap-free-restore",
                "unlock_a_closer": UNLOCK_A_CLOSER_REL,
                "destinations": [dest],
                "cap_burn_authorized": false,
                "cap1_inside_apply": false,
                "product": 0,
                "pass_product_clean_room": 0
            }))
            .expect("encode"),
        )
        .expect("write grant");

        let manifest_rel = CAP_FREE_MANIFEST_CANDIDATES[0];
        let manifest_path = root.join(manifest_rel);
        fs::create_dir_all(manifest_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&json!({
                "entries": [{
                    "destination": dest,
                    "preimageSha256": live_sha,
                    "candidateSha256": live_sha
                }]
            }))
            .expect("encode"),
        )
        .expect("write manifest");

        let report = run_chain(&root, true, None, None).expect("report");
        assert_eq!(report["status"], "validated_dry_run_full_chain");
        assert_eq!(report["applied"], false);
        assert_eq!(report["product"], 0);
        assert_eq!(report["pass_product_clean_room"], 0);
        assert_eq!(report["browser_ready"], false);
        assert_eq!(report["cap_free"], false); // dry-run does not flip
        assert_eq!(report["steps"][0]["ok"], true);
        assert_eq!(report["steps"][1]["ok"], true);
        assert_eq!(report["steps"][2]["ok"], true);
        assert_eq!(report["steps"][3]["ok"], true);
        let _ = fs::remove_dir_all(&root);
    }
}
