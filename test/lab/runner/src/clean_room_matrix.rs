//! Fail-closed Cap planner + usage-condition completeness for the Docker
//! clean-room product matrix.
//!
//! Dry-run only: serializes Cap=1 command preimages for later execution after
//! Root unlock, and Cap=0-enumerates every SoT-required usage-condition cell
//! with an honest blocker. Never invents `pass_product_clean_room`, never burns
//! Cap, and refuses Cap dispatch for `required_unavailable` / blocked /
//! component-hold cells (SoT honesty + no `--require-full` Cap spam).
use crate::docker::Result;
use serde_json::{Map, Value, json};
use std::{fs, path::Path};

const SCHEMA: &str = "humanymous.lab-clean-room-matrix-plan/1";
const COMPLETENESS_SCHEMA: &str = "humanymous.lab-clean-room-matrix-completeness/1";

/// SoT/ENVIRONMENTS + Cap-tracked product scenarios that the clean-room matrix
/// must enumerate (fail-closed). Adapter-home and pages/docs are optional.
const SOT_REQUIRED_SCENARIOS: &[&str] = &[
    "engine-affinity",
    "topology-fixture",
    "rust-controls",
    "authority-node-ipc",
    "go-product",
    "browser-product",
    "collector-component",
    "custody",
    "runtime",
    "witness",
    "m3-full-product",
    "w2-h2-product-binding",
    "g2-collector-product-binding",
    "g5-witness-transport",
    "clean-room-product",
    "g6-origin-audit-binding",
    "red-blue-product",
];

const G2_GRANT_CANDIDATE: &str = ".spec.local/root/evidence/20261109030000-cursor-g2-cap-ready-feeder/cap-ready/g2-collector-product-binding/candidate-pack/GRANT.json";
const G5_GRANT_CANDIDATE: &str = ".spec.local/root/evidence/20261109031000-cursor-g5-cap-ready-feeder/cap-ready/g5-witness-transport/candidate-pack/GRANT.json";
const G6_GRANT_CANDIDATE: &str = ".spec.local/root/evidence/20261109040600-cursor-g6-cap-ready-feeder/cap-ready/g6-origin-audit-binding/candidate-pack/GRANT.json";
const CR_GRANT_CANDIDATE: &str = ".spec.local/root/evidence/20261109040000-cursor-clean-room-product-cap-ready/cap-ready/clean-room-product/candidate-pack/GRANT.json";
const RB_GRANT_CANDIDATE: &str = ".spec.local/root/evidence/20261110044800-cursor-red-blue-product-cap-ready/cap-ready/red-blue-product/candidate-pack/GRANT.json";
const AUTH_HANDOFF_CANDIDATES: &[&str] = &[
    ".spec.local/root/evidence/root-n06-owner-join-auth-handoff/AUTH-HANDOFF.json",
    ".spec.local/root/grants/n06-owner-join/GRANT.json",
];
const AT_HANDOFF_CANDIDATES: &[&str] =
    &[".spec.local/root/evidence/root-interactive-at-windows-tuple/AT-HANDOFF.json"];

/// Root paths that must move before authority-node-ipc Cap CLI may serialize.
const AUTHORITY_NODE_IPC_ROOT_ASK_PATHS: &[&str] = &[
    "deployments/lab/INTEGRATION.md",
    "deployments/lab/authority/**",
    "build/rust/authority-lab.Dockerfile",
    "build/rust/authority-lab.Dockerfile.dockerignore",
    "test/lab/runner/src/snapshot.rs",
    "adapters/node/src/**",
    "crates/authority/**",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Disposition {
    RefuseCap,
    HoldComponent,
    SerializeWhenUnlocked,
    OutOfRunnerScope,
    /// Cap selector is known but Root-blocked; never Cap-serialize.
    BlockedClaim,
}

impl Disposition {
    fn as_str(self) -> &'static str {
        match self {
            Self::RefuseCap => "refuse_cap",
            Self::HoldComponent => "hold_component",
            Self::SerializeWhenUnlocked => "serialize_cap_when_unlocked",
            Self::OutOfRunnerScope => "out_of_lab_runner_scope",
            Self::BlockedClaim => "blocked_claim",
        }
    }
}

/// Plan Cap dispositions for a clean-room product matrix without executing Docker.
pub fn plan(args: &[String]) -> Result<i32> {
    let mut matrix_path = None;
    let mut emit_path = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--matrix" => {
                index += 1;
                matrix_path = Some(args.get(index).ok_or("--matrix path missing")?.clone());
            }
            "--emit" => {
                index += 1;
                emit_path = Some(args.get(index).ok_or("--emit path missing")?.clone());
            }
            _ => return Err("usage: matrix-plan --matrix PATH [--emit PATH]".into()),
        }
        index += 1;
    }
    let matrix_path = matrix_path.ok_or("--matrix required")?;
    let raw =
        fs::read_to_string(&matrix_path).map_err(|error| format!("matrix read failed: {error}"))?;
    let matrix: Value =
        serde_json::from_str(&raw).map_err(|error| format!("matrix json: {error}"))?;
    let plan = plan_matrix(&matrix)?;
    let rendered = serde_json::to_string_pretty(&plan)
        .map_err(|error| format!("plan encode failed: {error}"))?;
    if let Some(path) = emit_path {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).map_err(|error| format!("emit mkdir: {error}"))?;
        }
        fs::write(&path, format!("{rendered}\n"))
            .map_err(|error| format!("emit write failed: {error}"))?;
        println!(
            "{}",
            json!({
                "schema": SCHEMA,
                "emit": path,
                "cap_execute": false,
                "pass_product_clean_room": 0,
                "summary": plan["summary"],
            })
        );
    } else {
        println!("{rendered}");
    }
    Ok(0)
}

