//! G6 origin-audit-binding Cap CLI (Cap=0 scaffold → Cap=1 after Cap realloc).
//!
//! Cross-namespace origin/audit product binding feeder after spent G2/G5/CR Cap
//! twins. Cap topology remains Cap-ready until TRANSFER. Cap free stays false
//! after Unlock A EXIT0 unless a documented Cap realloc cites this structural
//! feeder. Never invents `pass_product_clean_room`, origin_audit_product_binding
//! product, Cap free, BrowserReady, or AUTH/AT.
use super::Lab;
use super::cap0_product_honesty::cap_free_after_unlock_a;
use crate::{docker::Result, snapshot};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Code-defined Cap selector (distinct from spent G2/G5/CR Cap twins).
pub const CAP_SELECTOR: &str = "--g6-origin-audit-binding";

/// Exact Cap argv shape for GRANT.pilot_command (evidence bundle filled at Cap day).
pub const CODE_DEFINED_PILOT_COMMAND: &str =
    "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --g6-origin-audit-binding";

/// Cap topology required before Cap can Docker-coordinate G6 origin/audit binding.
pub const REQUIRED_CAP_TOPOLOGY: &[&str] = &[
    "deployments/lab/compose.g6-origin-audit-binding.yaml",
    "build/rust/g6-origin-audit-binding-lab.Dockerfile",
];

/// Cap contract destination after Cap-ready TRANSFER apply.
pub const CONTRACT_REL: &str = "deployments/lab/origin/g6-origin-audit-binding-contract.json";

/// Cap-ready pack (private evidence; candidates out of live paths until TRANSFER).
pub const CAP_READY_PACK_REL: &str =
    ".spec.local/root/evidence/20261109040600-cursor-g6-cap-ready-feeder/cap-ready/g6-origin-audit-binding/candidate-pack/";

/// Documented Cap realloc receipt schema for this structural feeder only.
pub const CAP_REALLOC_SCHEMA: &str = "humanymous.cursor-g6-origin-audit-binding-cap-realloc/1";

/// Cap realloc eligibility schema (Cap=0; Cap free still false until realloc+burn).
pub const CAP_REALLOC_ELIGIBLE_SCHEMA: &str =
    "humanymous.cursor-g6-origin-audit-binding-cap-realloc-eligible/1";

/// Prior G5 Cap=1 component hold (≠ authenticated transport / product).
pub const G5_CAP1_RECEIPT_REL: &str =
    ".spec.local/root/evidence/20261109035200-cursor-g5-desktop-baseline-realloc/CAP1-RECEIPT.json";

/// Prior CR Cap=1 component hold (≠ pass_product_clean_room).
pub const CR_CAP1_RECEIPT_REL: &str =
    ".spec.local/root/evidence/20261109040100-cursor-clean-room-product-cleanup-realloc/CAP1-RECEIPT.json";

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
        "g6-origin-audit-binding Cap refused: missing Cap topology ({missing}); Cap CLI refuse-closed before Docker; Cap-ready pack at {CAP_READY_PACK_REL}; never invent origin/audit product from G5/CR Cap component holds",
        missing = missing.join(", ")
    )
    .into())
}

