//! Cap=0 AT-HANDOFF measured apply helper (ROOT-ONE-STEP twin).
//!
//! Standing autonomy: when Root places `AT-HANDOFF.json` schema
//! `humanymous.root-interactive-at-windows-tuple/1`, Cursor runs one command to
//! validate + bind the handoff. Never invents BrowserReady / product / Cap free /
//! AT tokens. Cap burn stays 0.
use crate::docker::Result;
use crate::orchestration::cap0_product_honesty::{
    BROWSER_READY_CAP_SELECTOR, BROWSER_READY_PILOT_COMMAND, BROWSER_READY_ROOT_ASK_PATHS,
    INTERACTIVE_AT_HANDOFF_CANDIDATES, cap_free_after_unlock_a,
};
use crate::snapshot;
use serde_json::{Value, json};
use std::{fs, path::Path};

const SCHEMA: &str = "humanymous.lab-at-handoff-apply/1";
const INTERACTIVE_AT_HANDOFF_SCHEMA: &str = "humanymous.root-interactive-at-windows-tuple/1";
const KIND_AT: &str = "at";

/// Cap=0 AT-HANDOFF apply / refuse-closed wake.
///
/// Usage: `handoff-apply --kind at [--workspace PATH] [--dry-run] [--emit PATH]`
pub fn run(args: &[String]) -> Result<i32> {
    let mut kind = None;
    let mut workspace = None;
    let mut emit_path = None;
    let mut dry_run = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--kind" => {
                index += 1;
                kind = Some(args.get(index).ok_or("--kind value missing")?.clone());
            }
            "--workspace" => {
                index += 1;
                workspace = Some(args.get(index).ok_or("--workspace path missing")?.clone());
            }
            "--emit" => {
                index += 1;
                emit_path = Some(args.get(index).ok_or("--emit path missing")?.clone());
            }
            "--dry-run" => dry_run = true,
            _ => {
                return Err(
                    "usage: handoff-apply --kind at [--workspace PATH] [--dry-run] [--emit PATH]"
                        .into(),
                );
            }
        }
        index += 1;
    }
    let kind = kind.ok_or("handoff-apply requires --kind at")?;
    if kind != KIND_AT {
        return Err(format!(
            "handoff-apply refused: unsupported kind={kind}; Cap=0 AT helper only supports --kind at"
        )
        .into());
    }

    let root = match workspace {
        Some(path) => Path::new(&path).canonicalize().map_err(|error| {
            format!("handoff-apply refused: workspace canonicalize {path}: {error}")
        })?,
        None => snapshot::root()?,
    };
    let report = apply_at_handoff(&root, dry_run)?;
    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("handoff-apply encode failed: {error}"))?;
    if let Some(path) = emit_path {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).map_err(|error| format!("emit mkdir: {error}"))?;
        }
        fs::write(&path, format!("{rendered}\n"))
            .map_err(|error| format!("emit write failed: {error}"))?;
    }
    println!("{rendered}");
    Ok(0)
}

fn first_present(root: &Path, candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find(|relative| root.join(relative).is_file())
        .map(|relative| (*relative).to_string())
}

fn validate_at_handoff(handoff: &Value) -> Result<()> {
    if handoff.get("schema").and_then(Value::as_str) != Some(INTERACTIVE_AT_HANDOFF_SCHEMA) {
        return Err(
            "handoff-apply refused: AT-HANDOFF schema mismatch; require humanymous.root-interactive-at-windows-tuple/1; never invent Root AT tokens"
                .into(),
        );
    }
    if handoff
        .get("browser_ready_authorized")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err(
            "handoff-apply refused: AT-HANDOFF browser_ready_authorized must be true; Cap=0 measured apply only"
                .into(),
        );
    }
    if handoff.get("product").and_then(Value::as_u64) != Some(0)
        || handoff
            .get("pass_product_clean_room")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err(
            "handoff-apply refused: AT-HANDOFF invented product/pass_product_clean_room".into(),
        );
    }
    Ok(())
}

