//! Clean-room product-matrix Cap CLI (Cap=0 require-full honesty → Cap=1 after Cap realloc).
//!
//! Distinct from spent `--clean-room-product` Cap1 topology twin and from
//! G2/G5/G6/RB Cap twins. Cap=0 `--clean-room-product-matrix --require-full`
//! tip-binds the product matrix, binds e2e EXIT0 desktop-oracle holds + AUTH/AT
//! refuse, and lists exact non-product cells without Cap burn. Cap topology
//! stays Cap-ready until TRANSFER. Cap=1 (after Cap realloc when Cap free
//! restores) is the preferred rank-14 product-path feeder — never invents
//! `pass_product_clean_room` / BrowserReady / AUTH/AT / Cap free.
use super::Lab;
use super::cap0_product_honesty::{
    INTERACTIVE_AT_HANDOFF_CANDIDATES, cap_free_after_unlock_a,
};
use crate::orchestration::h2_product_binding::N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES;
use crate::{docker::Result, snapshot};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Code-defined Cap selector (product-matrix require-full feeder — not a Cap twin).
pub const CAP_SELECTOR: &str = "--clean-room-product-matrix";

/// Exact Cap argv shape for GRANT.pilot_command (require-full product-matrix feeder).
pub const CODE_DEFINED_PILOT_COMMAND: &str = "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --clean-room-product-matrix --require-full";

/// Cap topology required before Cap can Docker-coordinate product-matrix feeder.
pub const REQUIRED_CAP_TOPOLOGY: &[&str] = &[
    "deployments/lab/compose.clean-room-product-matrix.yaml",
    "build/rust/clean-room-product-matrix-lab.Dockerfile",
];

/// Cap contract destination after Cap-ready TRANSFER apply.
pub const CONTRACT_REL: &str = "deployments/lab/product/clean-room-product-matrix-contract.json";

/// Cap-ready pack (private evidence; candidates out of live paths until TRANSFER).
pub const CAP_READY_PACK_REL: &str = ".spec.local/root/evidence/20261110065000-cursor-clean-room-product-matrix-cap-ready/cap-ready/clean-room-product-matrix/candidate-pack/";

/// Documented Cap realloc receipt schema for this structural feeder only.
pub const CAP_REALLOC_SCHEMA: &str = "humanymous.cursor-clean-room-product-matrix-cap-realloc/1";

/// Cap realloc eligibility schema (Cap=0; Cap free still false until realloc+burn).
pub const CAP_REALLOC_ELIGIBLE_SCHEMA: &str =
    "humanymous.cursor-clean-room-product-matrix-cap-realloc-eligible/1";

/// Clean-room product matrix inventory (tip-bound honesty source).
pub const MATRIX_REL: &str =
    ".spec.local/root/evidence/20261108164500-cursor-clean-room-matrix/matrix.json";

/// e2e EXIT0 desktop-oracle hold (≠ pass_product_clean_room / ≠ Cap realloc alone).
pub const E2E_EXIT0_HOLD_REL: &str =
    ".spec.local/root/evidence/20261110051000-cursor-red-blue-e2e-oneshot/FULL-ATTEMPT4-EXIT0.json";

/// Prior CR Cap=1 spent twin hold (≠ product; topology twin forbidden).
pub const CR_CAP1_RECEIPT_REL: &str = ".spec.local/root/evidence/20261109040100-cursor-clean-room-product-cleanup-realloc/CAP1-RECEIPT.json";

/// Prior RB Cap=1 feeder hold (≠ pass_product_clean_room).
pub const RB_CAP1_RECEIPT_REL: &str =
    ".spec.local/root/evidence/20261110050000-cursor-red-blue-product-cap-realloc/CAP1-RECEIPT.json";

/// Preferred Cap=1 product oneshot after Cap free restores (Docker-only).
pub const PRODUCT_ONESHOT: &[&str] = &[
    "bash scripts/e2e-docker.sh",
    "make e2e",
    "pwsh -File scripts/e2e-docker.ps1",
];

/// Missing Cap topology paths.
pub fn missing_cap_topology(root: &Path) -> Vec<&'static str> {
    REQUIRED_CAP_TOPOLOGY
        .iter()
        .copied()
        .filter(|relative| !root.join(relative).is_file())
        .collect()
}

/// Fail-closed Cap topology gate — cite missing paths; no Docker burn.
#[allow(dead_code)]
pub fn require_cap_topology(root: &Path) -> Result<()> {
    let missing = missing_cap_topology(root);
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "clean-room-product-matrix Cap refused: missing Cap topology ({missing}); Cap CLI refuse-closed before Docker; Cap-ready pack at {CAP_READY_PACK_REL}; never invent pass_product_clean_room from spent Cap twins / e2e EXIT0",
        missing = missing.join(", ")
    )
    .into())
}

