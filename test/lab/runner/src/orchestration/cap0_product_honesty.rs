//! Cap=0 product-honesty gates for G2 collector product-binding, G5 witness
//! transport, and BrowserReady / interactive AT path. Never Cap-burns; never
//! invents `pass_product_clean_room` / BrowserReady / N06 join / Root AT
//! tokens. Cap free remains false after Unlock A EXIT0 unless a real
//! structural next feeder reallocates Cap.
use super::Lab;
use crate::docker::Result;
use crate::snapshot;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Cap=0 G2 collector product-binding readiness selector (no Cap burn).
pub const G2_CAP_SELECTOR: &str = "--g2-collector-product-binding-readiness";

/// Cap=0 G5 witness transport honesty selector (no Cap burn).
pub const G5_CAP_SELECTOR: &str = "--g5-witness-transport-honesty";

/// Cap=0 BrowserReady / interactive AT honesty selector (no Cap burn).
pub const BROWSER_READY_CAP_SELECTOR: &str = "--browser-ready-interactive-at-honesty";

pub const G2_PILOT_COMMAND: &str = "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --g2-collector-product-binding-readiness";

pub const G5_PILOT_COMMAND: &str = "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --g5-witness-transport-honesty";

pub const BROWSER_READY_PILOT_COMMAND: &str = "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --browser-ready-interactive-at-honesty";

const G2_READINESS_SCHEMA: &str = "humanymous.lab-g2-collector-product-binding-readiness/1";
const G5_HONESTY_SCHEMA: &str = "humanymous.lab-g5-witness-transport-honesty/1";
const BROWSER_READY_HONESTY_SCHEMA: &str = "humanymous.lab-browser-ready-interactive-at-honesty/1";
const UNLOCK_A_CLOSER_SCHEMA: &str = "humanymous.cursor-unlock-a-desktop-baseline-realloc-closer/1";
const INTERACTIVE_AT_HANDOFF_SCHEMA: &str = "humanymous.root-interactive-at-windows-tuple/1";

const G2_COMPONENT_RECEIPT_REL: &str = ".spec.local/root/evidence/20261108213000-g2-collector-provenance/cap1-browser-pilot/cap1-receipt.json";

const G5_COMPONENT_SUMMARY_REL: &str =
    ".spec.local/root/evidence/cursor-product-custody-witness/attempt-5/summary.json";

/// Hosted / product-binding topology still Root-allocated for G2 close.
pub const G2_PRODUCT_BINDING_TOPOLOGY: &[&str] = &[
    "deployments/lab/compose.rust.collector.yaml",
    "deployments/lab/compose.browser-product.yaml",
];

/// Authenticated witness transport product paths remain Root-allocated (G5).
pub const G5_ROOT_ASK_PATHS: &[&str] = &[
    "crates/audit/**",
    "crates/sdk/humanymous-runtime/src/audit/**",
    "deployments/lab/compose.rust.witness.yaml",
    ".spec/sot/audit-key-custody.sot.md",
];

/// Root interactive AT Windows tuple surfaces (BrowserReady path; never invent).
pub const BROWSER_READY_ROOT_ASK_PATHS: &[&str] = &[
    "crates/browser/**",
    "scripts/ci/cef-pilot/**",
    "crates/apps/humanymous-browser/**",
];

/// Private evidence candidates where Root may place interactive AT handoff.
pub const INTERACTIVE_AT_HANDOFF_CANDIDATES: &[&str] = &[
    ".spec.local/root/evidence/root-interactive-at-windows-tuple/AT-HANDOFF.json",
    ".spec.local/root/grants/interactive-at-windows-tuple/GRANT.json",
];

const UNLOCK_A_CLOSER_REL: &str =
    ".spec.local/root/evidence/20261109020800-cursor-unlock-a-desktop-baseline-realloc/CLOSER.json";

