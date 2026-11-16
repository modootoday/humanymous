//! Increment 3: a real browser runs the Rust WASM collector against a
//! witness-required Gate, in its own network namespace.
//!
//! Reuses the existing owned actors instead of a new browser driver:
//! `--browser-gate` (runtime library `ServiceHost::bind_with_witness` plus the
//! lab frontend proxy on :8080) and `--browser-driver` (Firefox ESR through
//! geckodriver, driven by the Rust `wasm-bindgen-test-runner`, running the one
//! fixture-selected `humanymous-collector` runtime pilot test). The Gate's audit
//! checkpoints are counter-signed by the increment 2 witness actor through the
//! same runner broker. The runner records outcomes; it never rewrites them.
use super::witnessed::{State, attach, frames, next, relay};
use super::{COMPOSE, Lab, PROJECT, Result, empty, last_json_line};
use humanymous_runtime::audit::checkpoint::service::write_frame;
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const BWITNESS: &str = "humanymous-lab-product-e2e-bwitness-1";
const BGATE: &str = "humanymous-lab-product-e2e-bgate-1";
pub(super) const BROWSER: &str = "humanymous-lab-product-e2e-browser";
pub(super) const BROWSER_IMAGE: &str = "humanymous/product-e2e-browser:local";
const FIXTURE: &str = "crates/collector/pilot/fixture.json";
/// Audit node of the browser Gate installation
/// (`test/lab/runtime/src/installation/witnessed.rs`). Its writer identity
/// frame omits the node, so the broker names the witness's expected writer.
const NODE: &str = "isolated-node";
/// A private address outside every owned network. The browser actor requires
/// its UDP route to be refused; no canary container listens there.
const UNROUTED_DNS: &str = "10.255.255.254";
const DNS_NAME: &str = "product-e2e-browser.invalid";

