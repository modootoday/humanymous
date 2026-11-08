//! Fail-closed Cap planner for the Docker clean-room product matrix.
//!
//! Dry-run only: serializes Cap=1 command preimages for later execution after
//! Root unlock. Never invents `pass_product_clean_room`, never burns Cap, and
//! refuses Cap dispatch for `required_unavailable` / blocked / component-hold
//! cells (SoT honesty + no `--require-full` Cap spam).
use crate::docker::Result;
use serde_json::{Map, Value, json};
use std::{fs, path::Path};

const SCHEMA: &str = "humanymous.lab-clean-room-matrix-plan/1";

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
        _ => "absent",
    }
}

/// Only scenarios with an executable lab-runner Cap selector may serialize.
/// `authority-node-ipc` serializes only after Unlock C makes Cap CLI `bound`.
/// `w2-h2-product-binding` serializes only after Unlock B Cap CLI is `bound`.
fn scenario_has_cap_cli_binding(gate: &str, scenario: &str) -> bool {
    if cap_cli_status(scenario) != "bound" {
        return false;
    }
    match scenario {
        "browser-product"
        | "collector-component"
        | "go-product"
        | "authority-node-ipc"
        | "w2-h2-product-binding" => true,
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
    use super::{Disposition, decide, plan_matrix};
    use serde_json::json;

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
