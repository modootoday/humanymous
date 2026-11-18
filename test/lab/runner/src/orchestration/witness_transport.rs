//! Roadmap 2b Docker case `runtime-binary-witness-transport`
//! (container-test-environments SoT, "Runtime binary witness transport case").
//!
//! Containers (deployments/lab/compose.rust.witness-transport.yaml):
//! `wtx-supervisor`, a network-none runtime-image container whose trusted lab
//! supervisor starts `audit-witness` and the shipped `humanymous-runtime`
//! binary with the witness passed through inherited descriptors
//! (`audit.witness_input`); and `wtx-origin`, an owned origin in its own
//! container that joins only the supervisor's loopback namespace.
//!
//! The runner generates the synthetic writer, HMAC and witness keys and a
//! wrong witness pin, hands them to the supervisor privately, drives four
//! steps, and judges only what it observes itself: container inspection, the
//! origin container's own contact log, the raw readiness line and HTTP
//! statuses relayed per step, and the WAL records and checkpoints, which it
//! verifies with the keys it generated. No step reports a pass; absent or
//! malformed evidence fails its check.
use super::Lab;
use crate::{docker::Result, snapshot};
use humanymous_platform::testing::json::decode_strict_json;
use humanymous_runtime::audit::{
    checkpoint::{Checkpoint, Witness},
    record::Record,
    verification::verify_with_witness,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
    process::ChildStdout,
    sync::mpsc,
    time::{Duration, Instant},
};

/// Required product case ID (delivery pilot and roadmap capability 2b).
pub(super) const CASE: &str = "runtime-binary-witness-transport";
/// The `required_unavailable` label this case resolves when it passes.
pub(super) const CAPABILITY: &str = "runtime binary authenticated witness transport";
const SUPERVISOR: &str = "wtx-supervisor";
const ORIGIN: &str = "wtx-origin";
/// Supervisor steps, in order, one runner command each.
const STEPS: [&str; 4] = ["wrong_key", "witnessed", "witness_loss", "evidence"];
/// Must equal `NODE` in test/lab/runtime/src/witness_transport.rs.
const NODE: &str = "witness-transport-node";
/// Every check must be present and true for the case to pass.
pub(super) const CHECKS: &[&str] = &[
    "topology-supervisor-network-none",
    "topology-origin-shares-supervisor-loopback-only",
    "origin-contact-log-owned",
    "wrong-key-pin-configured",
    "wrong-key-refused-without-readiness",
    "wrong-key-no-audit-state",
    "wrong-key-no-origin-contact",
    "witnessed-pin-configured",
    "witnessed-readiness",
    "witnessed-exchanges",
    "witnessed-origin-contact",
    "witness-loss-fails-closed",
    "witness-loss-no-origin-contact",
    "durable-audit-co-signed",
    "poisoned-epoch-writes-nothing-further",
];

/// Public keys and the HMAC key the runner generated; seeds stay in `run`.
pub(super) struct Keys {
    pub writer_public: [u8; 32],
    pub witness_public: [u8; 32],
    pub wrong_public: [u8; 32],
    pub hmac: [u8; 32],
}

/// Everything the runner observed. Missing entries stay missing.
#[derive(Default)]
pub(super) struct Evidence {
    pub supervisor_id: String,
    pub supervisor_name: String,
    pub supervisor_network_mode: String,
    pub origin_network_mode: String,
    pub origin_networks: usize,
    pub origin_ready: bool,
    pub origin_log_valid: bool,
    /// Supervisor reply per step.
    pub steps: BTreeMap<String, Value>,
    /// Origin contacts the runner counted before and after each step.
    pub contacts: BTreeMap<String, (usize, usize)>,
    /// First orchestration error; evidence after it is absent.
    pub error: Option<String>,
}

fn public(seed: &[u8; 32]) -> [u8; 32] {
    // The audit Witness derives the Ed25519 verification key from a seed; the
    // writer and node arguments do not affect it.
    Witness::from_seed(seed, [0; 32], String::new()).public_key()
}

fn random<const N: usize>() -> Result<[u8; N]> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes)?;
    Ok(bytes)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn delta(evidence: &Evidence, step: &str) -> Option<usize> {
    evidence
        .contacts
        .get(step)
        .and_then(|(before, after)| after.checked_sub(*before))
}

