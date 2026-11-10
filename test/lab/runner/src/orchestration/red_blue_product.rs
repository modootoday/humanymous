//! Red-blue product Cap CLI (Cap=0 require-full honesty → Cap=1 after Cap realloc).
//!
//! Distinct from spent G2/G5/CR/G6 Cap twins and from G7 structural attestation.
//! Cap=0 `--red-blue-product --require-full` tip-binds the clean-room matrix and
//! lists exact missing product / red-blue cells without Cap burn. Cap topology
//! stays Cap-ready until TRANSFER. Cap=1 (after Cap realloc when Cap free
//! restores) is the direct product/red-blue feeder — never invents
//! `pass_product_clean_room` / BrowserReady / AUTH/AT / Cap free.
use super::Lab;
use super::cap0_product_honesty::cap_free_after_unlock_a;
use crate::{docker::Result, snapshot};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Code-defined Cap selector (product/red-blue feeder — not a Cap twin).
pub const CAP_SELECTOR: &str = "--red-blue-product";

/// Exact Cap argv shape for GRANT.pilot_command (require-full product feeder).
pub const CODE_DEFINED_PILOT_COMMAND: &str =
    "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --red-blue-product --require-full";

/// Cap topology required before Cap can Docker-coordinate red-blue product feeder.
pub const REQUIRED_CAP_TOPOLOGY: &[&str] = &[
    "deployments/lab/compose.red-blue-product.yaml",
    "build/rust/red-blue-product-lab.Dockerfile",
];

/// Cap contract destination after Cap-ready TRANSFER apply.
pub const CONTRACT_REL: &str = "deployments/lab/product/red-blue-product-contract.json";

/// Cap-ready pack (private evidence; candidates out of live paths until TRANSFER).
pub const CAP_READY_PACK_REL: &str =
    ".spec.local/root/evidence/20261110044800-cursor-red-blue-product-cap-ready/cap-ready/red-blue-product/candidate-pack/";

/// Documented Cap realloc receipt schema for this structural feeder only.
pub const CAP_REALLOC_SCHEMA: &str = "humanymous.cursor-red-blue-product-cap-realloc/1";

/// Cap realloc eligibility schema (Cap=0; Cap free still false until realloc+burn).
pub const CAP_REALLOC_ELIGIBLE_SCHEMA: &str =
    "humanymous.cursor-red-blue-product-cap-realloc-eligible/1";

/// Clean-room product matrix inventory (tip-bound honesty source).
pub const MATRIX_REL: &str =
    ".spec.local/root/evidence/20261108164500-cursor-clean-room-matrix/matrix.json";

/// Red-blue-validate skill (completion authority checklist).
pub const RED_BLUE_SKILL_REL: &str = ".agents/skills/red-blue-validate/SKILL.md";

/// Preferred Cap=1 product oneshot after Cap free restores (Docker-only).
/// Windows Desktop: prefer e2e-docker-launch.ps1 (mutex + pressure guard).
/// Host/loopback `node test/e2e/runner.mjs` ≠ completion authority.
pub const PRODUCT_ONESHOT: &[&str] = &[
    "pwsh -File scripts/e2e-docker-launch.ps1",
    "bash scripts/e2e-docker.sh",
    "make e2e",
    "pwsh -File scripts/e2e-docker.ps1",
];

/// Prior CR Cap=1 component hold (≠ pass_product_clean_room).
pub const CR_CAP1_RECEIPT_REL: &str =
    ".spec.local/root/evidence/20261109040100-cursor-clean-room-product-cleanup-realloc/CAP1-RECEIPT.json";

/// Prior G6 Cap=1 component hold (≠ origin/audit product).
pub const G6_CAP1_RECEIPT_REL: &str =
    ".spec.local/root/evidence/20261109040700-cursor-g6-cap-realloc/CAP1-RECEIPT.json";

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
        "red-blue-product Cap refused: missing Cap topology ({missing}); Cap CLI refuse-closed before Docker; Cap-ready pack at {CAP_READY_PACK_REL}; never invent pass_product_clean_room from Cap twins",
        missing = missing.join(", ")
    )
    .into())
}