/// Cap=0 enumerate every required usage-condition cell with an honest blocker.
/// Never invents product / Cap free / BrowserReady; Cap burn stays 0.
pub fn completeness(args: &[String]) -> Result<i32> {
    let mut matrix_path = None;
    let mut emit_path = None;
    let mut expect_head = None;
    let mut workspace = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--matrix" => {
                index += 1;
                matrix_path = Some(args.get(index).ok_or("--matrix path missing")?.clone());
            }
            "--emit" => {
                index += 1;
                emit_path = Some(args.get(index).ok_or("--emit path missing")?.clone());
            }
            "--expect-head" => {
                index += 1;
                expect_head = Some(args.get(index).ok_or("--expect-head sha missing")?.clone());
            }
            "--workspace" => {
                index += 1;
                workspace = Some(args.get(index).ok_or("--workspace path missing")?.clone());
            }
            _ => {
                return Err(
                    "usage: matrix-completeness --matrix PATH [--emit PATH] [--expect-head SHA] [--workspace PATH]"
                        .into(),
                );
            }
        }
        index += 1;
    }
    let matrix_path = matrix_path.ok_or("--matrix required")?;
    let workspace_root = workspace.map(std::path::PathBuf::from).unwrap_or_else(|| {
        crate::snapshot::root().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });
    let raw =
        fs::read_to_string(&matrix_path).map_err(|error| format!("matrix read failed: {error}"))?;
    let matrix: Value =
        serde_json::from_str(&raw).map_err(|error| format!("matrix json: {error}"))?;
    let report = completeness_report(&matrix, &workspace_root, expect_head.as_deref())?;
    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("completeness encode failed: {error}"))?;
    if let Some(path) = emit_path {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).map_err(|error| format!("emit mkdir: {error}"))?;
        }
        fs::write(&path, format!("{rendered}\n"))
            .map_err(|error| format!("emit write failed: {error}"))?;
        println!(
            "{}",
            json!({
                "schema": COMPLETENESS_SCHEMA,
                "emit": path,
                "cap": 0,
                "cap_burn": 0,
                "cap_free": report["wake"]["cap_free"].clone(),
                "pass_product_clean_room": 0,
                "product": 0,
                "summary": report["summary"].clone(),
            })
        );
    } else {
        println!("{rendered}");
    }
    Ok(0)
}

fn path_exists(root: &Path, relative: &str) -> bool {
    root.join(relative).is_file()
}

fn first_present(root: &Path, candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find(|path| path_exists(root, path))
        .map(|path| (*path).to_string())
}

fn is_measured_component_status(status: &str) -> bool {
    matches!(
        status,
        "pass_component_desktop"
            | "pass_component_hosted"
            | "pass_component_only"
            | "pass_component_desktop_measure"
            | "pass_component_hosted_measure"
            | "pass_component_g4_cef_at_executed"
            | "pass_component_desktop_cap_topology"
            | "fixture_ready"
    )
}

fn wake_state(root: &Path) -> Value {
    let g2_grant = path_exists(root, G2_GRANT_CANDIDATE);
    let g5_grant = path_exists(root, G5_GRANT_CANDIDATE);
    let g6_grant = path_exists(root, G6_GRANT_CANDIDATE);
    let cr_grant = path_exists(root, CR_GRANT_CANDIDATE);
    let rb_grant = path_exists(root, RB_GRANT_CANDIDATE);
    let auth = first_present(root, AUTH_HANDOFF_CANDIDATES);
    let at = first_present(root, AT_HANDOFF_CANDIDATES);
    // Measure Cap free from Unlock A CLOSER only (cap_free_after_unlock_a). Never
    // invent true; never hardcode false when CLOSER already restored Cap free.
    // GRANT presence alone ≠ Cap free.
    let cap_free = crate::orchestration::cap0_product_honesty::cap_free_after_unlock_a(root)
        .unwrap_or(false);
    let note = if cap_free {
        "Unlock A CLOSER.cap_free=true measured; Cap1 serial may proceed once LIVE topology/handoffs allow; never invent pass_product_clean_room / BrowserReady"
    } else if !(g2_grant
        || g5_grant
        || g6_grant
        || cr_grant
        || rb_grant
        || auth.is_some()
        || at.is_some())
    {
        "Live G2/G5/G6/CR/RB GRANT + AUTH/AT absent → Cap realloc not executable; Cap free conserved false; never forge handoffs"
    } else {
        "GRANT and/or handoff presence observed; Cap free measured false until Cap-free-restore / Cap realloc authorizes Cap=1; AUTH/AT still Root when absent; never invent pass_product_clean_room"
    };
    json!({
        "g2_grant_present": g2_grant,
        "g2_grant_candidate": G2_GRANT_CANDIDATE,
        "g5_grant_present": g5_grant,
        "g5_grant_candidate": G5_GRANT_CANDIDATE,
        "g6_grant_present": g6_grant,
        "g6_grant_candidate": G6_GRANT_CANDIDATE,
        "cr_grant_present": cr_grant,
        "cr_grant_candidate": CR_GRANT_CANDIDATE,
        "rb_grant_present": rb_grant,
        "rb_grant_candidate": RB_GRANT_CANDIDATE,
        "auth_handoff_present": auth.is_some(),
        "auth_handoff_path": auth,
        "at_handoff_present": at.is_some(),
        "at_handoff_path": at,
        "cap_free": cap_free,
        "cap_realloc_without_live_topology_authorized": false,
        "note": note
    })
}