fn status(exchanges: &Value, name: &str) -> Option<u64> {
    exchanges
        .as_array()?
        .iter()
        .find(|exchange| exchange["name"] == name)?["response"]["status"]
        .as_u64()
}

fn body(exchanges: &Value, name: &str) -> Option<Value> {
    let text = exchanges
        .as_array()?
        .iter()
        .find(|exchange| exchange["name"] == name)?["response"]["body"]
        .as_str()?;
    serde_json::from_str(text).ok()
}

fn issued(exchanges: &Value) -> Option<bool> {
    exchanges
        .as_array()?
        .iter()
        .find(|exchange| exchange["name"] == "session")?["response"]["issued"]
        .as_bool()
}

/// Fail-closed codes after the loss: the failed append itself, then the
/// poisoned serving epoch.
fn closed(exchanges: &Value, name: &str) -> bool {
    status(exchanges, name) == Some(503)
        && body(exchanges, name)
            .is_some_and(|b| b["error"] == "durable_audit" || b["error"] == "session_epoch")
}

fn key(value: &Value) -> Option<[u8; 32]> {
    serde_json::from_value(value.clone()).ok()
}

fn configured(step: &Value, pinned: &[u8; 32], keys: &Keys) -> bool {
    let input = &step["configured"];
    let (read, write) = (input["read_fd"].as_u64(), input["write_fd"].as_u64());
    key(&input["witness_public"]).as_ref() == Some(pinned)
        && key(&input["writer_public"]) == Some(keys.writer_public)
        && input["durability"] == "file_and_directory"
        && input["require_external_witness"] == true
        && read.is_some_and(|fd| fd >= 3)
        && write.is_some_and(|fd| fd >= 3)
        && read != write
}

fn readiness(step: &Value) -> bool {
    let Some(line) = step["readiness_line"].as_str() else {
        return false;
    };
    let Ok(ready) = decode_strict_json(line.trim_end().as_bytes(), 1 << 20) else {
        return false;
    };
    let profile = &ready["profile"];
    profile["audit_external_witness"] == true
        && profile["audit_witness_required_durability"] == "file_and_directory"
        && profile["audit_witness_independence_qualified"] == false
        && profile["product_qualified"] == false
        && ready["listen"]
            .as_str()
            .and_then(|listen| listen.parse::<std::net::SocketAddr>().ok())
            .is_some_and(|address| address.ip().is_loopback())
}

/// Verify the WAL with runner-held keys: every checkpoint writer- and
/// witness-signed, a collected ALLOW decision covered by one, and at most the
/// single record whose checkpoint the lost witness could not co-sign.
fn audit(step: &Value, keys: &Keys) -> (bool, bool) {
    let records: Option<Vec<Record>> = serde_json::from_value(step["records"].clone()).ok();
    let checkpoints: Option<Vec<Checkpoint>> =
        serde_json::from_value(step["checkpoints"].clone()).ok();
    let (Some(records), Some(checkpoints)) = (records, checkpoints) else {
        return (false, false);
    };
    let report = verify_with_witness(
        &records,
        &checkpoints,
        &keys.hmac,
        Some(&keys.writer_public),
        &keys.witness_public,
    );
    let covered = checkpoints.last().map_or(0, |cp| cp.tree_size);
    let decision = records.iter().any(|record| {
        record.event_type == "runtime.decision"
            && record.verdict == "ALLOW"
            && record.seq <= covered
    });
    let collect = records
        .iter()
        .any(|record| record.event_type == "runtime.collect" && record.seq <= covered);
    let signed = report.ok
        && report.scope.hmac_checked
        && !checkpoints.is_empty()
        && report.scope.witnessed_checkpoints == checkpoints.len()
        && checkpoints.iter().all(|cp| {
            cp.node_id == NODE
                && cp.verify_writer(&keys.writer_public)
                && cp.verify_witness(&keys.witness_public)
                && !cp.verify_witness(&keys.wrong_public)
        })
        && records.iter().all(|record| record.node_id == NODE)
        && decision
        && collect;
    let tail = (records.len() as u64).saturating_sub(covered);
    (signed, signed && tail <= 1)
}

