//! Cap=0 single-shot: apply chain only when measured ready.
//!
//! Gates (fail-closed, never invent):
//! 1. `until-complete-status` measure → `ready_for_cap0_chain`
//! 2. `cap0-chain --inspect-only` → `ready_for_apply`
//! 3. If both true → full `cap0-chain` (no inspect-only)
//! 4. Else → `refuse_closed` with incompletes list (exit non-zero)
//!
//! Optional `--then-cap1-lander-prep`: Cap0 serialize Cap1 landers **only** when
//! Cap free measured true **after** the chain. Cap1 Docker burn is NEVER automatic.
use crate::cap0_chain;
use crate::cap1_lander_prep;
use crate::docker::Result;
use crate::orchestration::cap0_product_honesty::cap_free_after_unlock_a;
use crate::snapshot;
use crate::until_complete_status;
use serde_json::{json, Value};
use std::{fs, path::Path};

const SCHEMA: &str = "humanymous.lab-cap0-apply-when-ready/1";
const PILOT_COMMAND: &str = "cargo run --locked -p humanymous-lab-runner -- cap0-apply-when-ready [--workspace PATH] [--emit PATH] [--grant PATH] [--manifest PATH] [--dry-run] [--then-cap1-lander-prep]";

/// Cap=0 single-shot apply-when-ready.
///
/// Usage:
/// `cap0-apply-when-ready [--workspace PATH] [--emit PATH] [--grant PATH] [--manifest PATH] [--dry-run] [--then-cap1-lander-prep]`
///
/// Exit 0 only when the Cap0 chain status is not refuse_closed.
/// Exit non-zero on gate refuse or chain refuse. Never invents Cap free /
/// BrowserReady / AUTH / AT / pass_product / Cap burn / goal_complete.
pub fn run(args: &[String]) -> Result<i32> {
    let mut workspace = None;
    let mut emit_path = None;
    let mut grant_override = None;
    let mut manifest_override = None;
    let mut dry_run = false;
    let mut then_cap1_lander_prep = false;
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
                manifest_override = Some(args.get(index).ok_or("--manifest path missing")?.clone());
            }
            "--dry-run" => dry_run = true,
            "--then-cap1-lander-prep" => then_cap1_lander_prep = true,
            _ => {
                return Err(
                    "usage: cap0-apply-when-ready [--workspace PATH] [--emit PATH] [--grant PATH] [--manifest PATH] [--dry-run] [--then-cap1-lander-prep]; Cap0 single-shot; never invent Cap free / BrowserReady / AUTH / AT / pass_product; Cap1 Docker burn never automatic"
                        .into(),
                );
            }
        }
        index += 1;
    }

    let root = match workspace {
        Some(path) => Path::new(&path).canonicalize().map_err(|error| {
            format!("cap0-apply-when-ready refused: workspace canonicalize {path}: {error}")
        })?,
        None => snapshot::root()?,
    };

    let (report, exit_code) = apply_when_ready(
        &root,
        dry_run,
        then_cap1_lander_prep,
        grant_override.as_deref(),
        manifest_override.as_deref(),
    )?;
    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("cap0-apply-when-ready encode failed: {error}"))?;
    if let Some(path) = emit_path {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).map_err(|error| format!("emit mkdir: {error}"))?;
        }
        fs::write(&path, format!("{rendered}\n"))
            .map_err(|error| format!("emit write failed: {error}"))?;
    }
    println!("{rendered}");
    Ok(exit_code)
}