/// Cap conservation after Unlock A EXIT0 — realloc only with structural feeder.
pub fn cap_free_after_unlock_a(root: &Path) -> Result<bool> {
    let path = root.join(UNLOCK_A_CLOSER_REL);
    if !path.is_file() {
        // Without Unlock A EXIT0 closer, Cap conservation is unknown; treat as
        // not free for product Cap burn (fail-closed Cap conservation).
        return Ok(false);
    }
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "Cap free probe refused: Unlock A closer read {}: {error}",
            path.display()
        )
    })?;
    let closer: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "Cap free probe refused: Unlock A closer JSON {}: {error}",
            path.display()
        )
    })?;
    if closer.get("cap_exit").and_then(Value::as_u64) != Some(0) {
        return Ok(false);
    }
    // Explicit Cap free=false after EXIT0 honesty; realloc needs structural delta.
    Ok(closer.get("cap_free").and_then(Value::as_bool) == Some(true))
}

fn missing_g2_product_binding_topology(root: &Path) -> Vec<&'static str> {
    G2_PRODUCT_BINDING_TOPOLOGY
        .iter()
        .copied()
        .filter(|relative| !root.join(relative).is_file())
        .collect()
}

fn validate_g2_component_receipt(receipt: &Value) -> Result<()> {
    if receipt.get("schema").and_then(Value::as_str) != Some("humanymous.cursor-cap1-receipt/1") {
        return Err(
            "G2 collector product-binding readiness refused: component receipt schema mismatch"
                .into(),
        );
    }
    if receipt.get("counts_as").and_then(Value::as_str) != Some("pass_component_desktop") {
        return Err(
            "G2 collector product-binding readiness refused: component must count as pass_component_desktop only"
                .into(),
        );
    }
    if receipt.get("product_still").and_then(Value::as_u64) != Some(0)
        || receipt
            .get("pass_product_clean_room")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err(
            "G2 collector product-binding readiness refused: component receipt invented product"
                .into(),
        );
    }
    if receipt.get("g2_matrix_cell_moved").and_then(Value::as_bool) == Some(true) {
        return Err(
            "G2 collector product-binding readiness refused: component cannot move G2 matrix cell; Cap=0"
                .into(),
        );
    }
    if receipt
        .get("component_hosted_earned")
        .and_then(Value::as_bool)
        == Some(true)
    {
        return Err(
            "G2 collector product-binding readiness refused: Desktop component ≠ hosted; never invent pass_component_hosted"
                .into(),
        );
    }
    Ok(())
}