/// Pure judgement over runner-observed evidence; one entry per `CHECKS` item.
pub(super) fn evaluate(evidence: &Evidence, keys: &Keys) -> Vec<(&'static str, bool)> {
    let missing = Value::Null;
    let step = |name: &str| evidence.steps.get(name).unwrap_or(&missing);
    let (wrong, witnessed, loss, wal) = (
        step("wrong_key"),
        step("witnessed"),
        step("witness_loss"),
        step("evidence"),
    );
    let origin_mode = evidence.origin_network_mode.strip_prefix("container:");
    let (signed, bounded) = audit(wal, keys);
    let lost = &loss["exchanges"];
    vec![
        (
            "topology-supervisor-network-none",
            evidence.supervisor_network_mode == "none",
        ),
        (
            "topology-origin-shares-supervisor-loopback-only",
            !evidence.supervisor_id.is_empty()
                && origin_mode.is_some_and(|target| {
                    target == evidence.supervisor_id
                        || target == evidence.supervisor_name.trim_start_matches('/')
                })
                && evidence.origin_networks == 0,
        ),
        (
            "origin-contact-log-owned",
            evidence.origin_ready && evidence.origin_log_valid,
        ),
        (
            "wrong-key-pin-configured",
            keys.wrong_public != keys.witness_public && configured(wrong, &keys.wrong_public, keys),
        ),
        (
            "wrong-key-refused-without-readiness",
            wrong["readiness_line"] == ""
                && wrong["exit_code"].as_i64().is_some_and(|code| code != 0),
        ),
        ("wrong-key-no-audit-state", wrong["audit_entries"] == 0),
        (
            "wrong-key-no-origin-contact",
            delta(evidence, "wrong_key") == Some(0),
        ),
        (
            "witnessed-pin-configured",
            configured(witnessed, &keys.witness_public, keys),
        ),
        ("witnessed-readiness", readiness(witnessed)),
        (
            "witnessed-exchanges",
            ["session", "collect", "effect"]
                .iter()
                .all(|name| status(&witnessed["exchanges"], name) == Some(200))
                && issued(&witnessed["exchanges"]) == Some(true)
                && witnessed["witness_head_entries"]
                    .as_u64()
                    .is_some_and(|n| n > 0),
        ),
        (
            "witnessed-origin-contact",
            delta(evidence, "witnessed") == Some(1),
        ),
        (
            "witness-loss-fails-closed",
            loss["witnessed_session"] == true
                // The append that met the lost witness answers durable_audit.
                && status(lost, "effect") == Some(503)
                && body(lost, "effect").is_some_and(|b| b["error"] == "durable_audit")
                && closed(lost, "effect_repeated")
                && closed(lost, "session")
                && issued(lost) == Some(false),
        ),
        (
            "witness-loss-no-origin-contact",
            delta(evidence, "witness_loss") == Some(0) && delta(evidence, "evidence") == Some(0),
        ),
        ("durable-audit-co-signed", signed),
        ("poisoned-epoch-writes-nothing-further", bounded),
    ]
}

/// Passed only with every required check present and true, and no error.
pub(super) fn passed(checks: &[(&'static str, bool)], evidence: &Evidence) -> bool {
    evidence.error.is_none()
        && CHECKS
            .iter()
            .all(|id| checks.iter().any(|(name, ok)| name == id && *ok))
}

/// True only when this run evaluated the case and every instance passed.
pub(super) fn case_passed(cases: &[Value]) -> bool {
    let mut seen = false;
    for case in cases.iter().filter(|case| case["id"] == CASE) {
        if case["status"] != "passed" {
            return false;
        }
        seen = true;
    }
    seen
}

type Lines = mpsc::Receiver<std::io::Result<Vec<u8>>>;

fn reader(output: ChildStdout) -> Lines {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let mut output = BufReader::new(output);
        loop {
            let mut line = Vec::new();
            match Read::take(&mut output, 8 << 20).read_until(b'\n', &mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if send.send(Ok(line)).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = send.send(Err(error));
                    break;
                }
            }
        }
    });
    receive
}

fn next(lines: &Lines, seconds: u64) -> Result<Value> {
    Ok(decode_strict_json(
        &lines.recv_timeout(Duration::from_secs(seconds))??,
        8 << 20,
    )?)
}

/// Origin contacts from the origin container's own log: (ready, valid, count).
fn contacts(lab: &mut Lab, origin: &str) -> Result<(bool, bool, usize)> {
    let logs = lab.docker.call(&["logs", origin])?;
    let (mut ready, mut valid, mut count) = (false, true, 0);
    for line in logs.lines().filter(|line| !line.trim().is_empty()) {
        match decode_strict_json(line.as_bytes(), 4096) {
            Ok(value) if value["role"] == "wtx_origin_ready" => ready = true,
            Ok(value)
                if value["role"] == "wtx_origin_contact"
                    && value["sequence"].as_u64() == Some(count as u64 + 1) =>
            {
                count += 1
            }
            _ => valid = false,
        }
    }
    Ok((ready, valid, count))
}

