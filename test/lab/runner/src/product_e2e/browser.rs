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
    net::Ipv4Addr,
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
const DNS_NAME: &str = "product-e2e-browser.invalid";
/// Increment 4: the owned DNS canary and its positive-control probes. The
/// canary sits on its own internal network; its address is the browser
/// actor's outside resolver control, so the actor requires its UDP route to
/// be refused, and the canary must log no arrival from the browser.
const DNSCANARY: &str = "humanymous-lab-product-e2e-dnscanary-1";
pub(super) const DNSPROBE_BEFORE: &str = "humanymous-lab-product-e2e-dnsprobe-before";
pub(super) const DNSPROBE_AFTER: &str = "humanymous-lab-product-e2e-dnsprobe-after";
const CANARY_SCHEMA: &str = "humanymous.product-e2e-dns-canary/1";
const PROBE_SCHEMA: &str = "humanymous.product-e2e-dns-probe/1";
const CANARY_CONTROL_NAME: &str = "product-e2e-canary-control.invalid";
/// Wrapper schema the drivers read on stdin (`test/lab/runtime/src/product_e2e/report.rs`).
const REPORT_SCHEMA: &str = "humanymous.product-e2e-collector-report/1";
const REPORT_SOURCE: &str = "humanymous-collector BrowserCollector loader report (report_json path: L1-L3, behavior, partial guard, environment, advanced, fingerprintId) in Firefox ESR (wasm32, feature runtime-pilot), POSTed to /_humanymous/browser/collect and captured at the browser Gate frontend proxy";

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

/// Every JSON line of one schema, in order.
fn schema_lines(text: &str, schema: &str) -> Vec<Value> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .filter(|value| value["schema"] == schema)
        .collect()
}

/// The single IPv4 subnet of an inspected Docker network.
fn subnet(network: &Value) -> Option<String> {
    let configs = network["IPAM"]["Config"].as_array()?;
    let mut v4 = configs
        .iter()
        .filter_map(|c| c["Subnet"].as_str())
        .filter(|s| cidr(s).is_some());
    let first = v4.next()?;
    v4.next().is_none().then(|| first.to_owned())
}

fn cidr(text: &str) -> Option<(Ipv4Addr, u32)> {
    let (address, prefix) = text.split_once('/')?;
    let prefix: u32 = prefix.parse().ok().filter(|p| (8..=32).contains(p))?;
    Some((address.parse().ok()?, prefix))
}

fn within(ip: Ipv4Addr, (network, prefix): (Ipv4Addr, u32)) -> bool {
    let mask = u32::MAX << (32 - prefix);
    u32::from(ip) & mask == u32::from(network) & mask
}

/// Pure judgement over the DNS canary: a live listener whose only arrivals are
/// the answered positive controls from its own network, none from the
/// browser, while the browser's own resolution failed at its deny resolver
/// and its UDP route to the canary was refused.
pub(super) fn dns_canary_checks(
    canary_log: &[Value],
    probes: &[Value],
    browser_subnet: Option<&str>,
    actor: &Value,
    canary: &Value,
    browser: &Value,
) -> Vec<(&'static str, bool)> {
    let arrivals: Vec<&Value> = canary_log
        .iter()
        .filter(|line| line["event"] == "arrival")
        .collect();
    let sources: Vec<&Value> = probes.iter().map(|probe| &probe["source"]).collect();
    vec![
        (
            "dns_canary_listening",
            canary_log.iter().any(|line| line["event"] == "listening"),
        ),
        (
            "dns_canary_positive_controls_answered",
            probes.len() == 2
                && probes
                    .iter()
                    .all(|probe| probe["answered"] == true && probe["rcode"] == 3),
        ),
        (
            "dns_canary_arrivals_only_from_positive_controls",
            arrivals.len() == probes.len()
                && arrivals
                    .iter()
                    .all(|arrival| sources.contains(&&arrival["source"])),
        ),
        (
            "dns_canary_zero_arrivals_from_browser",
            browser_subnet.and_then(cidr).is_some_and(|net| {
                arrivals.iter().all(|arrival| {
                    // An unparsable source is never presumed outside the subnet.
                    arrival["source"]
                        .as_str()
                        .and_then(|ip| ip.parse::<Ipv4Addr>().ok())
                        .is_some_and(|ip| !within(ip, net))
                })
            }),
        ),
        (
            "dns_canary_on_a_network_the_browser_is_not_on",
            canary["network_mode"]
                .as_str()
                .is_some_and(|mode| !mode.is_empty() && !mode.starts_with("container:"))
                && canary["network_mode"] != browser["network_mode"],
        ),
        (
            "browser_dns_resolution_failed_at_its_deny_resolver",
            actor["dns_control"]["local_deny_observed"] == true
                && actor["dns_control"]["lookup_error_kind"].is_string(),
        ),
        (
            "browser_udp_route_to_canary_refused",
            actor["dns_control"]["outside_udp_denied"]["kind"].is_string(),
        ),
    ]
}

