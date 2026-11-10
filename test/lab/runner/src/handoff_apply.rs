//! Cap=0 AT / AUTH / BrowserReady-join measured apply helpers (ROOT-ONE-STEP twins).
//!
//! Standing autonomy: when Root places live handoff JSON (never TEMPLATE), Cursor
//! runs one command to validate + bind. Never invents BrowserReady / n06_owner_join
//! / product / Cap free / AT or AUTH tokens. Cap burn stays 0.
use crate::docker::Result;
use crate::orchestration::cap0_product_honesty::{
    BROWSER_READY_CAP_SELECTOR, BROWSER_READY_PILOT_COMMAND, BROWSER_READY_ROOT_ASK_PATHS,
    INTERACTIVE_AT_HANDOFF_CANDIDATES, N06_OWNER_AUTH_CAP_SELECTOR, N06_OWNER_AUTH_PILOT_COMMAND,
    UNLOCK_A_CLOSER_REL, cap_free_after_unlock_a,
};
use crate::orchestration::h2_product_binding::{
    N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES, N06_OWNER_JOIN_AUTH_HANDOFF_SCHEMA,
    N06_OWNER_JOIN_ROOT_ASK_PATHS,
};
use crate::snapshot;
use serde_json::{Value, json};
use std::{fs, path::Path};

const AT_SCHEMA: &str = "humanymous.lab-at-handoff-apply/1";
const AUTH_SCHEMA: &str = "humanymous.lab-auth-handoff-apply/1";
const BROWSER_READY_JOIN_SCHEMA: &str = "humanymous.lab-browser-ready-join-apply/1";
const INTERACTIVE_AT_HANDOFF_SCHEMA: &str = "humanymous.root-interactive-at-windows-tuple/1";
const KIND_AT: &str = "at";
const KIND_AUTH: &str = "auth";
const KIND_BROWSER_READY_JOIN: &str = "browser-ready-join";

/// Cap=0 AT / AUTH / BrowserReady-join apply / refuse-closed wake.
///
/// Usage:
/// `handoff-apply --kind at|auth|browser-ready-join [--workspace PATH] [--dry-run] [--emit PATH]`
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
                    "usage: handoff-apply --kind at|auth|browser-ready-join [--workspace PATH] [--dry-run] [--emit PATH]"
                        .into(),
                );
            }
        }
        index += 1;
    }
    let kind = kind.ok_or(
        "handoff-apply requires --kind at|auth|browser-ready-join",
    )?;

    let root = match workspace {
        Some(path) => Path::new(&path).canonicalize().map_err(|error| {
            format!("handoff-apply refused: workspace canonicalize {path}: {error}")
        })?,
        None => snapshot::root()?,
    };
    let report = match kind.as_str() {
        KIND_AT => apply_at_handoff(&root, dry_run)?,
        KIND_AUTH => apply_auth_handoff(&root, dry_run)?,
        KIND_BROWSER_READY_JOIN => apply_browser_ready_join(&root, dry_run)?,
        other => {
            return Err(format!(
                "handoff-apply refused: unsupported kind={other}; Cap=0 supports --kind at|auth|browser-ready-join"
            )
            .into());
        }
    };
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
        .find(|relative| {
            let path = root.join(relative);
            path.is_file() && !is_template_path(&path)
        })
        .map(|relative| (*relative).to_string())
}

fn is_template_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.contains(".TEMPLATE.")
                || name.ends_with(".TEMPLATE.json")
                || name == "GRANT.TEMPLATE.json"
                || name == "AT-HANDOFF.TEMPLATE.json"
                || name == "AUTH-HANDOFF.TEMPLATE.json"
        })
}

