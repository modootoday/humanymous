//! Independent browser/runtime/witness orchestration; never a component alias.
use super::{
    Lab,
    witness::{Broker, frame_reader},
};
use crate::{docker::Result, snapshot};
use humanymous_audit::checkpoint::service::write_frame;
use humanymous_test_support::json::decode_strict_json;
use humanymous_test_support::process::capture_with_stderr;
use serde_json::{Value, json};
use std::{
    io::Write,
    sync::mpsc,
    time::{Duration, Instant},
};

fn frame(receive: &mpsc::Receiver<std::io::Result<Vec<u8>>>, seconds: u64) -> Result<Value> {
    Ok(decode_strict_json(
        &receive.recv_timeout(Duration::from_secs(seconds))??,
        65536,
    )?)
}
fn ip(container: &Value) -> Result<String> {
    let networks = container["NetworkSettings"]["Networks"]
        .as_object()
        .ok_or("actor networks")?;
    if networks.len() != 1 {
        return Err("browser actor must have exactly one internal network".into());
    }
    let value = networks
        .values()
        .next()
        .and_then(|v| v["IPAddress"].as_str())
        .ok_or("actor IPv4")?;
    if !value.parse::<std::net::Ipv4Addr>()?.is_private() {
        return Err("owned internal IPv4 required".into());
    }
    Ok(value.into())
}
fn sign(
    broker: &mut Broker,
    lab: &mut Lab,
    public: &Value,
    request: &Value,
    input: &mut std::process::ChildStdin,
) -> Result<()> {
    if request["operation"] != "counter_sign" {
        return Err("unexpected gate witness operation".into());
    }
    let reply = broker.transaction(lab, public, false, request)?;
    write_frame(input, &reply[2])?;
    input.flush()?;
    Ok(())
}
fn dns_snapshot(lab: &mut Lab, id: &str) -> Result<Value> {
    Ok(decode_strict_json(
        lab.docker
            .call(&[
                "exec",
                id,
                "/usr/local/bin/humanymous-lab-runner",
                "fixture",
                "dns-snapshot",
            ])?
            .as_bytes(),
        65536,
    )?)
}