fn find_product_matrix_cap_realloc(root: &Path) -> Option<PathBuf> {
    let evidence = root.join(".spec.local/root/evidence");
    let Ok(entries) = fs::read_dir(&evidence) else {
        return None;
    };
    let mut matches = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path().join("CAP-REALLOC.json");
        if !path.is_file() {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        if value.get("schema").and_then(Value::as_str) == Some(CAP_REALLOC_SCHEMA)
            && value.get("cap_profile").and_then(Value::as_str) == Some("clean-room-product-matrix")
            && value.get("cap_realloc").and_then(Value::as_bool) == Some(true)
            && value.get("product").and_then(Value::as_u64) == Some(0)
        {
            matches.push(path);
        }
    }
    matches.into_iter().max_by_key(|path| {
        path.metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    })
}

/// Cap=1 only when Cap free=true OR a documented product-matrix Cap realloc authorizes this feeder.
#[allow(dead_code)]
pub fn require_cap_free_or_product_matrix_realloc(root: &Path) -> Result<()> {
    if cap_free_after_unlock_a(root)? {
        return Ok(());
    }
    if find_product_matrix_cap_realloc(root).is_some() {
        return Ok(());
    }
    Err(format!(
        "clean-room-product-matrix Cap refused: Cap free=false after Unlock A EXIT0; Cap realloc only after live Cap topology TRANSFER + documented CAP-REALLOC ({CAP_REALLOC_SCHEMA}); Cap-ready pack at {CAP_READY_PACK_REL}; Cap=0 require-full honesty only; never invent Cap free or pass_product_clean_room"
    )
    .into())
}

fn tip_sha(root: &Path) -> Result<String> {
    let head = root.join(".git/HEAD");
    let bytes = fs::read(&head).map_err(|error| {
        format!(
            "clean-room-product-matrix Cap refused: read HEAD {}: {error}",
            head.display()
        )
    })?;
    let text = String::from_utf8_lossy(&bytes);
    let trimmed = text.trim();
    if let Some(sha) = trimmed.strip_prefix("ref: ") {
        let ref_path = root.join(".git").join(sha);
        let ref_bytes = fs::read(&ref_path).map_err(|error| {
            format!(
                "clean-room-product-matrix Cap refused: read ref {}: {error}",
                ref_path.display()
            )
        })?;
        Ok(String::from_utf8_lossy(&ref_bytes).trim().to_string())
    } else if trimmed.len() >= 7 {
        Ok(trimmed.to_string())
    } else {
        Err("clean-room-product-matrix Cap refused: HEAD tip unreadable".into())
    }
}

fn load_matrix(root: &Path) -> Result<Value> {
    let path = root.join(MATRIX_REL);
    if !path.is_file() {
        return Err(format!(
            "clean-room-product-matrix Cap refused: matrix absent ({MATRIX_REL}); Cap=0 honesty fail-closed"
        )
        .into());
    }
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "clean-room-product-matrix Cap refused: matrix read {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "clean-room-product-matrix Cap refused: matrix JSON {}: {error}",
            path.display()
        )
        .into()
    })
}

/// Enumerate clean-room-required cells that are not yet `pass_product_clean_room`.
pub fn missing_product_cells(matrix: &Value) -> Vec<Value> {
    let Some(combinations) = matrix.get("combinations").and_then(Value::as_array) else {
        return Vec::new();
    };
    combinations
        .iter()
        .filter(|cell| {
            cell.get("clean_room_required")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && cell.get("status").and_then(Value::as_str) != Some("pass_product_clean_room")
        })
        .map(|cell| {
            json!({
                "id": cell.get("id"),
                "status": cell.get("status"),
                "scenario": cell.get("scenario"),
                "gate": cell.get("gate"),
                "counts_as_product": cell.get("counts_as_product").and_then(Value::as_bool).unwrap_or(false),
            })
        })
        .collect()
}

fn product_matrix_open_cells(missing: &[Value]) -> Vec<Value> {
    missing
        .iter()
        .filter(|cell| {
            let scenario = cell.get("scenario").and_then(Value::as_str).unwrap_or("");
            scenario == "clean-room-product"
                || scenario == "m3-full-product"
                || scenario == "browser-product"
                || scenario == "oracle-detector-bots"
        })
        .cloned()
        .collect()
}