fn validate_g5_component_summary(summary: &Value) -> Result<()> {
    if summary.get("schema").and_then(Value::as_str)
        != Some("humanymous.product-custody-witness.attempt-summary/1")
    {
        return Err(
            "G5 witness transport honesty refused: custody/witness summary schema mismatch".into(),
        );
    }
    let status = summary.get("status").and_then(Value::as_str).unwrap_or("");
    if status != "pass_component_only" && status != "pass_component_desktop" {
        return Err(format!(
            "G5 witness transport honesty refused: status={status} is not component-only; never invent authenticated transport product"
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
            "G5 witness transport honesty refused: component summary invented product/M3".into(),
        );
    }
    Ok(())
}

/// Cap=0 G2 gate: bind Desktop collector component; refuse product invent;
/// report missing product-binding topology; Cap free conserved.
pub(super) fn run_g2_collector_product_binding_readiness(lab: &mut Lab) -> Result<()> {
    let root = snapshot::root()?;
    let cap_free = cap_free_after_unlock_a(&root)?;
    lab.stage = "g2_collector_product_binding_bind_component";
    let receipt_path = root.join(G2_COMPONENT_RECEIPT_REL);
    if !receipt_path.is_file() {
        lab.components["g2_collector_product_binding_readiness"] = json!({
            "status": "refused",
            "kind": "g2-collector-product-binding-readiness",
            "product": 0,
            "pass_product_clean_room": 0,
            "g2_closed": false,
            "cap": 0,
            "cap_free": cap_free,
            "missing_component_receipt": G2_COMPONENT_RECEIPT_REL,
            "note": "G2 Cap=0 readiness requires Desktop collector component receipt; never invent pass_product_clean_room"
        });
        snapshot::write(
            &lab.evidence,
            "g2-collector-product-binding-readiness-refuse.json",
            &lab.components["g2_collector_product_binding_readiness"],
        )?;
        return Err(
            "G2 collector product-binding readiness refused: component receipt absent; Cap=0; never invent pass_product_clean_room"
                .into(),
        );
    }
    let bytes = fs::read(&receipt_path).map_err(|error| {
        format!(
            "G2 collector product-binding readiness refused: receipt read {}: {error}",
            receipt_path.display()
        )
    })?;
    let receipt: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "G2 collector product-binding readiness refused: receipt JSON {}: {error}",
            receipt_path.display()
        )
    })?;
    validate_g2_component_receipt(&receipt)?;
    let missing_topology = missing_g2_product_binding_topology(&root);
    // Compose files may exist as component surfaces; product-binding still open.
    let product_binding_ready = false;
    let readiness = json!({
        "schema": G2_READINESS_SCHEMA,
        "record_kind": "g2-collector-product-binding-readiness",
        "cap": 0,
        "cap_free": cap_free,
        "cap_burn": false,
        "cap_selector": G2_CAP_SELECTOR,
        "pilot_command": G2_PILOT_COMMAND,
        "product": 0,
        "pass_product_clean_room": 0,
        "g2_closed": false,
        "g2_matrix_cell_moved": false,
        "component_counts_as": "pass_component_desktop",
        "component_hosted_earned": false,
        "component_receipt": G2_COMPONENT_RECEIPT_REL,
        "product_binding_ready": product_binding_ready,
        "missing_product_binding_topology": missing_topology,
        "topology_surfaces_present": G2_PRODUCT_BINDING_TOPOLOGY,
        "cap_realloc_policy": "only_real_structural_next_feeder",
        "no_g2_cap_cli_with_topology": true,
        "note": "Desktop collector Cap1 component ≠ G2 product-binding; Cap=0; Cap free conserved after Unlock A EXIT0; never invent pass_product_clean_room"
    });
    snapshot::write(
        &lab.evidence,
        "g2-collector-product-binding-readiness.json",
        &readiness,
    )?;
    lab.case(
        "g2-collector-component-bound-not-product",
        readiness.get("product") == Some(&json!(0))
            && readiness.get("g2_closed") == Some(&json!(false))
            && readiness.get("product_binding_ready") == Some(&json!(false)),
        json!({
            "counts_as": "pass_component_desktop",
            "product": 0,
            "g2_closed": false
        }),
    )?;
    lab.case(
        "g2-cap-conserved-no-cap1",
        readiness.get("cap") == Some(&json!(0))
            && readiness.get("cap_burn") == Some(&json!(false))
            && readiness.get("cap_free") == Some(&json!(cap_free)),
        json!({
            "cap": 0,
            "cap_free": cap_free,
            "reason": "no claimable G2 Cap CLI+topology; Cap free=false after Unlock A EXIT0; no Cap spam"
        }),
    )?;
    lab.components["g2_collector_product_binding_readiness"] = json!({
        "status": "readiness_landed_g2_open",
        "kind": "g2-collector-product-binding-readiness",
        "product": 0,
        "pass_product_clean_room": 0,
        "g2_closed": false,
        "cap": 0,
        "cap_free": cap_free,
        "counts_as": "pass_component_desktop",
        "note": "Cap=0 G2 honesty gate; component bound; product-binding still Root; Cap free conserved"
    });
    snapshot::write(
        &lab.evidence,
        "g2-collector-product-binding-readiness-component.json",
        &lab.components["g2_collector_product_binding_readiness"],
    )?;
    Ok(())
}