fn find_g6_cap_realloc(root: &Path) -> Option<PathBuf> {
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
            && value.get("cap_profile").and_then(Value::as_str) == Some("g6-origin-audit-binding")
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

/// Cap=1 only when Cap free=true OR a documented G6 Cap realloc authorizes this feeder.
#[allow(dead_code)]
pub fn require_cap_free_or_g6_realloc(root: &Path) -> Result<()> {
    if cap_free_after_unlock_a(root)? {
        return Ok(());
    }
    if find_g6_cap_realloc(root).is_some() {
        return Ok(());
    }
    Err(format!(
        "g6-origin-audit-binding Cap refused: Cap free=false after Unlock A EXIT0; Cap realloc only after live Cap topology TRANSFER + documented CAP-REALLOC ({CAP_REALLOC_SCHEMA}); Cap-ready pack at {CAP_READY_PACK_REL}; Cap=0 scaffold only; never invent Cap free or origin/audit product binding"
    )
    .into())
}

fn validate_cap1_hold(root: &Path, relative: &str, expected_selector: &str) -> Result<()> {
    let path = root.join(relative);
    if !path.is_file() {
        return Err(format!(
            "g6-origin-audit-binding Cap refused: Cap=1 hold absent ({relative}); Cap=0 honesty first; never invent origin/audit product"
        )
        .into());
    }
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "g6-origin-audit-binding Cap refused: Cap=1 receipt read {}: {error}",
            path.display()
        )
    })?;
    let receipt: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "g6-origin-audit-binding Cap refused: Cap=1 receipt JSON {}: {error}",
            path.display()
        )
    })?;
    if receipt.get("schema").and_then(Value::as_str) != Some("humanymous.cursor-cap1-receipt/1") {
        return Err(
            "g6-origin-audit-binding Cap refused: Cap=1 receipt schema mismatch; never invent origin/audit product"
                .into(),
        );
    }
    if receipt.get("cap_selector").and_then(Value::as_str) != Some(expected_selector) {
        return Err(format!(
            "g6-origin-audit-binding Cap refused: Cap=1 receipt selector mismatch (want {expected_selector})"
        )
        .into());
    }
    if receipt.get("counts_as").and_then(Value::as_str)
        != Some("pass_component_desktop_cap_topology")
    {
        return Err(
            "g6-origin-audit-binding Cap refused: Cap=1 hold must count as pass_component_desktop_cap_topology only"
                .into(),
        );
    }
    if receipt.get("product").and_then(Value::as_u64) != Some(0)
        || receipt
            .get("pass_product_clean_room")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err(
            "g6-origin-audit-binding Cap refused: Cap=1 receipt invented product".into(),
        );
    }
    if receipt.get("cap_exit").and_then(Value::as_u64) != Some(0) {
        return Err(
            "g6-origin-audit-binding Cap refused: Cap=1 hold must be EXIT0 component".into(),
        );
    }
    Ok(())
}

fn validate_component_holds(root: &Path) -> Result<()> {
    validate_cap1_hold(root, G5_CAP1_RECEIPT_REL, "--g5-witness-transport")?;
    validate_cap1_hold(root, CR_CAP1_RECEIPT_REL, "--clean-room-product")?;
    Ok(())
}

