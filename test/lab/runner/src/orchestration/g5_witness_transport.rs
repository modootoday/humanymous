//! G5 witness-transport Cap CLI (Cap=0 scaffold → Cap=1 after Cap realloc).
//!
//! Distinct from Cap=0 `--g5-witness-transport-honesty`. Cap topology
//! (compose + Dockerfile) remains Cap-ready / Root-transfer until live
//! destinations land. Cap free stays false after Unlock A EXIT0 unless a
//! documented Cap realloc cites this structural feeder. Never invent
//! `pass_product_clean_room`, authenticated witness transport product, Cap
//! free, or BrowserReady.
use super::Lab;
use super::cap0_product_honesty::{G5_COMPONENT_SUMMARY_REL, cap_free_after_unlock_a};
use crate::{docker::Result, snapshot};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Code-defined Cap selector (distinct from Cap=0 honesty).
pub const CAP_SELECTOR: &str = "--g5-witness-transport";

/// Exact Cap argv shape for GRANT.pilot_command (evidence bundle filled at Cap day).
pub const CODE_DEFINED_PILOT_COMMAND: &str =
    "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --g5-witness-transport";

/// Cap topology required before Cap can Docker-coordinate G5 witness transport.
pub const REQUIRED_CAP_TOPOLOGY: &[&str] = &[
    "deployments/lab/compose.g5-witness-transport.yaml",
    "build/rust/g5-witness-transport-lab.Dockerfile",
];

/// Cap contract destination after Cap-ready TRANSFER apply.
pub const CONTRACT_REL: &str = "deployments/lab/witness/g5-witness-transport-contract.json";

/// Cap-ready pack (private evidence; candidates out of live paths until TRANSFER).
pub const CAP_READY_PACK_REL: &str =
    ".spec.local/root/evidence/20261109031000-cursor-g5-cap-ready-feeder/cap-ready/g5-witness-transport/candidate-pack/";

/// Documented Cap realloc receipt schema for this structural feeder only.
pub const CAP_REALLOC_SCHEMA: &str = "humanymous.cursor-g5-witness-transport-cap-realloc/1";

/// Cap realloc eligibility schema (Cap=0; Cap free still false until realloc+burn).
pub const CAP_REALLOC_ELIGIBLE_SCHEMA: &str =
    "humanymous.cursor-g5-witness-transport-cap-realloc-eligible/1";

/// Missing Cap topology paths.
pub fn missing_cap_topology(root: &Path) -> Vec<&'static str> {
    REQUIRED_CAP_TOPOLOGY
        .iter()
        .copied()
        .filter(|relative| !root.join(relative).is_file())
        .collect()
}

/// Fail-closed Cap topology gate — cite missing paths; no Docker burn.
#[allow(dead_code)] // Cap=1 path + unit tests; Cap=0 scaffold refuses via missing list.
pub fn require_cap_topology(root: &Path) -> Result<()> {
    let missing = missing_cap_topology(root);
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "g5-witness-transport Cap refused: missing Cap topology ({missing}); Cap CLI refuse-closed before Docker; Cap-ready pack at {CAP_READY_PACK_REL}; never invent authenticated witness transport product from custody component",
        missing = missing.join(", ")
    )
    .into())
}

fn find_g5_cap_realloc(root: &Path) -> Option<PathBuf> {
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
            && value.get("cap_profile").and_then(Value::as_str) == Some("g5-witness-transport")
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

/// Cap=1 only when Cap free=true OR a documented G5 Cap realloc authorizes this feeder.
#[allow(dead_code)] // Cap=1 path + unit tests; Cap=0 scaffold records Cap free without burn.
pub fn require_cap_free_or_g5_realloc(root: &Path) -> Result<()> {
    if cap_free_after_unlock_a(root)? {
        return Ok(());
    }
    if find_g5_cap_realloc(root).is_some() {
        return Ok(());
    }
    Err(format!(
        "g5-witness-transport Cap refused: Cap free=false after Unlock A EXIT0; Cap realloc only after live Cap topology TRANSFER + documented CAP-REALLOC ({CAP_REALLOC_SCHEMA}); Cap-ready pack at {CAP_READY_PACK_REL}; Cap=0 scaffold only; never invent Cap free or authenticated witness transport product"
    )
    .into())
}

fn validate_component_bound(root: &Path) -> Result<()> {
    let summary_path = root.join(G5_COMPONENT_SUMMARY_REL);
    if !summary_path.is_file() {
        return Err(
            "g5-witness-transport Cap refused: custody/witness component summary absent; Cap=0 honesty first; never invent authenticated witness transport product"
                .into(),
        );
    }
    let bytes = fs::read(&summary_path).map_err(|error| {
        format!(
            "g5-witness-transport Cap refused: component summary read {}: {error}",
            summary_path.display()
        )
    })?;
    let summary: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "g5-witness-transport Cap refused: component summary JSON {}: {error}",
            summary_path.display()
        )
    })?;
    if summary.get("schema").and_then(Value::as_str)
        != Some("humanymous.product-custody-witness.attempt-summary/1")
    {
        return Err(
            "g5-witness-transport Cap refused: custody/witness summary schema mismatch".into(),
        );
    }
    let status = summary.get("status").and_then(Value::as_str).unwrap_or("");
    if status != "pass_component_only" && status != "pass_component_desktop" {
        return Err(format!(
            "g5-witness-transport Cap refused: status={status} is not component-only; never invent authenticated witness transport product"
        )
        .into());
    }
    if summary
        .get("full_product_qualified")
        .and_then(Value::as_bool)
        == Some(true)
        || summary.get("m3_claim").and_then(Value::as_bool) == Some(true)
    {
        return Err(
            "g5-witness-transport Cap refused: component summary invented product/M3".into(),
        );
    }
    Ok(())
}