fn find_red_blue_cap_realloc(root: &Path) -> Option<PathBuf> {
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
            && value.get("cap_profile").and_then(Value::as_str) == Some("red-blue-product")
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

/// Cap=1 only when Cap free=true OR a documented red-blue Cap realloc authorizes this feeder.
#[allow(dead_code)]
pub fn require_cap_free_or_red_blue_realloc(root: &Path) -> Result<()> {
    if cap_free_after_unlock_a(root)? {
        return Ok(());
    }
    if find_red_blue_cap_realloc(root).is_some() {
        return Ok(());
    }
    Err(format!(
        "red-blue-product Cap refused: Cap free=false after Unlock A EXIT0; Cap realloc only after live Cap topology TRANSFER + documented CAP-REALLOC ({CAP_REALLOC_SCHEMA}); Cap-ready pack at {CAP_READY_PACK_REL}; Cap=0 require-full honesty only; never invent Cap free or pass_product_clean_room"
    )
    .into())
}

fn tip_sha(root: &Path) -> Result<String> {
    let head = root.join(".git/HEAD");
    let bytes = fs::read(&head).map_err(|error| {
        format!(
            "red-blue-product Cap refused: read HEAD {}: {error}",
            head.display()
        )
    })?;
    let text = String::from_utf8_lossy(&bytes);
    let trimmed = text.trim();
    if let Some(sha) = trimmed.strip_prefix("ref: ") {
        let ref_path = root.join(".git").join(sha);
        let ref_bytes = fs::read(&ref_path).map_err(|error| {
            format!(
                "red-blue-product Cap refused: read ref {}: {error}",
                ref_path.display()
            )
        })?;
        Ok(String::from_utf8_lossy(&ref_bytes).trim().to_string())
    } else if trimmed.len() >= 7 {
        Ok(trimmed.to_string())
    } else {
        Err("red-blue-product Cap refused: HEAD tip unreadable".into())
    }
}

fn load_matrix(root: &Path) -> Result<Value> {
    let path = root.join(MATRIX_REL);
    if !path.is_file() {
        return Err(format!(
            "red-blue-product Cap refused: matrix absent ({MATRIX_REL}); Cap=0 honesty fail-closed"
        )
        .into());
    }
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "red-blue-product Cap refused: matrix read {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "red-blue-product Cap refused: matrix JSON {}: {error}",
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

fn red_blue_product_open_cells(missing: &[Value]) -> Vec<Value> {
    missing
        .iter()
        .filter(|cell| {
            let scenario = cell.get("scenario").and_then(Value::as_str).unwrap_or("");
            let gate = cell.get("gate").and_then(Value::as_str).unwrap_or("");
            scenario == "oracle-detector-bots"
                || scenario == "m3-full-product"
                || scenario == "clean-room-product"
                || gate.starts_with("red_blue_")
                || gate == "lab_runner_require_full"
        })
        .cloned()
        .collect()
}

fn validate_prior_component_hold(root: &Path, relative: &str, expected_selector: &str) -> Result<()> {
    let path = root.join(relative);
    if !path.is_file() {
        return Err(format!(
            "red-blue-product Cap refused: prior Cap=1 hold absent ({relative}); Cap=0 honesty first; never invent pass_product_clean_room"
        )
        .into());
    }
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "red-blue-product Cap refused: Cap=1 receipt read {}: {error}",
            path.display()
        )
    })?;
    let receipt: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "red-blue-product Cap refused: Cap=1 receipt JSON {}: {error}",
            path.display()
        )
    })?;
    if receipt.get("schema").and_then(Value::as_str) != Some("humanymous.cursor-cap1-receipt/1") {
        return Err(
            "red-blue-product Cap refused: Cap=1 receipt schema mismatch; never invent pass_product_clean_room"
                .into(),
        );
    }
    if receipt.get("cap_selector").and_then(Value::as_str) != Some(expected_selector) {
        return Err(format!(
            "red-blue-product Cap refused: Cap=1 receipt selector mismatch (want {expected_selector})"
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
            "red-blue-product Cap refused: Cap=1 receipt invented product".into(),
        );
    }
    Ok(())
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
    let red_blue_open = red_blue_product_open_cells(&missing);
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
        "red_blue_skill": RED_BLUE_SKILL_REL,
        "require_full": require_full,
        "product": 0,
        "pass_product_clean_room": 0,
        "clean_room_required_non_product": missing.len(),
        "missing_product_cells": missing,
        "red_blue_product_open_cells": red_blue_open,
        "product_oneshot_when_cap_free": PRODUCT_ONESHOT,
        "note": "Cap=0 require-full refuse-closed tip-bound honesty; lists exact missing product cells; never invent pass_product_clean_room"
    }))
}