/// Cap=0 G5 gate: bind custody/witness component; refuse authenticated transport invent.
pub(super) fn run_g5_witness_transport_honesty(lab: &mut Lab) -> Result<()> {
    let root = snapshot::root()?;
    let cap_free = cap_free_after_unlock_a(&root)?;
    lab.stage = "g5_witness_transport_bind_component";
    let summary_path = root.join(G5_COMPONENT_SUMMARY_REL);
    if !summary_path.is_file() {
        lab.components["g5_witness_transport_honesty"] = json!({
            "status": "refused",
            "kind": "g5-witness-transport-honesty",
            "product": 0,
            "pass_product_clean_room": 0,
            "authenticated_witness_transport": false,
            "cap": 0,
            "cap_free": cap_free,
            "missing_component_summary": G5_COMPONENT_SUMMARY_REL,
            "root_ask_paths": G5_ROOT_ASK_PATHS,
            "note": "G5 Cap=0 honesty requires custody/witness component summary; never invent authenticated transport product"
        });
        snapshot::write(
            &lab.evidence,
            "g5-witness-transport-honesty-refuse.json",
            &lab.components["g5_witness_transport_honesty"],
        )?;
        return Err(
            "G5 witness transport honesty refused: component summary absent; Cap=0; never invent pass_product_clean_room"
                .into(),
        );
    }
    let bytes = fs::read(&summary_path).map_err(|error| {
        format!(
            "G5 witness transport honesty refused: summary read {}: {error}",
            summary_path.display()
        )
    })?;
    let summary: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "G5 witness transport honesty refused: summary JSON {}: {error}",
            summary_path.display()
        )
    })?;
    validate_g5_component_summary(&summary)?;
    let honesty = json!({
        "schema": G5_HONESTY_SCHEMA,
        "record_kind": "g5-witness-transport-honesty",
        "cap": 0,
        "cap_free": cap_free,
        "cap_burn": false,
        "cap_selector": G5_CAP_SELECTOR,
        "pilot_command": G5_PILOT_COMMAND,
        "product": 0,
        "pass_product_clean_room": 0,
        "authenticated_witness_transport": false,
        "g5_closed": false,
        "component_status": summary.get("status"),
        "component_summary": G5_COMPONENT_SUMMARY_REL,
        "component_test_result": summary.get("test_result"),
        "full_product_qualified": false,
        "m3_claim": false,
        "root_ask_paths": G5_ROOT_ASK_PATHS,
        "cap_realloc_policy": "only_real_structural_next_feeder",
        "no_g5_cap_cli_with_topology": true,
        "note": "custody/witness Cap feeders = pass_component_* only; authenticated witness transport still Root; Cap=0; Cap free conserved; never invent pass_product_clean_room"
    });
    snapshot::write(&lab.evidence, "g5-witness-transport-honesty.json", &honesty)?;
    lab.case(
        "g5-witness-component-bound-not-authenticated-transport",
        honesty.get("product") == Some(&json!(0))
            && honesty.get("authenticated_witness_transport") == Some(&json!(false))
            && honesty.get("g5_closed") == Some(&json!(false)),
        json!({
            "counts_as": "pass_component_desktop",
            "product": 0,
            "authenticated_witness_transport": false
        }),
    )?;
    lab.case(
        "g5-cap-conserved-no-cap1",
        honesty.get("cap") == Some(&json!(0))
            && honesty.get("cap_burn") == Some(&json!(false))
            && honesty.get("cap_free") == Some(&json!(cap_free)),
        json!({
            "cap": 0,
            "cap_free": cap_free,
            "reason": "no claimable G5 Cap CLI+topology; Cap free=false after Unlock A EXIT0; no Cap spam"
        }),
    )?;
    lab.components["g5_witness_transport_honesty"] = json!({
        "status": "honesty_landed_g5_open",
        "kind": "g5-witness-transport-honesty",
        "product": 0,
        "pass_product_clean_room": 0,
        "authenticated_witness_transport": false,
        "g5_closed": false,
        "cap": 0,
        "cap_free": cap_free,
        "root_ask_paths": G5_ROOT_ASK_PATHS,
        "note": "Cap=0 G5 honesty gate; component bound; authenticated transport still Root; Cap free conserved"
    });
    snapshot::write(
        &lab.evidence,
        "g5-witness-transport-honesty-component.json",
        &lab.components["g5_witness_transport_honesty"],
    )?;
    Ok(())
}