/// Cap day entry: refuse-closed Cap=0 when Cap free/topology absent; Cap=1 Docker only after realloc+topology.
pub(super) fn run(lab: &mut Lab) -> Result<()> {
    let root = snapshot::root()?;
    lab.stage = "g5_witness_transport_cap_gate";
    validate_component_bound(&root)?;
    let cap_free = cap_free_after_unlock_a(&root)?;
    let missing = missing_cap_topology(&root);
    let realloc = find_g5_cap_realloc(&root);
    if !missing.is_empty() || (!cap_free && realloc.is_none()) {
        let refuse = json!({
            "schema": "humanymous.lab-g5-witness-transport-cap-refuse/1",
            "record_kind": "g5-witness-transport-cap-refuse",
            "cap": 0,
            "cap_burn": false,
            "cap_free": cap_free,
            "cap_realloc_present": realloc.is_some(),
            "cap_selector": CAP_SELECTOR,
            "pilot_command": CODE_DEFINED_PILOT_COMMAND,
            "product": 0,
            "pass_product_clean_room": 0,
            "authenticated_witness_transport": false,
            "g5_closed": false,
            "g5_matrix_cell_moved": false,
            "missing_cap_topology": missing,
            "cap_ready_pack": CAP_READY_PACK_REL,
            "required_cap_topology": REQUIRED_CAP_TOPOLOGY,
            "contract": CONTRACT_REL,
            "component_summary": G5_COMPONENT_SUMMARY_REL,
            "cap_realloc_eligible_schema": CAP_REALLOC_ELIGIBLE_SCHEMA,
            "bound_honesty_selector": "--g5-witness-transport-honesty",
            "note": "Cap=0 Cap CLI scaffold refuse-closed; Cap free conserved; Cap realloc only after live topology TRANSFER; authenticated_witness_transport=false until Cap measured; never invent pass_product_clean_room"
        });
        snapshot::write(
            &lab.evidence,
            "g5-witness-transport-cap-refuse.json",
            &refuse,
        )?;
        lab.components["g5_witness_transport"] = json!({
            "status": "cap0_scaffold_refused",
            "kind": "g5-witness-transport-cap",
            "product": 0,
            "pass_product_clean_room": 0,
            "authenticated_witness_transport": false,
            "g5_closed": false,
            "cap": 0,
            "cap_free": cap_free,
            "missing_cap_topology": missing,
            "note": "Cap CLI scaffold present; Cap topology / Cap realloc not live; Cap burn=0; custody component ≠ authenticated transport"
        });
        snapshot::write(
            &lab.evidence,
            "g5-witness-transport-component.json",
            &lab.components["g5_witness_transport"],
        )?;
        lab.case(
            "g5-cap-cli-scaffold-refuse-closed",
            true,
            json!({
                "cap": 0,
                "cap_free": cap_free,
                "missing_cap_topology": missing,
                "authenticated_witness_transport": false,
                "counts_as": "cap0_scaffold_not_product"
            }),
        )?;
        // Cap=0 EXIT0 honesty: Cap CLI scaffolded, Cap burn refused (no Docker).
        return Ok(());
    }
    // Cap=1 path reserved: Cap free or Cap realloc + live topology. Cap burn still
    // refuse-closed here until a future Cap cycle executes Docker under realloc.
    lab.components["g5_witness_transport"] = json!({
        "status": "cap1_path_reserved_not_burned",
        "kind": "g5-witness-transport-cap",
        "product": 0,
        "pass_product_clean_room": 0,
        "authenticated_witness_transport": false,
        "g5_closed": false,
        "cap": 0,
        "cap_free": cap_free,
        "note": "Live topology + Cap realloc observed but Cap=0 scaffold does not Cap-burn; Cap=1 is next Cap cycle only; authenticated_witness_transport stays false until Cap measured"
    });
    snapshot::write(
        &lab.evidence,
        "g5-witness-transport-component.json",
        &lab.components["g5_witness_transport"],
    )?;
    Err(
        "g5-witness-transport Cap refused: Cap=0 scaffold does not Cap-burn Docker; Cap=1 only after Cap realloc cycle with jobs=1; never invent authenticated witness transport product"
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn required_topology_lists_compose_and_dockerfile() {
        assert!(REQUIRED_CAP_TOPOLOGY
            .iter()
            .any(|p| p.contains("compose.g5-witness-transport.yaml")));
        assert!(REQUIRED_CAP_TOPOLOGY
            .iter()
            .any(|p| p.contains("g5-witness-transport-lab.Dockerfile")));
        assert!(CAP_REALLOC_ELIGIBLE_SCHEMA.contains("cap-realloc-eligible"));
        assert_ne!(CAP_SELECTOR, "--g5-witness-transport-honesty");
    }

    #[test]
    fn missing_topology_refuse_closed_without_live_files() {
        let root = std::env::temp_dir().join(format!(
            "humanymous-g5-cap-topology-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let missing = missing_cap_topology(&root);
        assert_eq!(missing.len(), REQUIRED_CAP_TOPOLOGY.len());
        assert!(require_cap_topology(&root).is_err());
        assert!(require_cap_free_or_g5_realloc(&root).is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}