pub(super) fn run(lab: &mut Lab) -> Result<()> {
    lab.stage = "browser_config";
    let config = lab.compose(&["config", "--format", "json"], true)?.stdout;
    let parsed: Value = serde_json::from_str(&config)?;
    snapshot::write(&lab.evidence, "resolved-compose.json", &parsed)?;
    lab.case(
        "browser-product-no-published-ports",
        parsed["services"]
            .as_object()
            .is_some_and(|s| s.values().all(|v| v.get("ports").is_none())),
        json!("separate profile; no host ports"),
    )?;
    lab.stage = "browser_gate_build";
    lab.compose(&["build", "browser-gate"], true)?;
    lab.compose(&["build", "dns-canary"], true)?;
    lab.compose(
        &["up", "-d", "--wait", "--wait-timeout", "30", "dns-canary"],
        true,
    )?;
    let dns_canary = lab.inspect("dns-canary")?;
    let dns_id = dns_canary["Id"].as_str().ok_or("DNS canary ID")?.to_owned();
    lab.docker.call(&[
        "exec",
        &dns_id,
        "/usr/local/bin/humanymous-lab-runner",
        "fixture",
        "dns-positive",
    ])?;
    let dns_before = dns_snapshot(lab, &dns_id)?;
    snapshot::write(&lab.evidence, "browser-dns-canary-before.json", &dns_before)?;
    lab.case(
        "browser-dns-positive-control",
        dns_before["received"] == 1 && dns_before["invalid"] == 0,
        json!("actual caps0 UID65532 UDP53 query and exact NXDOMAIN response"),
    )?;
    let control_image: Value = serde_json::from_str(&lab.docker.call(&[
        "image",
        "inspect",
        dns_canary["Image"].as_str().ok_or("DNS control image")?,
    ])?)?;
    snapshot::write(&lab.evidence, "controls-image.json", &control_image[0])?;
    lab.stage = "browser_image_build";
    lab.compose(&["build", "browser-product"], true)?;
    lab.stage = "browser_gate_start";
    lab.compose(&["create", "browser-gate"], true)?;
    let gate = lab.inspect("browser-gate")?;
    {
        let (role, actor) = ("runtime", &gate);
        let image: Value = serde_json::from_str(&lab.docker.call(&[
            "image",
            "inspect",
            actor["Image"].as_str().ok_or("image ID")?,
        ])?)?;
        snapshot::write(&lab.evidence, &format!("{role}-image.json"), &image[0])?;
    }
    let id = gate["Id"].as_str().ok_or("gate ID")?;
    let (mut child, mut input, output) =
        lab.docker
            .duplex(&["start".into(), "-a".into(), "-i".into(), id.into()])?;
    let receive = frame_reader(output);
    let identity = match frame(&receive, 20) {
        Ok(value) => value,
        Err(error) => {
            let _ = super::startup_failure::capture(lab, id);
            return Err(error);
        }
    };
    if identity["role"] != "writer_identity" {
        return Err("gate writer identity missing".into());
    }
    let public = identity["writer_public"].clone();
    let mut passphrase = vec![0; 32];
    getrandom::fill(&mut passphrase)?;
    let mut broker = Broker {
        passphrase,
        public: None,
        writer_public: Some(public.clone()),
        head: None,
        transactions: 0,
        inspections: Vec::new(),
    };
    lab.stage = "browser_witness_handshake";
    let initial = broker.transaction(lab, &public, true, &json!({"operation":"state"}))?;
    write_frame(&mut input, &json!({"witness_public":broker.public}))?;
    write_frame(&mut input, &initial[1])?;
    input.flush()?;
    let ready = loop {
        let value = frame(&receive, 20)?;
        if value["role"] == "browser_gate_ready" {
            break value;
        }
        sign(&mut broker, lab, &public, &value, &mut input)?;
    };
    snapshot::write(&lab.evidence, "browser-gate-readiness.json", &ready)?;
    lab.case(
        "browser-actual-host-requires-external-witness",
        ready["profile"]["audit_external_witness"] == true
            && ready["profile"]["audit_witness_required_durability"] == "file_and_directory",
        ready["profile"].clone(),
    )?;
    let running_gate = lab.inspect("browser-gate")?;
    lab.environment.extend([
        ("HM_BROWSER_DNS_CANARY".into(), ip(&dns_canary)?),
        (
            "HM_BROWSER_DNS_NAME".into(),
            format!(
                "lab-{}.invalid",
                lab.project
                    .strip_prefix("hm-rust-lab-")
                    .ok_or("owned project")?
            ),
        ),
        (
            "HM_BROWSER_GATE".into(),
            format!("{}:8080", ip(&running_gate)?),
        ),
        (
            "HM_BROWSER_RUNTIME".into(),
            ready["runtime"]
                .as_str()
                .ok_or("runtime readiness tuple")?
                .into(),
        ),
        (
            "HM_BROWSER_BACKEND".into(),
            ready["backend"]
                .as_str()
                .ok_or("backend readiness tuple")?
                .into(),
        ),
    ]);
    let resolved = lab.compose(&["config", "--format", "json"], true)?.stdout;
    snapshot::write(
        &lab.evidence,
        "resolved-compose-browser-ready.json",
        &serde_json::from_str::<Value>(&resolved)?,
    )?;
    lab.stage = "browser_actor";
    lab.compose(&["create", "browser-product"], true)?;
    let browser = lab.inspect("browser-product")?;
    lab.case(
        "browser-explicit-deny-only-dns",
        browser["HostConfig"]["Dns"] == json!(["127.0.0.1"])
            && gate["HostConfig"]["Dns"] == json!(["127.0.0.1"]),
        json!("no public/default resolver configuration"),
    )?;
    let image: Value = serde_json::from_str(&lab.docker.call(&[
        "image",
        "inspect",
        browser["Image"].as_str().ok_or("browser image ID")?,
    ])?)?;
    snapshot::write(&lab.evidence, "browser-product-image.json", &image[0])?;
    let mut command = lab.docker.command(
        &[
            "start".into(),
            "-a".into(),
            browser["Id"].as_str().ok_or("browser ID")?.into(),
        ],
        &[],
    )?;
    let raw_path = lab.evidence.join("browser-process-output.json");
    let browser_result = std::thread::scope(|scope| -> Result<_> {
        let (send, done) = mpsc::sync_channel(1);
        let handle = scope.spawn(move || {
            let outcome =
                capture_with_stderr(&mut command, b"", Duration::from_secs(130), 2 * 1024 * 1024);
            let raw=match &outcome {
                Ok(result)=>json!({"stdout":String::from_utf8_lossy(&result.stdout),"stderr":String::from_utf8_lossy(&result.stderr),"exit":result.status.code()}),
                Err(error)=>json!({"capture_error":error.to_string(),"partial_output_unavailable":true}),
            };
            std::fs::write(raw_path,serde_json::to_vec_pretty(&raw).map_err(std::io::Error::other)?)?;
            let _ = send.send(());
            outcome
        });
        let deadline = Instant::now() + Duration::from_secs(135);
        let mut isolated = false;
        loop {
            if done.try_recv().is_ok() {
                break;
            }
            if Instant::now() >= deadline {
                return Err("browser product supervision deadline".into());
            }
            match receive.recv_timeout(Duration::from_millis(100)) {
                Ok(bytes) => {
                    let value = decode_strict_json(&bytes?, 65536)?;
                    if !isolated {
                        let running_browser = lab.inspect("browser-product")?;
                        let a = running_gate["NetworkSettings"]["SandboxID"]
                            .as_str()
                            .unwrap_or("");
                        let b = running_browser["NetworkSettings"]["SandboxID"]
                            .as_str()
                            .unwrap_or("");
                        lab.case(
                            "browser-gate-distinct-network-namespaces",
                            !a.is_empty() && !b.is_empty() && a != b,
                            json!({"gate":a,"browser":b}),
                        )?;
                        isolated = true;
                    }
                    sign(&mut broker, lab, &public, &value, &mut input)?;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(error) => return Err(error.into()),
            }
        }
        lab.case(
            "browser-actual-witness-exchange",
            isolated && broker.transactions > 1,
            json!(broker.transactions),
        )?;
        handle
            .join()
            .map_err(|_| "browser capture worker failed")?
            .map_err(Into::into)
    })?;
    let actor = decode_strict_json(&browser_result.stdout, 2 * 1024 * 1024)?;
    snapshot::write(&lab.evidence, "browser-actor-report.json", &actor)?;
    let dns_after = dns_snapshot(lab, &dns_id)?;
    snapshot::write(&lab.evidence, "browser-dns-canary-after.json", &dns_after)?;
    let hash = actor["dns_control"]["query_hash"]
        .as_str()
        .ok_or("DNS query commitment")?;
    lab.case("browser-no-external-dns-arrival",dns_after==dns_before&&dns_after["question_hash_counts"].get(hash).is_none()&&actor["dns_policy"]["question_hash_counts"][hash].as_u64().is_some_and(|n|n>0)&&actor["dns_policy"]["forwards"]==0,json!("owned healthy outside canary saw zero new queries; actual libc lookup reached deny-only loopback policy"))?;
    lab.stage = "browser_runtime_drain";
    // Only this trusted Docker control action asks the private actor to drain.
    lab.docker.call(&[
        "exec",
        id,
        "/usr/local/bin/humanymous-runtime-pilot",
        "--browser-stop",
        &serde_json::to_string(
            broker
                .head
                .as_ref()
                .ok_or("independent external head missing")?,
        )?,
    ])?;
    let report = loop {
        let value = frame(&receive, 20)?;
        if value["role"] == "browser_gate_result" {
            break value;
        }
        sign(&mut broker, lab, &public, &value, &mut input)?;
    };
    drop(input);
    let status = child.wait(Duration::from_secs(15))?;
    snapshot::write(&lab.evidence, "browser-gate-report.json", &report)?;
    lab.stage = "browser_receipts";
    lab.case(
        "browser-product-actual-test-completion",
        browser_result.status.success() && actor["status"] == "passed",
        json!("one exact feature-gated Rust WASM test"),
    )?;
    lab.case(
        "browser-product-zero-origin-effect",
        report["origin_effects"] == 0
            && report["offline_log_authenticated"] == true
            && report["audit_sequence"]==2
            && report["audit_records"]==json!([{"seq":1,"event_type":"runtime.decision","verdict":"CHALLENGE"},{"seq":2,"event_type":"runtime.collect","verdict":"COLLECTED"}]),
        json!({"effects":report["origin_effects"],"sequence":report["audit_sequence"]}),
    )?;
    lab.case(
        "browser-product-clean-runtime-drain",
        status.success() && report["stopped"] == true,
        json!("actual audit flush and custody drop before Stopped"),
    )?;
    let frozen = lab
        .environment
        .iter()
        .find(|(name, _)| name == "HM_LAB_BUILD_CONTEXT")
        .ok_or("frozen context missing")?;
    let fixture: Value = serde_json::from_slice(&std::fs::read(
        std::path::Path::new(&frozen.1).join("crates/collector/pilot/fixture.json"),
    )?)?;
    for (key, status, ack) in [
        ("effect", 403, true),
        ("collect", 200, true),
        ("required_collector", 422, false),
    ] {
        let path = fixture["routes"][key].as_str().ok_or("fixture route")?;
        let response = format!("response:{path}:{status}:ack_{ack}");
        lab.case(
            &format!("browser-response-{key}"),
            report["proxy_hits"][&response] == 1 && actor["proxy_hits"][&response] == 1,
            json!(response),
        )?;
    }
    for (key, route) in fixture["routes"].as_object().ok_or("fixture routes")? {
        let route = route.as_str().ok_or("fixture path")?;
        lab.case(
            &format!("browser-single-request-{key}"),
            actor["proxy_hits"][route] == 1 && report["proxy_hits"][route] == 1,
            json!("no automatic retry; closed epoch creates no further effect"),
        )?;
    }
    lab.case(
        "browser-assets-use-owned-proxy",
        actor["proxy_hits"]["assets"]
            .as_u64()
            .is_some_and(|n| n > 0),
        json!("actual localhost proxy hit, not preference acceptance"),
    )?;
    lab.case(
        "browser-origin-and-egress-boundaries",
        actor["namespace_controls"].as_object().is_some_and(|m| {
            const REQUIRED: [&str; 10] = [
                "external_dns_local_policy_refused",
                "external_dns_udp_no_route",
                "runtime_loopback_unreachable",
                "backend_loopback_unreachable",
                "runtime_bridge_port_unreachable",
                "backend_bridge_port_unreachable",
                "metadata_egress_refused",
                "no_default_ipv4_route",
                "foreign_proxy_authority_refused",
                "unknown_proxy_api_refused",
            ];
            m.len() == REQUIRED.len() && REQUIRED.iter().all(|id| m.get(*id) == Some(&json!(true)))
        }),
        actor["namespace_controls"].clone(),
    )?;
    let checkpoints = report["audit_checkpoints"]
        .as_array()
        .ok_or("authenticated audit checkpoints missing")?;
    lab.case("browser-audit-matches-independent-head",checkpoints.last()==broker.head.as_ref(),json!("actual runtime WAL authenticated offline; external latest signed head retained by broker"))?;
    let coverage = &report["witness_coverage"];
    lab.case(
        "browser-audit-fully-covered-with-external-pin",
        coverage["fully_witnessed"] == true
            && coverage["ambiguous_tail_len"] == 0
            && coverage["durable_sequence"] == 2
            && coverage["witnessed_through"] == 2
            && coverage["authenticated_external_head"]
                == broker.head.as_ref().ok_or("external coverage pin")?.clone()
            && coverage["authentication"]["hmac_checked"] == true
            && coverage["authentication"]["writer_checked"] == true
            && coverage["authentication"]["witness_checked"] == true
            && report["checkpoint_repair_performed"] == false,
        coverage.clone(),
    )?;
    lab.compose(&["create", "witness-key-denied"], true)?;
    let denied = lab.inspect("witness-key-denied")?;
    let negative = lab.docker.invoke(
        &[
            "start".into(),
            "-a".into(),
            denied["Id"].as_str().ok_or("wrong UID actor ID")?.into(),
        ],
        &[],
        false,
    )?;
    lab.case(
        "browser-profile-witness-key-eacces",
        negative.success,
        json!("UID65532 actual populated UID65533 witness keys read denied"),
    )?;
    let result = json!({"schema":"humanymous.browser-runtime-product/1","status":"passed","browser":actor,"runtime":report,"witness_public":broker.public,"pinned_checkpoint":broker.head,"witness_transactions":broker.transactions,"full_product_qualified":false,"not_qualified":["human/AT","browser sandbox completeness","original TLS/H2","qualified collector provenance","physical power loss","organizational witness independence","browser-profile witness outage and recovery"]});
    snapshot::write(&lab.evidence, "browser-product-report.json", &result)?;
    lab.components["browser_product"] = result;
    Ok(())
}