fn honest_blocker(scenario: &str, status: &str, wake: &Value) -> (&'static str, String) {
    let g2 = wake["g2_grant_present"].as_bool().unwrap_or(false);
    let g5 = wake["g5_grant_present"].as_bool().unwrap_or(false);
    let auth = wake["auth_handoff_present"].as_bool().unwrap_or(false);
    let at = wake["at_handoff_present"].as_bool().unwrap_or(false);
    let cap_free = wake["cap_free"].as_bool().unwrap_or(false);

    // Measured Cap=1 / component cells hold first — do not re-open as grant/Cap
    // free blockers after Cap topology component EXIT0.
    if is_measured_component_status(status) {
        return (
            "measured_component_hold",
            format!(
                "status={status} measured component/fixture only; never promote to pass_product_clean_room"
            ),
        );
    }

    if scenario == "g2-collector-product-binding" {
        if !g2 {
            return (
                "root_grant_g2",
                "G2 GRANT.json absent; Cap topology TRANSFER + Cap realloc required before Cap=1"
                    .into(),
            );
        }
        if !cap_free {
            return (
                "cap_free_false",
                "G2 GRANT present but Cap free=false until Cap realloc after live topology".into(),
            );
        }
    }
    if scenario == "g5-witness-transport" {
        if !g5 {
            return (
                "root_grant_g5",
                "G5 GRANT.json absent; Cap topology TRANSFER + Cap realloc required before Cap=1"
                    .into(),
            );
        }
        if !cap_free {
            return (
                "cap_free_false",
                "G5 GRANT present but Cap free=false until Cap realloc after live topology".into(),
            );
        }
    }
    let g6 = wake["g6_grant_present"].as_bool().unwrap_or(false);
    if scenario == "g6-origin-audit-binding" {
        if !g6 {
            return (
                "root_grant_g6",
                "G6 GRANT.json absent; Cap topology TRANSFER + Cap realloc required before Cap=1"
                    .into(),
            );
        }
        if !cap_free {
            return (
                "cap_free_false",
                "G6 GRANT present but Cap free=false until Cap realloc after live topology".into(),
            );
        }
    }
    let rb = wake["rb_grant_present"].as_bool().unwrap_or(false);
    if scenario == "red-blue-product" {
        if !rb {
            return (
                "root_grant_rb",
                "RB GRANT.json absent; Cap topology TRANSFER + Cap realloc required before Cap=1 --red-blue-product --require-full".into(),
            );
        }
        if !cap_free {
            return (
                "cap_free_false",
                "RB GRANT present but Cap free=false until Cap realloc after live topology; Cap=0 require-full honesty only".into(),
            );
        }
    }
    if (scenario.contains("n06") || scenario == "w2-h2-hosted-venue") && !auth {
        return (
            "root_auth_handoff",
            "AUTH-HANDOFF absent; N06 owner join / hosted product binding cannot invent join"
                .into(),
        );
    }
    if matches!(scenario, "g4-cef-at" | "browser-product")
        && status == "required_unavailable"
        && !at
    {
        return (
            "root_at_handoff",
            "AT-HANDOFF absent; BrowserReady/interactive AT remains false".into(),
        );
    }
    match status {
        "pass_product_clean_room" => (
            "measured_product",
            "cell already measured pass_product_clean_room".into(),
        ),
        "pass_component_desktop" | "pass_component_hosted" | "pass_component_only"
        | "pass_component_desktop_measure" | "pass_component_hosted_measure"
        | "pass_component_g4_cef_at_executed" | "pass_component_desktop_cap_topology"
        | "fixture_ready" => (
            "measured_component_hold",
            format!("status={status} measured component/fixture only; never promote to pass_product_clean_room"),
        ),
        "required_unavailable" => (
            "root_structural_g1_g9",
            "required_unavailable until Root allocates structural delta (TLS/H2/AT/G1–G9); Cap spam forbidden"
                .into(),
        ),
        "blocked_hosted_linux" => (
            "blocked_hosted_linux",
            "hosted Linux allocation / Specfold provenance still required".into(),
        ),
        "blocked" | "blocked_claim" => (
            "blocked_root_claim",
            format!("status={status}; Root path release / claim still required"),
        ),
        "fail" => (
            "fail_needs_structural_delta",
            "failed gate needs Root-reviewed structural delta before Cap retry".into(),
        ),
        "unrun" => {
            if !cap_free {
                (
                    "cap_free_false",
                    "required cell still unrun and Cap free=false; Cap=1 refuse-closed until Cap realloc + live topology / Root unlock"
                        .into(),
                )
            } else {
                (
                    "unrun_awaiting_cap_or_root",
                    "required cell still unrun; Cap may serialize only after unlock binding".into(),
                )
            }
        }
        "n/a" => ("n_a", "not a required usage-condition cell".into()),
        other => (
            "unknown_status_refuse_closed",
            format!("status={other} has no product promotion path; refuse-closed"),
        ),
    }
}