/// Static stage markers (safe-diagnostics vocabulary) for the stage a failed
/// drive last entered; they name a stage, never an exact failing operation.
pub(super) const PHASES: &[&str] = &[
    "wtx_supervisor_attach",
    "wtx_supervisor_ready",
    "wtx_origin_start",
    "wtx_origin_ready",
    "wtx_step_wrong_key",
    "wtx_step_witnessed",
    "wtx_step_witness_loss",
    "wtx_step_evidence",
    "wtx_shutdown",
    "wtx_supervisor_exit",
];

fn step_phase(step: &str) -> &'static str {
    match step {
        "wrong_key" => "wtx_step_wrong_key",
        "witnessed" => "wtx_step_witnessed",
        "witness_loss" => "wtx_step_witness_loss",
        _ => "wtx_step_evidence",
    }
}

fn send(input: &mut impl Write, value: &Value) -> Result<()> {
    serde_json::to_writer(&mut *input, value)?;
    input.write_all(b"\n")?;
    input.flush()?;
    Ok(())
}

fn drive(
    lab: &mut Lab,
    evidence: &mut Evidence,
    phase: &mut &'static str,
    supervisor: &str,
    origin: &str,
    control: &Value,
) -> Result<()> {
    *phase = "wtx_supervisor_attach";
    let (mut child, mut input, output) =
        lab.docker
            .duplex(&["start".into(), "-a".into(), "-i".into(), supervisor.into()])?;
    let lines = reader(output);
    send(&mut input, control)?;
    *phase = "wtx_supervisor_ready";
    if next(&lines, 120)?["role"] != "wtx_supervisor_ready" {
        return Err("supervisor readiness missing".into());
    }
    *phase = "wtx_origin_start";
    lab.docker.call(&["start", origin])?;
    *phase = "wtx_origin_ready";
    let deadline = Instant::now() + Duration::from_secs(60);
    while !contacts(lab, origin)?.0 {
        if Instant::now() > deadline {
            return Err("owned origin not ready".into());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    for step in STEPS {
        *phase = step_phase(step);
        let before = contacts(lab, origin)?.2;
        send(&mut input, &json!({ "step": step }))?;
        let reply = next(&lines, 180)?;
        // Let the origin container's log catch up, so a late contact is counted.
        std::thread::sleep(Duration::from_secs(1));
        let after = contacts(lab, origin)?.2;
        if reply["step"] != step {
            return Err("supervisor step mismatch".into());
        }
        evidence.steps.insert(step.into(), reply);
        evidence.contacts.insert(step.into(), (before, after));
    }
    let (ready, valid, _) = contacts(lab, origin)?;
    evidence.origin_ready = ready;
    evidence.origin_log_valid = valid;
    // Compose creates the supervisor with stdin open and StdinOnce=false, so
    // closing the attached input never reaches the container as EOF: end the
    // protocol explicitly, as the witness and Go actors do, then close stdin.
    *phase = "wtx_shutdown";
    send(&mut input, &json!({"step":"shutdown"}))?;
    if next(&lines, 60)?["step"] != "shutdown" {
        return Err("supervisor shutdown acknowledgement missing".into());
    }
    drop(input);
    *phase = "wtx_supervisor_exit";
    if !child.wait(Duration::from_secs(60))?.success() {
        return Err("supervisor process failed".into());
    }
    Ok(())
}

/// Failed-case rows for the public diagnostics: the case (with the stage a
/// failed drive last entered) and one static row per failed check ID.
pub(super) fn failure_rows(
    checks: &[(&'static str, bool)],
    phase: Option<&'static str>,
) -> Vec<Value> {
    let mut rows: Vec<Value> = CHECKS
        .iter()
        .filter(|id| !checks.iter().any(|(name, ok)| name == *id && *ok))
        .map(|id| json!({"id":format!("{CASE}--{id}"),"status":"failed","error_code":"assertion_failed"}))
        .collect();
    if let Some(phase) = phase {
        // Only a declared stage marker is projected; anything else is generic.
        let code = PHASES.iter().find(|known| **known == phase);
        rows.push(json!({"id":format!("{CASE}--drive"),"status":"failed",
            "error_code":code.copied().unwrap_or("assertion_failed")}));
    }
    rows
}

pub(super) fn run(lab: &mut Lab) -> Result<()> {
    eprintln!("lab: shipped runtime binary with an inherited-descriptor witness");
    lab.compose(&["create", SUPERVISOR, ORIGIN], true)?;
    let supervisor = lab.inspect(SUPERVISOR)?;
    let origin = lab.inspect(ORIGIN)?;
    let id = |container: &Value| -> Result<String> {
        Ok(container["Id"]
            .as_str()
            .ok_or("container ID absent")?
            .to_owned())
    };
    let (supervisor_id, origin_id) = (id(&supervisor)?, id(&origin)?);
    let mut evidence = Evidence {
        supervisor_id: supervisor_id.clone(),
        supervisor_name: supervisor["Name"].as_str().unwrap_or("").to_owned(),
        supervisor_network_mode: supervisor["HostConfig"]["NetworkMode"]
            .as_str()
            .unwrap_or("")
            .to_owned(),
        origin_network_mode: origin["HostConfig"]["NetworkMode"]
            .as_str()
            .unwrap_or("")
            .to_owned(),
        origin_networks: origin["NetworkSettings"]["Networks"]
            .as_object()
            .map_or(0, |networks| networks.len()),
        ..Default::default()
    };
    let (writer_seed, witness_seed, wrong_seed) = (random()?, random()?, random()?);
    let keys = Keys {
        writer_public: public(&writer_seed),
        witness_public: public(&witness_seed),
        wrong_public: public(&wrong_seed),
        hmac: random()?,
    };
    let control = json!({"writer_seed":writer_seed,"hmac":keys.hmac,"witness_seed":witness_seed,
        "witness_public":keys.witness_public,"writer_public":keys.writer_public,
        "wrong_witness_public":keys.wrong_public,
        "writer_passphrase":hex(&random::<24>()?),"witness_passphrase":hex(&random::<24>()?)});
    lab.stage = "witness_transport_process";
    let mut phase = "wtx_supervisor_attach";
    let mut failed_phase = None;
    if let Err(error) = drive(
        lab,
        &mut evidence,
        &mut phase,
        &supervisor_id,
        &origin_id,
        &control,
    ) {
        evidence.error = Some(error.to_string());
        failed_phase = Some(phase);
        let _ = lab.docker.invoke(
            &[
                "logs".into(),
                "--tail".into(),
                "32".into(),
                supervisor_id.clone(),
            ],
            &[],
            false,
        );
    }
    drop(control);
    let checks = evaluate(&evidence, &keys);
    let ok = passed(&checks, &evidence);
    // Static labels only: check IDs and a stage marker, never evidence text.
    lab.cases.extend(failure_rows(&checks, failed_phase));
    let checks = json!(
        checks
            .iter()
            .map(|(id, ok)| json!({"id":id,"passed":ok}))
            .collect::<Vec<_>>()
    );
    let ledger = json!({"schema":"humanymous.runtime-binary-witness-transport-ledger/1",
        "writer_public":keys.writer_public,"witness_public":keys.witness_public,
        "wrong_witness_public":keys.wrong_public,"supervisor_network_mode":evidence.supervisor_network_mode,
        "origin_network_mode":evidence.origin_network_mode,"steps":evidence.steps,
        "contacts":evidence.contacts,"error":evidence.error,"checks":checks});
    snapshot::write(
        &lab.evidence,
        "runtime-binary-witness-transport-ledger.json",
        &ledger,
    )?;
    lab.components["runtime_binary_witness_transport"] = json!({
        "status": if ok { "passed" } else { "failed" },
        "evidence_type": CASE,
        "checks": checks,
        "transport": "inherited anonymous pipes (audit.witness_input) from the lab supervisor",
        "placement": "audit-witness and the shipped runtime are sibling processes in one container under one UID",
        "witness_independence_qualified": false,
        "product_qualified": false,
    });
    lab.case(
        CASE,
        ok,
        json!({"checks":checks,"error":evidence.error,
            "note":"runner-observed: inspection, origin container log, raw readiness/status relays, WAL verified with runner-generated keys; independence not qualified"}),
    )
}

#[cfg(test)]
mod tests;