/// Validate Unlock A Cap EXIT0 closer for BrowserReady Cap=0 honesty bind.
fn validate_unlock_a_closer_for_browser_ready(closer: &Value) -> Result<()> {
    if closer.get("schema").and_then(Value::as_str) != Some(UNLOCK_A_CLOSER_SCHEMA) {
        return Err(
            "BrowserReady interactive-AT honesty refused: Unlock A Cap EXIT0 closer schema mismatch"
                .into(),
        );
    }
    if closer.get("cap_exit").and_then(Value::as_u64) != Some(0) {
        return Err(
            "BrowserReady interactive-AT honesty refused: Unlock A Cap EXIT0 closer cap_exit != 0"
                .into(),
        );
    }
    if closer.get("cap_free").and_then(Value::as_bool) != Some(false) {
        return Err(
            "BrowserReady interactive-AT honesty refused: Unlock A Cap EXIT0 closer must record cap_free=false"
                .into(),
        );
    }
    if closer.get("browser_ready").and_then(Value::as_bool) != Some(false) {
        return Err(
            "BrowserReady interactive-AT honesty refused: Unlock A Cap EXIT0 closer invented BrowserReady"
                .into(),
        );
    }
    if closer.get("product").and_then(Value::as_u64) != Some(0)
        || closer
            .get("pass_product_clean_room")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err(
            "BrowserReady interactive-AT honesty refused: Unlock A Cap EXIT0 closer invented product/pass_product_clean_room"
                .into(),
        );
    }
    if closer.get("g4_cef_at_component").and_then(Value::as_str) != Some("executed") {
        return Err(
            "BrowserReady interactive-AT honesty refused: Unlock A Cap EXIT0 closer missing g4_cef_at executed"
                .into(),
        );
    }
    Ok(())
}

/// Locate a Root-authored interactive AT Windows tuple handoff (never invent).
pub fn find_interactive_at_handoff(root: &Path) -> Option<PathBuf> {
    INTERACTIVE_AT_HANDOFF_CANDIDATES
        .iter()
        .map(|relative| root.join(relative))
        .find(|path| path.is_file())
}

/// Fail-closed Root interactive AT handoff — Cap=0 honesty only when absent.
pub fn require_interactive_at_handoff(root: &Path) -> Result<Value> {
    let Some(path) = find_interactive_at_handoff(root) else {
        return Err(format!(
            "BrowserReady Cap refused: Root interactive AT Windows tuple absent; Cap=0 honesty only; exact Root ask paths ({paths}); never invent AT tokens or BrowserReady",
            paths = BROWSER_READY_ROOT_ASK_PATHS.join(", ")
        )
        .into());
    };
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "BrowserReady Cap refused: interactive AT handoff read {}: {error}",
            path.display()
        )
    })?;
    let handoff: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "BrowserReady Cap refused: interactive AT handoff JSON {}: {error}",
            path.display()
        )
    })?;
    if handoff.get("schema").and_then(Value::as_str) != Some(INTERACTIVE_AT_HANDOFF_SCHEMA) {
        return Err(
            "BrowserReady Cap refused: interactive AT handoff schema mismatch; never invent Root AT tokens"
                .into(),
        );
    }
    if handoff
        .get("browser_ready_authorized")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err(
            "BrowserReady Cap refused: interactive AT handoff does not authorize BrowserReady; Cap=0"
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
            "BrowserReady Cap refused: interactive AT handoff invented product/pass_product_clean_room"
                .into(),
        );
    }
    Ok(handoff)
}