fn refuse_template_bytes(label: &str, bytes: &[u8]) -> Result<()> {
    let text = String::from_utf8_lossy(bytes);
    if text.contains("TEMPLATE only") || text.contains("\"record_kind\": \"template\"") {
        return Err(format!(
            "handoff-apply refused: {label} still TEMPLATE bytes; Root must author live JSON (not TEMPLATE); never invent AT/AUTH/BrowserReady"
        )
        .into());
    }
    Ok(())
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

fn validate_auth_handoff(handoff: &Value) -> Result<()> {
    if handoff.get("schema").and_then(Value::as_str) != Some(N06_OWNER_JOIN_AUTH_HANDOFF_SCHEMA) {
        return Err(
            "handoff-apply refused: AUTH-HANDOFF schema mismatch; require humanymous.root-n06-owner-join-auth-handoff/1; never invent Root AUTH tokens"
                .into(),
        );
    }
    if handoff
        .get("n06_owner_join_authorized")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err(
            "handoff-apply refused: AUTH-HANDOFF n06_owner_join_authorized must be true; Cap=0 measured apply only"
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
            "handoff-apply refused: AUTH-HANDOFF invented product/pass_product_clean_room".into(),
        );
    }
    if handoff.get("n06_owner_join").and_then(Value::as_bool) == Some(true) {
        return Err(
            "handoff-apply refused: AUTH-HANDOFF invented n06_owner_join=true; Cap=0 measured apply never invents join"
                .into(),
        );
    }
    Ok(())
}