fn validate_prior_component_hold(
    root: &Path,
    relative: &str,
    expected_selector: &str,
) -> Result<()> {
    let path = root.join(relative);
    if !path.is_file() {
        return Err(format!(
            "clean-room-product-matrix Cap refused: prior Cap=1 hold absent ({relative}); Cap=0 honesty first; never invent pass_product_clean_room"
        )
        .into());
    }
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "clean-room-product-matrix Cap refused: Cap=1 receipt read {}: {error}",
            path.display()
        )
    })?;
    let receipt: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "clean-room-product-matrix Cap refused: Cap=1 receipt JSON {}: {error}",
            path.display()
        )
    })?;
    if receipt.get("schema").and_then(Value::as_str) != Some("humanymous.cursor-cap1-receipt/1") {
        return Err(
            "clean-room-product-matrix Cap refused: Cap=1 receipt schema mismatch; never invent pass_product_clean_room"
                .into(),
        );
    }
    if receipt.get("cap_selector").and_then(Value::as_str) != Some(expected_selector) {
        return Err(format!(
            "clean-room-product-matrix Cap refused: Cap=1 receipt selector mismatch (want {expected_selector})"
        )
        .into());
    }
    if receipt.get("product").and_then(Value::as_u64) != Some(0)
        || receipt
            .get("pass_product_clean_room")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err(
            "clean-room-product-matrix Cap refused: Cap=1 receipt invented product".into(),
        );
    }
    Ok(())
}

fn validate_e2e_exit0_hold(root: &Path) -> Result<()> {
    let path = root.join(E2E_EXIT0_HOLD_REL);
    if !path.is_file() {
        return Err(format!(
            "clean-room-product-matrix Cap refused: e2e EXIT0 hold absent ({E2E_EXIT0_HOLD_REL}); Cap=0 honesty first; never invent pass_product_clean_room"
        )
        .into());
    }
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "clean-room-product-matrix Cap refused: e2e EXIT0 read {}: {error}",
            path.display()
        )
    })?;
    let hold: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "clean-room-product-matrix Cap refused: e2e EXIT0 JSON {}: {error}",
            path.display()
        )
    })?;
    if hold.get("exit").and_then(Value::as_i64) != Some(0) {
        return Err(
            "clean-room-product-matrix Cap refused: e2e EXIT0 hold exit≠0; never invent pass_product_clean_room"
                .into(),
        );
    }
    if hold.get("product").and_then(Value::as_u64) != Some(0)
        || hold
            .get("pass_product_clean_room")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err(
            "clean-room-product-matrix Cap refused: e2e EXIT0 hold invented product".into(),
        );
    }
    Ok(())
}

fn auth_at_refuse(root: &Path) -> Value {
    let at_present = INTERACTIVE_AT_HANDOFF_CANDIDATES
        .iter()
        .any(|relative| root.join(relative).is_file());
    let auth_present = N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES
        .iter()
        .any(|relative| root.join(relative).is_file());
    json!({
        "at_present": at_present,
        "auth_present": auth_present,
        "browser_ready": false,
        "n06_owner_join": false,
        "status": if at_present || auth_present { "handoff_present_apply_separately" } else { "absent_ROOT-ONE-STEP_refuse" },
        "note": "AUTH/AT refuse-closed honesty; never forge; Cap feeder does not invent BrowserReady / n06_owner_join"
    })
}

fn honesty_bundle(root: &Path, require_full: bool) -> Result<Value> {
    let tip = tip_sha(root)?;
    let matrix = load_matrix(root)?;
    let matrix_head = matrix
        .get("head_sha")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let missing = missing_product_cells(&matrix);
    let open = product_matrix_open_cells(&missing);
    let tip_bound_exact = tip == matrix_head;
    let tip_bound_prefix_ok = !matrix_head.is_empty()
        && !tip.is_empty()
        && (tip.starts_with(&matrix_head) || matrix_head.starts_with(&tip));
    Ok(json!({
        "tip": tip,
        "matrix_head_sha": matrix_head,
        "tip_bound_exact": tip_bound_exact,
        "tip_bound_prefix_ok": tip_bound_prefix_ok,
        "matrix": MATRIX_REL,
        "require_full": require_full,
        "product": 0,
        "pass_product_clean_room": 0,
        "clean_room_required_non_product": missing.len(),
        "missing_product_cells": missing,
        "product_matrix_open_cells": open,
        "e2e_exit0_hold": E2E_EXIT0_HOLD_REL,
        "auth_at_refuse": auth_at_refuse(root),
        "product_oneshot_when_cap_free": PRODUCT_ONESHOT,
        "not_spent_clean_room_product_cap_twin": true,
        "note": "Cap=0 require-full refuse-closed tip-bound honesty; binds e2e EXIT0 + AUTH/AT refuse; lists exact missing product cells; never invent pass_product_clean_room"
    }))
}