/// Cap day entry: refuse-closed Cap=0 when Cap free/topology absent; Cap=1 Docker only after realloc+topology.
pub(super) fn run(lab: &mut Lab) -> Result<()> {
    let root = snapshot::root()?;
    lab.stage = "g6_origin_audit_binding_cap_gate";
    validate_component_holds(&root)?;
    let cap_free = cap_free_after_unlock_a(&root)?;
    let missing = missing_cap_topology(&root);
    let realloc = find_g6_cap_realloc(&root);
    if !missing.is_empty() || (!cap_free && realloc.is_none()) {
        let refuse = json!({
            "schema": "humanymous.lab-g6-origin-audit-binding-cap-refuse/1",
            "record_kind": "g6-origin-audit-binding-cap-refuse",
            "cap": 0,
            "cap_burn": false,
            "cap_free": cap_free,
            "cap_realloc_present": realloc.is_some(),
            "cap_selector": CAP_SELECTOR,
            "pilot_command": CODE_DEFINED_PILOT_COMMAND,
            "product": 0,
            "pass_product_clean_room": 0,
            "origin_audit_product_binding": false,
            "g6_closed": false,
            "g6_matrix_cell_moved": false,
            "missing_cap_topology": missing,
            "cap_ready_pack": CAP_READY_PACK_REL,
            "required_cap_topology": REQUIRED_CAP_TOPOLOGY,
            "contract": CONTRACT_REL,
            "bound_g5_cap1": G5_CAP1_RECEIPT_REL,
            "bound_cr_cap1": CR_CAP1_RECEIPT_REL,
            "cap_realloc_eligible_schema": CAP_REALLOC_ELIGIBLE_SCHEMA,
            "note": "Cap=0 Cap CLI scaffold refuse-closed; Cap free conserved; Cap realloc only after live topology TRANSFER; origin_audit_product_binding=false until Cap measured; never invent pass_product_clean_room; not a G2/G5/CR Cap twin spam"
        });
        snapshot::write(
            &lab.evidence,
            "g6-origin-audit-binding-cap-refuse.json",
            &refuse,
        )?;
        lab.components["g6_origin_audit_binding"] = json!({
            "status": "cap0_scaffold_refused",
            "kind": "g6-origin-audit-binding-cap",
            "product": 0,
            "pass_product_clean_room": 0,
            "origin_audit_product_binding": false,
            "g6_closed": false,
            "cap": 0,
            "cap_free": cap_free,
            "missing_cap_topology": missing,
            "note": "Cap CLI scaffold present; Cap topology / Cap realloc not live; Cap burn=0; G5/CR Cap holds ≠ origin/audit product"
        });
        snapshot::write(
            &lab.evidence,
            "g6-origin-audit-binding-component.json",
            &lab.components["g6_origin_audit_binding"],
        )?;
        lab.case(
            "g6-cap-cli-scaffold-refuse-closed",
            true,
            json!({
                "cap": 0,
                "cap_free": cap_free,
                "missing_cap_topology": missing,
                "origin_audit_product_binding": false,
                "counts_as": "cap0_scaffold_not_product"
            }),
        )?;
        // Cap=0 EXIT0 honesty: Cap CLI scaffolded, Cap burn refused (no Docker).
        return Ok(());
    }
    // Cap=1 Docker burn: live Cap topology + Cap free OR documented G6 Cap realloc.
    // G5/CR Cap green ≠ origin_audit_product_binding ≠ pass_product_clean_room.
    let realloc_path = realloc
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    lab.stage = "g6_origin_audit_binding_compose_config";
    let config = lab.compose(&["config", "--format", "json"], true)?.stdout;
    let parsed: Value = serde_json::from_str(&config)?;
    snapshot::write(
        &lab.evidence,
        "g6-origin-audit-binding-resolved-compose.json",
        &parsed,
    )?;
    lab.case(
        "g6-no-published-ports",
        parsed["services"]
            .as_object()
            .ok_or("services absent")?
            .values()
            .all(|service| service.get("ports").is_none()),
        json!(snapshot::digest(config.as_bytes())),
    )?;
    eprintln!("lab: building immutable g6-origin-audit-binding-lab image (Cap=1)");
    lab.stage = "g6_origin_audit_binding_build";
    lab.compose(&["build", "g6-origin-audit-binding-pilot"], true)?;
    lab.stage = "g6_origin_audit_binding_up";
    lab.compose(&["up", "-d", "g6-origin-audit-binding-pilot"], true)?;
    let container = lab.inspect("g6-origin-audit-binding-pilot")?;
    lab.case(
        "g6-network-none",
        container["HostConfig"]["NetworkMode"] == "none",
        json!("compose.g6-origin-audit-binding.yaml network_mode=none"),
    )?;
    let exit_code = container["State"]["ExitCode"].as_i64().unwrap_or(-1);
    let running = container["State"]["Running"] == true;
    lab.case(
        "g6-pilot-oneshot-or-running",
        running || exit_code == 0,
        json!({"running": running, "exit_code": exit_code}),
    )?;
    lab.components["g6_origin_audit_binding"] = json!({
        "status": "executed",
        "kind": "g6-origin-audit-binding-cap",
        "product": 0,
        "pass_product_clean_room": 0,
        "origin_audit_product_binding": false,
        "g6_closed": false,
        "g6_matrix_cell_moved": false,
        "cap": 1,
        "cap_free": cap_free,
        "cap_realloc": realloc.is_some(),
        "cap_realloc_path": realloc_path,
        "cap_selector": CAP_SELECTOR,
        "contract": CONTRACT_REL,
        "pilot_exit_code": exit_code,
        "pilot_running": running,
        "counts_as": "pass_component_desktop_cap_topology",
        "note": "Cap=1 Docker oneshot component only; G5/CR Cap holds ≠ origin/audit product; never invent pass_product_clean_room"
    });
    snapshot::write(
        &lab.evidence,
        "g6-origin-audit-binding-component.json",
        &lab.components["g6_origin_audit_binding"],
    )?;
    lab.case(
        "g6-cap1-component-not-origin-audit-product",
        true,
        json!({
            "product": 0,
            "pass_product_clean_room": 0,
            "origin_audit_product_binding": false,
            "g6_closed": false,
            "counts_as": "pass_component_desktop_cap_topology"
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
            .any(|p| p.contains("compose.g6-origin-audit-binding.yaml")));
        assert!(REQUIRED_CAP_TOPOLOGY
            .iter()
            .any(|p| p.contains("g6-origin-audit-binding-lab.Dockerfile")));
        assert!(CAP_REALLOC_ELIGIBLE_SCHEMA.contains("cap-realloc-eligible"));
        assert_ne!(CAP_SELECTOR, "--g5-witness-transport");
        assert_ne!(CAP_SELECTOR, "--clean-room-product");
        assert_ne!(CAP_SELECTOR, "--g2-collector-product-binding");
    }

    #[test]
    fn missing_topology_refuse_closed_without_live_files() {
        let root = std::env::temp_dir().join(format!(
            "humanymous-g6-cap-topology-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let missing = missing_cap_topology(&root);
        assert_eq!(missing.len(), REQUIRED_CAP_TOPOLOGY.len());
        assert!(require_cap_topology(&root).is_err());
        assert!(require_cap_free_or_g6_realloc(&root).is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}