/// The stripped epoch's captured collect entity (no guard hooks, as before the
/// two-epoch pilot) as a driver stdin line, plus a receipt summary that omits
/// the bytes. Exactly one entity in that epoch, carried within the frame bound,
/// is required; otherwise the drivers get no report and no fixture.
pub(super) fn collector_report(gate: &Value) -> (Value, Option<String>) {
    let captured = gate["collector_reports"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let stripped = crate::browser_expectations::EPOCHS
        .iter()
        .position(|id| *id == "stripped")
        .and_then(|index| gate["epochs"][index]["collector_reports"].as_array())
        .map(|indices| {
            indices
                .iter()
                .map(|index| {
                    index
                        .as_u64()
                        .and_then(|i| captured.get(i as usize))
                        .cloned()
                        .unwrap_or(Value::Null)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let summary = json!({"captured":captured.len(),"epoch":"stripped","source":REPORT_SOURCE,
        "entities":captured.iter().map(|c| json!({"sha256":c["sha256"],"bytes":c["bytes"],"oversize":c["oversize"]})).collect::<Vec<_>>()});
    let line = match stripped.as_slice() {
        [one] if one["base64"].is_string() => Some(
            json!({"schema":REPORT_SCHEMA,"base64":one["base64"],"sha256":one["sha256"],
                "bytes":one["bytes"],"source":REPORT_SOURCE})
            .to_string(),
        ),
        _ => None,
    };
    (summary, line)
}

/// One positive-control probe from the canary's own network.
fn probe(lab: &mut Lab, compose: &[&str], name: &str, canary: &str) -> Result<Value> {
    let run = [
        compose,
        &[
            "run",
            "--rm",
            "--no-deps",
            "--no-TTY",
            "--interactive=false",
            "--pull",
            "never",
            "--name",
            name,
            "dnsprobe",
            "--product-dns-probe",
            canary,
            CANARY_CONTROL_NAME,
        ],
    ]
    .concat();
    let output = lab.docker(&run, 120, false)?;
    Ok(schema_lines(&output.stdout, PROBE_SCHEMA)
        .pop()
        .unwrap_or(Value::Null))
}

/// Run the browser topology. Every container is torn down by `finish`.
/// Returns the report and, when exactly one was captured, the real collector
/// report line the product drivers submit.
pub(super) fn run(
    lab: &mut Lab,
    image: &str,
    browser_image: &str,
) -> Result<(Value, Option<String>)> {
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
    // The canary listens before the browser starts; a positive control from
    // its own network proves the listener answers.
    let up = [
        &compose[..],
        &[
            "up",
            "--detach",
            "--no-build",
            "--pull",
            "never",
            "dnscanary",
        ],
    ]
    .concat();
    lab.docker(&up, 120, true)?;
    let until = Instant::now() + Duration::from_secs(60);
    while !schema_lines(
        &lab.docker(&["logs", DNSCANARY], 30, false)?.stdout,
        CANARY_SCHEMA,
    )
    .iter()
    .any(|line| line["event"] == "listening")
        && Instant::now() < until
    {
        std::thread::sleep(Duration::from_millis(250));
    }
    let canary = lab.json(&["inspect", "--format", "{{json .}}", DNSCANARY])?;
    let canary_ip = ipv4(&canary).ok_or("dnscanary must have exactly one internal IPv4")?;
    let mut probes = vec![probe(lab, &compose, DNSPROBE_BEFORE, &canary_ip)?];
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
            &canary_ip,
            DNS_NAME,
        ],
    ]
    .concat();
    let driven = lab.docker(&drive, 300, false)?;
    let actor =
        last_json_line(&driven.stdout, "humanymous.browser-product-actor/1").unwrap_or(Value::Null);
    // The listener must still answer after the browser ran, then stop it and
    // read every arrival it logged.
    probes.push(probe(lab, &compose, DNSPROBE_AFTER, &canary_ip)?);
    let canary_stopped = lab
        .docker(&["stop", "--time", "2", DNSCANARY], 60, false)?
        .success;
    let canary_log = schema_lines(
        &lab.docker(&["logs", DNSCANARY], 30, false)?.stdout,
        CANARY_SCHEMA,
    );
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
    for name in [BWITNESS, BGATE, BROWSER, DNSCANARY] {
        containers.push(lab.inspect(name)?);
    }
    let network = lab.json(&[
        "network",
        "inspect",
        "--format",
        "{{json .}}",
        &format!("{PROJECT}_browser"),
    ])?;
    // The browser container has exited, so its address is gone; the browser
    // network's own subnet attributes arrivals instead.
    let browser_subnet = subnet(&network);
    let dns = dns_canary_checks(
        &canary_log,
        &probes,
        browser_subnet.as_deref(),
        &actor,
        &containers[3],
        &containers[2],
    );
    let arrivals: Vec<&Value> = canary_log
        .iter()
        .filter(|line| line["event"] == "arrival")
        .collect();
    report["dns_canary"] = json!({"canary_address":canary_ip,"browser_network_subnet":browser_subnet,
        "canary_stopped":canary_stopped,"positive_controls":probes,"arrivals":arrivals,
        "arrivals_total":arrivals.len(),
        "browser_actor_dns_control":actor["dns_control"],
        "assertions":dns.iter().map(|(id,pass)|json!({"id":id,"pass":pass})).collect::<Vec<_>>(),
        "pass":dns.iter().all(|(_, p)| *p)});
    let (summary, report_line) = collector_report(&gate_result);
    report["collector_report"] = summary;
    let shape = topology(&network, &containers, image, browser_image);
    // The ceiling epoch's decision is observed, never expected: it is filled
    // into the whole-run expectation only when bound to that epoch.
    let ceiling =
        crate::orchestration::coherent_ceiling::decision(&gate_result, &actor, &expected, &fixture)
            .ok();
    let whole = crate::orchestration::coherent_ceiling::with_decision(&expected, ceiling.as_ref());
    let checks = outcome(&actor, &gate_result, &whole, &fixture, head.as_ref());
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
    // The receipt keeps digests and lengths; the bytes go only to the drivers.
    let mut gate_result = gate_result;
    if let Some(entities) = gate_result["collector_reports"].as_array_mut() {
        for entity in entities {
            entity["base64"] = json!(
                entity["base64"]
                    .is_string()
                    .then_some("<omitted; see sha256>")
            );
        }
    }
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
            && dns.iter().all(|(_, p)| *p)
            && report_line.is_some()
    );
    let _ = witness_guard.stop();
    let _ = gate_guard.stop();
    Ok((report, report_line))
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
    fn canary_case() -> (Vec<Value>, Vec<Value>, Value, Value, Value) {
        let log = vec![
            json!({"event":"listening"}),
            json!({"event":"arrival","source":"172.31.0.3"}),
            json!({"event":"arrival","source":"172.31.0.4"}),
        ];
        let probes = vec![
            json!({"source":"172.31.0.3","answered":true,"rcode":3}),
            json!({"source":"172.31.0.4","answered":true,"rcode":3}),
        ];
        let actor = json!({"dns_control":{"local_deny_observed":true,"lookup_error_kind":"Other",
            "outside_udp_denied":{"kind":"NetworkUnreachable"}}});
        (
            log,
            probes,
            actor,
            json!({"network_mode":"net_dnscanary"}),
            json!({"network_mode":"net_browser"}),
        )
    }
    #[test]
    fn dns_canary_requires_live_listener_and_no_browser_arrival() {
        let (log, probes, actor, canary, browser) = canary_case();
        let check = |log: &[Value], probes: &[Value], net: Option<&str>, canary: &Value| {
            dns_canary_checks(log, probes, net, &actor, canary, &browser)
        };
        let ip = Some("172.30.0.0/16");
        // An unknown or empty browser attribution never passes vacuously.
        assert!(!check(&log, &probes, Some(""), &canary)[3].1);
        assert!(check(&log, &probes, ip, &canary).iter().all(|(_, p)| *p));
        // Zero arrivals with a dead listener is not a denial observation.
        assert!(!check(&[], &[], ip, &canary)[1].1);
        let mut leaked = log.clone();
        leaked.push(json!({"event":"arrival","source":"172.30.0.5"}));
        let checks = check(&leaked, &probes, ip, &canary);
        assert!(!checks[2].1 && !checks[3].1);
        assert!(!check(&log, &probes, None, &canary)[3].1);
        assert!(!check(&log, &probes, ip, &browser)[4].1);
        let mut silent = probes.clone();
        silent[1]["answered"] = json!(false);
        assert!(!check(&log, &silent, ip, &canary)[1].1);
    }
    #[test]
    fn subnet_is_the_single_ipv4_config() {
        let network =
            json!({"IPAM":{"Config":[{"Subnet":"172.30.0.0/16","Gateway":"172.30.0.1"}]}});
        assert_eq!(subnet(&network).as_deref(), Some("172.30.0.0/16"));
        assert!(subnet(&json!({"IPAM":{"Config":[]}})).is_none());
        let two =
            json!({"IPAM":{"Config":[{"Subnet":"172.30.0.0/16"},{"Subnet":"172.31.0.0/16"}]}});
        assert!(subnet(&two).is_none());
        let net = cidr("172.30.0.0/16").unwrap();
        assert!(within("172.30.9.9".parse().unwrap(), net));
        assert!(!within("172.31.0.1".parse().unwrap(), net));
        assert!(cidr("172.30.0.0/4").is_none() && cidr("").is_none());
    }
    #[test]
    fn collector_report_line_requires_exactly_one_carried_stripped_entity() {
        let stripped = json!({"sha256":"ab","bytes":2,"base64":"e30=","oversize":false});
        let complete = json!({"sha256":"cd","bytes":7,"base64":"eyJhIjoxfQ==","oversize":false});
        let run = |reports: Value, epochs: Value| {
            collector_report(&json!({"collector_reports":reports,"epochs":epochs}))
        };
        let two = json!([{"collector_reports":[0]},{"collector_reports":[1]}]);
        let (summary, line) = run(json!([stripped, complete]), two.clone());
        assert_eq!(summary["captured"], 2);
        assert_eq!(summary["epoch"], "stripped");
        assert!(summary.to_string().find("e30=").is_none());
        let line: Value = serde_json::from_str(&line.unwrap()).unwrap();
        assert_eq!(line["schema"], REPORT_SCHEMA);
        assert_eq!(line["base64"], "e30=");
        // Never the complete (hooked) epoch's report, and never a fallback.
        let swapped = json!([{"collector_reports":[1]},{"collector_reports":[0]}]);
        let line = run(json!([stripped, complete]), swapped).1.unwrap();
        assert!(line.contains("eyJhIjoxfQ==") && !line.contains("e30="));
        assert!(collector_report(&Value::Null).1.is_none());
        assert!(run(json!([stripped]), Value::Null).1.is_none());
        assert!(
            run(json!([stripped]), json!([{"collector_reports":[3]}]))
                .1
                .is_none()
        );
        let big = json!({"sha256":"ab","bytes":2,"base64":null,"oversize":true});
        assert!(run(json!([big, complete]), two).1.is_none());
        let doubled = json!([{"collector_reports":[0, 0]},{"collector_reports":[1]}]);
        assert!(run(json!([stripped, complete]), doubled).1.is_none());
    }
    #[test]
    fn steps_keep_only_finite_labels() {
        let text = "HM-LAB-ACTOR-STEP actor_browser_test\nnoise\nHM-LAB-ACTOR-STEP Bad Label /x\n";
        assert_eq!(steps(text), vec!["actor_browser_test".to_owned()]);
    }
}
