//! Clean-room product Cap CLI (Cap=0 scaffold → Cap=1 after Cap realloc).
//!
//! Binds G2+G5 Cap=1 component holds and Cap=0 AUTH/AT refuse without inventing
//! `pass_product_clean_room`, BrowserReady, N06 join, or Cap free. Cap topology
//! remains Cap-ready until TRANSFER.
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

/// Code-defined Cap selector (distinct from bare `--require-full` Cap spam).
pub const CAP_SELECTOR: &str = "--clean-room-product";

/// Exact Cap argv shape for GRANT.pilot_command (evidence bundle filled at Cap day).
pub const CODE_DEFINED_PILOT_COMMAND: &str =
    "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --clean-room-product";

/// Cap topology required before Cap can Docker-coordinate clean-room product feeder.
pub const REQUIRED_CAP_TOPOLOGY: &[&str] = &[
    "deployments/lab/compose.clean-room-product.yaml",
    "build/rust/clean-room-product-lab.Dockerfile",
];

/// Cap contract destination after Cap-ready TRANSFER apply.
pub const CONTRACT_REL: &str = "deployments/lab/product/clean-room-product-contract.json";

/// Cap-ready pack (private evidence; candidates out of live paths until TRANSFER).
pub const CAP_READY_PACK_REL: &str =
    ".spec.local/root/evidence/20261109040000-cursor-clean-room-product-cap-ready/cap-ready/clean-room-product/candidate-pack/";

/// Documented Cap realloc receipt schema for this structural feeder only.
pub const CAP_REALLOC_SCHEMA: &str = "humanymous.cursor-clean-room-product-cap-realloc/1";

/// Cap realloc eligibility schema (Cap=0; Cap free still false until realloc+burn).
pub const CAP_REALLOC_ELIGIBLE_SCHEMA: &str =
    "humanymous.cursor-clean-room-product-cap-realloc-eligible/1";

/// G2 Cap=1 component hold (≠ pass_product_clean_room).
pub const G2_CAP1_RECEIPT_REL: &str =
    ".spec.local/root/evidence/20261109034100-cursor-g2-cap-cleanup-realloc/CAP1-RECEIPT.json";

/// G5 Cap=1 component hold (≠ authenticated_witness_transport / product).
pub const G5_CAP1_RECEIPT_REL: &str =
    ".spec.local/root/evidence/20261109035200-cursor-g5-desktop-baseline-realloc/CAP1-RECEIPT.json";

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
        "clean-room-product Cap refused: missing Cap topology ({missing}); Cap CLI refuse-closed before Docker; Cap-ready pack at {CAP_READY_PACK_REL}; never invent pass_product_clean_room from G2/G5 component holds",
        missing = missing.join(", ")
    )
    .into())
}

fn find_clean_room_cap_realloc(root: &Path) -> Option<PathBuf> {
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
            && value.get("cap_profile").and_then(Value::as_str) == Some("clean-room-product")
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

/// Cap=1 only when Cap free=true OR a documented clean-room Cap realloc authorizes this feeder.
#[allow(dead_code)]
pub fn require_cap_free_or_clean_room_realloc(root: &Path) -> Result<()> {
    if cap_free_after_unlock_a(root)? {
        return Ok(());
    }
    if find_clean_room_cap_realloc(root).is_some() {
        return Ok(());
    }
    Err(format!(
        "clean-room-product Cap refused: Cap free=false after Unlock A EXIT0; Cap realloc only after live Cap topology TRANSFER + documented CAP-REALLOC ({CAP_REALLOC_SCHEMA}); Cap-ready pack at {CAP_READY_PACK_REL}; Cap=0 scaffold only; never invent Cap free or pass_product_clean_room"
    )
    .into())
}

fn validate_cap1_component_hold(root: &Path, relative: &str, expected_selector: &str) -> Result<()> {
    let path = root.join(relative);
    if !path.is_file() {
        return Err(format!(
            "clean-room-product Cap refused: Cap=1 component hold absent ({relative}); Cap=0 honesty first; never invent pass_product_clean_room"
        )
        .into());
    }
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "clean-room-product Cap refused: Cap=1 receipt read {}: {error}",
            path.display()
        )
    })?;
    let receipt: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "clean-room-product Cap refused: Cap=1 receipt JSON {}: {error}",
            path.display()
        )
    })?;
    if receipt.get("schema").and_then(Value::as_str) != Some("humanymous.cursor-cap1-receipt/1") {
        return Err(
            "clean-room-product Cap refused: Cap=1 receipt schema mismatch; never invent pass_product_clean_room"
                .into(),
        );
    }
    if receipt.get("cap_selector").and_then(Value::as_str) != Some(expected_selector) {
        return Err(format!(
            "clean-room-product Cap refused: Cap=1 receipt selector mismatch (want {expected_selector})"
        )
        .into());
    }
    if receipt.get("counts_as").and_then(Value::as_str)
        != Some("pass_component_desktop_cap_topology")
    {
        return Err(
            "clean-room-product Cap refused: Cap=1 hold must count as pass_component_desktop_cap_topology only"
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
            "clean-room-product Cap refused: Cap=1 receipt invented product".into(),
        );
    }
    if receipt.get("cap_exit").and_then(Value::as_u64) != Some(0) {
        return Err(
            "clean-room-product Cap refused: Cap=1 hold must be EXIT0 component".into(),
        );
    }
    Ok(())
}