/// Cap=0 BrowserReady gate: bind Unlock A g4 component; refuse AT invent;
/// report missing Root interactive AT Windows tuple; Cap free conserved.
pub(super) fn run_browser_ready_interactive_at_honesty(lab: &mut Lab) -> Result<()> {
    let root = snapshot::root()?;
    let cap_free = cap_free_after_unlock_a(&root)?;
    lab.stage = "browser_ready_bind_unlock_a_component";
    let closer_path = root.join(UNLOCK_A_CLOSER_REL);
    if !closer_path.is_file() {
        lab.components["browser_ready_interactive_at_honesty"] = json!({
            "status": "refused",
            "kind": "browser-ready-interactive-at-honesty",
            "product": 0,
            "pass_product_clean_room": 0,
            "browser_ready": false,
            "cap": 0,
            "cap_free": cap_free,
            "missing_unlock_a_closer": UNLOCK_A_CLOSER_REL,
            "root_ask_paths": BROWSER_READY_ROOT_ASK_PATHS,
            "note": "BrowserReady Cap=0 honesty requires Unlock A Cap EXIT0 closer; never invent AT or BrowserReady"
        });
        snapshot::write(
            &lab.evidence,
            "browser-ready-interactive-at-honesty-refuse.json",
            &lab.components["browser_ready_interactive_at_honesty"],
        )?;
        return Err(
            "BrowserReady interactive-AT honesty refused: Unlock A Cap EXIT0 closer absent; Cap=0; never invent BrowserReady or Root AT tokens"
                .into(),
        );
    }
    let bytes = fs::read(&closer_path).map_err(|error| {
        format!(
            "BrowserReady interactive-AT honesty refused: Unlock A closer read {}: {error}",
            closer_path.display()
        )
    })?;
    let closer: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "BrowserReady interactive-AT honesty refused: Unlock A closer JSON {}: {error}",
            closer_path.display()
        )
    })?;
    validate_unlock_a_closer_for_browser_ready(&closer)?;

    lab.stage = "browser_ready_interactive_at_handoff";
    let auth = require_interactive_at_handoff(&root);
    let auth_present = auth.is_ok();
    if let Err(error) = &auth {
        snapshot::write(
            &lab.evidence,
            "browser-ready-interactive-at-handoff-refuse.json",
            &json!({
                "schema": "humanymous.lab-browser-ready-interactive-at-handoff-refuse/1",
                "cap": 0,
                "cap_free": cap_free,
                "product": 0,
                "pass_product_clean_room": 0,
                "browser_ready": false,
                "unlock_a_g4_cef_at_component": "executed",
                "interactive_at_present": false,
                "root_ask_paths": BROWSER_READY_ROOT_ASK_PATHS,
                "handoff_candidates": INTERACTIVE_AT_HANDOFF_CANDIDATES,
                "detail": error.to_string(),
                "note": "g4_cef_at executed ≠ BrowserReady; Root interactive AT Windows tuple still required"
            }),
        )?;
    } else if let Ok(handoff) = &auth {
        snapshot::write(
            &lab.evidence,
            "browser-ready-interactive-at-handoff.json",
            handoff,
        )?;
    }

    // Cap=0 honesty always records browser_ready=false until Root AT is present
    // AND a separate measured BrowserReady join is proven (never invent here).
    let browser_ready = false;
    let honesty = json!({
        "schema": BROWSER_READY_HONESTY_SCHEMA,
        "record_kind": "browser-ready-interactive-at-honesty",
        "cap": 0,
        "cap_free": cap_free,
        "cap_burn": false,
        "cap_selector": BROWSER_READY_CAP_SELECTOR,
        "pilot_command": BROWSER_READY_PILOT_COMMAND,
        "product": 0,
        "pass_product_clean_room": 0,
        "browser_ready": browser_ready,
        "unlock_a_cap_exit": 0,
        "unlock_a_cap_free": false,
        "unlock_a_g4_cef_at_component": "executed",
        "unlock_a_counts_as": "pass_component_g4_cef_at_executed",
        "unlock_a_closer": UNLOCK_A_CLOSER_REL,
        "interactive_at_present": auth_present,
        "root_ask_paths": BROWSER_READY_ROOT_ASK_PATHS,
        "handoff_candidates": INTERACTIVE_AT_HANDOFF_CANDIDATES,
        "cap_realloc_policy": "only_real_structural_next_feeder",
        "no_browser_ready_cap1_without_root_at": true,
        "note": "Unlock A g4_cef_at component ≠ BrowserReady; Cap=0; Cap free conserved; Root interactive AT Windows tuple still required; never invent AT tokens or pass_product_clean_room"
    });
    snapshot::write(
        &lab.evidence,
        "browser-ready-interactive-at-honesty.json",
        &honesty,
    )?;
    lab.case(
        "browser-ready-unlock-a-component-bound-not-browser-ready",
        honesty.get("product") == Some(&json!(0))
            && honesty.get("browser_ready") == Some(&json!(false))
            && honesty.get("unlock_a_g4_cef_at_component") == Some(&json!("executed")),
        json!({
            "counts_as": "pass_component_g4_cef_at_executed",
            "product": 0,
            "browser_ready": false
        }),
    )?;
    lab.case(
        "browser-ready-cap-conserved-no-cap1",
        honesty.get("cap") == Some(&json!(0))
            && honesty.get("cap_burn") == Some(&json!(false))
            && honesty.get("cap_free") == Some(&json!(cap_free))
            && !auth_present,
        json!({
            "cap": 0,
            "cap_free": cap_free,
            "interactive_at_present": auth_present,
            "reason": "Root interactive AT Windows tuple absent; Cap free=false after Unlock A EXIT0; no Cap spam"
        }),
    )?;
    lab.case(
        "browser-ready-interactive-at-refuse-closed",
        !auth_present && honesty.get("browser_ready") == Some(&json!(false)),
        json!({
            "interactive_at_present": false,
            "browser_ready": false,
            "root_ask_paths": BROWSER_READY_ROOT_ASK_PATHS
        }),
    )?;
    lab.components["browser_ready_interactive_at_honesty"] = json!({
        "status": "honesty_landed_browser_ready_open",
        "kind": "browser-ready-interactive-at-honesty",
        "product": 0,
        "pass_product_clean_room": 0,
        "browser_ready": false,
        "interactive_at_present": auth_present,
        "cap": 0,
        "cap_free": cap_free,
        "root_ask_paths": BROWSER_READY_ROOT_ASK_PATHS,
        "note": "Cap=0 BrowserReady honesty gate; Unlock A component bound; interactive AT still Root; Cap free conserved"
    });
    snapshot::write(
        &lab.evidence,
        "browser-ready-interactive-at-honesty-component.json",
        &lab.components["browser_ready_interactive_at_honesty"],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn g2_receipt_accepts_desktop_component_only() {
        let receipt = json!({
            "schema": "humanymous.cursor-cap1-receipt/1",
            "counts_as": "pass_component_desktop",
            "product_still": 0,
            "pass_product_clean_room": 0,
            "g2_matrix_cell_moved": false,
            "component_hosted_earned": false
        });
        assert!(validate_g2_component_receipt(&receipt).is_ok());
    }

    #[test]
    fn g2_receipt_refuses_product_invent() {
        let receipt = json!({
            "schema": "humanymous.cursor-cap1-receipt/1",
            "counts_as": "pass_component_desktop",
            "product_still": 1,
            "pass_product_clean_room": 0,
            "g2_matrix_cell_moved": false,
            "component_hosted_earned": false
        });
        assert!(validate_g2_component_receipt(&receipt).is_err());
    }

    #[test]
    fn g5_summary_accepts_component_only() {
        let summary = json!({
            "schema": "humanymous.product-custody-witness.attempt-summary/1",
            "status": "pass_component_only",
            "full_product_qualified": false,
            "m3_claim": false
        });
        assert!(validate_g5_component_summary(&summary).is_ok());
    }

    #[test]
    fn g5_summary_refuses_product_invent() {
        let summary = json!({
            "schema": "humanymous.product-custody-witness.attempt-summary/1",
            "status": "pass_component_only",
            "full_product_qualified": true,
            "m3_claim": false
        });
        assert!(validate_g5_component_summary(&summary).is_err());
    }

    #[test]
    fn browser_ready_unlock_a_closer_accepts_component_only() {
        let closer = json!({
            "schema": UNLOCK_A_CLOSER_SCHEMA,
            "cap_exit": 0,
            "cap_free": false,
            "browser_ready": false,
            "product": 0,
            "pass_product_clean_room": 0,
            "g4_cef_at_component": "executed"
        });
        assert!(validate_unlock_a_closer_for_browser_ready(&closer).is_ok());
    }

    #[test]
    fn browser_ready_unlock_a_closer_refuses_browser_ready_invent() {
        let closer = json!({
            "schema": UNLOCK_A_CLOSER_SCHEMA,
            "cap_exit": 0,
            "cap_free": false,
            "browser_ready": true,
            "product": 0,
            "pass_product_clean_room": 0,
            "g4_cef_at_component": "executed"
        });
        assert!(validate_unlock_a_closer_for_browser_ready(&closer).is_err());
    }

    #[test]
    fn interactive_at_handoff_refuses_without_root_token() {
        let root = snapshot::root().expect("workspace root");
        assert!(find_interactive_at_handoff(&root).is_none());
        let err = require_interactive_at_handoff(&root).expect_err("no invent");
        assert!(err.to_string().contains("Root interactive AT"));
    }
}