/// Cap day entry: Cap=0 require-full honesty lists missing product cells; Cap=1 only after realloc+topology.
pub(super) fn run(lab: &mut Lab, require_full: bool) -> Result<()> {
    let root = snapshot::root()?;
    lab.stage = "clean_room_product_matrix_cap_gate";
    validate_prior_component_hold(&root, CR_CAP1_RECEIPT_REL, "--clean-room-product")?;
    validate_prior_component_hold(&root, RB_CAP1_RECEIPT_REL, "--red-blue-product")?;
    validate_e2e_exit0_hold(&root)?;
    let cap_free = cap_free_after_unlock_a(&root)?;
    let missing_topology = missing_cap_topology(&root);
    let realloc = find_product_matrix_cap_realloc(&root);
    let honesty = honesty_bundle(&root, require_full)?;
    snapshot::write(
        &lab.evidence,
        "clean-room-product-matrix-require-full-honesty.json",
        &honesty,
    )?;

    // Cap=0 path: refuse Cap burn; tip-bound honesty is the Cap=0 deliverable.
    if !missing_topology.is_empty() || (!cap_free && realloc.is_none()) || !require_full {
        let refuse = json!({
            "schema": "humanymous.lab-clean-room-product-matrix-cap-refuse/1",
            "record_kind": "clean-room-product-matrix-cap-refuse",
            "cap": 0,
            "cap_burn": false,
            "cap_free": cap_free,
            "cap_realloc_present": realloc.is_some(),
            "cap_selector": CAP_SELECTOR,
            "pilot_command": CODE_DEFINED_PILOT_COMMAND,
            "require_full": require_full,
            "require_full_required_for_cap1": true,
            "product": 0,
            "pass_product_clean_room": 0,
            "cr_cap1_bound": true,
            "rb_cap1_bound": true,
            "e2e_exit0_bound": true,
            "auth_at_refuse": honesty.get("auth_at_refuse"),
            "missing_cap_topology": missing_topology,
            "cap_ready_pack": CAP_READY_PACK_REL,
            "required_cap_topology": REQUIRED_CAP_TOPOLOGY,
            "contract": CONTRACT_REL,
            "cap_realloc_eligible_schema": CAP_REALLOC_ELIGIBLE_SCHEMA,
            "honesty": honesty,
            "not_g2_g5_cr_g6_rb_cap_twin": true,
            "not_spent_clean_room_product_topology_twin": true,
            "note": "Cap=0 Cap CLI scaffold / require-full honesty refuse-closed; Cap free conserved; Cap realloc only after live topology TRANSFER; Cap=1 product-matrix feeder waits Cap free; never invent pass_product_clean_room"
        });
        snapshot::write(
            &lab.evidence,
            "clean-room-product-matrix-cap-refuse.json",
            &refuse,
        )?;
        lab.components["clean_room_product_matrix"] = json!({
            "status": "cap0_require_full_honesty_refused",
            "kind": "clean-room-product-matrix-cap",
            "product": 0,
            "pass_product_clean_room": 0,
            "cap": 0,
            "cap_free": cap_free,
            "require_full": require_full,
            "cr_cap1_bound": true,
            "rb_cap1_bound": true,
            "e2e_exit0_bound": true,
            "missing_cap_topology": missing_topology,
            "clean_room_required_non_product": honesty.get("clean_room_required_non_product"),
            "counts_as": "cap0_require_full_honesty_not_product",
            "note": "Cap=0 tip-bound missing product cells listed; Cap topology / Cap realloc not live or --require-full absent; Cap burn=0"
        });
        snapshot::write(
            &lab.evidence,
            "clean-room-product-matrix-component.json",
            &lab.components["clean_room_product_matrix"],
        )?;
        lab.case(
            "clean-room-product-matrix-cap-cli-require-full-honesty-refuse-closed",
            true,
            json!({
                "cap": 0,
                "cap_free": cap_free,
                "require_full": require_full,
                "missing_cap_topology": missing_topology,
                "clean_room_required_non_product": honesty.get("clean_room_required_non_product"),
                "counts_as": "cap0_require_full_honesty_not_product"
            }),
        )?;
        return Ok(());
    }

    // Cap=1 product-matrix require-full feeder (authorized only after Cap realloc + live topology).
    let realloc_path = realloc
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    lab.stage = "clean_room_product_matrix_compose_config";
    let config = lab.compose(&["config", "--format", "json"], true)?.stdout;
    let parsed: Value = serde_json::from_str(&config)?;
    snapshot::write(
        &lab.evidence,
        "clean-room-product-matrix-resolved-compose.json",
        &parsed,
    )?;
    lab.case(
        "clean-room-product-matrix-compose-config",
        parsed.get("services").is_some(),
        json!(snapshot::digest(config.as_bytes())),
    )?;
    eprintln!(
        "lab: building immutable clean-room-product-matrix-lab image (Cap=1 require-full feeder)"
    );
    lab.stage = "clean_room_product_matrix_build";
    lab.compose(&["build", "clean-room-product-matrix-pilot"], true)?;
    lab.stage = "clean_room_product_matrix_up";
    lab.compose(&["up", "-d", "clean-room-product-matrix-pilot"], true)?;
    let container = lab.inspect("clean-room-product-matrix-pilot")?;
    let exit_code = container["State"]["ExitCode"].as_i64().unwrap_or(-1);
    let running = container["State"]["Running"] == true;
    lab.case(
        "clean-room-product-matrix-pilot-oneshot-or-running",
        running || exit_code == 0,
        json!({"running": running, "exit_code": exit_code}),
    )?;
    lab.components["clean_room_product_matrix"] = json!({
        "status": "executed",
        "kind": "clean-room-product-matrix-cap",
        "product": 0,
        "pass_product_clean_room": 0,
        "cap": 1,
        "cap_free": cap_free,
        "cap_realloc": realloc.is_some(),
        "cap_realloc_path": realloc_path,
        "cap_selector": CAP_SELECTOR,
        "require_full": true,
        "contract": CONTRACT_REL,
        "pilot_exit_code": exit_code,
        "pilot_running": running,
        "cr_cap1_bound": true,
        "rb_cap1_bound": true,
        "e2e_exit0_bound": true,
        "auth_at_refuse": honesty.get("auth_at_refuse"),
        "clean_room_required_non_product": honesty.get("clean_room_required_non_product"),
        "counts_as": "clean_room_product_matrix_require_full_feeder_not_product",
        "not_spent_clean_room_product_topology_twin": true,
        "next_product_oneshot": PRODUCT_ONESHOT,
        "note": "Cap=1 require-full product-matrix feeder component; tip-bound gaps retained; AUTH/AT refuse; never invent pass_product_clean_room; subsequent Docker oneshot still required for product cells"
    });
    snapshot::write(
        &lab.evidence,
        "clean-room-product-matrix-component.json",
        &lab.components["clean_room_product_matrix"],
    )?;
    lab.case(
        "clean-room-product-matrix-cap1-require-full-feeder-not-product",
        true,
        json!({
            "product": 0,
            "pass_product_clean_room": 0,
            "counts_as": "clean_room_product_matrix_require_full_feeder_not_product"
        }),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_topology_lists_compose_and_dockerfile() {
        assert!(
            REQUIRED_CAP_TOPOLOGY
                .iter()
                .any(|p| p.contains("compose.clean-room-product-matrix.yaml"))
        );
        assert!(
            REQUIRED_CAP_TOPOLOGY
                .iter()
                .any(|p| p.contains("clean-room-product-matrix-lab.Dockerfile"))
        );
        assert!(CODE_DEFINED_PILOT_COMMAND.contains("--require-full"));
        assert_eq!(CAP_SELECTOR, "--clean-room-product-matrix");
    }

    #[test]
    fn missing_topology_refuse_closed_without_live_files() {
        let root = std::env::temp_dir().join(format!(
            "humanymous-pm-cap-topology-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let missing = missing_cap_topology(&root);
        assert_eq!(missing.len(), REQUIRED_CAP_TOPOLOGY.len());
        assert!(require_cap_topology(&root).is_err());
        assert!(require_cap_free_or_product_matrix_realloc(&root).is_err());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn missing_product_cells_exclude_pass_product_only() {
        let matrix = json!({
            "combinations": [
                {
                    "id": "a",
                    "clean_room_required": true,
                    "status": "unrun",
                    "scenario": "clean-room-product",
                    "gate": "lab_runner_require_full"
                },
                {
                    "id": "b",
                    "clean_room_required": true,
                    "status": "pass_product_clean_room",
                    "scenario": "x",
                    "gate": "y"
                },
                {
                    "id": "c",
                    "clean_room_required": false,
                    "status": "unrun",
                    "scenario": "y",
                    "gate": "z"
                }
            ]
        });
        let missing = missing_product_cells(&matrix);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0]["id"], "a");
        let open = product_matrix_open_cells(&missing);
        assert_eq!(open.len(), 1);
    }
}
