//! Cap=0 until-complete status cutter.
//!
//! Measures the ten incompletes that block PRODUCT COMPLETION (including
//! Docker-only e2e), plus Cap free, `ready_for_cap0_chain`, and `ready_for_cap1`,
//! as machine-readable JSON. Distinct from `cap0-chain --inspect-only` (which
//! only covers handoff/GRANT presence+schema). Never invents Cap free /
//! BrowserReady / AUTH / AT / pass_product / Cap burn / goal_complete /
//! docker_e2e_validate_product.
//!
//! Definition of Done (DoD; all measured; never invent): currently implemented
//! automation-detection tech is correctly evaluated on this deployment via
//! red-blue-validate product + clean-room Docker e2e
//! (`e2e-docker-launch` / lab-runner product path), with
//! `pass_product_clean_room=true`, Docker clean-room full matrix `--require-full`
//! green, and BrowserReady only when measured true. Host/loopback, host gate
//! e2e (`node test/gate/e2e.mjs`), component Cap EXIT0, AffinityOnly/PressureOnly,
//! SkipSwarm-only, red-blue/matrix without `--require-full`, and Docker e2e
//! product oneshots without mutex+pressure ≠ DoD. Detection freeze:
//! do not change scoring weights/thresholds/hard-rules; honesty proves existing
//! SoT detection correctly evaluates implementable automation (bots/catalog).
use crate::cap_realloc_apply;
use crate::docker::Result;
use crate::handoff_apply;
use crate::orchestration::cap0_product_honesty::{
    INTERACTIVE_AT_HANDOFF_CANDIDATES, cap_free_after_unlock_a,
};
use crate::orchestration::h2_product_binding::N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES;
use crate::snapshot;
use crate::unlock_gate;
use serde_json::{Value, json};
use std::{fs, path::Path};

const SCHEMA: &str = "humanymous.lab-until-complete-status/1";
const PILOT_COMMAND: &str = "cargo run --locked -p humanymous-lab-runner -- until-complete-status [--workspace PATH] [--emit PATH]";
const INTERACTIVE_AT_HANDOFF_SCHEMA: &str = "humanymous.root-interactive-at-windows-tuple/1";
const N06_OWNER_JOIN_AUTH_HANDOFF_SCHEMA: &str = "humanymous.root-n06-owner-join-auth-handoff/1";
const GRANT_SCHEMA: &str = "humanymous.root-cap-free-restore-grant/1";

/// Cap-free-restore LIVE GRANT candidates (never `*.TEMPLATE.json` / candidate-pack scaffold).
const CAP_FREE_GRANT_CANDIDATES: &[&str] = &[
    ".spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/GRANT.json",
    ".spec.local/root/grants/cap-free-restore/GRANT.json",
    "deployments/lab/product/GRANT.json",
];

/// Cap-free-restore LIVE TRANSFER manifest candidates (never Cap0 candidate-pack scaffold).
const CAP_FREE_MANIFEST_CANDIDATES: &[&str] = &[
    ".spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/transfer-manifest.json",
    ".spec.local/root/grants/cap-free-restore/transfer-manifest.json",
    "deployments/lab/product/transfer-manifest.json",
];

/// Cap0 Cap-ready scaffold only — present ≠ LIVE GRANT / LIVE TRANSFER for Cap free restore.
const CAP_FREE_CANDIDATE_PACK_GRANT: &str = ".spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/cap-ready/cap-free-restore/candidate-pack/GRANT.json";
const CAP_FREE_CANDIDATE_PACK_MANIFEST: &str = ".spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/cap-ready/cap-free-restore/candidate-pack/transfer-manifest.json";

const TEMPLATE_GRANT_MARKERS: &[&str] = &[
    ".spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/GRANT.TEMPLATE.json",
    ".spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/cap-ready/cap-free-restore/candidate-pack/GRANT.TEMPLATE.json",
];

/// Measured Cap1/product receipts only — scaffolds and component holds never count.
const PASS_PRODUCT_RECEIPT_CANDIDATES: &[&str] = &[
    ".spec.local/lab/product/pass-product-clean-room.json",
    ".spec.local/lab/product/clean-room-product-measure-receipt.json",
];

const MATRIX_GREEN_RECEIPT_CANDIDATES: &[&str] = &[
    ".spec.local/lab/product/clean-room-matrix-green.json",
    ".spec.local/lab/product/clean-room-product-matrix-measure-receipt.json",
];

const RED_BLUE_PRODUCT_RECEIPT_CANDIDATES: &[&str] = &[
    ".spec.local/lab/product/red-blue-validate-product.json",
    ".spec.local/lab/product/red-blue-product-measure-receipt.json",
];

/// Docker-only e2e / red-blue-validate product receipts (host/loopback ≠ completion).
const DOCKER_E2E_PRODUCT_RECEIPT_CANDIDATES: &[&str] = &[
    ".spec.local/lab/product/docker-e2e-validate-product.json",
    ".spec.local/lab/product/docker-e2e-product-measure-receipt.json",
];

const BROWSER_READY_RECEIPT_CANDIDATES: &[&str] = &[
    ".spec.local/lab/product/browser-ready.json",
    ".spec.local/lab/product/browser-ready-measure-receipt.json",
];

/// Cap1 Docker e2e launcher preimages (Windows Desktop mutex+pressure guard).
const DOCKER_E2E_LAUNCHER_CANDIDATES: &[&str] = &[
    "scripts/e2e-docker-launch.ps1",
    "scripts/e2e-docker.sh",
    "scripts/e2e-docker.ps1",
];

/// Cap1 serial Docker e2e / clean-room / matrix topology honesty (present ≠ product green).
const DOCKER_E2E_TOPOLOGY_CANDIDATES: &[&str] = &[
    "deployments/lab/compose.clean-room-product.yaml",
    "build/rust/clean-room-product-lab.Dockerfile",
    "deployments/lab/compose.clean-room-product-matrix.yaml",
    "build/rust/clean-room-product-matrix-lab.Dockerfile",
    "deployments/lab/compose.red-blue-product.yaml",
    "build/rust/red-blue-product-lab.Dockerfile",
    "deployments/compose.yaml",
];

/// Structural land targets for apps/humanymous-browser incomplete.
const APPS_BROWSER_STRUCTURAL: &[&str] = &[
    "crates/apps/humanymous-browser/Cargo.toml",
    "apps/humanymous-browser/Cargo.toml",
    "crates/apps/humanymous-browser",
    "apps/humanymous-browser",
];

/// Cap=0 until-complete status (measure only; never invent product gates).
///
/// Usage: `until-complete-status [--workspace PATH] [--emit PATH]`
///
/// Exit 0 only when `goal_complete=true` (all ten incompletes closed with
/// measured evidence, including Docker-only e2e). Exit 1 when any incomplete
/// remains (fail-closed).
pub fn run(args: &[String]) -> Result<i32> {
    let mut workspace = None;
    let mut emit_path = None;
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
            _ => {
                return Err(
                    "usage: until-complete-status [--workspace PATH] [--emit PATH]; Cap0 measure; never invent Cap free / BrowserReady / pass_product / goal_complete"
                        .into(),
                );
            }
        }
        index += 1;
    }

    let root = match workspace {
        Some(path) => Path::new(&path).canonicalize().map_err(|error| {
            format!("until-complete-status refused: workspace canonicalize {path}: {error}")
        })?,
        None => snapshot::root()?,
    };

    let report = measure(&root)?;
    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("until-complete-status encode failed: {error}"))?;
    if let Some(path) = emit_path {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).map_err(|error| format!("emit mkdir: {error}"))?;
        }
        fs::write(&path, format!("{rendered}\n"))
            .map_err(|error| format!("emit write failed: {error}"))?;
    }
    println!("{rendered}");

    let goal_complete = report.get("goal_complete").and_then(Value::as_bool) == Some(true);
    if goal_complete {
        Ok(0)
    } else {
        // Fail-closed: incompletes remain or product gates unverified.
        Ok(1)
    }
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

fn load_json(root: &Path, relative: &str) -> Result<Value> {
    let path = root.join(relative);
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "until-complete-status refused: read {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "until-complete-status refused: JSON {}: {error}",
            path.display()
        )
        .into()
    })
}