/// Cap day entry: Cap=0 require-full honesty lists missing product cells; Cap=1 only after realloc+topology.
pub(super) fn run(lab: &mut Lab, require_full: bool) -> Result<()> {
    let root = snapshot::root()?;
    lab.stage = "red_blue_product_cap_gate";
    validate_prior_component_hold(&root, CR_CAP1_RECEIPT_REL, "--clean-room-product")?;
    validate_prior_component_hold(&root, G6_CAP1_RECEIPT_REL, "--g6-origin-audit-binding")?;
    let cap_free = cap_free_after_unlock_a(&root)?;
    let missing_topology = missing_cap_topology(&root);
    let realloc = find_red_blue_cap_realloc(&root);
    let honesty = honesty_bundle(&root, require_full)?;
    snapshot::write(
        &lab.evidence,
        "red-blue-product-require-full-honesty.json",
        &honesty,
    )?;

    // Cap=0 path: refuse Cap burn; tip-bound honesty is the Cap=0 deliverable.
    if !missing_topology.is_empty() || (!cap_free && realloc.is_none()) || !require_full {
        let refuse = json!({
            "schema": "humanymous.lab-red-blue-product-cap-refuse/1",
            "record_kind": "red-blue-product-cap-refuse",
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
            "g6_cap1_bound": true,
            "missing_cap_topology": missing_topology,
            "cap_ready_pack": CAP_READY_PACK_REL,
            "required_cap_topology": REQUIRED_CAP_TOPOLOGY,
            "contract": CONTRACT_REL,
            "cap_realloc_eligible_schema": CAP_REALLOC_ELIGIBLE_SCHEMA,
            "honesty": honesty,
            "not_g2_g5_cr_g6_cap_twin": true,
            "not_g7_structural_twin": true,
            "note": "Cap=0 Cap CLI scaffold / require-full honesty refuse-closed; Cap free conserved; Cap realloc only after live topology TRANSFER; Cap=1 product/red-blue feeder waits Cap free; never invent pass_product_clean_room"
        });
        snapshot::write(
            &lab.evidence,
            "red-blue-product-cap-refuse.json",
            &refuse,
        )?;
        lab.components["red_blue_product"] = json!({
            "status": "cap0_require_full_honesty_refused",
            "kind": "red-blue-product-cap",
            "product": 0,
            "pass_product_clean_room": 0,
            "cap": 0,
            "cap_free": cap_free,
            "require_full": require_full,
            "cr_cap1_bound": true,
            "g6_cap1_bound": true,
            "missing_cap_topology": missing_topology,
            "clean_room_required_non_product": honesty.get("clean_room_required_non_product"),
            "counts_as": "cap0_require_full_honesty_not_product",
            "note": "Cap=0 tip-bound missing product cells listed; Cap topology / Cap realloc not live or --require-full absent; Cap burn=0"
        });
        snapshot::write(
            &lab.evidence,
            "red-blue-product-component.json",
            &lab.components["red_blue_product"],
        )?;
        lab.case(
            "red-blue-cap-cli-require-full-honesty-refuse-closed",
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

    // Cap=1 product/red-blue feeder (authorized only after Cap realloc + live topology).
    // Distinct from Cap twin network_mode=none component holds: require-full feeder
    // resolves red-blue product compose (oracle include) and re-emits tip-bound gaps.
    let realloc_path = realloc
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    lab.stage = "red_blue_product_compose_config";
    let config = lab.compose(&["config", "--format", "json"], true)?.stdout;
    let parsed: Value = serde_json::from_str(&config)?;
    snapshot::write(
        &lab.evidence,
        "red-blue-product-resolved-compose.json",
        &parsed,
    )?;
    lab.case(
        "red-blue-product-compose-config",
        parsed.get("services").is_some(),
        json!(snapshot::digest(config.as_bytes())),
    )?;
    eprintln!("lab: building immutable red-blue-product-lab image (Cap=1 require-full feeder)");
    lab.stage = "red_blue_product_build";
    lab.compose(&["build", "red-blue-product-pilot"], true)?;
    lab.stage = "red_blue_product_up";
    lab.compose(&["up", "-d", "red-blue-product-pilot"], true)?;
    let container = lab.inspect("red-blue-product-pilot")?;
    let exit_code = container["State"]["ExitCode"].as_i64().unwrap_or(-1);
    let running = container["State"]["Running"] == true;
    lab.case(
        "red-blue-product-pilot-oneshot-or-running",
        running || exit_code == 0,
        json!({"running": running, "exit_code": exit_code}),
    )?;
    lab.components["red_blue_product"] = json!({
        "status": "executed",
        "kind": "red-blue-product-cap",
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
        "g6_cap1_bound": true,
        "clean_room_required_non_product": honesty.get("clean_room_required_non_product"),
        "counts_as": "red_blue_product_require_full_feeder_not_product",
        "not_pass_component_desktop_cap_topology_twin": true,
        "next_product_oneshot": PRODUCT_ONESHOT,
        "note": "Cap=1 require-full product/red-blue feeder component; tip-bound gaps retained; never invent pass_product_clean_room; subsequent Docker red-blue-validate oneshot still required for red_blue_* cells"
    });
    snapshot::write(
        &lab.evidence,
        "red-blue-product-component.json",
        &lab.components["red_blue_product"],
    )?;
    lab.case(
        "red-blue-cap1-require-full-feeder-not-product",
        true,
        json!({
            "product": 0,
            "pass_product_clean_room": 0,
            "counts_as": "red_blue_product_require_full_feeder_not_product"
        }),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_topology_lists_compose_and_dockerfile() {
        assert!(REQUIRED_CAP_TOPOLOGY
            .iter()
            .any(|p| p.contains("compose.red-blue-product.yaml")));
        assert!(REQUIRED_CAP_TOPOLOGY
            .iter()
            .any(|p| p.contains("red-blue-product-lab.Dockerfile")));
        assert!(CODE_DEFINED_PILOT_COMMAND.contains("--require-full"));
        assert_eq!(CAP_SELECTOR, "--red-blue-product");
    }

    #[test]
    fn missing_topology_refuse_closed_without_live_files() {
        let root = std::env::temp_dir().join(format!(
            "humanymous-red-blue-cap-topology-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let missing = missing_cap_topology(&root);
        assert_eq!(missing.len(), REQUIRED_CAP_TOPOLOGY.len());
        assert!(require_cap_topology(&root).is_err());
        assert!(require_cap_free_or_red_blue_realloc(&root).is_err());
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
                    "scenario": "oracle-detector-bots",
                    "gate": "red_blue_attack"
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
        let open = red_blue_product_open_cells(&missing);
        assert_eq!(open.len(), 1);
    }
}
