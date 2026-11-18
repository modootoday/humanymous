//! Increment 2: witness-required Gate with an independent witness container.
//! The runner is the trusted broker. It relays typed audit witness frames
//! between the two containers' private stdio and never invents a signature.
//! After the witness container stops it answers each outstanding or new
//! counter-sign request with the typed `unavailable` refusal, exactly as a
//! broker that lost its witness must. Audit owns framing and verification.
use super::{COMPOSE, ENDPOINT, Lab, PROJECT, Result, last_json_line};
use humanymous_platform::testing::process::ManagedChild;
use humanymous_runtime::audit::checkpoint::service::{
    WitnessServiceRefusal, WitnessServiceResponse, read_frame, write_frame,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Write},
    process::{ChildStdin, ChildStdout, Command},
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

const WITNESS: &str = "humanymous-lab-product-e2e-witness-1";
const WGATE: &str = "humanymous-lab-product-e2e-wgate-1";
const WORIGIN: &str = "humanymous-lab-product-e2e-worigin-1";
const WDRIVER: &str = "humanymous-lab-product-e2e-wdriver-1";
const AWAITING_LOSS: &str = "awaiting_dependency_loss";
const DEPENDENCY_LOST: &str = "dependency-lost\n";

/// Shared broker state; `pending` counts requests forwarded but unanswered.
/// `roles` keeps the Gate's own typed supervisor frames (they carry a `role`
/// and are never witness requests); `head` is the latest checkpoint the
/// witness counter-signed, the independent pin a clean drain needs.
#[derive(Default)]
pub(super) struct State {
    pub(super) lost: bool,
    pending: usize,
    pub(super) gate_to_witness: BTreeMap<String, usize>,
    pub(super) witness_to_gate: BTreeMap<String, usize>,
    pub(super) injected_unavailable: usize,
    pub(super) roles: Vec<Value>,
    pub(super) head: Option<Value>,
}

fn kind(frame: &[u8], key: &str) -> String {
    serde_json::from_slice::<Value>(frame)
        .ok()
        .and_then(|v| v[key].as_str().map(str::to_owned))
        .unwrap_or_else(|| "untyped".into())
}

pub(super) fn frames(output: ChildStdout) -> mpsc::Receiver<std::io::Result<Vec<u8>>> {
    let (send, receive) = mpsc::sync_channel(4);
    std::thread::spawn(move || {
        let mut reader = BufReader::new(output);
        loop {
            match read_frame(&mut reader) {
                Ok(Some(frame)) => {
                    if send.send(Ok(frame)).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    let _ = send.send(Err(error));
                    break;
                }
            }
        }
    });
    receive
}

fn unavailable(gate: &Mutex<ChildStdin>) -> std::io::Result<()> {
    let refused = WitnessServiceResponse::Refused {
        reason: WitnessServiceRefusal::Unavailable,
    };
    write_frame(
        &mut *gate
            .lock()
            .map_err(|_| std::io::Error::other("gate pipe"))?,
        &refused,
    )
}

/// Start both relay directions after the identity handshake.
pub(super) fn relay(
    state: Arc<Mutex<State>>,
    gate_in: Arc<Mutex<ChildStdin>>,
    mut witness_in: ChildStdin,
    from_gate: mpsc::Receiver<std::io::Result<Vec<u8>>>,
    from_witness: mpsc::Receiver<std::io::Result<Vec<u8>>>,
) {
    let (s, g) = (state.clone(), gate_in.clone());
    std::thread::spawn(move || {
        while let Ok(Ok(frame)) = from_witness.recv() {
            let Ok(mut st) = s.lock() else { return };
            st.pending = st.pending.saturating_sub(1);
            let status = kind(&frame, "status");
            if status == "signed"
                && let Ok(value) = serde_json::from_slice::<Value>(&frame)
            {
                st.head = Some(value["checkpoint"].clone());
            }
            *st.witness_to_gate.entry(status).or_default() += 1;
            let Ok(mut gate) = g.lock() else { return };
            if gate.write_all(&frame).and_then(|_| gate.flush()).is_err() {
                return;
            }
        }
        // Witness stdout closed: the container is gone. Never fabricate a head.
        let Ok(mut st) = s.lock() else { return };
        st.lost = true;
        let outstanding = std::mem::take(&mut st.pending);
        for _ in 0..outstanding {
            if unavailable(&g).is_err() {
                return;
            }
            st.injected_unavailable += 1;
        }
    });
    std::thread::spawn(move || {
        while let Ok(Ok(frame)) = from_gate.recv() {
            let Ok(mut st) = state.lock() else { return };
            if let Ok(value) = serde_json::from_slice::<Value>(&frame)
                && value["role"].is_string()
            {
                st.roles.push(value);
                continue;
            }
            *st.gate_to_witness
                .entry(kind(&frame, "operation"))
                .or_default() += 1;
            if !st.lost {
                st.pending += 1;
                if witness_in
                    .write_all(&frame)
                    .and_then(|_| witness_in.flush())
                    .is_ok()
                {
                    continue;
                }
                st.pending -= 1;
                st.lost = true;
            }
            if unavailable(&gate_in).is_err() {
                return;
            }
            st.injected_unavailable += 1;
        }
    });
}