pub(crate) fn apply_when_ready(
    root: &Path,
    dry_run: bool,
    then_cap1_lander_prep: bool,
    grant_override: Option<&str>,
    manifest_override: Option<&str>,
) -> Result<(Value, i32)> {
    let status = until_complete_status::measure(root)?;
    let inspect = cap0_chain::run_inspect(root, grant_override, manifest_override)?;

    let ready_for_cap0_chain = status
        .get("ready_for_cap0_chain")
        .and_then(Value::as_bool)
        == Some(true);
    let inspect_ready = inspect.get("ready_for_apply").and_then(Value::as_bool) == Some(true);
    let incompletes = status
        .get("open_incomplete_ids")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let incomplete_count = status
        .get("incomplete_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let cap_free_before = status
        .get("cap_free")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if !(ready_for_cap0_chain && inspect_ready) {
        let report = json!({
            "schema": SCHEMA,
            "record_kind": "lab-cap0-apply-when-ready",
            "cap": 0,
            "cap_burn": 0,
            "cap_free": cap_free_before,
            "cap_free_before": cap_free_before,
            "cap_free_after": cap_free_before,
            "product": 0,
            "pass_product_clean_room": 0,
            "browser_ready": false,
            "goal_complete": false,
            "applied": false,
            "chain_ran": false,
            "cap1_lander_prep_ran": false,
            "cap1_burn_authorized": false,
            "cap1_executed": false,
            "status": "refuse_closed",
            "refuse_reason": if !ready_for_cap0_chain {
                "until-complete-status ready_for_cap0_chain=false"
            } else {
                "cap0-chain inspect ready_for_apply=false"
            },
            "ready_for_cap0_chain": ready_for_cap0_chain,
            "inspect_ready_for_apply": inspect_ready,
            "incomplete_count": incomplete_count,
            "incompletes": incompletes,
            "until_complete_status": {
                "schema": status.get("schema").cloned().unwrap_or(Value::Null),
                "status": status.get("status").cloned().unwrap_or(Value::Null),
                "ready_for_cap0_chain": ready_for_cap0_chain,
                "ready_for_cap1": status.get("ready_for_cap1").cloned().unwrap_or(Value::Null),
                "cap_free": cap_free_before,
                "incomplete_count": incomplete_count,
                "open_incomplete_ids": incompletes.clone()
            },
            "inspect": {
                "schema": inspect.get("schema").cloned().unwrap_or(Value::Null),
                "status": inspect.get("status").cloned().unwrap_or(Value::Null),
                "ready_for_apply": inspect_ready,
                "blocker_count": inspect.get("blocker_count").cloned().unwrap_or(Value::Null),
                "blockers": inspect.get("blockers").cloned().unwrap_or(json!([]))
            },
            "then_cap1_lander_prep_requested": then_cap1_lander_prep,
            "dry_run": dry_run,
            "pilot_command": PILOT_COMMAND,
            "next": "Root place LIVE AT-HANDOFF + AUTH-HANDOFF + Cap-free-restore GRANT.json (not TEMPLATE) + LIVE-matching TRANSFER; re-run until-complete-status / cap0-apply-when-ready; Cap1 HOLD; Cap burn=0",
            "honesty": honesty_block()
        });
        return Ok((report, 1));
    }

    let chain = cap0_chain::run_chain(root, dry_run, grant_override, manifest_override)?;
    let chain_status = chain
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("refuse_closed");
    let chain_refused = chain_status.starts_with("refuse_closed") || chain_status == "refused";
    let cap_free_after = cap_free_after_unlock_a(root)?;

    let mut cap1_lander_prep_ran = false;
    let mut cap1_lander_prep_report = Value::Null;
    let mut cap1_lander_prep_note = "not_requested";
    if then_cap1_lander_prep {
        if chain_refused {
            cap1_lander_prep_note = "skipped_chain_refused";
        } else if !cap_free_after {
            cap1_lander_prep_note =
                "skipped_cap_free_false_after_chain; Cap1 Docker burn never automatic";
        } else {
            // Cap free measured true after chain — Cap0 serialize Cap1 landers only.
            // Cap1 Docker burn stays NOT automatic unless Cap free+flags explicit and Cap1 product allowed.
            cap1_lander_prep_report = cap1_lander_prep::prep(root)?;
            cap1_lander_prep_ran = true;
            cap1_lander_prep_note =
                "cap_free_true_after_chain; Cap1 lander prep armed (serialize only; Cap1 Docker burn NOT automatic)";
        }
    }

    let applied = chain.get("applied").and_then(Value::as_bool) == Some(true);
    let status_out = if chain_refused {
        "refuse_closed_chain"
    } else if dry_run {
        "validated_dry_run_chain"
    } else if cap1_lander_prep_ran {
        "applied_chain_cap1_lander_prep_armed"
    } else {
        "applied_chain"
    };
    let next = if chain_refused {
        "cap0-chain refused; Cap free stays measured; Cap1 HOLD; Cap burn=0; never invent Cap free / pass_product"
    } else if cap_free_after && then_cap1_lander_prep && cap1_lander_prep_ran {
        "Cap free=true after chain; Cap1 lander prep armed — Cap1 Docker burn still requires Cap free+explicit Cap1 product flags; never invent pass_product_clean_room"
    } else if cap_free_after {
        "Cap free=true after chain; run cap1-lander-prep then preferred Cap1 lander once with explicit Cap1 product flags; Cap1 Docker burn NOT automatic via this command"
    } else {
        "Chain validated/applied but Cap free still false; Cap1 HOLD; Cap burn=0; never invent Cap free"
    };

    let report = json!({
        "schema": SCHEMA,
        "record_kind": "lab-cap0-apply-when-ready",
        "cap": 0,
        "cap_burn": 0,
        "cap_free": cap_free_after,
        "cap_free_before": cap_free_before,
        "cap_free_after": cap_free_after,
        "product": 0,
        "pass_product_clean_room": 0,
        "browser_ready": false,
        "goal_complete": false,
        "applied": applied,
        "chain_ran": true,
        "chain_status": chain_status,
        "dry_run": dry_run,
        "cap1_lander_prep_ran": cap1_lander_prep_ran,
        "cap1_lander_prep_note": cap1_lander_prep_note,
        "cap1_burn_authorized": false,
        "cap1_executed": false,
        "status": status_out,
        "ready_for_cap0_chain": true,
        "inspect_ready_for_apply": true,
        "incomplete_count": incomplete_count,
        "incompletes": incompletes,
        "until_complete_status": {
            "schema": status.get("schema").cloned().unwrap_or(Value::Null),
            "status": status.get("status").cloned().unwrap_or(Value::Null),
            "ready_for_cap0_chain": true,
            "cap_free": cap_free_before,
            "incomplete_count": incomplete_count
        },
        "inspect": {
            "schema": inspect.get("schema").cloned().unwrap_or(Value::Null),
            "status": inspect.get("status").cloned().unwrap_or(Value::Null),
            "ready_for_apply": true
        },
        "chain": chain,
        "cap1_lander_prep": cap1_lander_prep_report,
        "then_cap1_lander_prep_requested": then_cap1_lander_prep,
        "pilot_command": PILOT_COMMAND,
        "next": next,
        "honesty": honesty_block()
    });

    let exit = if chain_refused { 1 } else { 0 };
    Ok((report, exit))
}

fn honesty_block() -> Value {
    json!({
        "never_invent_pass_product_clean_room": true,
        "never_invent_browser_ready": true,
        "never_invent_cap_free": true,
        "never_forge_auth_at": true,
        "never_invent_goal_complete": true,
        "never_apply_template_grant": true,
        "detection_freeze": true,
        "cap0_only": true,
        "cap_burn_0_while_cap_free_false": true,
        "no_cap_twin_reburn_73_80": true,
        "cap1_docker_burn_never_automatic": true,
        "then_cap1_lander_prep_serialize_only": true,
        "fail_closed_exit_nonzero_when_not_ready": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::cap0_product_honesty::{
        INTERACTIVE_AT_HANDOFF_CANDIDATES, UNLOCK_A_CLOSER_REL, UNLOCK_A_CLOSER_SCHEMA,
    };
    use crate::orchestration::h2_product_binding::N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES;
    use std::time::{SystemTime, UNIX_EPOCH};

    const INTERACTIVE_AT_HANDOFF_SCHEMA: &str = "humanymous.root-interactive-at-windows-tuple/1";
    const N06_OWNER_JOIN_AUTH_HANDOFF_SCHEMA: &str =
        "humanymous.root-n06-owner-join-auth-handoff/1";
    const GRANT_SCHEMA: &str = "humanymous.root-cap-free-restore-grant/1";
    const CAP_FREE_GRANT: &str =
        ".spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/GRANT.json";
    const CAP_FREE_MANIFEST: &str =
        ".spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/transfer-manifest.json";

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("hmn-cap0-when-ready-{label}-{nanos}"));
        fs::create_dir_all(&root).expect("tmpdir");
        root
    }

    fn write_unlock_a(root: &Path, cap_free: bool) {
        let path = root.join(UNLOCK_A_CLOSER_REL);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &path,
            serde_json::to_vec_pretty(&json!({
                "schema": UNLOCK_A_CLOSER_SCHEMA,
                "cap_exit": 0,
                "cap_free": cap_free,
                "product": 0,
                "pass_product_clean_room": 0,
                "browser_ready": false
            }))
            .expect("encode"),
        )
        .expect("write");
    }

    fn plant_live_chain_artifacts(root: &Path) -> String {
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
        let live_bytes = b"fixture-cap0-apply-when-ready";
        fs::write(&dest_path, live_bytes).expect("write dest");
        let live_sha = snapshot::digest(live_bytes);

        let grant_path = root.join(CAP_FREE_GRANT);
        fs::create_dir_all(grant_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &grant_path,
            serde_json::to_vec_pretty(&json!({
                "schema": GRANT_SCHEMA,
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

        let manifest_path = root.join(CAP_FREE_MANIFEST);
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

        dest.to_string()
    }

    #[test]
    fn not_ready_refuses_closed_with_incompletes_nonzero() {
        let root = temp_root("refuse");
        write_unlock_a(&root, false);
        let (report, code) = apply_when_ready(&root, false, false, None, None).expect("apply");
        assert_eq!(code, 1);
        assert_eq!(report["schema"], SCHEMA);
        assert_eq!(report["status"], "refuse_closed");
        assert_eq!(report["chain_ran"], false);
        assert_eq!(report["applied"], false);
        assert_eq!(report["cap_free"], false);
        assert_eq!(report["cap_burn"], 0);
        assert_eq!(report["product"], 0);
        assert_eq!(report["pass_product_clean_room"], 0);
        assert_eq!(report["browser_ready"], false);
        assert_eq!(report["goal_complete"], false);
        assert_eq!(report["ready_for_cap0_chain"], false);
        assert_eq!(report["inspect_ready_for_apply"], false);
        assert!(report["incomplete_count"].as_u64().expect("count") >= 1);
        assert!(
            report["incompletes"]
                .as_array()
                .expect("ids")
                .iter()
                .any(|id| id == "AT-HANDOFF")
        );
        assert_eq!(report["honesty"]["cap1_docker_burn_never_automatic"], true);
        assert_eq!(report["cap1_executed"], false);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ready_dry_run_runs_chain_without_inventing_product_or_cap1_burn() {
        let root = temp_root("ready-dry");
        write_unlock_a(&root, false);
        let _ = plant_live_chain_artifacts(&root);
        let (report, code) = apply_when_ready(&root, true, true, None, None).expect("apply");
        assert_eq!(code, 0);
        assert_eq!(report["schema"], SCHEMA);
        assert_eq!(report["ready_for_cap0_chain"], true);
        assert_eq!(report["inspect_ready_for_apply"], true);
        assert_eq!(report["chain_ran"], true);
        assert_eq!(report["applied"], false);
        assert_eq!(report["dry_run"], true);
        assert_eq!(report["status"], "validated_dry_run_chain");
        assert_eq!(report["product"], 0);
        assert_eq!(report["pass_product_clean_room"], 0);
        assert_eq!(report["browser_ready"], false);
        assert_eq!(report["cap_burn"], 0);
        assert_eq!(report["cap1_executed"], false);
        assert_eq!(report["cap1_burn_authorized"], false);
        // dry-run does not flip Cap free; --then-cap1-lander-prep must skip.
        assert_eq!(report["cap_free_after"], false);
        assert_eq!(report["cap1_lander_prep_ran"], false);
        let note = report["cap1_lander_prep_note"].as_str().unwrap_or("");
        assert!(
            note.contains("skipped_cap_free_false_after_chain"),
            "note={note}"
        );
        assert_eq!(report["chain"]["status"], "validated_dry_run_full_chain");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn then_cap1_lander_prep_arms_only_when_cap_free_true_after_chain() {
        let root = temp_root("cap1-arm");
        write_unlock_a(&root, true);
        let _ = plant_live_chain_artifacts(&root);
        // Cap free already true before chain; dry-run keeps it true → lander prep may arm.
        let (report, code) = apply_when_ready(&root, true, true, None, None).expect("apply");
        assert_eq!(code, 0);
        assert_eq!(report["cap_free_after"], true);
        assert_eq!(report["cap1_lander_prep_ran"], true);
        assert_eq!(report["cap1_executed"], false);
        assert_eq!(report["cap1_burn_authorized"], false);
        assert_eq!(report["product"], 0);
        assert_eq!(report["pass_product_clean_room"], 0);
        assert_eq!(
            report["cap1_lander_prep"]["status"],
            "cap_free_true_cap1_serial_armed"
        );
        assert_eq!(report["cap1_lander_prep"]["cap1_executed"], false);
        assert_eq!(report["honesty"]["cap1_docker_burn_never_automatic"], true);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn then_cap1_flag_absent_never_runs_lander_prep() {
        let root = temp_root("no-then");
        write_unlock_a(&root, true);
        let _ = plant_live_chain_artifacts(&root);
        let (report, code) = apply_when_ready(&root, true, false, None, None).expect("apply");
        assert_eq!(code, 0);
        assert_eq!(report["then_cap1_lander_prep_requested"], false);
        assert_eq!(report["cap1_lander_prep_ran"], false);
        assert_eq!(report["cap1_lander_prep_note"], "not_requested");
        assert_eq!(report["cap1_executed"], false);
        let _ = fs::remove_dir_all(&root);
    }
}