fn apply_at_handoff(root: &Path, dry_run: bool) -> Result<Value> {
    let cap_free = cap_free_after_unlock_a(root)?;
    let present = first_present(root, INTERACTIVE_AT_HANDOFF_CANDIDATES);
    let Some(relative) = present else {
        return Ok(json!({
            "schema": SCHEMA,
            "record_kind": "lab-at-handoff-apply",
            "kind": KIND_AT,
            "cap": 0,
            "cap_burn": 0,
            "cap_free": cap_free,
            "product": 0,
            "pass_product_clean_room": 0,
            "browser_ready": false,
            "applied": false,
            "dry_run": dry_run,
            "at_handoff_present": false,
            "handoff_candidates": INTERACTIVE_AT_HANDOFF_CANDIDATES,
            "root_ask_paths": BROWSER_READY_ROOT_ASK_PATHS,
            "status": "refuse_closed_at_absent",
            "next_honesty": BROWSER_READY_PILOT_COMMAND,
            "next_honesty_selector": BROWSER_READY_CAP_SELECTOR,
            "note": "AT-HANDOFF absent; Cap=0 ROOT-ONE-STEP twin waits for Root schema humanymous.root-interactive-at-windows-tuple/1; never invent BrowserReady / AT tokens",
            "honesty": {
                "never_invent_browser_ready": true,
                "never_forge_at_handoff": true,
                "never_invent_pass_product_clean_room": true,
                "never_invent_cap_free": true,
                "detection_freeze": true,
                "cap0_only": true
            }
        }));
    };

    let path = root.join(&relative);
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "handoff-apply refused: AT-HANDOFF read {}: {error}",
            path.display()
        )
    })?;
    let handoff: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "handoff-apply refused: AT-HANDOFF JSON {}: {error}",
            path.display()
        )
    })?;
    validate_at_handoff(&handoff)?;

    // Measured bind only — BrowserReady stays false until separate honesty/join.
    Ok(json!({
        "schema": SCHEMA,
        "record_kind": "lab-at-handoff-apply",
        "kind": KIND_AT,
        "cap": 0,
        "cap_burn": 0,
        "cap_free": cap_free,
        "product": 0,
        "pass_product_clean_room": 0,
        "browser_ready": false,
        "applied": !dry_run,
        "dry_run": dry_run,
        "at_handoff_present": true,
        "at_handoff_path": relative,
        "at_handoff_schema": INTERACTIVE_AT_HANDOFF_SCHEMA,
        "browser_ready_authorized": true,
        "root_ask_paths": BROWSER_READY_ROOT_ASK_PATHS,
        "status": if dry_run { "validated_dry_run" } else { "measured_apply_bound" },
        "next_honesty": BROWSER_READY_PILOT_COMMAND,
        "next_honesty_selector": BROWSER_READY_CAP_SELECTOR,
        "note": "AT-HANDOFF schema validated and Cap=0 measured-bound; BrowserReady remains false until honesty join measures it; never invent BrowserReady from handoff alone",
        "honesty": {
            "never_invent_browser_ready": true,
            "never_forge_at_handoff": true,
            "never_invent_pass_product_clean_room": true,
            "never_invent_cap_free": true,
            "detection_freeze": true,
            "cap0_only": true,
            "handoff_authorize_ne_browser_ready": true
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("hmn-at-apply-{label}-{nanos}"));
        fs::create_dir_all(&root).expect("tmpdir");
        root
    }

    #[test]
    fn absent_at_refuses_closed_exit_shape() {
        let root = temp_root("absent");
        let report = apply_at_handoff(&root, true).expect("report");
        assert_eq!(report["schema"], SCHEMA);
        assert_eq!(report["applied"], false);
        assert_eq!(report["browser_ready"], false);
        assert_eq!(report["product"], 0);
        assert_eq!(report["at_handoff_present"], false);
        assert_eq!(report["status"], "refuse_closed_at_absent");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn present_valid_at_binds_without_inventing_browser_ready() {
        let root = temp_root("present");
        let relative = INTERACTIVE_AT_HANDOFF_CANDIDATES[0];
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        let handoff = json!({
            "schema": INTERACTIVE_AT_HANDOFF_SCHEMA,
            "browser_ready_authorized": true,
            "product": 0,
            "pass_product_clean_room": 0
        });
        fs::write(&path, serde_json::to_vec_pretty(&handoff).expect("encode")).expect("write");
        let report = apply_at_handoff(&root, false).expect("report");
        assert_eq!(report["applied"], true);
        assert_eq!(report["browser_ready"], false);
        assert_eq!(report["at_handoff_present"], true);
        assert_eq!(report["status"], "measured_apply_bound");
        let _ = fs::remove_dir_all(&root);
    }
}