fn role(state: &Mutex<State>, name: &str, seconds: u64) -> Result<Value> {
    let until = Instant::now() + Duration::from_secs(seconds);
    loop {
        if let Some(found) = state
            .lock()
            .map_err(|_| "broker state")?
            .roles
            .iter()
            .find(|value| value["role"] == name)
        {
            return Ok(found.clone());
        }
        if Instant::now() > until {
            return Err(format!("{name}: not announced").into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn ipv4(container: &Value) -> Option<String> {
    let networks = container["NetworkSettings"]["Networks"].as_object()?;
    if networks.len() != 1 {
        return None;
    }
    networks.values().next()?["IPAddress"]
        .as_str()
        .map(str::to_owned)
}

/// Finite actor stage labels only; never arbitrary stderr text.
fn steps(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter_map(|line| line.trim().strip_prefix("HM-LAB-ACTOR-STEP "))
        .filter(|label| {
            !label.is_empty()
                && label.len() <= 64
                && label.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
        .map(str::to_owned)
        .collect()
}

/// Pure judgement over the observed browser topology.
pub(super) fn topology(
    network: &Value,
    containers: &[Value],
    image: &str,
    browser_image: &str,
) -> Vec<(&'static str, bool)> {
    let find = |name: &str| containers.iter().find(|c| c["name"] == name);
    let (witness, gate, browser) = (find(BWITNESS), find(BGATE), find(BROWSER));
    vec![
        (
            "browser_network_internal_no_egress",
            network["Internal"] == true,
        ),
        (
            "browser_no_published_ports",
            containers
                .iter()
                .all(|c| empty(&c["port_bindings"]) && empty(&c["published_ports"])),
        ),
        (
            "browser_has_its_own_network_namespace",
            gate.zip(browser).is_some_and(|(g, b)| {
                let mode = b["network_mode"].as_str().unwrap_or("");
                !mode.starts_with("container:") && mode == g["network_mode"]
            }),
        ),
        (
            "bwitness_has_no_network_and_own_uid",
            witness.is_some_and(|w| w["network_mode"] == "none" && w["user"] == "65533:65533"),
        ),
        (
            "bwitness_shares_no_volume_with_bgate",
            witness.zip(gate).is_some_and(|(w, g)| {
                w["mounts"].as_array().is_some_and(Vec::is_empty)
                    && g["mounts"].as_array().is_some_and(Vec::is_empty)
            }),
        ),
        (
            "browser_runs_browser_image_and_gate_witness_run_built_image",
            browser.is_some_and(|b| b["image"] == browser_image)
                && witness
                    .zip(gate)
                    .is_some_and(|(w, g)| w["image"] == image && g["image"] == image),
        ),
        (
            "browser_hardened_sandbox",
            containers
                .iter()
                .all(|c| c["read_only_rootfs"] == true && c["init"] == true),
        ),
    ]
}

/// Pure judgement over the browser actor and Gate drain reports against the
/// collector fixture's own expectations.
pub(super) fn outcome(
    actor: &Value,
    gate: &Value,
    expected: &Value,
    fixture: &Value,
    head: Option<&Value>,
) -> Vec<(String, bool)> {
    let mut checks = vec![
        (
            "browser_wasm_test_inventory_matched".to_owned(),
            actor["test_inventory"]
                .as_array()
                .is_some_and(|names| names.iter().any(|n| *n == actor["selected_test"]))
                && actor["declared_selected"] == 1,
        ),
        (
            "browser_wasm_test_passed".to_owned(),
            actor["status"] == "passed"
                && actor["summary_verified"] == true
                && actor["test_process_success"] == true,
        ),
        (
            "browser_namespace_and_egress_controls".to_owned(),
            actor["namespace_controls"]
                .as_object()
                .is_some_and(|m| !m.is_empty() && m.values().all(|v| *v == true)),
        ),
        (
            "browser_gate_zero_origin_effect".to_owned(),
            gate["origin_effects"] == expected["origin_effects"],
        ),
        (
            "browser_gate_audit_matches_fixture".to_owned(),
            gate["audit_sequence"] == expected["audit_sequence"]
                && gate["audit_records"] == expected["audit_records"]
                && gate["offline_log_authenticated"] == true,
        ),
        (
            "browser_gate_audit_fully_witnessed".to_owned(),
            gate["witness_coverage"]["fully_witnessed"] == true
                && gate["witness_coverage"]["ambiguous_tail_len"] == 0
                && head.is_some_and(|h| {
                    gate["audit_checkpoints"].as_array().and_then(|c| c.last()) == Some(h)
                }),
        ),
        (
            "browser_gate_clean_drain".to_owned(),
            gate["stopped"] == true && gate["checkpoint_repair_performed"] == false,
        ),
    ];
    if let Some(routes) = fixture["routes"].as_object() {
        for (key, path) in routes {
            let count = &expected["request_counts"][key];
            checks.push((
                format!("browser_request_count_{key}"),
                path.as_str().is_some_and(|p| {
                    actor["proxy_hits"][p] == *count && gate["proxy_hits"][p] == *count
                }),
            ));
        }
    }
    checks
}

/// Run the browser topology. Every container is torn down by `finish`.
pub(super) fn run(lab: &mut Lab, image: &str, browser_image: &str) -> Result<Value> {
    let fixture: Value = serde_json::from_slice(&std::fs::read(lab.root.join(FIXTURE))?)?;
    let expected = crate::browser_expectations::from_fixture(&fixture)?;
    let compose = [
        "compose",
        "--file",
        COMPOSE,
        "--project-name",
        PROJECT,
        "--profile",
        "browser",
    ];
    let create = [
        &compose[..],
        &[
            "create",
            "--no-build",
            "--pull",
            "never",
            "bwitness",
            "bgate",
        ],
    ]
    .concat();
    lab.docker(&create, 120, true)?;
    let (mut witness_guard, mut witness_in, witness_out) = attach(lab, BWITNESS)?;
    let (mut gate_guard, gate_in, gate_out) = attach(lab, BGATE)?;
    let (from_witness, from_gate) = (frames(witness_out), frames(gate_out));
    let writer = next(&from_gate, "writer identity")?;
    if writer["role"] != "writer_identity" {
        return Err("bgate did not announce its writer identity".into());
    }
    write_frame(
        &mut witness_in,
        &json!({"writer_public":writer["writer_public"],"node_id":NODE}),
    )?;
    let identity = next(&from_witness, "witness identity")?;
    if identity["role"] != "witness_identity" {
        return Err("bwitness did not announce its identity".into());
    }
    let gate_in = Arc::new(Mutex::new(gate_in));
    write_frame(
        &mut *gate_in.lock().map_err(|_| "gate pipe")?,
        &json!({"witness_public":identity["witness_public"]}),
    )?;
    let state = Arc::new(Mutex::new(State::default()));
    relay(state.clone(), gate_in, witness_in, from_gate, from_witness);
    let mut report = json!({"handshake":{"writer_identity_announced":true,"witness_identity_announced":true,
        "witness_public_distinct_from_writer":identity["witness_public"]!=writer["writer_public"],
        "node_named_by_broker":NODE}});
    let ready = role(&state, "browser_gate_ready", 120)?;
    report["gate_ready"] = json!({"profile":ready["profile"],"hardening":ready["hardening"],
        "frontend":ready["frontend"]});
    let gate = lab.json(&["inspect", "--format", "{{json .}}", BGATE])?;
    let address = ipv4(&gate).ok_or("bgate must have exactly one internal IPv4")?;
    let gate_tuple = format!("{address}:8080");
    let runtime = ready["runtime"].as_str().ok_or("runtime tuple")?.to_owned();
    let backend = ready["backend"].as_str().ok_or("backend tuple")?.to_owned();
    let drive = [
        &compose[..],
        &[
            "run",
            "--no-deps",
            "--no-TTY",
            "--interactive=false",
            "--pull",
            "never",
            "--name",
            BROWSER,
            "browser",
            "--browser-driver",
            &gate_tuple,
            &runtime,
            &backend,
            UNROUTED_DNS,
            DNS_NAME,
        ],
    ]
    .concat();
    let driven = lab.docker(&drive, 300, false)?;
    let actor =
        last_json_line(&driven.stdout, "humanymous.browser-product-actor/1").unwrap_or(Value::Null);
    report["browser_exit_success"] = json!(driven.success);
    report["browser_steps"] = json!(steps(&driven.stderr));
    // Drain with the independently pinned latest witness head, then read the
    // Gate's authenticated offline audit report.
    let head = state.lock().map_err(|_| "broker state")?.head.clone();
    let gate_result = match &head {
        Some(head) => {
            let pin = serde_json::to_string(head)?;
            let stop = lab.docker(
                &[
                    "exec",
                    BGATE,
                    "/usr/local/bin/humanymous-runtime-pilot",
                    "--browser-stop",
                    &pin,
                ],
                60,
                false,
            )?;
            report["drain_requested"] = json!(stop.success);
            role(&state, "browser_gate_result", 60).unwrap_or(Value::Null)
        }
        None => {
            report["drain_requested"] = json!(false);
            Value::Null
        }
    };
    let drained = gate_guard.wait(Duration::from_secs(20)).ok();
    report["gate_exit_success"] = json!(drained.is_some_and(|s| s.success()));
    let mut containers = Vec::new();
    for name in [BWITNESS, BGATE, BROWSER] {
        containers.push(lab.inspect(name)?);
    }
    let network = lab.json(&[
        "network",
        "inspect",
        "--format",
        "{{json .}}",
        &format!("{PROJECT}_browser"),
    ])?;
    let shape = topology(&network, &containers, image, browser_image);
    let checks = outcome(&actor, &gate_result, &expected, &fixture, head.as_ref());
    {
        let st = state.lock().map_err(|_| "broker state")?;
        report["broker"] = json!({"gate_to_witness_operations":st.gate_to_witness,
            "witness_to_gate_statuses":st.witness_to_gate,
            "injected_unavailable":st.injected_unavailable,
            "gate_roles":st.roles.iter().map(|r|r["role"].clone()).collect::<Vec<_>>(),
            "signatures_fabricated":0,"frame_contents_recorded":false});
    }
    let signed = report["broker"]["witness_to_gate_statuses"]["signed"]
        .as_u64()
        .unwrap_or(0);
    report["witness_counter_signatures_relayed"] = json!(signed);
    report["expected"] = expected;
    report["actor"] = actor;
    report["gate"] = gate_result;
    report["topology"] = json!({"network":{"name":network["Name"],"internal":network["Internal"]},
        "containers":containers,
        "assertions":shape.iter().map(|(id,pass)|json!({"id":id,"pass":pass})).collect::<Vec<_>>()});
    report["assertions"] = checks
        .iter()
        .map(|(id, pass)| json!({"id":id,"pass":pass}))
        .collect();
    report["pass"] = json!(
        driven.success
            && signed > 0
            && shape.iter().all(|(_, p)| *p)
            && checks.iter().all(|(_, p)| *p)
    );
    let _ = witness_guard.stop();
    let _ = gate_guard.stop();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn container(name: &str, mode: &str, user: &str, image: &str) -> Value {
        json!({"name":name,"image":image,"network_mode":mode,"port_bindings":{},
            "published_ports":{},"user":user,"mounts":[],"read_only_rootfs":true,"init":true})
    }
    fn good() -> Vec<Value> {
        vec![
            container(BWITNESS, "none", "65533:65533", "sha256:i"),
            container(BGATE, "net_browser", "65532:65532", "sha256:i"),
            container(BROWSER, "net_browser", "65532:65532", "sha256:b"),
        ]
    }
    #[test]
    fn browser_topology_requires_own_namespace_and_isolated_witness() {
        let internal = json!({"Internal":true});
        let check = |c: &[Value]| topology(&internal, c, "sha256:i", "sha256:b");
        assert!(check(&good()).iter().all(|(_, p)| *p));
        let mut shared = good();
        shared[2]["network_mode"] = json!("container:g");
        assert!(!check(&shared)[2].1);
        let mut networked = good();
        networked[0]["network_mode"] = json!("net_browser");
        assert!(!check(&networked)[3].1);
        let mut image = good();
        image[2]["image"] = json!("sha256:i");
        assert!(!check(&image)[5].1);
        assert!(!topology(&json!({"Internal":false}), &good(), "sha256:i", "sha256:b")[0].1);
    }
    #[test]
    fn browser_outcome_never_passes_a_failed_or_absent_test() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../../crates/collector/pilot/fixture.json"
        ))
        .unwrap();
        let expected = crate::browser_expectations::from_fixture(&fixture).unwrap();
        let checks = outcome(&Value::Null, &Value::Null, &expected, &fixture, None);
        assert!(checks.iter().all(|(_, p)| !*p));
        let failed =
            json!({"status":"failed","summary_verified":false,"test_process_success":false});
        let checks = outcome(&failed, &Value::Null, &expected, &fixture, None);
        assert!(!checks[1].1);
    }
    #[test]
    fn steps_keep_only_finite_labels() {
        let text = "HM-LAB-ACTOR-STEP actor_browser_test\nnoise\nHM-LAB-ACTOR-STEP Bad Label /x\n";
        assert_eq!(steps(text), vec!["actor_browser_test".to_owned()]);
    }
}