fn apply_at_handoff(root: &Path, dry_run: bool) -> Result<Value> {
    let cap_free = cap_free_after_unlock_a(root)?;
    let present = first_present(root, INTERACTIVE_AT_HANDOFF_CANDIDATES);
    let Some(relative) = present else {
        return Ok(json!({
            "schema": AT_SCHEMA,
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
    refuse_template_bytes("AT-HANDOFF", &bytes)?;
    let handoff: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "handoff-apply refused: AT-HANDOFF JSON {}: {error}",
            path.display()
        )
    })?;
    validate_at_handoff(&handoff)?;

    // Measured bind only — BrowserReady stays false until separate honesty/join.
    Ok(json!({
        "schema": AT_SCHEMA,
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
        "next_join": "cargo run --locked -p humanymous-lab-runner -- handoff-apply --kind browser-ready-join",
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

fn apply_auth_handoff(root: &Path, dry_run: bool) -> Result<Value> {
    let cap_free = cap_free_after_unlock_a(root)?;
    let present = first_present(root, N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES);
    let Some(relative) = present else {
        return Ok(json!({
            "schema": AUTH_SCHEMA,
            "record_kind": "lab-auth-handoff-apply",
            "kind": KIND_AUTH,
            "cap": 0,
            "cap_burn": 0,
            "cap_free": cap_free,
            "product": 0,
            "pass_product_clean_room": 0,
            "n06_owner_join": false,
            "applied": false,
            "dry_run": dry_run,
            "auth_handoff_present": false,
            "handoff_candidates": N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES,
            "root_ask_paths": N06_OWNER_JOIN_ROOT_ASK_PATHS,
            "status": "refuse_closed_auth_absent",
            "next_honesty": N06_OWNER_AUTH_PILOT_COMMAND,
            "next_honesty_selector": N06_OWNER_AUTH_CAP_SELECTOR,
            "note": "AUTH-HANDOFF absent; Cap=0 ROOT-ONE-STEP twin waits for Root schema humanymous.root-n06-owner-join-auth-handoff/1; never invent n06_owner_join / AUTH tokens",
            "honesty": {
                "never_invent_n06_owner_join": true,
                "never_forge_auth_handoff": true,
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
            "handoff-apply refused: AUTH-HANDOFF read {}: {error}",
            path.display()
        )
    })?;
    refuse_template_bytes("AUTH-HANDOFF", &bytes)?;
    let handoff: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "handoff-apply refused: AUTH-HANDOFF JSON {}: {error}",
            path.display()
        )
    })?;
    validate_auth_handoff(&handoff)?;

    // Measured bind only — n06_owner_join stays false until separate honesty/join.
    Ok(json!({
        "schema": AUTH_SCHEMA,
        "record_kind": "lab-auth-handoff-apply",
        "kind": KIND_AUTH,
        "cap": 0,
        "cap_burn": 0,
        "cap_free": cap_free,
        "product": 0,
        "pass_product_clean_room": 0,
        "n06_owner_join": false,
        "applied": !dry_run,
        "dry_run": dry_run,
        "auth_handoff_present": true,
        "auth_handoff_path": relative,
        "auth_handoff_schema": N06_OWNER_JOIN_AUTH_HANDOFF_SCHEMA,
        "n06_owner_join_authorized": true,
        "root_ask_paths": N06_OWNER_JOIN_ROOT_ASK_PATHS,
        "status": if dry_run { "validated_dry_run" } else { "measured_apply_bound" },
        "next_honesty": N06_OWNER_AUTH_PILOT_COMMAND,
        "next_honesty_selector": N06_OWNER_AUTH_CAP_SELECTOR,
        "note": "AUTH-HANDOFF schema validated and Cap=0 measured-bound; n06_owner_join remains false until honesty join measures it; never invent n06_owner_join from handoff alone",
        "honesty": {
            "never_invent_n06_owner_join": true,
            "never_forge_auth_handoff": true,
            "never_invent_pass_product_clean_room": true,
            "never_invent_cap_free": true,
            "detection_freeze": true,
            "cap0_only": true,
            "handoff_authorize_ne_join": true
        }
    }))
}

fn apply_browser_ready_join(root: &Path, dry_run: bool) -> Result<Value> {
    let cap_free = cap_free_after_unlock_a(root)?;
    let unlock_a_present = root.join(UNLOCK_A_CLOSER_REL).is_file();
    let at_present = first_present(root, INTERACTIVE_AT_HANDOFF_CANDIDATES);
    let cef_cap_day_present = crate::cef_bundle::cap_day_present(root).unwrap_or(false);
    let cef_env_present = std::env::var_os("HUMANYMOUS_CEF_PILOT_BUNDLE")
        .as_ref()
        .is_some_and(|value| !value.is_empty());

    if !unlock_a_present {
        return Ok(json!({
            "schema": BROWSER_READY_JOIN_SCHEMA,
            "record_kind": "lab-browser-ready-join-apply",
            "kind": KIND_BROWSER_READY_JOIN,
            "cap": 0,
            "cap_burn": 0,
            "cap_free": cap_free,
            "product": 0,
            "pass_product_clean_room": 0,
            "browser_ready": false,
            "applied": false,
            "dry_run": dry_run,
            "unlock_a_closer_present": false,
            "unlock_a_closer": UNLOCK_A_CLOSER_REL,
            "at_handoff_present": at_present.is_some(),
            "cef_cap_day_present": cef_cap_day_present,
            "cef_bundle_env_present": cef_env_present,
            "status": "refuse_closed_unlock_a_absent",
            "root_ask_paths": BROWSER_READY_ROOT_ASK_PATHS,
            "note": "BrowserReady join refused: Unlock A Cap EXIT0 closer absent; Cap=0; never invent BrowserReady",
            "honesty": {
                "never_invent_browser_ready": true,
                "never_invent_pass_product_clean_room": true,
                "never_invent_cap_free": true,
                "detection_freeze": true,
                "cap0_only": true,
                "unlock_a_ne_browser_ready": true
            }
        }));
    }

    let Some(relative) = at_present else {
        return Ok(json!({
            "schema": BROWSER_READY_JOIN_SCHEMA,
            "record_kind": "lab-browser-ready-join-apply",
            "kind": KIND_BROWSER_READY_JOIN,
            "cap": 0,
            "cap_burn": 0,
            "cap_free": cap_free,
            "product": 0,
            "pass_product_clean_room": 0,
            "browser_ready": false,
            "applied": false,
            "dry_run": dry_run,
            "unlock_a_closer_present": true,
            "unlock_a_closer": UNLOCK_A_CLOSER_REL,
            "at_handoff_present": false,
            "handoff_candidates": INTERACTIVE_AT_HANDOFF_CANDIDATES,
            "cef_cap_day_present": cef_cap_day_present,
            "cef_bundle_env_present": cef_env_present,
            "status": "refuse_closed_at_absent",
            "root_ask_paths": BROWSER_READY_ROOT_ASK_PATHS,
            "next_at_apply": "cargo run --locked -p humanymous-lab-runner -- handoff-apply --kind at",
            "next_honesty": BROWSER_READY_PILOT_COMMAND,
            "note": "BrowserReady join refused: AT-HANDOFF absent; Cap=0; Unlock A ≠ BrowserReady; never invent BrowserReady",
            "honesty": {
                "never_invent_browser_ready": true,
                "never_forge_at_handoff": true,
                "never_invent_pass_product_clean_room": true,
                "never_invent_cap_free": true,
                "detection_freeze": true,
                "cap0_only": true,
                "unlock_a_ne_browser_ready": true
            }
        }));
    };

    let path = root.join(&relative);
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "handoff-apply refused: BrowserReady-join AT-HANDOFF read {}: {error}",
            path.display()
        )
    })?;
    refuse_template_bytes("AT-HANDOFF", &bytes)?;
    let handoff: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "handoff-apply refused: BrowserReady-join AT-HANDOFF JSON {}: {error}",
            path.display()
        )
    })?;
    validate_at_handoff(&handoff)?;

    // Fail-closed Cap0 join: prerequisites bound, BrowserReady stays false until
    // a separate measured honesty pilot proves it. Never invent BrowserReady=true.
    let prerequisites_complete = cef_cap_day_present || cef_env_present;
    let status = if dry_run {
        "validated_dry_run"
    } else if prerequisites_complete {
        "prerequisites_bound_await_honesty_measure"
    } else {
        "at_bound_cef_missing_await_cap_day"
    };

    Ok(json!({
        "schema": BROWSER_READY_JOIN_SCHEMA,
        "record_kind": "lab-browser-ready-join-apply",
        "kind": KIND_BROWSER_READY_JOIN,
        "cap": 0,
        "cap_burn": 0,
        "cap_free": cap_free,
        "product": 0,
        "pass_product_clean_room": 0,
        "browser_ready": false,
        "applied": !dry_run,
        "dry_run": dry_run,
        "unlock_a_closer_present": true,
        "unlock_a_closer": UNLOCK_A_CLOSER_REL,
        "at_handoff_present": true,
        "at_handoff_path": relative,
        "at_handoff_schema": INTERACTIVE_AT_HANDOFF_SCHEMA,
        "browser_ready_authorized": true,
        "cef_cap_day_present": cef_cap_day_present,
        "cef_bundle_env_present": cef_env_present,
        "prerequisites_complete": prerequisites_complete,
        "status": status,
        "root_ask_paths": BROWSER_READY_ROOT_ASK_PATHS,
        "next_honesty": BROWSER_READY_PILOT_COMMAND,
        "next_honesty_selector": BROWSER_READY_CAP_SELECTOR,
        "note": "AT-HANDOFF + Unlock A Cap=0 join prerequisites validated; BrowserReady remains false until honesty measure proves it; never invent BrowserReady",
        "honesty": {
            "never_invent_browser_ready": true,
            "never_forge_at_handoff": true,
            "never_invent_pass_product_clean_room": true,
            "never_invent_cap_free": true,
            "detection_freeze": true,
            "cap0_only": true,
            "handoff_authorize_ne_browser_ready": true,
            "unlock_a_ne_browser_ready": true,
            "join_prereq_ne_browser_ready": true
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
        let root = std::env::temp_dir().join(format!("hmn-handoff-apply-{label}-{nanos}"));
        fs::create_dir_all(&root).expect("tmpdir");
        root
    }

    #[test]
    fn absent_at_refuses_closed_exit_shape() {
        let root = temp_root("absent-at");
        let report = apply_at_handoff(&root, true).expect("report");
        assert_eq!(report["schema"], AT_SCHEMA);
        assert_eq!(report["applied"], false);
        assert_eq!(report["browser_ready"], false);
        assert_eq!(report["product"], 0);
        assert_eq!(report["at_handoff_present"], false);
        assert_eq!(report["status"], "refuse_closed_at_absent");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn present_valid_at_binds_without_inventing_browser_ready() {
        let root = temp_root("present-at");
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

    #[test]
    fn absent_auth_refuses_closed_exit_shape() {
        let root = temp_root("absent-auth");
        let report = apply_auth_handoff(&root, true).expect("report");
        assert_eq!(report["schema"], AUTH_SCHEMA);
        assert_eq!(report["applied"], false);
        assert_eq!(report["n06_owner_join"], false);
        assert_eq!(report["product"], 0);
        assert_eq!(report["auth_handoff_present"], false);
        assert_eq!(report["status"], "refuse_closed_auth_absent");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn present_valid_auth_binds_without_inventing_n06_join() {
        let root = temp_root("present-auth");
        let relative = N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES[0];
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        let handoff = json!({
            "schema": N06_OWNER_JOIN_AUTH_HANDOFF_SCHEMA,
            "n06_owner_join_authorized": true,
            "n06_owner_join": false,
            "product": 0,
            "pass_product_clean_room": 0
        });
        fs::write(&path, serde_json::to_vec_pretty(&handoff).expect("encode")).expect("write");
        let report = apply_auth_handoff(&root, false).expect("report");
        assert_eq!(report["applied"], true);
        assert_eq!(report["n06_owner_join"], false);
        assert_eq!(report["auth_handoff_present"], true);
        assert_eq!(report["status"], "measured_apply_bound");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn auth_refuses_invented_n06_owner_join_true() {
        let root = temp_root("auth-invent");
        let relative = N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES[0];
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        let handoff = json!({
            "schema": N06_OWNER_JOIN_AUTH_HANDOFF_SCHEMA,
            "n06_owner_join_authorized": true,
            "n06_owner_join": true,
            "product": 0,
            "pass_product_clean_room": 0
        });
        fs::write(&path, serde_json::to_vec_pretty(&handoff).expect("encode")).expect("write");
        let err = apply_auth_handoff(&root, false).expect_err("must refuse invent");
        assert!(err.to_string().contains("n06_owner_join=true"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn browser_ready_join_refuses_closed_without_at() {
        let root = temp_root("br-join-no-at");
        // Unlock A closer present so we reach AT-absent branch.
        let unlock = root.join(UNLOCK_A_CLOSER_REL);
        if let Some(parent) = unlock.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(
            &unlock,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.cursor-unlock-a-desktop-baseline-realloc-closer/1",
                "cap_exit": 0,
                "cap_free": false
            }))
            .expect("encode"),
        )
        .expect("write");
        let report = apply_browser_ready_join(&root, true).expect("report");
        assert_eq!(report["schema"], BROWSER_READY_JOIN_SCHEMA);
        assert_eq!(report["browser_ready"], false);
        assert_eq!(report["applied"], false);
        assert_eq!(report["status"], "refuse_closed_at_absent");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn browser_ready_join_binds_prereqs_without_inventing_ready() {
        let root = temp_root("br-join-bound");
        let unlock = root.join(UNLOCK_A_CLOSER_REL);
        if let Some(parent) = unlock.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(
            &unlock,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.cursor-unlock-a-desktop-baseline-realloc-closer/1",
                "cap_exit": 0,
                "cap_free": false
            }))
            .expect("encode"),
        )
        .expect("write");
        let relative = INTERACTIVE_AT_HANDOFF_CANDIDATES[0];
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        let handoff = json!({
            "schema": INTERACTIVE_AT_HANDOFF_SCHEMA,
            "browser_ready_authorized": true,
            "product": 0,
            "pass_product_clean_room": 0,
            "browser_ready": false
        });
        fs::write(&path, serde_json::to_vec_pretty(&handoff).expect("encode")).expect("write");
        let report = apply_browser_ready_join(&root, false).expect("report");
        assert_eq!(report["applied"], true);
        assert_eq!(report["browser_ready"], false);
        assert_eq!(report["at_handoff_present"], true);
        assert_eq!(report["product"], 0);
        assert_ne!(report["status"], "refuse_closed_at_absent");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn template_bytes_refused() {
        let root = temp_root("template");
        let relative = INTERACTIVE_AT_HANDOFF_CANDIDATES[0];
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        let handoff = json!({
            "schema": INTERACTIVE_AT_HANDOFF_SCHEMA,
            "browser_ready_authorized": true,
            "product": 0,
            "pass_product_clean_room": 0,
            "note": "TEMPLATE only — Root authors the live AT-HANDOFF.json."
        });
        fs::write(&path, serde_json::to_vec_pretty(&handoff).expect("encode")).expect("write");
        let err = apply_at_handoff(&root, false).expect_err("template refuse");
        assert!(err.to_string().contains("TEMPLATE"));
        let _ = fs::remove_dir_all(&root);
    }
}