fn auth_at_handoff_present(root: &Path) -> (bool, bool) {
    let auth = N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES
        .iter()
        .any(|relative| root.join(relative).is_file());
    let at = INTERACTIVE_AT_HANDOFF_CANDIDATES
        .iter()
        .any(|relative| root.join(relative).is_file());
    (auth, at)
}

fn validate_component_binds_and_auth_at_refuse(root: &Path) -> Result<(bool, bool)> {
    validate_cap1_component_hold(root, G2_CAP1_RECEIPT_REL, "--g2-collector-product-binding")?;
    validate_cap1_component_hold(root, G5_CAP1_RECEIPT_REL, "--g5-witness-transport")?;
    let (auth, at) = auth_at_handoff_present(root);
    // Cap=0 product feeder advances with AUTH/AT refuse when absent; never forge.
    // When present, Cap=0 measured apply belongs to AUTH/AT Cap paths — not this feeder.
    if auth || at {
        return Err(
            "clean-room-product Cap refused: AUTH/AT handoff present — Cap=0 measured AUTH/AT apply path only; this Cap feeder must not invent BrowserReady/N06 join from Cap topology"
                .into(),
        );
    }
    Ok((auth, at))
}

/// Cap day entry: refuse-closed Cap=0 when Cap free/topology absent; Cap=1 Docker only after realloc+topology.
pub(super) fn run(lab: &mut Lab) -> Result<()> {
    let root = snapshot::root()?;
    lab.stage = "clean_room_product_cap_gate";
    let (auth_present, at_present) = validate_component_binds_and_auth_at_refuse(&root)?;
    let cap_free = cap_free_after_unlock_a(&root)?;
    let missing = missing_cap_topology(&root);
    let realloc = find_clean_room_cap_realloc(&root);
    if !missing.is_empty() || (!cap_free && realloc.is_none()) {
        let refuse = json!({
            "schema": "humanymous.lab-clean-room-product-cap-refuse/1",
            "record_kind": "clean-room-product-cap-refuse",
            "cap": 0,
            "cap_burn": false,
            "cap_free": cap_free,
            "cap_realloc_present": realloc.is_some(),
            "cap_selector": CAP_SELECTOR,
            "pilot_command": CODE_DEFINED_PILOT_COMMAND,
            "product": 0,
            "pass_product_clean_room": 0,
            "g2_cap1_bound": true,
            "g5_cap1_bound": true,
            "auth_handoff_present": auth_present,
            "at_handoff_present": at_present,
            "browser_ready": false,
            "n06_owner_join": false,
            "missing_cap_topology": missing,
            "cap_ready_pack": CAP_READY_PACK_REL,
            "required_cap_topology": REQUIRED_CAP_TOPOLOGY,
            "contract": CONTRACT_REL,
            "cap_realloc_eligible_schema": CAP_REALLOC_ELIGIBLE_SCHEMA,
            "bound_g2_receipt": G2_CAP1_RECEIPT_REL,
            "bound_g5_receipt": G5_CAP1_RECEIPT_REL,
            "note": "Cap=0 Cap CLI scaffold refuse-closed; G2+G5 Cap=1 holds bound; AUTH/AT refuse; Cap free conserved; Cap realloc only after live topology TRANSFER; never invent pass_product_clean_room"
        });
        snapshot::write(
            &lab.evidence,
            "clean-room-product-cap-refuse.json",
            &refuse,
        )?;
        lab.components["clean_room_product"] = json!({
            "status": "cap0_scaffold_refused",
            "kind": "clean-room-product-cap",
            "product": 0,
            "pass_product_clean_room": 0,
            "cap": 0,
            "cap_free": cap_free,
            "g2_cap1_bound": true,
            "g5_cap1_bound": true,
            "auth_handoff_present": auth_present,
            "at_handoff_present": at_present,
            "browser_ready": false,
            "n06_owner_join": false,
            "missing_cap_topology": missing,
            "note": "Cap CLI scaffold present; Cap topology / Cap realloc not live; Cap burn=0; AUTH/AT refuse"
        });
        snapshot::write(
            &lab.evidence,
            "clean-room-product-component.json",
            &lab.components["clean_room_product"],
        )?;
        lab.case(
            "clean-room-cap-cli-scaffold-refuse-closed",
            true,
            json!({
                "cap": 0,
                "cap_free": cap_free,
                "missing_cap_topology": missing,
                "g2_cap1_bound": true,
                "g5_cap1_bound": true,
                "auth_handoff_present": auth_present,
                "at_handoff_present": at_present,
                "counts_as": "cap0_scaffold_not_product"
            }),
        )?;
        return Ok(());
    }

    // Cap=1 Docker burn: live Cap topology + Cap free OR documented Cap realloc.
    let realloc_path = realloc
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    lab.stage = "clean_room_product_compose_config";
    let config = lab.compose(&["config", "--format", "json"], true)?.stdout;
    let parsed: Value = serde_json::from_str(&config)?;
    snapshot::write(
        &lab.evidence,
        "clean-room-product-resolved-compose.json",
        &parsed,
    )?;
    lab.case(
        "clean-room-no-published-ports",
        parsed["services"]
            .as_object()
            .ok_or("services absent")?
            .values()
            .all(|service| service.get("ports").is_none()),
        json!(snapshot::digest(config.as_bytes())),
    )?;
    eprintln!("lab: building immutable clean-room-product-lab image (Cap=1)");
    lab.stage = "clean_room_product_build";
    lab.compose(&["build", "clean-room-product-pilot"], true)?;
    lab.stage = "clean_room_product_up";
    lab.compose(&["up", "-d", "clean-room-product-pilot"], true)?;
    let container = lab.inspect("clean-room-product-pilot")?;
    lab.case(
        "clean-room-network-none",
        container["HostConfig"]["NetworkMode"] == "none",
        json!("compose.clean-room-product.yaml network_mode=none"),
    )?;
    let exit_code = container["State"]["ExitCode"].as_i64().unwrap_or(-1);
    let running = container["State"]["Running"] == true;
    lab.case(
        "clean-room-pilot-oneshot-or-running",
        running || exit_code == 0,
        json!({"running": running, "exit_code": exit_code}),
    )?;
    lab.components["clean_room_product"] = json!({
        "status": "executed",
        "kind": "clean-room-product-cap",
        "product": 0,
        "pass_product_clean_room": 0,
        "cap": 1,
        "cap_free": cap_free,
        "cap_realloc": realloc.is_some(),
        "cap_realloc_path": realloc_path,
        "cap_selector": CAP_SELECTOR,
        "contract": CONTRACT_REL,
        "pilot_exit_code": exit_code,
        "pilot_running": running,
        "g2_cap1_bound": true,
        "g5_cap1_bound": true,
        "auth_handoff_present": auth_present,
        "at_handoff_present": at_present,
        "browser_ready": false,
        "n06_owner_join": false,
        "counts_as": "pass_component_desktop_cap_topology",
        "note": "Cap=1 Docker oneshot component only; G2+G5 holds bound; AUTH/AT refuse; never invent pass_product_clean_room"
    });
    snapshot::write(
        &lab.evidence,
        "clean-room-product-component.json",
        &lab.components["clean_room_product"],
    )?;
    lab.case(
        "clean-room-cap1-component-not-product",
        true,
        json!({
            "product": 0,
            "pass_product_clean_room": 0,
            "browser_ready": false,
            "n06_owner_join": false,
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
            .any(|p| p.contains("compose.clean-room-product.yaml")));
        assert!(REQUIRED_CAP_TOPOLOGY
            .iter()
            .any(|p| p.contains("clean-room-product-lab.Dockerfile")));
        assert!(CAP_REALLOC_ELIGIBLE_SCHEMA.contains("cap-realloc-eligible"));
        assert_eq!(CAP_SELECTOR, "--clean-room-product");
    }

    #[test]
    fn missing_topology_refuse_closed_without_live_files() {
        let root = std::env::temp_dir().join(format!(
            "humanymous-clean-room-cap-topology-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let missing = missing_cap_topology(&root);
        assert_eq!(missing.len(), REQUIRED_CAP_TOPOLOGY.len());
        assert!(require_cap_topology(&root).is_err());
        assert!(require_cap_free_or_clean_room_realloc(&root).is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}