pub(super) fn attach(lab: &mut Lab, name: &str) -> Result<(ManagedChild, ChildStdin, ChildStdout)> {
    let mut command = Command::new("docker");
    command
        .args([
            "--host",
            ENDPOINT,
            "start",
            "--attach",
            "--interactive",
            name,
        ])
        .current_dir(&lab.root);
    lab.transcript
        .push(json!({"argv":["start","--attach","--interactive",name],
        "transport":"private-duplex","input_recorded":false}));
    Ok(ManagedChild::spawn_duplex(&mut command)?)
}

pub(super) fn next(
    receive: &mpsc::Receiver<std::io::Result<Vec<u8>>>,
    what: &str,
) -> Result<Value> {
    let frame = receive
        .recv_timeout(Duration::from_secs(120))
        .map_err(|_| format!("{what}: no frame"))??;
    Ok(serde_json::from_slice(&frame)?)
}

pub(super) fn wait_health(lab: &mut Lab, name: &str, target: &str, seconds: u64) -> Result<Value> {
    let until = Instant::now() + Duration::from_secs(seconds);
    loop {
        let health = lab.json(&["inspect", "--format", "{{json .State.Health.Status}}", name])?;
        if health == target || Instant::now() > until {
            return Ok(health);
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn wait_healthy(lab: &mut Lab, name: &str, seconds: u64) -> Result<Value> {
    wait_health(lab, name, "healthy", seconds)
}

/// Wait for one stdout line carrying `phase`, then return remaining lines.
fn lines(output: ChildStdout) -> mpsc::Receiver<String> {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(output).lines() {
            let Ok(line) = line else { break };
            if send.send(line).is_err() {
                break;
            }
        }
    });
    receive
}

/// Pure judgement over the observed witnessed topology.
pub(super) fn assertions(
    network: &Value,
    containers: &[Value],
    image: &str,
    readiness: &Value,
) -> Vec<(&'static str, bool)> {
    let find = |name: &str| containers.iter().find(|c| c["name"] == name);
    let gate_id = find(WGATE).and_then(|c| c["id"].as_str()).unwrap_or("");
    let shared = format!("container:{gate_id}");
    let witness = find(WITNESS);
    vec![
        (
            "witnessed_network_internal_no_egress",
            network["Internal"] == true,
        ),
        (
            "witnessed_no_published_ports",
            containers
                .iter()
                .all(|c| super::empty(&c["port_bindings"]) && super::empty(&c["published_ports"])),
        ),
        (
            "witness_has_no_network_and_own_uid",
            witness.is_some_and(|w| w["network_mode"] == "none" && w["user"] == "65533:65533"),
        ),
        (
            "witness_shares_no_volume_with_gate",
            witness.is_some_and(|w| w["mounts"].as_array().is_some_and(Vec::is_empty))
                && find(WGATE).is_some_and(|g| g["mounts"].as_array().is_some_and(Vec::is_empty)),
        ),
        (
            "worigin_and_wdriver_share_wgate_namespace",
            !gate_id.is_empty()
                && [WORIGIN, WDRIVER]
                    .iter()
                    .all(|n| find(n).is_some_and(|c| c["network_mode"] == shared.as_str())),
        ),
        (
            "all_witnessed_containers_run_built_image",
            containers.len() == 4 && containers.iter().all(|c| c["image"] == image),
        ),
        (
            "wgate_readiness_reports_external_witness",
            readiness["profile"]["audit_external_witness"] == true,
        ),
    ]
}

/// Product case whose audit-outage injection half this topology measures
/// (owner decision C11, issue #75). Restart/replay and cross-namespace binding
/// stay unqualified, so this topology never emits the case as passed.
pub(super) const AUDIT_RESTART_CASE: &str = "audit-restart";

fn driver_case<'a>(report: &'a Value, id: &str) -> Option<&'a Value> {
    report["driver"]["cases"]
        .as_array()?
        .iter()
        .find(|case| case["id"] == id)
}