fn artifact_handoff(
    root: &Path,
    id: &str,
    schema: &str,
    candidates: &[&str],
    validate: fn(&Value) -> Result<()>,
) -> Value {
    match first_live_file(root, candidates) {
        Some(relative) => match load_json(root, &relative) {
            Ok(value) => match validate(&value) {
                Ok(()) => json!({
                    "id": id,
                    "klass": "Root_blocked",
                    "present": true,
                    "schema_ok": true,
                    "closed": true,
                    "path": relative,
                    "expected_schema": schema
                }),
                Err(error) => json!({
                    "id": id,
                    "klass": "Root_blocked",
                    "present": true,
                    "schema_ok": false,
                    "closed": false,
                    "path": relative,
                    "expected_schema": schema,
                    "error": error.to_string()
                }),
            },
            Err(error) => json!({
                "id": id,
                "klass": "Root_blocked",
                "present": true,
                "schema_ok": false,
                "closed": false,
                "path": relative,
                "expected_schema": schema,
                "error": error.to_string()
            }),
        },
        None => json!({
            "id": id,
            "klass": "Root_blocked",
            "present": false,
            "schema_ok": false,
            "closed": false,
            "path": Value::Null,
            "expected_schema": schema,
            "candidates": candidates
        }),
    }
}

fn grant_status(root: &Path) -> Value {
    match first_live_file(root, CAP_FREE_GRANT_CANDIDATES) {
        Some(relative) if is_template_path(Path::new(&relative)) => json!({
            "id": "Cap-free-restore-GRANT",
            "klass": "Root_blocked",
            "present": true,
            "schema_ok": false,
            "closed": false,
            "path": relative,
            "error": "GRANT path is TEMPLATE; never invent Cap free"
        }),
        Some(relative) => match load_json(root, &relative) {
            Ok(value) => match cap_realloc_apply::validate_grant(&value) {
                Ok(()) => json!({
                    "id": "Cap-free-restore-GRANT",
                    "klass": "Root_blocked",
                    "present": true,
                    "schema_ok": true,
                    "closed": true,
                    "path": relative,
                    "expected_schema": GRANT_SCHEMA
                }),
                Err(error) => json!({
                    "id": "Cap-free-restore-GRANT",
                    "klass": "Root_blocked",
                    "present": true,
                    "schema_ok": false,
                    "closed": false,
                    "path": relative,
                    "expected_schema": GRANT_SCHEMA,
                    "error": error.to_string()
                }),
            },
            Err(error) => json!({
                "id": "Cap-free-restore-GRANT",
                "klass": "Root_blocked",
                "present": true,
                "schema_ok": false,
                "closed": false,
                "path": relative,
                "error": error.to_string()
            }),
        },
        None => {
            let template_only = TEMPLATE_GRANT_MARKERS
                .iter()
                .find(|relative| root.join(relative).is_file())
                .map(|relative| (*relative).to_string());
            json!({
                "id": "Cap-free-restore-GRANT",
                "klass": "Root_blocked",
                "present": false,
                "schema_ok": false,
                "closed": false,
                "path": Value::Null,
                "template_only": template_only,
                "expected_schema": GRANT_SCHEMA,
                "candidates": CAP_FREE_GRANT_CANDIDATES
            })
        }
    }
}

fn transfer_present(root: &Path) -> bool {
    first_live_file(root, CAP_FREE_MANIFEST_CANDIDATES).is_some()
}

fn candidate_pack_grant_present(root: &Path) -> bool {
    root.join(CAP_FREE_CANDIDATE_PACK_GRANT).is_file()
}

fn candidate_pack_transfer_present(root: &Path) -> bool {
    root.join(CAP_FREE_CANDIDATE_PACK_MANIFEST).is_file()
}