fn completeness_report(
    matrix: &Value,
    workspace_root: &Path,
    expect_head: Option<&str>,
) -> Result<Value> {
    let combinations = matrix
        .get("combinations")
        .and_then(Value::as_array)
        .ok_or("matrix.combinations array required")?;
    let wake = wake_state(workspace_root);
    let matrix_head = matrix.get("head_sha").and_then(Value::as_str).unwrap_or("");
    let tip_bind_ok = match expect_head {
        Some(expected) => matrix_head.starts_with(expected) || expected.starts_with(matrix_head),
        None => true,
    };

    let mut cells = Vec::new();
    let mut blocker_counts: Map<String, Value> = Map::new();
    let mut required = 0u64;
    let mut clean_room_required = 0u64;
    let mut pass_product = 0u64;
    let present_scenarios: std::collections::BTreeSet<String> = combinations
        .iter()
        .filter_map(|cell| {
            cell.get("scenario")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect();

    for cell in combinations {
        let id = cell
            .get("id")
            .and_then(Value::as_str)
            .ok_or("cell.id required")?;
        let status = cell
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unrun");
        let gate = cell.get("gate").and_then(Value::as_str).unwrap_or("");
        let scenario = cell.get("scenario").and_then(Value::as_str).unwrap_or("");
        let is_required = cell
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let is_clean_room = cell
            .get("clean_room_required")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !is_required {
            continue;
        }
        required += 1;
        if is_clean_room {
            clean_room_required += 1;
        }
        if status == "pass_product_clean_room" {
            pass_product += 1;
        }
        let (blocker, detail) = honest_blocker(scenario, status, &wake);
        let entry = blocker_counts
            .entry(blocker.to_string())
            .or_insert(json!(0));
        if let Some(count) = entry.as_u64() {
            *entry = json!(count + 1);
        }
        cells.push(json!({
            "id": id,
            "env": cell.get("env").cloned().unwrap_or(Value::Null),
            "adapter": cell.get("adapter").cloned().unwrap_or(Value::Null),
            "scenario": scenario,
            "gate": gate,
            "status": status,
            "required": true,
            "clean_room_required": is_clean_room,
            "counts_as_product": cell.get("counts_as_product").cloned().unwrap_or(json!(false)),
            "honest_blocker": blocker,
            "blocker_detail": detail,
            "pass_product_clean_room": status == "pass_product_clean_room",
        }));
    }

    let missing_scenarios: Vec<Value> = SOT_REQUIRED_SCENARIOS
        .iter()
        .filter(|scenario| !present_scenarios.contains(**scenario))
        .map(|scenario| {
            let (blocker, detail) = honest_blocker(scenario, "unrun", &wake);
            json!({
                "scenario": scenario,
                "present_in_matrix": false,
                "honest_blocker": blocker,
                "blocker_detail": format!("SoT-required scenario missing from matrix combinations; {detail}"),
            })
        })
        .collect();

    let completeness_ok = missing_scenarios.is_empty() && tip_bind_ok && pass_product == 0;
    let at_present = wake["at_handoff_present"].as_bool().unwrap_or(false);
    let auth_present = wake["auth_handoff_present"].as_bool().unwrap_or(false);
    let humanymous_browser_app_present =
        path_exists(workspace_root, "crates/apps/humanymous-browser/Cargo.toml");
    let cef_env_present = std::env::var_os("HUMANYMOUS_CEF_PILOT_BUNDLE")
        .as_ref()
        .is_some_and(|value| !value.is_empty());
    let cef_cap_day_present = crate::cef_bundle::cap_day_present(workspace_root).unwrap_or(false);
    // First-class BrowserReady stack blocker (PRIORITY #1) — not buried under
    // AT-HANDOFF token-only or lumped G1/G3/G4. Unlock A ≠ BrowserReady.
    let browser_ready_stack = json!({
        "rank": 1,
        "first_class": true,
        "browser_ready": false,
        "blocker": if at_present { "root_at_present_await_measured_join" } else { "root_at_handoff" },
        "blocker_detail": if at_present {
            "AT-HANDOFF present; Cap=0 measured apply + BrowserReady honesty join still required; never invent BrowserReady"
        } else {
            "AT-HANDOFF absent; BrowserReady/interactive AT remains false; Cap=0 handoff-apply refuse-closed"
        },
        "surfaces": {
            "at_handoff_present": at_present,
            "at_handoff_path": wake.get("at_handoff_path").cloned().unwrap_or(Value::Null),
            "cef_bundle_env_present": cef_env_present,
            "cef_cap_day_present": cef_cap_day_present,
            "cef_bundle_env": "HUMANYMOUS_CEF_PILOT_BUNDLE",
            "cef_durable_bind": crate::cef_bundle::CAP_DAY_BIND_RELATIVE,
            "crates_browser_present": path_exists(workspace_root, "crates/browser/Cargo.toml"),
            "cef_pilot_present": path_exists(workspace_root, "scripts/ci/cef-pilot/Cargo.toml"),
            "humanymous_browser_app_present": humanymous_browser_app_present,
            "humanymous_browser_app_path": "crates/apps/humanymous-browser",
            "unlock_a_ne_browser_ready": true
        },
        "cap0_helpers": {
            "cap0_chain": "cargo run --locked -p humanymous-lab-runner -- cap0-chain",
            "cap0_chain_inspect_only": "cargo run --locked -p humanymous-lab-runner -- cap0-chain --inspect-only",
            "cap0_chain_dry_run": "cargo run --locked -p humanymous-lab-runner -- cap0-chain --dry-run",
            "handoff_apply_at": "cargo run --locked -p humanymous-lab-runner -- handoff-apply --kind at",
            "handoff_apply_auth": "cargo run --locked -p humanymous-lab-runner -- handoff-apply --kind auth",
            "handoff_apply_browser_ready_join": "cargo run --locked -p humanymous-lab-runner -- handoff-apply --kind browser-ready-join",
            "cap_realloc_apply": "cargo run --locked -p humanymous-lab-runner -- cap-realloc-apply --grant GRANT.json --manifest transfer-manifest.json",
            "cef_bundle_bind": "cargo run --locked -p humanymous-lab-runner -- cef-bundle-bind",
            "cef_bundle_probe": "cargo run --locked -p humanymous-lab-runner -- cef-bundle-probe",
            "browser_ready_honesty": "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --browser-ready-interactive-at-honesty",
            "n06_owner_auth_honesty": "cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --n06-owner-auth-handoff-honesty"
        }
    });
    Ok(json!({
        "schema": COMPLETENESS_SCHEMA,
        "record_kind": "lab-runner-clean-room-matrix-completeness",
        "cap": 0,
        "cap_burn": 0,
        "cap_execute": false,
        "product": 0,
        "pass_product": 0,
        "pass_product_clean_room": pass_product,
        "full_product_qualified": false,
        "m3_claim": false,
        "browser_ready": false,
        "tip_bind": {
            "matrix_head_sha": matrix_head,
            "expect_head": expect_head,
            "ok": tip_bind_ok,
        },
        "wake": wake,
        "browser_ready_stack": browser_ready_stack,
        "honesty": {
            "never_invent_pass_product_clean_room": true,
            "never_invent_cap_free": true,
            "never_forge_grant_auth_at": true,
            "never_invent_browser_ready": true,
            "detection_freeze": true,
            "component_ne_product": true,
            "cap_realloc_requires_live_topology": true,
            "browser_ready_stack_first_class": true,
            "unlock_a_ne_browser_ready": true
        },
        "source_matrix": {
            "schema": matrix.get("schema").cloned().unwrap_or(Value::Null),
            "head_sha": matrix_head,
            "observed_utc": matrix.get("observed_utc").cloned().unwrap_or(Value::Null),
            "cell_count": combinations.len(),
        },
        "sot_required_scenarios": SOT_REQUIRED_SCENARIOS,
        "missing_sot_scenarios": missing_scenarios,
        "summary": {
            "required_cells_enumerated": required,
            "clean_room_required_cells": clean_room_required,
            "pass_product_clean_room": pass_product,
            "missing_sot_scenario_count": missing_scenarios.len(),
            "completeness_enumerated": true,
            "completeness_ok_fail_closed": completeness_ok,
            "blocker_counts": blocker_counts,
            "product": 0,
            "browser_ready": false,
            "browser_ready_stack_rank": 1,
            "at_handoff_present": at_present,
            "auth_handoff_present": auth_present
        },
        "cells": cells,
        "blocked_on_root": [
            "PRIORITY#1 BrowserReady stack: AT-HANDOFF humanymous.root-interactive-at-windows-tuple/1 + CEF bundle env + crates/browser/** / scripts/ci/cef-pilot/** (+ crates/apps/humanymous-browser consolidation)",
            "AUTH-HANDOFF humanymous.root-n06-owner-join-auth-handoff/1",
            "Root structural allocation for G1/G3/G4/G7/G8/G9 product deltas (G2/G5/CR/G6 Cap topology holds ≠ product)",
            "optional: Cap-ready SoT path transfer for clean-room usage-condition gate slices"
        ],
        "next": "PRIORITY#1 BrowserReady stack: Root AT-HANDOFF + AUTH-HANDOFF + Cap-free GRANT → Cap=0 cap0-chain (at→browser-ready-join→auth→cap-realloc-apply); CEF env still Cap-day; Cap1 HOLD; never Cap twin spam / invent pass_product_clean_room / BrowserReady; product remains 0 until pass_product_clean_room measured",
    }))
}

fn plan_matrix(matrix: &Value) -> Result<Value> {
    let combinations = matrix
        .get("combinations")
        .and_then(Value::as_array)
        .ok_or("matrix.combinations array required")?;
    let mut cells = Vec::new();
    let mut refuse = 0u64;
    let mut hold = 0u64;
    let mut serialize = 0u64;
    let mut out_of_scope = 0u64;
    let mut blocked_claim = 0u64;
    for cell in combinations {
        let planned = plan_cell(cell)?;
        match planned["disposition"].as_str() {
            Some("refuse_cap") => refuse += 1,
            Some("hold_component") => hold += 1,
            Some("serialize_cap_when_unlocked") => serialize += 1,
            Some("out_of_lab_runner_scope") => out_of_scope += 1,
            Some("blocked_claim") => blocked_claim += 1,
            _ => {}
        }
        cells.push(planned);
    }
    Ok(json!({
        "schema": SCHEMA,
        "record_kind": "lab-runner-clean-room-matrix-cap-plan",
        "cap_execute": false,
        "cap_burn": false,
        "pass_product": 0,
        "pass_product_clean_room": 0,
        "full_product_qualified": false,
        "m3_claim": false,
        "honesty": {
            "never_invent_pass_product_clean_room": true,
            "required_unavailable_refuses_cap": true,
            "component_status_holds_no_require_full_reburn": true,
            "blocked_claim_never_serializes": true,
            "detection_freeze": true,
        },
        "source_matrix": {
            "schema": matrix.get("schema").cloned().unwrap_or(Value::Null),
            "head_sha": matrix.get("head_sha").cloned().unwrap_or(Value::Null),
            "observed_utc": matrix.get("observed_utc").cloned().unwrap_or(Value::Null),
            "cell_count": combinations.len(),
        },
        "summary": {
            "cells": combinations.len(),
            "refuse_cap": refuse,
            "hold_component": hold,
            "serialize_cap_when_unlocked": serialize,
            "out_of_lab_runner_scope": out_of_scope,
            "blocked_claim": blocked_claim,
        },
        "cells": cells,
    }))
}

fn plan_cell(cell: &Value) -> Result<Value> {
    let id = cell
        .get("id")
        .and_then(Value::as_str)
        .ok_or("cell.id required")?;
    let status = cell
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unrun");
    let gate = cell.get("gate").and_then(Value::as_str).unwrap_or("");
    let scenario = cell.get("scenario").and_then(Value::as_str).unwrap_or("");
    let required = cell
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let clean_room_required = cell
        .get("clean_room_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let counts_as_product = cell
        .get("counts_as_product")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let (disposition, reason, command) = decide(
        status,
        gate,
        scenario,
        required,
        clean_room_required,
        counts_as_product,
    );

    let mut object = Map::new();
    object.insert("id".into(), json!(id));
    object.insert("status".into(), json!(status));
    object.insert("gate".into(), json!(gate));
    object.insert("scenario".into(), json!(scenario));
    object.insert("required".into(), json!(required));
    object.insert("clean_room_required".into(), json!(clean_room_required));
    object.insert("counts_as_product".into(), json!(counts_as_product));
    object.insert("disposition".into(), json!(disposition.as_str()));
    object.insert("reason".into(), json!(reason));
    object.insert("cap_execute".into(), json!(false));
    object.insert("pass_product_clean_room".into(), json!(false));
    object.insert("cap_cli_status".into(), json!(cap_cli_status(scenario)));
    object.insert("command".into(), command);
    if disposition == Disposition::BlockedClaim {
        object.insert(
            "root_ask".into(),
            json!({
                "scenario": scenario,
                "cap_cli_status": "blocked_claim",
                "why": [
                    "deployments/lab/INTEGRATION.md: lab-runner must not execute authority/pilot.mjs (foreign prototype debt)",
                    "test/lab/runner freeze allowlist excludes deployments/lab/authority/pilot.mjs and authority-lab Dockerfile inputs",
                    "compose.authority.yaml remains a historical/component composition; Cap --require-full alone cannot close G1/G5/G6",
                    "never invent pass_product_clean_room"
                ],
                "exact_paths": AUTHORITY_NODE_IPC_ROOT_ASK_PATHS,
                "allocate": "Rust-authored lab-runner authority Cap pilot + freeze allowlist expansion, or INTEGRATION amendment authorizing Docker build-only authority inputs; then release Cap CLI execute path to cursor-integration",
                "after_unlock": "matrix-plan may serialize run --evidence BUNDLE --authority-node-ipc --require-full once; Cap=1 burns once with honest exit2/component receipts only"
            }),
        );
    }
    Ok(Value::Object(object))
}

fn decide(
    status: &str,
    gate: &str,
    scenario: &str,
    required: bool,
    clean_room_required: bool,
    counts_as_product: bool,
) -> (Disposition, String, Value) {
    if matches!(
        status,
        "required_unavailable" | "blocked" | "blocked_hosted_linux" | "blocked_claim"
    ) {
        return (
            Disposition::RefuseCap,
            format!(
                "status={status} refuses Cap until Root structural delta; never invent pass_product_clean_room"
            ),
            Value::Null,
        );
    }
    if matches!(
        status,
        "pass_component_desktop"
            | "pass_component_hosted"
            | "pass_component_only"
            | "pass_component_desktop_measure"
            | "pass_component_hosted_measure"
            | "pass_component_g4_cef_at_executed"
            | "pass_component_desktop_cap_topology"
            | "fixture_ready"
    ) {
        return (
            Disposition::HoldComponent,
            format!(
                "status={status} is component/fixture only; Cap --require-full reburn forbidden; never promote to pass_product_clean_room"
            ),
            Value::Null,
        );
    }
    if status == "pass_product_clean_room" {
        // Honesty hold: planner never invents this; if matrix already records it, do not Cap-reburn.
        return (
            Disposition::HoldComponent,
            "matrix already records pass_product_clean_room; planner does not Cap-reburn or invent additional product".into(),
            Value::Null,
        );
    }
    if status == "fail" {
        return (
            Disposition::RefuseCap,
            "failed gate needs Root-reviewed structural delta before Cap retry; no Cap spam".into(),
            Value::Null,
        );
    }
    if !gate.starts_with("lab_runner_") {
        return (
            Disposition::OutOfRunnerScope,
            format!("gate={gate} is outside lab-runner Cap serializer scope"),
            Value::Null,
        );
    }
    if !required || status == "n/a" {
        return (
            Disposition::OutOfRunnerScope,
            "cell not required or n/a; Cap not scheduled".into(),
            Value::Null,
        );
    }
    // Remaining Cap-serializable case: required lab-runner cell still unrun.
    if status != "unrun" {
        return (
            Disposition::RefuseCap,
            format!("status={status} has no Cap serializer rule; refuse closed"),
            Value::Null,
        );
    }
    match cap_cli_status(scenario) {
        "blocked_claim" => {
            return (
                Disposition::BlockedClaim,
                format!(
                    "scenario={scenario} Cap CLI is blocked_claim (Root-owned); selector recognized but not Cap-serializable until Root allocates Rust authority Cap pilot + freeze allowlist / INTEGRATION amendment; bare --require-full would Cap-spam wrong composition; never invent pass_product_clean_room"
                ),
                Value::Null,
            );
        }
        "bound" => {}
        _ => {
            return (
                Disposition::RefuseCap,
                format!(
                    "scenario={scenario} has no lab-runner Cap CLI binding; bare --require-full preimage would Cap-spam the wrong composition; hold until Root allocates a bound path or matrix remaps; never invent pass_product_clean_room"
                ),
                Value::Null,
            );
        }
    }
    if !scenario_has_cap_cli_binding(gate, scenario) {
        return (
            Disposition::RefuseCap,
            format!(
                "scenario={scenario} Cap CLI status is bound but gate={gate} has no serializer rule; refuse closed; never invent pass_product_clean_room"
            ),
            Value::Null,
        );
    }
    let command = lab_runner_command(gate, scenario);
    let reason = if clean_room_required || counts_as_product {
        "required lab-runner cell still unrun; serialize Cap=1 preimage only — execute after Root unlock; product remains 0 until measured pass_product_clean_room".into()
    } else {
        "required lab-runner cell still unrun; serialize Cap=1 preimage only — execute after Root unlock; component-eligible cells still cannot invent product".into()
    };
    (Disposition::SerializeWhenUnlocked, reason, command)
}

/// Cap CLI status for a matrix scenario. `blocked_claim` is recognized but
/// Root-owned until Unlock C TRANSFER apply; only `bound` may serialize.
fn cap_cli_status(scenario: &str) -> &'static str {
    match scenario {
        "browser-product" | "collector-component" | "go-product" | "topology-fixture" => "bound",
        "authority-node-ipc" => match crate::snapshot::root()
            .ok()
            .and_then(|root| crate::unlock_gate::authority_cap_cli_status(&root).ok())
        {
            Some("bound") => "bound",
            _ => "blocked_claim",
        },
        "w2-h2-product-binding" => match crate::snapshot::root()
            .ok()
            .and_then(|root| crate::unlock_gate::w2_h2_cap_cli_status(&root).ok())
        {
            Some("bound") => "bound",
            _ => "blocked_claim",
        },
        "g4-cef-at" => match crate::snapshot::root()
            .ok()
            .and_then(|root| crate::unlock_gate::g4_cef_at_cap_cli_status(&root).ok())
        {
            Some("bound") => "bound",
            _ => "blocked_claim",
        },
        _ => "absent",
    }
}

/// Only scenarios with an executable lab-runner Cap selector may serialize.
/// `authority-node-ipc` serializes only after Unlock C makes Cap CLI `bound`.
/// `w2-h2-product-binding` serializes only after Unlock B Cap CLI is `bound`.
/// `g4-cef-at` serializes only after Unlock A Cap CLI is `bound`.
fn scenario_has_cap_cli_binding(gate: &str, scenario: &str) -> bool {
    if cap_cli_status(scenario) != "bound" {
        return false;
    }
    match scenario {
        "browser-product"
        | "collector-component"
        | "go-product"
        | "authority-node-ipc"
        | "w2-h2-product-binding"
        | "g4-cef-at" => true,
        "topology-fixture" if gate == "lab_runner_topology_or_controls" => true,
        _ => false,
    }
}

fn lab_runner_command(gate: &str, scenario: &str) -> Value {
    let mut args = vec![
        "run".to_string(),
        "--evidence".to_string(),
        "BUNDLE".to_string(),
    ];
    match scenario {
        "browser-product" => args.push("--browser-product".into()),
        "collector-component" => args.push("--collector".into()),
        "go-product" => args.push("--go-product".into()),
        "authority-node-ipc" => args.push("--authority-node-ipc".into()),
        "w2-h2-product-binding" => args.push("--w2-h2-product-binding".into()),
        "g4-cef-at" => args.push("--g4-cef-at".into()),
        "topology-fixture" if gate == "lab_runner_topology_or_controls" => {
            args.push("--topology-only".into());
        }
        _ => {}
    }
    match gate {
        "lab_runner_require_full" => args.push("--require-full".into()),
        "lab_runner_require_go_component" => args.push("--require-go-component".into()),
        "lab_runner_topology_or_controls" => {
            // Topology/controls Cap may later take --require-full only under Root unlock;
            // serializer keeps the scoped form by default to avoid Cap spam preimages.
        }
        _ => {}
    }
    json!(args)
}

#[cfg(test)]
mod tests {
    use super::{
        Disposition, completeness_report, decide, honest_blocker, plan_matrix, wake_state,
    };
    use serde_json::json;
    use std::path::{Path, PathBuf};

    #[test]
    fn completeness_enumerates_required_cells_fail_closed_product_zero() {
        let matrix = json!({
            "schema": "humanymous.cursor-clean-room-product-matrix/1",
            "head_sha": "a984a87101c725e001a672a695f8d41b7b5e4362",
            "combinations": [
                {
                    "id": "desktop-linux|none|m3-full-product|lab_runner_require_full",
                    "env": "desktop-linux",
                    "adapter": "none",
                    "scenario": "m3-full-product",
                    "gate": "lab_runner_require_full",
                    "required": true,
                    "clean_room_required": true,
                    "counts_as_product": true,
                    "status": "required_unavailable"
                },
                {
                    "id": "desktop-linux|none|g2-collector-product-binding|lab_runner_require_full",
                    "env": "desktop-linux",
                    "adapter": "none",
                    "scenario": "g2-collector-product-binding",
                    "gate": "lab_runner_require_full",
                    "required": true,
                    "clean_room_required": true,
                    "counts_as_product": false,
                    "status": "blocked"
                },
                {
                    "id": "desktop-linux|none|browser-product|compose_config",
                    "env": "desktop-linux",
                    "adapter": "none",
                    "scenario": "browser-product",
                    "gate": "compose_config",
                    "required": false,
                    "clean_room_required": false,
                    "counts_as_product": false,
                    "status": "n/a"
                }
            ]
        });
        let root = PathBuf::from(".");
        let report = completeness_report(&matrix, &root, Some("a984a871")).expect("report");
        assert_eq!(report["cap"], 0);
        assert_eq!(report["product"], 0);
        assert_eq!(report["pass_product_clean_room"], 0);
        assert_eq!(report["tip_bind"]["ok"], true);
        assert_eq!(report["summary"]["required_cells_enumerated"], 2);
        assert_eq!(report["honesty"]["never_invent_cap_free"], true);
        let cells = report["cells"].as_array().expect("cells");
        assert_eq!(cells.len(), 2);
        let g2 = cells
            .iter()
            .find(|cell| cell["scenario"] == "g2-collector-product-binding")
            .expect("g2 cell");
        assert_eq!(g2["honest_blocker"], "root_grant_g2");
        let m3 = cells
            .iter()
            .find(|cell| cell["scenario"] == "m3-full-product")
            .expect("m3 cell");
        assert_eq!(m3["honest_blocker"], "root_structural_g1_g9");
    }

    #[test]
    fn g2_blocker_prefers_root_grant_when_absent() {
        let wake = json!({
            "g2_grant_present": false,
            "g5_grant_present": false,
            "auth_handoff_present": false,
            "at_handoff_present": false,
            "cap_free": false
        });
        let (blocker, detail) = honest_blocker("g2-collector-product-binding", "blocked", &wake);
        assert_eq!(blocker, "root_grant_g2");
        assert!(detail.contains("GRANT.json"));
    }

    #[test]
    fn g2_cap_topology_component_holds_without_reopening_grant() {
        let wake = json!({
            "g2_grant_present": true,
            "g5_grant_present": false,
            "auth_handoff_present": false,
            "at_handoff_present": false,
            "cap_free": false
        });
        let (blocker, detail) = honest_blocker(
            "g2-collector-product-binding",
            "pass_component_desktop_cap_topology",
            &wake,
        );
        assert_eq!(blocker, "measured_component_hold");
        assert!(detail.contains("pass_component_desktop_cap_topology"));
        assert!(!detail.contains("GRANT.json"));
    }

    #[test]
    fn wake_state_does_not_invent_cap_free_or_handoffs() {
        let wake = wake_state(Path::new("."));
        // Live Unlock A CLOSER currently measures Cap free=false; never invent true.
        assert_eq!(wake["cap_free"], false);
        assert_eq!(wake["cap_realloc_without_live_topology_authorized"], false);
        assert_eq!(wake["g2_grant_present"], false);
        assert_eq!(wake["g5_grant_present"], false);
        assert_eq!(wake["auth_handoff_present"], false);
        assert_eq!(wake["at_handoff_present"], false);
    }

    #[test]
    fn wake_state_measures_cap_free_from_unlock_a_closer_not_hardcoded() {
        use crate::orchestration::cap0_product_honesty::{
            UNLOCK_A_CLOSER_REL, UNLOCK_A_CLOSER_SCHEMA,
        };
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("hmn-matrix-wake-cap-free-{nanos}"));
        let closer = root.join(UNLOCK_A_CLOSER_REL);
        std::fs::create_dir_all(closer.parent().expect("parent")).expect("mkdir");
        std::fs::write(
            &closer,
            serde_json::to_vec_pretty(&json!({
                "schema": UNLOCK_A_CLOSER_SCHEMA,
                "cap_exit": 0,
                "cap_free": true,
                "product": 0,
                "pass_product_clean_room": 0,
                "browser_ready": false
            }))
            .expect("encode"),
        )
        .expect("write closer");
        let wake = wake_state(&root);
        assert_eq!(
            wake["cap_free"], true,
            "wake_state must measure Cap free from Unlock A CLOSER after Cap-free-restore"
        );
        assert_eq!(wake["at_handoff_present"], false);
        assert_eq!(wake["auth_handoff_present"], false);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn red_blue_product_blocker_prefers_cap_free_when_grant_present() {
        let wake = json!({
            "g2_grant_present": false,
            "g5_grant_present": false,
            "g6_grant_present": false,
            "rb_grant_present": true,
            "auth_handoff_present": false,
            "at_handoff_present": false,
            "cap_free": false
        });
        let (blocker, detail) = honest_blocker("red-blue-product", "unrun", &wake);
        assert_eq!(blocker, "cap_free_false");
        assert!(detail.contains("Cap free=false"));
        assert!(!detail.contains("GRANT.json absent"));
    }

    #[test]
    fn required_unavailable_refuses_cap_and_never_invents_product() {
        let (disposition, reason, command) = decide(
            "required_unavailable",
            "lab_runner_require_full",
            "m3-full-product",
            true,
            true,
            true,
        );
        assert_eq!(disposition, Disposition::RefuseCap);
        assert!(reason.contains("refuses Cap"));
        assert!(command.is_null());
    }

    #[test]
    fn component_status_holds_without_require_full_reburn() {
        for status in [
            "pass_component_desktop",
            "pass_component_hosted",
            "fixture_ready",
        ] {
            let (disposition, reason, command) = decide(
                status,
                "lab_runner_require_full",
                "browser-product",
                true,
                true,
                false,
            );
            assert_eq!(disposition, Disposition::HoldComponent);
            assert!(reason.contains("reburn forbidden"));
            assert!(command.is_null());
        }
    }

    #[test]
    fn unrun_lab_runner_cell_serializes_command_without_executing() {
        let (disposition, _reason, command) = decide(
            "unrun",
            "lab_runner_require_full",
            "browser-product",
            true,
            true,
            false,
        );
        assert_eq!(disposition, Disposition::SerializeWhenUnlocked);
        assert_eq!(
            command,
            json!([
                "run",
                "--evidence",
                "BUNDLE",
                "--browser-product",
                "--require-full"
            ])
        );
    }

    #[test]
    fn go_component_gate_serializes_bounded_requirement() {
        let (disposition, _reason, command) = decide(
            "unrun",
            "lab_runner_require_go_component",
            "go-product",
            true,
            true,
            false,
        );
        assert_eq!(disposition, Disposition::SerializeWhenUnlocked);
        assert_eq!(
            command,
            json!([
                "run",
                "--evidence",
                "BUNDLE",
                "--go-product",
                "--require-go-component"
            ])
        );
    }

    #[test]
    fn authority_node_ipc_serializes_only_when_unlock_c_bound() {
        let (disposition, reason, command) = decide(
            "unrun",
            "lab_runner_require_full",
            "authority-node-ipc",
            true,
            true,
            false,
        );
        match crate::unlock_gate::authority_cap_cli_status(
            &crate::snapshot::root().expect("workspace root"),
        )
        .expect("unlock status")
        {
            "bound" => {
                assert_eq!(disposition, Disposition::SerializeWhenUnlocked);
                assert!(reason.contains("serialize Cap=1"));
                assert!(
                    command
                        .as_array()
                        .expect("command")
                        .iter()
                        .any(|arg| arg.as_str() == Some("--authority-node-ipc"))
                );
            }
            _ => {
                assert_eq!(disposition, Disposition::BlockedClaim);
                assert!(reason.contains("blocked_claim"));
                assert!(reason.contains("never invent pass_product_clean_room"));
                assert!(command.is_null());
            }
        }
    }

    #[test]
    fn plan_matrix_summary_is_fail_closed_and_product_zero() {
        let matrix = json!({
            "schema": "humanymous.cursor-clean-room-product-matrix/1",
            "head_sha": "test",
            "combinations": [
                {
                    "id": "a|m3|lab_runner_require_full",
                    "status": "required_unavailable",
                    "gate": "lab_runner_require_full",
                    "scenario": "m3-full-product",
                    "required": true,
                    "clean_room_required": true,
                    "counts_as_product": true
                },
                {
                    "id": "b|browser|lab_runner_require_full",
                    "status": "pass_component_desktop",
                    "gate": "lab_runner_require_full",
                    "scenario": "browser-product",
                    "required": true,
                    "clean_room_required": true,
                    "counts_as_product": false
                },
                {
                    "id": "c|authority|lab_runner_require_full",
                    "status": "unrun",
                    "gate": "lab_runner_require_full",
                    "scenario": "authority-node-ipc",
                    "required": true,
                    "clean_room_required": true,
                    "counts_as_product": false
                },
                {
                    "id": "d|affinity|engine_affinity_preflight",
                    "status": "unrun",
                    "gate": "engine_affinity_preflight",
                    "scenario": "engine-affinity",
                    "required": true,
                    "clean_room_required": false,
                    "counts_as_product": false
                },
                {
                    "id": "e|browser|lab_runner_require_full|unrun",
                    "status": "unrun",
                    "gate": "lab_runner_require_full",
                    "scenario": "browser-product",
                    "required": true,
                    "clean_room_required": true,
                    "counts_as_product": false
                }
            ]
        });
        let plan = plan_matrix(&matrix).expect("plan");
        assert_eq!(plan["cap_execute"], false);
        assert_eq!(plan["pass_product_clean_room"], 0);
        assert_eq!(plan["full_product_qualified"], false);
        assert_eq!(plan["summary"]["refuse_cap"], 1);
        assert_eq!(plan["summary"]["hold_component"], 1);
        assert_eq!(plan["summary"]["out_of_lab_runner_scope"], 1);
        assert_eq!(
            plan["honesty"]["never_invent_pass_product_clean_room"],
            true
        );
        assert_eq!(plan["honesty"]["blocked_claim_never_serializes"], true);
        let authority = plan["cells"]
            .as_array()
            .expect("cells")
            .iter()
            .find(|cell| cell["scenario"] == "authority-node-ipc")
            .expect("authority cell");
        match authority["cap_cli_status"].as_str() {
            Some("bound") => {
                assert_eq!(plan["summary"]["blocked_claim"], 0);
                assert_eq!(plan["summary"]["serialize_cap_when_unlocked"], 2);
                assert_eq!(authority["disposition"], "serialize_cap_when_unlocked");
                assert!(
                    authority["command"]
                        .as_array()
                        .expect("command")
                        .iter()
                        .any(|arg| arg.as_str() == Some("--authority-node-ipc"))
                );
            }
            _ => {
                assert_eq!(plan["summary"]["blocked_claim"], 1);
                assert_eq!(plan["summary"]["serialize_cap_when_unlocked"], 1);
                assert_eq!(authority["disposition"], "blocked_claim");
                assert!(authority["command"].is_null());
                assert!(
                    authority["root_ask"]["exact_paths"]
                        .as_array()
                        .expect("paths")
                        .iter()
                        .any(|path| path.as_str() == Some("deployments/lab/INTEGRATION.md"))
                );
            }
        }
    }
}