/// A Gate-local refusal: the exact status and `{"error":code}` body, no
/// transport error, and no new origin record.
fn refused(case: Option<&Value>, status: u16, codes: &[&str]) -> bool {
    case.is_some_and(|case| {
        case["observed_status"] == status
            && case["origin_delta"] == 0
            && case["transport_error"].is_null()
            && case["gate_local_body"]["error"]
                .as_str()
                .is_some_and(|code| codes.contains(&code))
    })
}

/// Audit-outage injection judgement (C11), computed by the runner from the
/// observed facts and never from the driver's own failed count, so a driver
/// that skips or renames a case cannot pass silently. Per the runtime SoT the
/// first append after the loss fails, poisons the epoch and answers
/// 503 `durable_audit`; later work, including session issue, stays refused.
pub(super) fn outage_assertions(report: &Value) -> Vec<(&'static str, bool)> {
    let issued = driver_case(report, "witness_lost_session_issue_observed");
    vec![
        (
            "audit_outage_injected_after_readiness",
            report["driver_reached_loss_phase"] == true
                && report["health_before_driver"]["witness"] == "healthy"
                && report["health_before_driver"]["wgate"] == "healthy"
                && report["witness_after_loss"]["state"] == "exited"
                && report["witness_after_loss"]["broker_observed_witness_stream_closed"] == true
                && report["broker"]["injected_unavailable_after_loss"]
                    .as_u64()
                    .is_some_and(|n| n > 0),
        ),
        (
            "audit_outage_positive_control_before_loss",
            driver_case(report, "witnessed_signed_effect_forwarded")
                .is_some_and(|case| case["observed_status"] == 200 && case["origin_delta"] == 1),
        ),
        (
            "audit_outage_first_append_poisons_with_durable_audit",
            refused(
                driver_case(report, "witness_lost_presigned_effect_fails_closed"),
                503,
                &["durable_audit"],
            ),
        ),
        (
            "audit_outage_poisoned_epoch_refuses_later_request",
            refused(
                driver_case(report, "witness_lost_unsigned_request_fails_closed"),
                503,
                &["session_epoch", "durable_audit"],
            ),
        ),
        (
            "audit_outage_session_issue_not_granted",
            issued.is_some_and(|case| {
                case["observed_status"]
                    .as_u64()
                    .map_or(!case["transport_error"].is_null(), |code| {
                        !(200..300).contains(&code)
                    })
            }),
        ),
        (
            "audit_outage_readiness_withdrawn",
            report["wgate_readiness_withdrawn_after_failed_append"] == true,
        ),
        (
            "audit_outage_no_fabricated_signature",
            report["broker"]["signatures_fabricated"] == 0,
        ),
    ]
}