/// Fail-closed: only accept measured product=1 receipts that are not component-only.
fn measured_pass_product(root: &Path) -> (u64, Option<String>) {
    for relative in PASS_PRODUCT_RECEIPT_CANDIDATES {
        let path = root.join(relative);
        if !path.is_file() {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        if value.get("pass_product_clean_room").and_then(Value::as_u64) != Some(1) {
            continue;
        }
        if value.get("product").and_then(Value::as_u64) != Some(1) {
            continue;
        }
        // Component-only scaffolds never close the incomplete.
        let counts = value.get("counts_as").and_then(Value::as_str).unwrap_or("");
        if counts.contains("component") {
            continue;
        }
        return (1, Some((*relative).to_string()));
    }
    (0, None)
}

fn measured_bool_receipt(root: &Path, candidates: &[&str], field: &str) -> (bool, Option<String>) {
    for relative in candidates {
        let path = root.join(relative);
        if !path.is_file() {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        if value.get(field).and_then(Value::as_bool) == Some(true) {
            // Refuse component scaffolds that invent green without product.
            if value.get("pass_product_clean_room").and_then(Value::as_u64) == Some(0)
                && value.get("product").and_then(Value::as_u64) == Some(0)
                && field != "browser_ready"
            {
                continue;
            }
            if field == "browser_ready" {
                // BrowserReady must be explicit true and not a Cap0 honesty refuse.
                if value.get("browser_ready").and_then(Value::as_bool) != Some(true) {
                    continue;
                }
            }
            return (true, Some((*relative).to_string()));
        }
        if field == "green" && value.get("green").and_then(Value::as_bool) == Some(true) {
            return (true, Some((*relative).to_string()));
        }
        if field == "matrix_green"
            && value.get("matrix_green").and_then(Value::as_bool) == Some(true)
        {
            return (true, Some((*relative).to_string()));
        }
    }
    (false, None)
}

fn measured_matrix_green(root: &Path) -> (bool, Option<String>) {
    for relative in MATRIX_GREEN_RECEIPT_CANDIDATES {
        let path = root.join(relative);
        if !path.is_file() {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        let green = value.get("green").and_then(Value::as_bool) == Some(true)
            || value.get("matrix_green").and_then(Value::as_bool) == Some(true)
            || value
                .get("docker_clean_room_full_matrix_green")
                .and_then(Value::as_bool)
                == Some(true);
        if !green {
            continue;
        }
        // Fail-closed: product must be measured, not component scaffold.
        if value.get("product").and_then(Value::as_u64) != Some(1)
            && value.get("pass_product_clean_room").and_then(Value::as_u64) != Some(1)
        {
            continue;
        }
        // DoD matrix gate requires `--require-full`; subset/partial ≠ DoD.
        if receipt_is_without_require_full_not_dod(&value) {
            continue;
        }
        return (true, Some((*relative).to_string()));
    }
    (false, None)
}

fn measured_red_blue_product(root: &Path) -> (bool, Option<String>) {
    for relative in RED_BLUE_PRODUCT_RECEIPT_CANDIDATES {
        let path = root.join(relative);
        if !path.is_file() {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        let green = value.get("green").and_then(Value::as_bool) == Some(true)
            || value
                .get("red_blue_validate_product")
                .and_then(Value::as_bool)
                == Some(true)
            || value.get("product").and_then(Value::as_u64) == Some(1);
        if !green {
            continue;
        }
        if value.get("product").and_then(Value::as_u64) != Some(1)
            && value.get("pass_product_clean_room").and_then(Value::as_u64) != Some(1)
        {
            continue;
        }
        let counts = value.get("counts_as").and_then(Value::as_str).unwrap_or("");
        if counts.contains("component") {
            continue;
        }
        // Host/loopback node runners (test/e2e/runner.mjs, test/gate/e2e.mjs) ≠ DoD.
        if receipt_is_host_loopback_or_gate_e2e_not_dod(&value) {
            continue;
        }
        // SkipSwarm-only catalog/subset ≠ DoD detection-eval proof.
        if receipt_is_skip_swarm_only_not_dod(&value) {
            continue;
        }
        // DoD red-blue proof requires `--require-full`; partial catalog ≠ DoD.
        if receipt_is_without_require_full_not_dod(&value) {
            continue;
        }
        return (true, Some((*relative).to_string()));
    }
    (false, None)
}

/// Fail-closed: DoD matrix / red-blue gates require `--require-full` on measured
/// product receipts. Partial catalog/subset or explicit `require_full: false` never
/// closes detection-eval (detection freeze; no scoring change).
fn receipt_is_without_require_full_not_dod(value: &Value) -> bool {
    if value
        .get("require_full_without_dod")
        .and_then(Value::as_bool)
        == Some(true)
        || value
            .get("catalog_subset")
            .and_then(Value::as_bool)
            == Some(true)
        || value
            .get("partial_catalog")
            .and_then(Value::as_bool)
            == Some(true)
    {
        return true;
    }
    if value.get("require_full").and_then(Value::as_bool) == Some(false) {
        return true;
    }
    let counts = value.get("counts_as").and_then(Value::as_str).unwrap_or("");
    let counts_l = counts.to_ascii_lowercase();
    if counts_l.contains("without_require_full")
        || counts_l.contains("require_full_false")
        || counts_l.contains("subset_not_full")
        || counts_l.contains("partial_catalog")
        || counts_l.contains("catalog_subset")
    {
        return true;
    }
    let selector = value
        .get("cap_selector")
        .or_else(|| value.get("selector"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let selector_l = selector.to_ascii_lowercase();
    if selector_l.contains("red-blue") && !selector_l.contains("require-full") && !selector.is_empty()
    {
        return true;
    }
    if selector_l.contains("clean-room-product-matrix")
        && !selector_l.contains("require-full")
        && !selector.is_empty()
    {
        return true;
    }
    let claims_dod_green = value.get("green").and_then(Value::as_bool) == Some(true)
        || value
            .get("red_blue_validate_product")
            .and_then(Value::as_bool)
            == Some(true)
        || value
            .get("docker_clean_room_full_matrix_green")
            .and_then(Value::as_bool)
            == Some(true)
        || value.get("matrix_green").and_then(Value::as_bool) == Some(true);
    let product_measured = value.get("product").and_then(Value::as_u64) == Some(1)
        || value.get("pass_product_clean_room").and_then(Value::as_u64) == Some(1);
    if claims_dod_green && product_measured {
        return value.get("require_full").and_then(Value::as_bool) != Some(true);
    }
    false
}

/// Fail-closed: Cap1 Docker e2e product oneshot requires single-flight mutex +
/// pressure guard (Windows `e2e-docker-launch.ps1` path). Missing/false never
/// closes `docker_e2e_validate_product`.
fn docker_e2e_receipt_lacks_mutex_pressure_not_product(value: &Value) -> bool {
    if value
        .get("single_flight_mutex_required")
        .and_then(Value::as_bool)
        == Some(false)
        || value
            .get("pressure_guard_required")
            .and_then(Value::as_bool)
            == Some(false)
    {
        return true;
    }
    let counts = value.get("counts_as").and_then(Value::as_str).unwrap_or("");
    let counts_l = counts.to_ascii_lowercase();
    if counts_l.contains("without_mutex")
        || counts_l.contains("without_pressure")
        || counts_l.contains("alternate_launcher_only")
        || counts_l.contains("make_e2e_only")
    {
        return true;
    }
    let launcher = value
        .get("launcher")
        .or_else(|| value.get("preferred_launcher"))
        .or_else(|| value.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let launcher_l = launcher.to_ascii_lowercase();
    if (launcher_l.contains("make e2e")
        || launcher_l.contains("e2e-docker.sh")
        || launcher_l.contains("e2e-docker.ps1"))
        && value.get("single_flight_mutex_passed").and_then(Value::as_bool) != Some(true)
    {
        return true;
    }
    let claims_product = value.get("green").and_then(Value::as_bool) == Some(true)
        || value
            .get("docker_e2e_validate_product")
            .and_then(Value::as_bool)
            == Some(true)
        || value.get("docker_e2e_product").and_then(Value::as_bool) == Some(true);
    let product_measured = value.get("product").and_then(Value::as_u64) == Some(1)
        || value.get("pass_product_clean_room").and_then(Value::as_u64) == Some(1);
    if claims_product && product_measured {
        let mutex_ok = value.get("single_flight_mutex_passed").and_then(Value::as_bool)
            == Some(true)
            || value
                .get("single_flight_mutex_executed")
                .and_then(Value::as_bool)
                == Some(true);
        let pressure_ok = value.get("pressure_guard_passed").and_then(Value::as_bool) == Some(true)
            || value
                .get("pressure_guard_executed")
                .and_then(Value::as_bool)
                == Some(true);
        return !(mutex_ok && pressure_ok);
    }
    false
}

/// True when a receipt is host/loopback or host gate-e2e (or marks itself as
/// such). Host `node test/e2e/runner.mjs` / `node test/gate/e2e.mjs` never close
/// DoD detection-eval — Docker-only e2e / `--red-blue-product --require-full`
/// is required (rule 60 / test AGENTS.md).
fn receipt_is_host_loopback_or_gate_e2e_not_dod(value: &Value) -> bool {
    if value
        .get("host_loopback_ne_product")
        .and_then(Value::as_bool)
        == Some(false)
        || value.get("host_loopback_ne_dod").and_then(Value::as_bool) == Some(false)
        || value.get("host_gate_e2e_ne_dod").and_then(Value::as_bool) == Some(false)
    {
        return true;
    }
    let counts = value.get("counts_as").and_then(Value::as_str).unwrap_or("");
    let counts_l = counts.to_ascii_lowercase();
    if counts_l.contains("host")
        || counts_l.contains("loopback")
        || counts_l.contains("gate_e2e")
        || counts_l.contains("gate-e2e")
        || counts_l.contains("gate/e2e")
    {
        return true;
    }
    let authority = value
        .get("completion_authority")
        .and_then(Value::as_str)
        .unwrap_or("");
    let authority_l = authority.to_ascii_lowercase();
    if authority_l.contains("host")
        || authority_l.contains("loopback")
        || authority_l.contains("gate_e2e")
        || authority_l.contains("gate-e2e")
        || authority_l.contains("gate/e2e")
        || authority_l.contains("runner.mjs")
        || authority_l.contains("e2e.mjs")
    {
        return true;
    }
    for key in [
        "runner",
        "command",
        "pilot_command",
        "launcher",
        "mode",
        "probe_mode",
        "runner_mode",
    ] {
        let raw = value.get(key).and_then(Value::as_str).unwrap_or("");
        let raw_l = raw.to_ascii_lowercase();
        if raw_l.contains("test/e2e/runner.mjs")
            || raw_l.contains("test/gate/e2e.mjs")
            || raw_l.contains("node test/e2e")
            || raw_l.contains("node test/gate")
            || (raw_l.contains("host") && raw_l.contains("e2e"))
            || raw_l.contains("host_loopback")
            || raw_l.contains("host-loopback")
        {
            return true;
        }
    }
    false
}

/// True when a receipt is SkipSwarm-only (or marks itself as such). SkipSwarm
/// catalog/subset EXIT0 never closes DoD detection-eval — full Docker e2e /
/// `--red-blue-product --require-full` is required.
fn receipt_is_skip_swarm_only_not_dod(value: &Value) -> bool {
    if value.get("skip_swarm_only_ne_dod").and_then(Value::as_bool) == Some(true)
        && value.get("skip_swarm_only").and_then(Value::as_bool) == Some(true)
    {
        return true;
    }
    if value.get("skip_swarm_only").and_then(Value::as_bool) == Some(true)
        || value.get("skip_swarm").and_then(Value::as_bool) == Some(true)
    {
        return true;
    }
    let mode = value
        .get("mode")
        .or_else(|| value.get("probe_mode"))
        .or_else(|| value.get("runner_mode"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let mode_l = mode.to_ascii_lowercase();
    if mode_l.contains("skipswarm")
        || mode_l.contains("skip_swarm")
        || mode_l == "skip-swarm"
        || mode_l.contains("swarm_only")
    {
        return true;
    }
    let counts = value.get("counts_as").and_then(Value::as_str).unwrap_or("");
    let counts_l = counts.to_ascii_lowercase();
    if counts_l.contains("skipswarm")
        || counts_l.contains("skip_swarm")
        || counts_l.contains("skip-swarm")
        || counts_l.contains("swarm_only_not_product")
        || counts_l.contains("swarm_only_ne_dod")
    {
        return true;
    }
    let authority = value
        .get("completion_authority")
        .and_then(Value::as_str)
        .unwrap_or("");
    let authority_l = authority.to_ascii_lowercase();
    authority_l.contains("skipswarm")
        || authority_l.contains("skip_swarm")
        || authority_l.contains("swarm_only")
}

/// True when a receipt is Cap0 AffinityOnly / PressureOnly / mutex / pressure
/// probe (or marks itself as such). Those EXIT0 probes never close Docker e2e
/// product — only Cap1 full `e2e-docker-launch.ps1` / red-blue product can.
fn docker_e2e_receipt_is_cap0_probe_not_product(value: &Value) -> bool {
    if value.get("cap0_probe_ne_product").and_then(Value::as_bool) == Some(true)
        || value
            .get("affinity_pressure_ne_product")
            .and_then(Value::as_bool)
            == Some(true)
        || value
            .get("cap0_affinity_pressure_ne_product")
            .and_then(Value::as_bool)
            == Some(true)
    {
        return true;
    }
    let mode = value
        .get("mode")
        .or_else(|| value.get("probe_mode"))
        .or_else(|| value.get("launcher_mode"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let mode_l = mode.to_ascii_lowercase();
    if mode_l.contains("affinityonly")
        || mode_l.contains("pressureonly")
        || mode_l == "affinity_only"
        || mode_l == "pressure_only"
        || mode_l.contains("mutex_only")
        || mode_l.contains("pressure_guard_only")
    {
        return true;
    }
    let schema = value.get("schema").and_then(Value::as_str).unwrap_or("");
    if schema.contains("pressure-guard") || schema.contains("launcher-cap0-probe") {
        return true;
    }
    let counts = value.get("counts_as").and_then(Value::as_str).unwrap_or("");
    let counts_l = counts.to_ascii_lowercase();
    if counts_l.contains("affinity")
        || counts_l.contains("pressure")
        || counts_l.contains("mutex")
        || counts_l.contains("cap0_probe")
        || counts_l.contains("probe_not_product")
    {
        return true;
    }
    let authority = value
        .get("completion_authority")
        .and_then(Value::as_str)
        .unwrap_or("");
    let authority_l = authority.to_ascii_lowercase();
    authority_l.contains("affinity")
        || authority_l.contains("pressure")
        || authority_l.contains("mutex_only")
        || authority_l.contains("cap0_probe")
}

/// Fail-closed Docker-only e2e product measure. Host/loopback runners and Cap0
/// AffinityOnly/PressureOnly/mutex/pressure probes never close.
fn measured_docker_e2e_product(root: &Path) -> (bool, Option<String>) {
    for relative in DOCKER_E2E_PRODUCT_RECEIPT_CANDIDATES {
        let path = root.join(relative);
        if !path.is_file() {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        let green = value.get("green").and_then(Value::as_bool) == Some(true)
            || value
                .get("docker_e2e_validate_product")
                .and_then(Value::as_bool)
                == Some(true)
            || value.get("docker_e2e_product").and_then(Value::as_bool) == Some(true);
        if !green {
            continue;
        }
        if value.get("product").and_then(Value::as_u64) != Some(1)
            && value.get("pass_product_clean_room").and_then(Value::as_u64) != Some(1)
        {
            continue;
        }
        let counts = value.get("counts_as").and_then(Value::as_str).unwrap_or("");
        if counts.contains("component") {
            continue;
        }
        // Explicit refuse: host/loopback + host gate-e2e never count as Docker product.
        if receipt_is_host_loopback_or_gate_e2e_not_dod(&value) {
            continue;
        }
        // Cap0 AffinityOnly / PressureOnly / mutex / pressure EXIT0 ≠ product oneshot.
        if docker_e2e_receipt_is_cap0_probe_not_product(&value) {
            continue;
        }
        // SkipSwarm-only catalog/subset ≠ Docker e2e DoD proof.
        if receipt_is_skip_swarm_only_not_dod(&value) {
            continue;
        }
        // Alternate launchers / make e2e without mutex+pressure ≠ product oneshot.
        if docker_e2e_receipt_lacks_mutex_pressure_not_product(&value) {
            continue;
        }
        return (true, Some((*relative).to_string()));
    }
    (false, None)
}

fn docker_e2e_launcher_readiness(root: &Path) -> Value {
    let launchers: Vec<Value> = DOCKER_E2E_LAUNCHER_CANDIDATES
        .iter()
        .map(|relative| {
            json!({
                "path": relative,
                "present": root.join(relative).is_file(),
                "preferred_windows": *relative == "scripts/e2e-docker-launch.ps1"
            })
        })
        .collect();
    let topology: Vec<Value> = DOCKER_E2E_TOPOLOGY_CANDIDATES
        .iter()
        .map(|relative| {
            json!({
                "path": relative,
                "present": root.join(relative).is_file()
            })
        })
        .collect();
    let launcher_ok = launchers
        .iter()
        .any(|item| item.get("present").and_then(Value::as_bool) == Some(true));
    let preferred_ok = root.join("scripts/e2e-docker-launch.ps1").is_file();
    let topology_ok = topology
        .iter()
        .all(|item| item.get("present").and_then(Value::as_bool) == Some(true));
    json!({
        "launcher_present": launcher_ok,
        "preferred_windows_launcher_present": preferred_ok,
        "preferred_windows_launcher": "scripts/e2e-docker-launch.ps1",
        "topology_present": topology_ok,
        "launchers": launchers,
        "topology": topology,
        "pressure_guard_required": true,
        "single_flight_mutex_required": true,
        "product_oneshot_when_cap_free": [
            "pwsh -File scripts/e2e-docker-launch.ps1",
            "bash scripts/e2e-docker.sh",
            "make e2e",
            "pwsh -File scripts/e2e-docker.ps1"
        ],
        "cap0_probes_allowed_without_product": [
            "pwsh -File scripts/e2e-docker-launch.ps1 -AffinityOnly",
            "pwsh -File scripts/e2e-docker-launch.ps1 -PressureOnly"
        ],
        "cap0_readiness_ne_product": true,
        "cap0_affinity_pressure_ne_product": true,
        "cap0_mutex_pressure_probe_ne_product_oneshot": true,
        "host_loopback_ne_product": true,
        "note": "Cap0 launcher/topology honesty only (clean-room→matrix→red-blue/Docker e2e); AffinityOnly/PressureOnly/mutex/pressure EXIT0 ≠ product oneshot; Cap1 Docker burn after Cap free via e2e-docker-launch.ps1 (mutex+pressure) or lab-runner --red-blue-product --require-full; never invent docker_e2e_validate_product"
    })
}

fn measured_browser_ready(root: &Path) -> (bool, Option<String>) {
    measured_bool_receipt(root, BROWSER_READY_RECEIPT_CANDIDATES, "browser_ready")
}

fn apps_browser_present(root: &Path) -> (bool, Option<String>) {
    for relative in APPS_BROWSER_STRUCTURAL {
        let path = root.join(relative);
        if path.is_file() || path.is_dir() {
            // Empty dir alone is not a structural land; require Cargo.toml for dirs.
            if path.is_dir() {
                if path.join("Cargo.toml").is_file() {
                    return (true, Some(format!("{relative}/Cargo.toml")));
                }
                continue;
            }
            return (true, Some((*relative).to_string()));
        }
    }
    (false, None)
}

pub(crate) fn measure(root: &Path) -> Result<Value> {
    let cap_free = cap_free_after_unlock_a(root)?;
    let at = artifact_handoff(
        root,
        "AT-HANDOFF",
        INTERACTIVE_AT_HANDOFF_SCHEMA,
        INTERACTIVE_AT_HANDOFF_CANDIDATES,
        handoff_apply::validate_at_handoff,
    );
    let auth = artifact_handoff(
        root,
        "AUTH-HANDOFF",
        N06_OWNER_JOIN_AUTH_HANDOFF_SCHEMA,
        N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES,
        handoff_apply::validate_auth_handoff,
    );
    let grant = grant_status(root);
    let transfer_ok = transfer_present(root);
    let candidate_pack_grant = candidate_pack_grant_present(root);
    let candidate_pack_transfer = candidate_pack_transfer_present(root);

    let (browser_ready, browser_ready_path) = measured_browser_ready(root);
    let (pass_product, pass_product_path) = measured_pass_product(root);
    let (matrix_green, matrix_path) = measured_matrix_green(root);
    let (red_blue_green, red_blue_path) = measured_red_blue_product(root);
    let (docker_e2e_green, docker_e2e_path) = measured_docker_e2e_product(root);
    let docker_e2e_readiness = docker_e2e_launcher_readiness(root);

    let unlock = unlock_gate::probe_root(root, None)?;
    let unlock_c = unlock.c_authority_cap;

    let (apps_present, apps_path) = apps_browser_present(root);

    let incompletes = vec![
        json!({
            "id": "AT-HANDOFF",
            "klass": "Root_blocked",
            "closed": at.get("closed").and_then(Value::as_bool) == Some(true),
            "present": at.get("present").cloned().unwrap_or(Value::Bool(false)),
            "blocks": ["BrowserReady", "cap0-chain step1"],
            "detail": at
        }),
        json!({
            "id": "AUTH-HANDOFF",
            "klass": "Root_blocked",
            "closed": auth.get("closed").and_then(Value::as_bool) == Some(true),
            "present": auth.get("present").cloned().unwrap_or(Value::Bool(false)),
            "blocks": ["N06 join", "cap0-chain step3"],
            "detail": auth
        }),
        json!({
            "id": "Cap-free-restore-GRANT",
            "klass": "Root_blocked",
            "closed": grant.get("closed").and_then(Value::as_bool) == Some(true),
            "present": grant.get("present").cloned().unwrap_or(Value::Bool(false)),
            "blocks": ["Cap free=true", "Cap1 serial"],
            "detail": grant
        }),
        json!({
            "id": "BrowserReady",
            "klass": "measured_join",
            "dod_gate": true,
            "closed": browser_ready,
            "value": browser_ready,
            "invent_forbidden": true,
            "path": browser_ready_path,
            "note": "DoD: BrowserReady only when measured true; never invent"
        }),
        json!({
            "id": "pass_product_clean_room",
            "klass": "Cap1_serial",
            "dod_gate": true,
            "closed": pass_product == 1,
            "value": pass_product,
            "invent_forbidden": true,
            "path": pass_product_path,
            "note": "DoD gate: measured pass_product_clean_room=true after Cap1 --clean-room-product; never invent"
        }),
        json!({
            "id": "docker_clean_room_full_matrix_green",
            "klass": "Cap1_serial",
            "dod_gate": true,
            "closed": matrix_green,
            "green": matrix_green,
            "invent_forbidden": true,
            "path": matrix_path,
            "note": "DoD gate: Cap1 --clean-room-product-matrix --require-full green; component Cap EXIT0 ≠ DoD"
        }),
        json!({
            "id": "red_blue_validate_product",
            "klass": "Cap1_serial",
            "dod_gate": true,
            "dod_role": "detection_eval_proof",
            "closed": red_blue_green,
            "green": red_blue_green,
            "invent_forbidden": true,
            "path": red_blue_path,
            "blocks": ["Cap1 --red-blue-product --require-full feeder", "DoD detection-eval proof"],
            "note": "DoD detection-eval proof: lab-runner --red-blue-product --require-full proves currently implemented automation-detection correctly evaluates implementable bots/catalog on this deployment; host/loopback / host gate e2e / SkipSwarm-only ≠ DoD; detection freeze (no scoring change)"
        }),
        json!({
            "id": "docker_e2e_validate_product",
            "klass": "Cap1_serial_docker_only",
            "dod_gate": true,
            "dod_role": "detection_eval_proof",
            "closed": docker_e2e_green,
            "green": docker_e2e_green,
            "invent_forbidden": true,
            "path": docker_e2e_path,
            "blocks": [
                "Docker clean-room/full e2e",
                "red-blue-validate product",
                "scripts/e2e-docker-launch.ps1",
                "DoD detection-eval proof"
            ],
            "launcher_readiness": docker_e2e_readiness,
            "host_loopback_ne_product": true,
            "note": "DoD detection-eval proof: Docker-only e2e via e2e-docker-launch.ps1 (mutex+pressure) / e2e-docker.sh proves clean-room product evaluation; AffinityOnly/PressureOnly/component Cap EXIT0 / host node test/e2e/runner.mjs / host node test/gate/e2e.mjs ≠ DoD; Cap1 after Cap free"
        }),
        json!({
            "id": "Unlock-C",
            "klass": if unlock_c {
                "Cap0_unlock_c_bound"
            } else if unlock.reason_c.contains("candidate digests") {
                "Cap0_digest_mismatch"
            } else if unlock.reason_c.contains("transfer-manifest absent") {
                "Cap0_manifest_absent"
            } else if unlock.reason_c.contains("required Cap destinations missing") {
                "Cap0_transfer_not_applied"
            } else {
                "Cap0_blocked_claim"
            },
            "closed": unlock_c,
            "unlock_c": unlock_c,
            "authority_cap_cli_status": unlock.authority_cap_cli_status,
            "reason": unlock.reason_c,
            "default_manifest": unlock_gate::DEFAULT_UNLOCK_C_MANIFEST,
            "note": "Unlock C fail-closed until Root LIVE TRANSFER matches candidate digests; never forge manifest or GRANT"
        }),
        json!({
            "id": "apps_humanymous_browser",
            "klass": "Root_structural",
            "closed": apps_present,
            "present": apps_present,
            "path": apps_path,
            "note": "package path still absent unless Root lands crates/apps/humanymous-browser; Cursor scope does not grant create"
        }),
    ];

    let open: Vec<&Value> = incompletes
        .iter()
        .filter(|item| item.get("closed").and_then(Value::as_bool) != Some(true))
        .collect();
    let incomplete_count = open.len();

    let ready_for_cap0_chain = at.get("closed").and_then(Value::as_bool) == Some(true)
        && auth.get("closed").and_then(Value::as_bool) == Some(true)
        && grant.get("closed").and_then(Value::as_bool) == Some(true)
        && transfer_ok;

    // Cap1 ready only when Cap free measured true — never invent; HOLD otherwise.
    let ready_for_cap1 = cap_free;
    let cap1_serial = if cap_free {
        "ARMED_cap_free_true"
    } else {
        "HOLD"
    };

    let product_gates_verified =
        pass_product == 1 && matrix_green && red_blue_green && docker_e2e_green && browser_ready;
    // DoD = measured detection-eval proof + product gates; never invent.
    let dod_verified = product_gates_verified
        && red_blue_green
        && docker_e2e_green
        && pass_product == 1
        && matrix_green
        && browser_ready;
    let goal_complete = incomplete_count == 0 && product_gates_verified && dod_verified;

    let status = if goal_complete {
        "goal_complete"
    } else if incomplete_count == 0 {
        "incompletes_closed_product_gates_unverified"
    } else {
        "incompletes_remain"
    };

    let next = if goal_complete {
        "goal_complete measured (DoD detection-eval verified); stop"
    } else if ready_for_cap0_chain && !cap_free {
        "Artifacts ready; run cap0-chain (no --inspect-only) to restore Cap free; then Cap1 serial clean-room → matrix --require-full → Docker e2e / --red-blue-product --require-full via e2e-docker-launch.ps1; DoD requires measured detection-eval; never invent Cap free / pass_product / docker_e2e"
    } else if cap_free {
        "Cap free=true; Cap1 serial armed — cap1-lander-prep then once: --clean-room-product → --clean-room-product-matrix --require-full → Docker e2e (e2e-docker-launch.ps1) / --red-blue-product --require-full; DoD = honest evaluation of implementable automation on this deployment; never invent pass_product / docker_e2e / BrowserReady"
    } else {
        "Root place LIVE AT-HANDOFF + AUTH-HANDOFF + Cap-free-restore GRANT.json (not TEMPLATE) + LIVE-matching TRANSFER; re-run until-complete-status; Cap1 HOLD; Cap burn=0; DoD unverified until Cap1 red-blue product + Docker e2e green"
    };

    let definition_of_done = json!({
        "schema": "humanymous.lab-definition-of-done/1",
        "measured_only": true,
        "never_invent": true,
        "detection_freeze": true,
        "summary": "Currently implemented automation-detection tech is correctly evaluated on this deployment via red-blue-validate product + clean-room Docker e2e; host/loopback / host gate e2e / component Cap EXIT0 / AffinityOnly/PressureOnly / SkipSwarm-only / red-blue or matrix without --require-full / Docker e2e without mutex+pressure ≠ DoD",
        "gates": [
            {
                "id": "pass_product_clean_room",
                "required": true,
                "closed": pass_product == 1,
                "value": pass_product
            },
            {
                "id": "docker_clean_room_full_matrix_green",
                "required": true,
                "selector": "--clean-room-product-matrix --require-full",
                "closed": matrix_green
            },
            {
                "id": "BrowserReady",
                "required": true,
                "closed": browser_ready,
                "note": "only when measured true"
            },
            {
                "id": "red_blue_validate_product",
                "required": true,
                "dod_role": "detection_eval_proof",
                "selector": "--red-blue-product --require-full",
                "closed": red_blue_green
            },
            {
                "id": "docker_e2e_validate_product",
                "required": true,
                "dod_role": "detection_eval_proof",
                "selector": "pwsh -File scripts/e2e-docker-launch.ps1",
                "closed": docker_e2e_green
            }
        ],
        "non_dod": [
            "host/loopback node test/e2e/runner.mjs",
            "host node test/gate/e2e.mjs",
            "component Cap EXIT0",
            "AffinityOnly / PressureOnly launcher probes",
            "SkipSwarm-only",
            "red-blue or matrix without --require-full",
            "Docker e2e product oneshot without mutex+pressure",
            "candidate-pack GRANT/TRANSFER scaffold"
        ],
        "verified": dod_verified
    });

    Ok(json!({
        "schema": SCHEMA,
        "record_kind": "lab-until-complete-status",
        "cap": 0,
        "cap_burn": 0,
        "cap_free": cap_free,
        "product": if pass_product == 1 { 1 } else { 0 },
        "pass_product_clean_room": pass_product,
        "browser_ready": browser_ready,
        "docker_e2e_validate_product": docker_e2e_green,
        "docker_e2e_launcher_readiness": docker_e2e_readiness,
        "goal_complete": goal_complete,
        "product_gates_verified": product_gates_verified,
        "definition_of_done": definition_of_done,
        "dod_verified": dod_verified,
        "incomplete_count": incomplete_count,
        "incompletes": incompletes,
        "open_incomplete_ids": open.iter().filter_map(|item| item.get("id").cloned()).collect::<Vec<_>>(),
        "ready_for_cap0_chain": ready_for_cap0_chain,
        "ready_for_cap1": ready_for_cap1,
        "cap1_serial": cap1_serial,
        "cap1_serial_preferred_order": [
            "--clean-room-product",
            "--clean-room-product-matrix --require-full",
            "pwsh -File scripts/e2e-docker-launch.ps1 / --red-blue-product --require-full"
        ],
        "transfer_manifest_present": transfer_ok,
        "candidate_pack_grant_present": candidate_pack_grant,
        "candidate_pack_transfer_present": candidate_pack_transfer,
        "transfer_live_matching": transfer_ok,
        "status": status,
        "pilot_command": PILOT_COMMAND,
        "next": next,
        "distinct_from_cap0_chain_inspect_only": {
            "inspect_only_covers": "AT/AUTH/GRANT/TRANSFER presence+schema wake only",
            "this_command_covers": [
                "ten incompletes including DoD detection-eval (red_blue_validate_product + docker_e2e_validate_product)",
                "definition_of_done",
                "cap_free",
                "ready_for_cap0_chain",
                "ready_for_cap1",
                "Docker e2e launcher/topology Cap0 honesty",
                "goal_complete fail-closed"
            ]
        },
        "honesty": {
            "never_invent_pass_product_clean_room": true,
            "never_invent_browser_ready": true,
            "never_invent_cap_free": true,
            "never_count_candidate_pack_grant_as_live": true,
            "never_count_candidate_pack_transfer_as_live": true,
            "never_invent_docker_e2e_validate_product": true,
            "host_loopback_ne_product": true,
            "host_loopback_ne_dod": true,
            "host_gate_e2e_ne_dod": true,
            "component_ne_dod": true,
            "affinity_pressure_ne_dod": true,
            "skip_swarm_only_ne_dod": true,
            "require_full_ne_dod_when_absent_on_product_receipt": true,
            "docker_e2e_mutex_pressure_required_for_product_oneshot": true,
            "never_forge_auth_at": true,
            "never_invent_goal_complete": true,
            "never_apply_template_grant": true,
            "detection_freeze": true,
            "dod_is_honest_measurement_not_scoring_change": true,
            "cap0_only": true,
            "cap_burn_0_while_cap_free_false": true,
            "no_cap_twin_reburn_73_80": true,
            "fail_closed_exit_1_while_incomplete": true,
            "component_ne_product": true,
            "cap0_affinity_pressure_ne_product": true,
            "cap0_mutex_pressure_probe_ne_product_oneshot": true
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::cap0_product_honesty::{UNLOCK_A_CLOSER_REL, UNLOCK_A_CLOSER_SCHEMA};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("hmn-until-complete-{label}-{nanos}"));
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

    #[test]
    fn empty_workspace_reports_ten_incompletes_fail_closed() {
        let root = temp_root("empty");
        write_unlock_a(&root, false);
        let report = measure(&root).expect("measure");
        assert_eq!(report["schema"], SCHEMA);
        assert_eq!(report["cap_free"], false);
        assert_eq!(report["ready_for_cap0_chain"], false);
        assert_eq!(report["ready_for_cap1"], false);
        assert_eq!(report["goal_complete"], false);
        assert_eq!(report["product_gates_verified"], false);
        assert_eq!(report["dod_verified"], false);
        assert_eq!(report["pass_product_clean_room"], 0);
        assert_eq!(report["browser_ready"], false);
        assert_eq!(report["docker_e2e_validate_product"], false);
        assert_eq!(report["incomplete_count"], 10);
        assert_eq!(report["cap1_serial"], "HOLD");
        assert_eq!(report["status"], "incompletes_remain");
        assert_eq!(report["definition_of_done"]["verified"], false);
        assert_eq!(report["definition_of_done"]["detection_freeze"], true);
        assert_eq!(report["honesty"]["host_loopback_ne_dod"], true);
        assert_eq!(report["honesty"]["host_gate_e2e_ne_dod"], true);
        assert_eq!(report["honesty"]["component_ne_dod"], true);
        assert_eq!(
            report["honesty"]["dod_is_honest_measurement_not_scoring_change"],
            true
        );
        let incompletes = report["incompletes"].as_array().expect("incompletes");
        let red_blue = incompletes
            .iter()
            .find(|item| item.get("id") == Some(&json!("red_blue_validate_product")))
            .expect("red_blue incomplete");
        assert_eq!(red_blue["dod_gate"], true);
        assert_eq!(red_blue["dod_role"], "detection_eval_proof");
        let docker_e2e = incompletes
            .iter()
            .find(|item| item.get("id") == Some(&json!("docker_e2e_validate_product")))
            .expect("docker_e2e incomplete");
        assert_eq!(docker_e2e["dod_gate"], true);
        assert_eq!(docker_e2e["dod_role"], "detection_eval_proof");
        assert!(
            report["open_incomplete_ids"]
                .as_array()
                .expect("ids")
                .iter()
                .any(|id| id == "docker_e2e_validate_product")
        );
        assert_eq!(
            report["distinct_from_cap0_chain_inspect_only"]["inspect_only_covers"],
            "AT/AUTH/GRANT/TRANSFER presence+schema wake only"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn docker_e2e_topology_honesty_covers_cap1_serial_clean_room_matrix() {
        assert!(
            DOCKER_E2E_TOPOLOGY_CANDIDATES
                .iter()
                .any(|p| *p == "deployments/lab/compose.clean-room-product.yaml")
        );
        assert!(
            DOCKER_E2E_TOPOLOGY_CANDIDATES
                .iter()
                .any(|p| *p == "deployments/lab/compose.clean-room-product-matrix.yaml")
        );
        assert!(
            DOCKER_E2E_TOPOLOGY_CANDIDATES
                .iter()
                .any(|p| *p == "deployments/lab/compose.red-blue-product.yaml")
        );
        assert!(DOCKER_E2E_TOPOLOGY_CANDIDATES.len() >= 7);
        let root = temp_root("topo");
        write_unlock_a(&root, false);
        let report = measure(&root).expect("measure");
        let topology = report["docker_e2e_launcher_readiness"]["topology"]
            .as_array()
            .expect("topology");
        assert_eq!(topology.len(), DOCKER_E2E_TOPOLOGY_CANDIDATES.len());
        assert_eq!(
            report["docker_e2e_launcher_readiness"]["topology_present"],
            false
        );
        assert_eq!(report["docker_e2e_validate_product"], false);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn host_loopback_docker_e2e_receipt_never_closes() {
        let root = temp_root("host-e2e");
        write_unlock_a(&root, false);
        let receipt = root.join(DOCKER_E2E_PRODUCT_RECEIPT_CANDIDATES[0]);
        fs::create_dir_all(receipt.parent().expect("parent")).expect("mkdir");
        fs::write(
            &receipt,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.lab-docker-e2e-product-measure-receipt/1",
                "product": 1,
                "pass_product_clean_room": 1,
                "docker_e2e_validate_product": true,
                "completion_authority": "host_loopback_node_runner",
                "counts_as": "host_loopback_debug_not_product"
            }))
            .expect("encode"),
        )
        .expect("write");
        let report = measure(&root).expect("measure");
        assert_eq!(report["docker_e2e_validate_product"], false);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn host_gate_e2e_receipt_never_closes_red_blue_or_docker_dod() {
        let root = temp_root("host-gate-e2e");
        write_unlock_a(&root, false);

        let rb = root.join(RED_BLUE_PRODUCT_RECEIPT_CANDIDATES[0]);
        fs::create_dir_all(rb.parent().expect("parent")).expect("mkdir");
        fs::write(
            &rb,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.lab-red-blue-product-measure-receipt/1",
                "product": 1,
                "pass_product_clean_room": 1,
                "red_blue_validate_product": true,
                "green": true,
                "completion_authority": "host_node_test_gate_e2e",
                "command": "node test/gate/e2e.mjs",
                "counts_as": "host_gate_e2e_debug_not_product"
            }))
            .expect("encode"),
        )
        .expect("write");

        let e2e = root.join(DOCKER_E2E_PRODUCT_RECEIPT_CANDIDATES[0]);
        fs::create_dir_all(e2e.parent().expect("parent")).expect("mkdir");
        fs::write(
            &e2e,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.lab-docker-e2e-product-measure-receipt/1",
                "product": 1,
                "pass_product_clean_room": 1,
                "docker_e2e_validate_product": true,
                "green": true,
                "completion_authority": "host_loopback",
                "runner": "node test/e2e/runner.mjs",
                "counts_as": "host_loopback_runner_mjs_not_product"
            }))
            .expect("encode"),
        )
        .expect("write");

        let report = measure(&root).expect("measure");
        let incompletes = report["incompletes"].as_array().expect("incompletes");
        let red_blue = incompletes
            .iter()
            .find(|item| item.get("id") == Some(&json!("red_blue_validate_product")))
            .expect("red_blue incomplete");
        assert_eq!(red_blue["closed"], false);
        assert_eq!(red_blue["green"], false);
        assert_eq!(report["docker_e2e_validate_product"], false);
        assert_eq!(report["definition_of_done"]["gates"][3]["closed"], false);
        assert_eq!(report["definition_of_done"]["gates"][4]["closed"], false);
        assert_eq!(report["dod_verified"], false);
        assert_eq!(report["goal_complete"], false);
        assert_eq!(report["honesty"]["host_gate_e2e_ne_dod"], true);
        assert_eq!(report["honesty"]["host_loopback_ne_dod"], true);
        let non_dod = report["definition_of_done"]["non_dod"]
            .as_array()
            .expect("non_dod");
        assert!(
            non_dod
                .iter()
                .any(|item| item.as_str() == Some("host node test/gate/e2e.mjs"))
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn affinity_only_pressure_only_probe_receipt_never_closes_product_oneshot() {
        let root = temp_root("affinity-pressure");
        write_unlock_a(&root, false);
        let receipt = root.join(DOCKER_E2E_PRODUCT_RECEIPT_CANDIDATES[0]);
        fs::create_dir_all(receipt.parent().expect("parent")).expect("mkdir");
        fs::write(
            &receipt,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.lab-docker-e2e-product-measure-receipt/1",
                "product": 1,
                "pass_product_clean_room": 1,
                "docker_e2e_validate_product": true,
                "green": true,
                "completion_authority": "docker_only",
                "mode": "AffinityOnly",
                "counts_as": "cap0_AffinityOnly_probe_not_product",
                "cap0_probe_ne_product": true
            }))
            .expect("encode"),
        )
        .expect("write");
        let report = measure(&root).expect("measure");
        assert_eq!(report["docker_e2e_validate_product"], false);
        assert_eq!(
            report["docker_e2e_launcher_readiness"]["cap0_mutex_pressure_probe_ne_product_oneshot"],
            true
        );
        assert_eq!(
            report["docker_e2e_launcher_readiness"]["pressure_guard_required"],
            true
        );
        assert_eq!(
            report["docker_e2e_launcher_readiness"]["single_flight_mutex_required"],
            true
        );
        assert_eq!(
            report["honesty"]["cap0_mutex_pressure_probe_ne_product_oneshot"],
            true
        );

        fs::write(
            &receipt,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.e2e-desktop-pressure-guard/1",
                "product": 1,
                "pass_product_clean_room": 1,
                "docker_e2e_validate_product": true,
                "green": true,
                "result": "PASS",
                "completion_authority": "docker_only",
                "mode": "PressureOnly",
                "counts_as": "cap0_PressureOnly_probe_not_product"
            }))
            .expect("encode"),
        )
        .expect("write");
        let report2 = measure(&root).expect("measure");
        assert_eq!(report2["docker_e2e_validate_product"], false);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn red_blue_without_require_full_never_closes_detection_eval_dod() {
        let root = temp_root("rb-no-req-full");
        write_unlock_a(&root, false);

        let rb = root.join(RED_BLUE_PRODUCT_RECEIPT_CANDIDATES[0]);
        fs::create_dir_all(rb.parent().expect("parent")).expect("mkdir");
        fs::write(
            &rb,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.lab-red-blue-product-measure-receipt/1",
                "product": 1,
                "pass_product_clean_room": 1,
                "red_blue_validate_product": true,
                "green": true,
                "require_full": false,
                "counts_as": "red_blue_partial_catalog_not_dod"
            }))
            .expect("encode"),
        )
        .expect("write");

        let report = measure(&root).expect("measure");
        assert_eq!(report["docker_e2e_validate_product"], false);
        let incompletes = report["incompletes"].as_array().expect("incompletes");
        let red_blue = incompletes
            .iter()
            .find(|item| item.get("id") == Some(&json!("red_blue_validate_product")))
            .expect("red_blue incomplete");
        assert_eq!(red_blue["closed"], false);
        assert_eq!(red_blue["green"], false);
        assert_eq!(report["dod_verified"], false);
        assert_eq!(
            report["honesty"]["require_full_ne_dod_when_absent_on_product_receipt"],
            true
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn docker_e2e_without_mutex_pressure_never_closes_product_oneshot() {
        let root = temp_root("e2e-no-mutex");
        write_unlock_a(&root, false);

        let e2e = root.join(DOCKER_E2E_PRODUCT_RECEIPT_CANDIDATES[0]);
        fs::create_dir_all(e2e.parent().expect("parent")).expect("mkdir");
        fs::write(
            &e2e,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.lab-docker-e2e-product-measure-receipt/1",
                "product": 1,
                "pass_product_clean_room": 1,
                "docker_e2e_validate_product": true,
                "green": true,
                "completion_authority": "docker_only",
                "launcher": "make e2e",
                "counts_as": "make_e2e_only_without_mutex"
            }))
            .expect("encode"),
        )
        .expect("write");

        let report = measure(&root).expect("measure");
        assert_eq!(report["docker_e2e_validate_product"], false);
        assert_eq!(
            report["honesty"]["docker_e2e_mutex_pressure_required_for_product_oneshot"],
            true
        );
        assert_eq!(report["dod_verified"], false);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn skip_swarm_only_receipt_never_closes_detection_eval_dod() {
        let root = temp_root("skip-swarm");
        write_unlock_a(&root, false);

        let rb = root.join(RED_BLUE_PRODUCT_RECEIPT_CANDIDATES[0]);
        fs::create_dir_all(rb.parent().expect("parent")).expect("mkdir");
        fs::write(
            &rb,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.lab-red-blue-product-measure-receipt/1",
                "product": 1,
                "pass_product_clean_room": 1,
                "red_blue_validate_product": true,
                "green": true,
                "mode": "SkipSwarm",
                "counts_as": "skip_swarm_only_ne_dod_not_product",
                "skip_swarm_only": true
            }))
            .expect("encode"),
        )
        .expect("write");

        let e2e = root.join(DOCKER_E2E_PRODUCT_RECEIPT_CANDIDATES[0]);
        fs::create_dir_all(e2e.parent().expect("parent")).expect("mkdir");
        fs::write(
            &e2e,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.lab-docker-e2e-product-measure-receipt/1",
                "product": 1,
                "pass_product_clean_room": 1,
                "docker_e2e_validate_product": true,
                "green": true,
                "completion_authority": "skip_swarm_only",
                "counts_as": "skip_swarm_catalog_subset_not_product",
                "skip_swarm_only": true
            }))
            .expect("encode"),
        )
        .expect("write");

        let report = measure(&root).expect("measure");
        let incompletes = report["incompletes"].as_array().expect("incompletes");
        let red_blue = incompletes
            .iter()
            .find(|item| item.get("id") == Some(&json!("red_blue_validate_product")))
            .expect("red_blue incomplete");
        assert_eq!(red_blue["closed"], false);
        assert_eq!(red_blue["green"], false);
        assert_eq!(report["docker_e2e_validate_product"], false);
        assert_eq!(report["definition_of_done"]["gates"][3]["closed"], false);
        assert_eq!(report["definition_of_done"]["gates"][4]["closed"], false);
        assert_eq!(report["dod_verified"], false);
        assert_eq!(report["goal_complete"], false);
        assert_eq!(report["honesty"]["skip_swarm_only_ne_dod"], true);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn scaffold_product_receipt_does_not_invent_pass_product() {
        let root = temp_root("scaffold");
        write_unlock_a(&root, false);
        let receipt = root.join(PASS_PRODUCT_RECEIPT_CANDIDATES[0]);
        fs::create_dir_all(receipt.parent().expect("parent")).expect("mkdir");
        fs::write(
            &receipt,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.lab-clean-room-product-measure-receipt/1",
                "product": 0,
                "pass_product_clean_room": 0,
                "counts_as": "pass_component_desktop_cap_topology"
            }))
            .expect("encode"),
        )
        .expect("write");
        let report = measure(&root).expect("measure");
        assert_eq!(report["pass_product_clean_room"], 0);
        assert_eq!(report["goal_complete"], false);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cap_free_true_arms_ready_for_cap1_without_inventing_goal() {
        let root = temp_root("capfree");
        write_unlock_a(&root, true);
        let report = measure(&root).expect("measure");
        assert_eq!(report["cap_free"], true);
        assert_eq!(report["ready_for_cap1"], true);
        assert_eq!(report["cap1_serial"], "ARMED_cap_free_true");
        assert_eq!(report["goal_complete"], false);
        assert!(report["incomplete_count"].as_u64().expect("count") >= 8);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ready_for_cap0_chain_requires_at_auth_grant_and_transfer() {
        let root = temp_root("chain-ready");
        write_unlock_a(&root, false);

        let at_rel = INTERACTIVE_AT_HANDOFF_CANDIDATES[0];
        let at_path = root.join(at_rel);
        fs::create_dir_all(at_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &at_path,
            serde_json::to_vec_pretty(&json!({
                "schema": INTERACTIVE_AT_HANDOFF_SCHEMA,
                "browser_ready_authorized": true,
                "product": 0,
                "pass_product_clean_room": 0
            }))
            .expect("encode"),
        )
        .expect("write");

        let auth_rel = N06_OWNER_JOIN_AUTH_HANDOFF_CANDIDATES[0];
        let auth_path = root.join(auth_rel);
        fs::create_dir_all(auth_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &auth_path,
            serde_json::to_vec_pretty(&json!({
                "schema": N06_OWNER_JOIN_AUTH_HANDOFF_SCHEMA,
                "n06_owner_join_authorized": true,
                "product": 0,
                "pass_product_clean_room": 0
            }))
            .expect("encode"),
        )
        .expect("write");

        let grant_rel = CAP_FREE_GRANT_CANDIDATES[1];
        let grant_path = root.join(grant_rel);
        fs::create_dir_all(grant_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &grant_path,
            serde_json::to_vec_pretty(&json!({
                "schema": GRANT_SCHEMA,
                "root_authorized": true,
                "allow_apply": true,
                "authorize_cap_free_restore": true,
                "new_non_twin": true,
                "cap_burn_authorized": false,
                "cap1_inside_apply": false,
                "product": 0,
                "pass_product_clean_room": 0,
                "feeder": "product-cap-free-restore",
                "unlock_a_closer": UNLOCK_A_CLOSER_REL,
                "destinations": ["deployments/lab/product/cap-free-restore-contract.json"]
            }))
            .expect("encode"),
        )
        .expect("write");

        // Without TRANSFER → not ready for chain.
        let before = measure(&root).expect("measure");
        assert_eq!(before["ready_for_cap0_chain"], false);
        assert_eq!(
            before["incompletes"]
                .as_array()
                .expect("arr")
                .iter()
                .find(|item| item["id"] == "AT-HANDOFF")
                .expect("at")["closed"],
            true
        );

        let manifest_rel = CAP_FREE_MANIFEST_CANDIDATES[2];
        let manifest_path = root.join(manifest_rel);
        fs::create_dir_all(manifest_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.transfer-manifest/1",
                "entries": []
            }))
            .expect("encode"),
        )
        .expect("write");

        let after = measure(&root).expect("measure");
        assert_eq!(after["ready_for_cap0_chain"], true);
        assert_eq!(after["goal_complete"], false);
        assert_eq!(after["cap_free"], false);
        assert_eq!(after["ready_for_cap1"], false);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn component_counts_as_never_closes_pass_product() {
        let root = temp_root("component");
        write_unlock_a(&root, false);
        let receipt = root.join(PASS_PRODUCT_RECEIPT_CANDIDATES[0]);
        fs::create_dir_all(receipt.parent().expect("parent")).expect("mkdir");
        fs::write(
            &receipt,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.lab-clean-room-product-measure-receipt/1",
                "product": 1,
                "pass_product_clean_room": 1,
                "counts_as": "pass_component_desktop_cap_topology"
            }))
            .expect("encode"),
        )
        .expect("write");
        let report = measure(&root).expect("measure");
        assert_eq!(report["pass_product_clean_room"], 0);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn candidate_pack_transfer_is_not_live_matching() {
        let root = temp_root("candidate-pack-xfer");
        write_unlock_a(&root, false);

        let scaffold = root.join(CAP_FREE_CANDIDATE_PACK_MANIFEST);
        fs::create_dir_all(scaffold.parent().expect("parent")).expect("mkdir");
        fs::write(
            &scaffold,
            serde_json::to_vec_pretty(&json!({
                "schema": "humanymous.cursor-cap-free-restore-candidate/1",
                "entries": [{
                    "destination": "deployments/lab/product/cap-free-restore-contract.json",
                    "preimageSha256": "00"
                }]
            }))
            .expect("encode"),
        )
        .expect("write");

        let report = measure(&root).expect("measure");
        assert_eq!(report["candidate_pack_transfer_present"], true);
        assert_eq!(report["transfer_manifest_present"], false);
        assert_eq!(report["transfer_live_matching"], false);
        assert_eq!(report["ready_for_cap0_chain"], false);
        assert_eq!(
            report["honesty"]["never_count_candidate_pack_transfer_as_live"],
            true
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn candidate_pack_grant_is_not_live_matching() {
        let root = temp_root("candidate-pack-grant");
        write_unlock_a(&root, false);

        let scaffold = root.join(CAP_FREE_CANDIDATE_PACK_GRANT);
        fs::create_dir_all(scaffold.parent().expect("parent")).expect("mkdir");
        fs::write(
            &scaffold,
            serde_json::to_vec_pretty(&json!({
                "schema": GRANT_SCHEMA,
                "root_authorized": true,
                "allow_apply": true,
                "authorize_cap_free_restore": true,
                "new_non_twin": true,
                "cap_burn_authorized": false,
                "cap1_inside_apply": false,
                "product": 0,
                "pass_product_clean_room": 0,
                "feeder": "product-cap-free-restore",
                "unlock_a_closer": UNLOCK_A_CLOSER_REL,
                "destinations": ["deployments/lab/product/cap-free-restore-contract.json"]
            }))
            .expect("encode"),
        )
        .expect("write");

        let report = measure(&root).expect("measure");
        assert_eq!(report["candidate_pack_grant_present"], true);
        assert_eq!(report["incompletes"][2]["id"], "Cap-free-restore-GRANT");
        assert_eq!(report["incompletes"][2]["present"], false);
        assert_eq!(report["incompletes"][2]["closed"], false);
        assert_eq!(report["ready_for_cap0_chain"], false);
        assert_eq!(
            report["honesty"]["never_count_candidate_pack_grant_as_live"],
            true
        );
        let _ = fs::remove_dir_all(&root);
    }
}