/// Run the witnessed topology and its witness-loss-after-readiness case.
/// `report_line` is the captured collector report line (with its newline), or empty.
pub(super) fn run(lab: &mut Lab, image: &str, report_line: &str) -> Result<Value> {
    let compose = [
        "compose",
        "--file",
        COMPOSE,
        "--project-name",
        PROJECT,
        "--profile",
        "witnessed",
    ];
    let create = [
        &compose[..],
        &[
            "create",
            "--no-build",
            "--pull",
            "never",
            "witness",
            "wgate",
        ],
    ]
    .concat();
    lab.docker(&create, 120, true)?;
    let (mut witness_guard, mut witness_in, witness_out) = attach(lab, WITNESS)?;
    let (mut gate_guard, gate_in, gate_out) = attach(lab, WGATE)?;
    let (from_witness, from_gate) = (frames(witness_out), frames(gate_out));
    let writer = next(&from_gate, "writer identity")?;
    if writer["role"] != "writer_identity" {
        return Err("wgate did not announce its writer identity".into());
    }
    write_frame(
        &mut witness_in,
        &json!({"writer_public":writer["writer_public"],"node_id":writer["node_id"]}),
    )?;
    let identity = next(&from_witness, "witness identity")?;
    if identity["role"] != "witness_identity" {
        return Err("witness did not announce its identity".into());
    }
    let gate_in = Arc::new(Mutex::new(gate_in));
    write_frame(
        &mut *gate_in.lock().map_err(|_| "gate pipe")?,
        &json!({"witness_public":identity["witness_public"]}),
    )?;
    let state = Arc::new(Mutex::new(State::default()));
    relay(state.clone(), gate_in, witness_in, from_gate, from_witness);
    let mut report = json!({"handshake":{"writer_identity_announced":true,"witness_identity_announced":true,
        "witness_public_distinct_from_writer":identity["witness_public"]!=writer["writer_public"]}});
    report["health_before_driver"] = json!({
        "witness": wait_healthy(lab, WITNESS, 180)?,
        "wgate": wait_healthy(lab, WGATE, 180)?});
    let up = [
        &compose[..],
        &[
            "up",
            "--detach",
            "--no-recreate",
            "--no-build",
            "--pull",
            "never",
            "--wait",
            "--wait-timeout",
            "180",
            "worigin",
        ],
    ]
    .concat();
    report["worigin_up_waited_healthy"] = json!(lab.docker(&up, 300, false)?.success);
    let create_driver = [
        &compose[..],
        &[
            "create",
            "--no-recreate",
            "--no-build",
            "--pull",
            "never",
            "wdriver",
        ],
    ]
    .concat();
    lab.docker(&create_driver, 120, true)?;
    let (mut driver_guard, mut driver_in, driver_out) = attach(lab, WDRIVER)?;
    // First stdin line: the real collector report, or an empty line so the
    // driver records its absence instead of waiting for one.
    driver_in.write_all(if report_line.is_empty() {
        b"\n"
    } else {
        report_line.as_bytes()
    })?;
    driver_in.flush()?;
    let driver_lines = lines(driver_out);
    let mut seen = Vec::new();
    let until = Instant::now() + Duration::from_secs(240);
    let paused = loop {
        let Ok(line) = driver_lines.recv_timeout(Duration::from_secs(5)) else {
            if Instant::now() > until {
                break false;
            }
            continue;
        };
        let phase = serde_json::from_str::<Value>(&line).is_ok_and(|v| v["phase"] == AWAITING_LOSS);
        seen.push(line);
        if phase {
            break true;
        }
    };
    report["driver_reached_loss_phase"] = json!(paused);
    if paused {
        let before = lab.inspect(WITNESS)?;
        report["witness_before_loss"] = json!({"state":before["state"],"health":before["health"]});
        lab.docker(&["stop", "--time", "2", WITNESS], 60, true)?;
        let until = Instant::now() + Duration::from_secs(30);
        while !state.lock().map_err(|_| "broker state")?.lost && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(100));
        }
        let after = lab.inspect(WITNESS)?;
        let lost = state.lock().map_err(|_| "broker state")?.lost;
        // Before any request after the loss: the latch has not failed yet,
        // because the runtime only learns of the loss on its next append.
        let wgate = lab.inspect(WGATE)?;
        report["witness_after_loss"] = json!({"state":after["state"],"exit_code":after["exit_code"],
            "broker_observed_witness_stream_closed":lost,
            "wgate_health_before_next_append":wgate["health"]});
        driver_in.write_all(DEPENDENCY_LOST.as_bytes())?;
        driver_in.flush()?;
    }
    let until = Instant::now() + Duration::from_secs(120);
    while Instant::now() < until {
        match driver_lines.recv_timeout(Duration::from_secs(5)) {
            Ok(line) => seen.push(line),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    drop(driver_in);
    let driver_exit = driver_guard.wait(Duration::from_secs(30)).ok();
    report["driver_exit_success"] = json!(driver_exit.is_some_and(|s| s.success()));
    report["driver"] = last_json_line(
        &seen.join("\n"),
        "humanymous.product-e2e-witnessed-driver/1",
    )
    .unwrap_or(Value::Null);
    // Readiness probe: the runtime's serving-epoch latch, mirrored to a marker.
    // Docker marks the container unhealthy after three failed probes.
    let health_after = if paused {
        wait_health(lab, WGATE, "unhealthy", 60)?
    } else {
        Value::Null
    };
    let wgate_after = lab.inspect(WGATE)?;
    report["wgate_after_loss"] = json!({"state":wgate_after["state"],"health":health_after,
        "probe":"gate-witnessed: TCP connect and the serving marker mirrored from ServiceHost::epoch_health",
        "note":"the latch fails on the first failed audit append after the loss, not at the moment the witness stops"});
    let logs = lab.docker(&["logs", WGATE], 60, false)?;
    let readiness = last_json_line(&logs.stderr, "humanymous.product-e2e-gate-ready/1")
        .map_or(Value::Null, |value| value["readiness"].clone());
    report["wgate_readiness"] = readiness.clone();
    {
        let st = state.lock().map_err(|_| "broker state")?;
        report["broker"] = json!({"gate_to_witness_operations":st.gate_to_witness,
            "witness_to_gate_statuses":st.witness_to_gate,
            "injected_unavailable_after_loss":st.injected_unavailable,
            "signatures_fabricated":0,"frame_contents_recorded":false});
    }
    let mut containers = Vec::new();
    for name in [WITNESS, WGATE, WORIGIN, WDRIVER] {
        containers.push(lab.inspect(name)?);
    }
    let network = lab.json(&[
        "network",
        "inspect",
        "--format",
        "{{json .}}",
        &format!("{PROJECT}_witnessed"),
    ])?;
    let checks = assertions(&network, &containers, image, &readiness);
    let signed = report["broker"]["witness_to_gate_statuses"]["signed"]
        .as_u64()
        .unwrap_or(0);
    let driver_pass = report["driver_exit_success"] == true
        && report["driver"]["failed"] == 0
        && report["driver"]["driver_error"].is_null();
    let loss_pass = report["witness_after_loss"]["state"] == "exited"
        && report["witness_after_loss"]["broker_observed_witness_stream_closed"] == true;
    let readiness_withdrawn = report["wgate_after_loss"]["health"] == "unhealthy"
        && report["wgate_after_loss"]["state"] == "running";
    report["wgate_readiness_withdrawn_after_failed_append"] = json!(readiness_withdrawn);
    let outage = outage_assertions(&report);
    let outage_pass = outage.iter().all(|(_, p)| *p);
    report["audit_outage_injection"] = json!({"product_case":AUDIT_RESTART_CASE,
        "scope":"audit-outage injection half only; restart/replay and cross-namespace origin/audit binding stay required_unavailable",
        "qualifies_product_case":false,"pass":outage_pass,
        "assertions":outage.iter().map(|(id,pass)|json!({"id":id,"pass":pass})).collect::<Vec<_>>()});
    let pass = driver_pass
        && loss_pass
        && outage_pass
        && readiness_withdrawn
        && signed > 0
        && checks.iter().all(|(_, p)| *p)
        && report["health_before_driver"]["witness"] == "healthy"
        && report["health_before_driver"]["wgate"] == "healthy";
    report["topology"] = json!({"network":{"name":network["Name"],"internal":network["Internal"]},
        "containers":containers,
        "assertions":checks.iter().map(|(id,pass)|json!({"id":id,"pass":pass})).collect::<Vec<_>>()});
    report["witness_counter_signatures_relayed"] = json!(signed);
    report["pass"] = json!(pass);
    let _ = witness_guard.stop();
    let _ = gate_guard.stop();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn container(name: &str, id: &str, mode: &str, user: &str) -> Value {
        json!({"name":name,"id":id,"image":"sha256:i","network_mode":mode,"port_bindings":{},
            "published_ports":{},"user":user,"mounts":[]})
    }
    #[test]
    fn witnessed_topology_requires_isolated_witness_and_shared_gate_namespace() {
        let good = vec![
            container(WITNESS, "w", "none", "65533:65533"),
            container(WGATE, "g", "net_witnessed", "65532:65532"),
            container(WORIGIN, "o", "container:g", "65532:65532"),
            container(WDRIVER, "d", "container:g", "65532:65532"),
        ];
        let internal = json!({"Internal":true});
        let ready = json!({"profile":{"audit_external_witness":true}});
        assert!(
            assertions(&internal, &good, "sha256:i", &ready)
                .iter()
                .all(|(_, p)| *p)
        );
        let mut networked = good.clone();
        networked[0]["network_mode"] = json!("net_witnessed");
        assert!(!assertions(&internal, &networked, "sha256:i", &ready)[2].1);
        let mut same_uid = good.clone();
        same_uid[0]["user"] = json!("65532:65532");
        assert!(!assertions(&internal, &same_uid, "sha256:i", &ready)[2].1);
        let mut shared = good.clone();
        shared[1]["mounts"] = json!([{"Name":"v"}]);
        assert!(!assertions(&internal, &shared, "sha256:i", &ready)[3].1);
        let unwitnessed = json!({"profile":{"audit_external_witness":false}});
        assert!(!assertions(&internal, &good, "sha256:i", &unwitnessed)[6].1);
    }
    /// Shape of the measured witnessed receipts
    /// (.spec/evidence/20261116113042-product-e2e-dns-collector-reports).
    fn measured_outage() -> Value {
        json!({"driver_reached_loss_phase":true,
            "health_before_driver":{"witness":"healthy","wgate":"healthy"},
            "witness_after_loss":{"state":"exited","broker_observed_witness_stream_closed":true},
            "broker":{"injected_unavailable_after_loss":1,"signatures_fabricated":0},
            "wgate_readiness_withdrawn_after_failed_append":true,
            "driver":{"failed":0,"cases":[
                {"id":"witnessed_signed_effect_forwarded","observed_status":200,"origin_delta":1,"pass":true},
                {"id":"witness_lost_presigned_effect_fails_closed","observed_status":503,
                 "gate_local_body":{"error":"durable_audit"},"origin_delta":0,"transport_error":null,"pass":true},
                {"id":"witness_lost_session_issue_observed","observed_status":503,"transport_error":null,
                 "observation_only":true,"pass":true},
                {"id":"witness_lost_unsigned_request_fails_closed","observed_status":503,
                 "gate_local_body":{"error":"session_epoch"},"origin_delta":0,"transport_error":null,"pass":true}]}})
    }
    fn failing(report: &Value) -> Vec<&'static str> {
        outage_assertions(report)
            .into_iter()
            .filter(|(_, pass)| !pass)
            .map(|(id, _)| id)
            .collect()
    }
    fn set_case(report: &mut Value, id: &str, key: &str, value: Value) {
        let cases = report["driver"]["cases"].as_array_mut().unwrap();
        let case = cases.iter_mut().find(|case| case["id"] == id).unwrap();
        case[key] = value;
    }
    #[test]
    fn audit_outage_injection_passes_only_on_measured_fail_closed_shape() {
        assert_eq!(failing(&measured_outage()), Vec::<&str>::new());
        assert_eq!(outage_assertions(&measured_outage()).len(), 7);
    }
    #[test]
    fn audit_outage_driver_self_report_cannot_pass_silently() {
        // A driver that reports failed=0 but never ran the outage cases.
        let mut skipped = measured_outage();
        skipped["driver"]["cases"] = json!([skipped["driver"]["cases"][0].clone()]);
        assert_eq!(
            failing(&skipped),
            [
                "audit_outage_first_append_poisons_with_durable_audit",
                "audit_outage_poisoned_epoch_refuses_later_request",
                "audit_outage_session_issue_not_granted"
            ]
        );
        // No refusal was ever injected: the outage was not exercised.
        let mut idle = measured_outage();
        idle["broker"]["injected_unavailable_after_loss"] = json!(0);
        assert_eq!(failing(&idle), ["audit_outage_injected_after_readiness"]);
        let mut never_paused = measured_outage();
        never_paused["driver_reached_loss_phase"] = json!(false);
        assert_eq!(
            failing(&never_paused),
            ["audit_outage_injected_after_readiness"]
        );
        // Refusals after a broken positive control prove nothing.
        let mut no_control = measured_outage();
        set_case(
            &mut no_control,
            "witnessed_signed_effect_forwarded",
            "origin_delta",
            json!(0),
        );
        assert_eq!(
            failing(&no_control),
            ["audit_outage_positive_control_before_loss"]
        );
    }
    #[test]
    fn audit_outage_requires_poison_and_zero_origin_effect() {
        let presigned = "witness_lost_presigned_effect_fails_closed";
        let first = "audit_outage_first_append_poisons_with_durable_audit";
        for (key, value) in [
            ("origin_delta", json!(1)),
            ("observed_status", json!(200)),
            ("observed_status", json!(403)),
            ("gate_local_body", json!({"error":"session"})),
            ("gate_local_body", Value::Null),
            ("transport_error", json!("reset")),
        ] {
            let mut report = measured_outage();
            set_case(&mut report, presigned, key, value);
            assert_eq!(failing(&report), [first], "{key}");
        }
        let unsigned = "witness_lost_unsigned_request_fails_closed";
        let later = "audit_outage_poisoned_epoch_refuses_later_request";
        let mut poisoned_again = measured_outage();
        set_case(
            &mut poisoned_again,
            unsigned,
            "gate_local_body",
            json!({"error":"durable_audit"}),
        );
        assert!(failing(&poisoned_again).is_empty());
        for (key, value) in [
            ("origin_delta", json!(1)),
            ("observed_status", json!(401)),
            ("gate_local_body", json!({"error":"session"})),
        ] {
            let mut report = measured_outage();
            set_case(&mut report, unsigned, key, value);
            assert_eq!(failing(&report), [later], "{key}");
        }
        let issued = "witness_lost_session_issue_observed";
        let granted = "audit_outage_session_issue_not_granted";
        let mut report = measured_outage();
        set_case(&mut report, issued, "observed_status", json!(200));
        assert_eq!(failing(&report), [granted]);
        let mut silent = measured_outage();
        set_case(&mut silent, issued, "observed_status", Value::Null);
        assert_eq!(failing(&silent), [granted]);
        set_case(&mut silent, issued, "transport_error", json!("closed"));
        assert!(failing(&silent).is_empty());
    }
    #[test]
    fn audit_outage_requires_withdrawn_readiness_and_no_fabrication() {
        let mut ready = measured_outage();
        ready["wgate_readiness_withdrawn_after_failed_append"] = json!(false);
        assert_eq!(failing(&ready), ["audit_outage_readiness_withdrawn"]);
        let mut forged = measured_outage();
        forged["broker"]["signatures_fabricated"] = json!(1);
        assert_eq!(failing(&forged), ["audit_outage_no_fabricated_signature"]);
        assert_eq!(AUDIT_RESTART_CASE, "audit-restart");
    }
    /// The committed Docker receipts that measured readiness withdrawal already
    /// satisfy every outage assertion; the oldest receipt predates readiness
    /// withdrawal and is honestly rejected on that assertion alone.
    #[test]
    fn audit_outage_assertions_hold_on_committed_measured_receipts() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.spec/evidence");
        let witnessed = |bundle: &str| -> Value {
            let bytes = std::fs::read(root.join(bundle).join("receipt.json")).unwrap();
            serde_json::from_slice::<Value>(&bytes).unwrap()["witnessed"].clone()
        };
        for bundle in [
            "20261116110546-product-e2e-browser-collector",
            "20261116113042-product-e2e-dns-collector-reports",
        ] {
            assert_eq!(failing(&witnessed(bundle)), Vec::<&str>::new(), "{bundle}");
        }
        assert_eq!(
            failing(&witnessed("20261116103304-product-e2e-witnessed-health")),
            ["audit_outage_readiness_withdrawn"]
        );
    }
    #[test]
    fn frame_kind_is_typed_or_untyped() {
        assert_eq!(kind(br#"{"status":"signed"}"#, "status"), "signed");
        assert_eq!(kind(b"not json", "status"), "untyped");
    }
}
