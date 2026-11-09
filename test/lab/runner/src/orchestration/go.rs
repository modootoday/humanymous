//! Eight real isolated host cases; failures remain in the fixed ledger.
use super::{
    Lab, go_fixture,
    witness::{Broker, frame_reader},
};
use crate::{
    docker::{Docker, Result},
    snapshot,
};
use humanymous_platform::testing::{json::decode_strict_json, process::capture_with_stderr};
use humanymous_runtime::audit::checkpoint::service::write_frame;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    sync::mpsc,
    time::{Duration, Instant},
};

fn frame(rx: &mpsc::Receiver<std::io::Result<Vec<u8>>>) -> Result<Value> {
    Ok(decode_strict_json(
        &rx.recv_timeout(Duration::from_secs(20))??,
        65536,
    )?)
}
fn named_ip(container: &Value, suffix: &str) -> Result<String> {
    let entries = container["NetworkSettings"]["Networks"]
        .as_object()
        .ok_or("Go actual networks")?;
    let values: Vec<_> = entries
        .iter()
        .filter(|(n, _)| n.ends_with(suffix))
        .collect();
    if values.len() != 1 {
        return Err("Go owned network membership missing/ambiguous".into());
    }
    let value = values[0].1["IPAddress"]
        .as_str()
        .ok_or("Go actual address")?;
    if !value.parse::<std::net::Ipv4Addr>()?.is_private() {
        return Err("Go private IPv4 required".into());
    }
    Ok(value.into())
}
fn sign(
    broker: &mut Broker,
    lab: &mut Lab,
    writer: &Value,
    value: &Value,
    input: &mut std::process::ChildStdin,
) -> Result<()> {
    if value["operation"] != "counter_sign" {
        return Err("Go unexpected private witness frame".into());
    }
    let reply = broker.transaction(lab, writer, false, value)?;
    write_frame(input, &reply[2])?;
    input.flush()?;
    Ok(())
}
fn set(lab: &mut Lab, name: &str, value: String) {
    lab.environment.retain(|(n, _)| n != name);
    lab.environment.push((name.into(), value));
}
fn isolate(lab: &mut Lab, gate: &Value, backend: &Value, client: &Value) -> Result<()> {
    let g = gate["NetworkSettings"]["Networks"]
        .as_object()
        .ok_or("Gate network inventory")?;
    let b = backend["NetworkSettings"]["Networks"]
        .as_object()
        .ok_or("backend network inventory")?;
    let c = client["NetworkSettings"]["Networks"]
        .as_object()
        .ok_or("client network inventory")?;
    lab.case(
        "go-three-namespace-topology",
        g.len() == 2
            && b.len() == 1
            && c.len() == 1
            && b.keys().all(|k| !c.contains_key(k))
            && b.keys().chain(c.keys()).all(|k| g.contains_key(k))
            && gate["NetworkSettings"]["SandboxID"] != backend["NetworkSettings"]["SandboxID"]
            && gate["NetworkSettings"]["SandboxID"] != client["NetworkSettings"]["SandboxID"]
            && backend["NetworkSettings"]["SandboxID"] != client["NetworkSettings"]["SandboxID"],
        json!("two internal bridges; Gate alone joins both; no shared namespace"),
    )?;
    for name in g.keys() {
        let network: Value =
            serde_json::from_str(&lab.docker.call(&["network", "inspect", name])?)?;
        lab.case(
            "go-internal-owned-network",
            network[0]["Internal"] == true
                && network[0]["EnableIPv6"] == false
                && network[0]["Labels"]["com.docker.compose.project"] == lab.project,
            json!(name),
        )?;
    }
    Ok(())
}
fn copy_digest(lab: &mut Lab, id: &str, source: &str, name: &str) -> Result<String> {
    let path = lab.evidence.join(name);
    lab.docker.call(&[
        "cp",
        &format!("{id}:{source}"),
        path.to_str().ok_or("private artifact path")?,
    ])?;
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > 128 * 1024 * 1024
    {
        return Err("Go artifact copy bound/type".into());
    }
    Ok(snapshot::digest(&fs::read(path)?))
}
fn evaluate(lab: &mut Lab, observation: &Value) -> Result<Value> {
    let command = lab.compose_args(&[
        "run",
        "--rm",
        "-T",
        "--no-deps",
        "runtime-pilot",
        "--go-evaluate",
    ])?;
    let output = lab.docker.invoke_input(
        &command,
        &lab.environment,
        true,
        &serde_json::to_vec(observation)?,
    )?;
    fs::write(
        lab.evidence.join("go-case-evaluation.raw.json"),
        output.stdout.as_bytes(),
    )?;
    Ok(decode_strict_json(output.stdout.as_bytes(), 65536)?)
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
fn egress(lab: &mut Lab, actor: &Value, canary: &str, role: &str) -> Result<()> {
    lab.case(
        "go-explicit-local-deny-dns",
        actor["HostConfig"]["Dns"] == json!(["127.0.0.1"]),
        json!(role),
    )?;
    let name = format!("{}-{role}.invalid", lab.project);
    let hash = snapshot::digest(name.as_bytes());
    let report = decode_strict_json(
        lab.docker
            .call(&[
                "exec",
                actor["Id"].as_str().ok_or("Go egress actor ID")?,
                "/usr/local/bin/humanymous-runtime-pilot",
                "--go-egress",
                canary,
                &name,
            ])?
            .as_bytes(),
        65536,
    )?;
    let denied = |v: &Value| {
        matches!(
            v.as_str(),
            Some("NetworkUnreachable" | "HostUnreachable" | "PermissionDenied")
        )
    };
    lab.case(
        "go-actual-namespace-egress-denied",
        report["schema"] == "humanymous.go-egress-control/1"
            && report["no_default_ipv4_route"] == true
            && denied(&report["outside_tcp_denied"]["kind"])
            && denied(&report["dns_control"]["outside_udp_denied"]["kind"])
            && report["dns_control"]["query_hash"] == hash
            && report["dns_control"]["local_deny_observed"] == true
            && report["dns_policy"]["question_hash_counts"][&hash]
                .as_u64()
                .is_some_and(|n| n > 0)
            && report["dns_policy"]["forwards"] == 0
            && report["timeout_counted_as_denial"] == false,
        json!(role),
    )?;
    snapshot::write(&lab.evidence, &format!("go-egress-{role}.json"), &report)
}
fn one(lab: &mut Lab, id: &str, contract: &Value, fixture: &Value) -> Result<Value> {
    set(lab, "HM_GO_CASE", id.into());
    lab.compose(
        &[
            "up",
            "-d",
            "--no-build",
            "--wait",
            "--wait-timeout",
            "30",
            "go-egress-canary",
        ],
        true,
    )?;
    let canary = lab.inspect("go-egress-canary")?;
    let canary_id = canary["Id"]
        .as_str()
        .ok_or("Go outside canary ID")?
        .to_owned();
    let canary_ip = named_ip(&canary, "_outside")?;
    lab.docker.call(&[
        "exec",
        &canary_id,
        "/usr/local/bin/humanymous-lab-runner",
        "fixture",
        "dns-positive",
    ])?;
    let dns_before = dns_snapshot(lab, &canary_id)?;
    lab.case(
        "go-owned-outside-canary-positive",
        dns_before["received"] == 1 && dns_before["invalid"] == 0,
        json!("actual healthy outside UDP53 and TCP8080 control"),
    )?;
    snapshot::write(&lab.evidence, "go-dns-canary-before.json", &dns_before)?;
    lab.compose(&["up", "-d", "--no-build", "go-backend"], true)?;
    let mut backend = lab.inspect("go-backend")?;
    let backend_id = backend["Id"]
        .as_str()
        .ok_or("Go backend actor ID")?
        .to_owned();
    let until = Instant::now() + Duration::from_secs(15);
    loop {
        let output = lab.docker.call(&["logs", "--tail", "2", &backend_id])?;
        if output.lines().any(|l| {
            decode_strict_json(l.as_bytes(), 65536)
                .is_ok_and(|v| v["schema"] == "humanymous.go-backend-ready/1")
        }) {
            break;
        }
        if Instant::now() >= until {
            return Err("Go actual backend readiness deadline".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    backend = lab.inspect("go-backend")?;
    let actual_binary = copy_digest(
        lab,
        &backend_id,
        "/usr/local/bin/go-httpbin",
        "go-httpbin.actual",
    )?;
    lab.case(
        "go-untouched-actual-artifact",
        actual_binary == fixture["binary_sha256"] && backend["State"]["Running"] == true,
        json!(actual_binary),
    )?;
    set(
        lab,
        "HM_GO_BACKEND",
        format!("{}:18082", named_ip(&backend, "_go-backend")?),
    );
    lab.compose(&["create", "go-gate"], true)?;
    let gate = lab.inspect("go-gate")?;
    let gate_id = gate["Id"].as_str().ok_or("Go Gate ID")?.to_owned();
    let engine = copy_digest(
        lab,
        &gate_id,
        "/usr/local/bin/humanymous-runtime-pilot",
        "runtime-pilot.actual",
    )?;
    let image = gate["Image"].as_str().ok_or("Go Gate image ID")?.to_owned();
    let (mut child, mut input, output) =
        lab.docker
            .duplex(&["start".into(), "-a".into(), "-i".into(), gate_id.clone()])?;
    let receive = frame_reader(output);
    let identity = match frame(&receive) {
        Ok(value) => value,
        Err(error) => {
            let _ = super::startup_failure::capture(lab, &gate_id);
            return Err(error);
        }
    };
    if identity["role"] != "writer_identity" {
        return Err("Go writer identity missing".into());
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
        inspections: vec![],
    };
    let initial = broker.transaction(lab, &public, true, &json!({"operation":"state"}))?;
    write_frame(&mut input, &json!({"witness_public":broker.public}))?;
    write_frame(&mut input, &initial[1])?;
    input.flush()?;
    let ready = loop {
        let value = frame(&receive)?;
        if value["role"] == "go_gate_ready" {
            break value;
        }
        sign(&mut broker, lab, &public, &value, &mut input)?;
    };
    snapshot::write(&lab.evidence, "go-gate-readiness.json", &ready)?;
    let contact: std::net::SocketAddr = ready["contact_witness"]
        .as_str()
        .ok_or("Go contact witness tuple")?
        .parse()?;
    if !contact.ip().is_loopback() || contact.port() == 0 {
        return Err("Go origin contact witness not loopback".into());
    }
    let mut expected = decode_strict_json(
        &fs::read(lab.config_file("test/lab/go-configuration.json")?)?,
        65536,
    )?;
    expected["origin"] = json!(contact);
    expected["routes"] = contract["baseline_routes"].clone();
    if id == "forged-provenance" {
        expected["routes"][0]["required"]["raw_tls"] = json!(true);
    }
    lab.case(
        "go-exact-provisioned-configuration",
        ready["configuration"] == expected
            && ready["configuration_sha256"] == snapshot::digest(&serde_json::to_vec(&expected)?),
        json!("complete parent-selected sanitized configuration, actual private loopback origin"),
    )?;
    let running_gate = lab.inspect("go-gate")?;
    egress(lab, &running_gate, &canary_ip, "gate")?;
    egress(lab, &backend, &canary_ip, "backend")?;
    set(
        lab,
        "HM_GO_GATE",
        format!("{}:8080", named_ip(&running_gate, "_go-ingress")?),
    );
    set(
        lab,
        "HM_GO_WITNESS",
        format!(
            "{}:{}",
            named_ip(&running_gate, "_go-ingress")?,
            contact.port()
        ),
    );
    let service = if id == "origin-bypass" {
        "go-bypass"
    } else {
        "go-client"
    };
    lab.compose(&["create", service], true)?;
    let client = lab.inspect(service)?;
    let client_id = client["Id"]
        .as_str()
        .ok_or("Go client actor ID")?
        .to_owned();
    let mut command = lab
        .docker
        .command(&["start".into(), "-a".into(), client_id.clone()], &[])?;
    let raw_path = lab.evidence.join("go-client-output.json");
    let captured = std::thread::scope(|scope| -> Result<_> {
        let (send, done) = mpsc::sync_channel(1);
        let handle=scope.spawn(move||{
            let result=capture_with_stderr(&mut command,&[],Duration::from_secs(70),1048576);
            let raw=match &result{Ok(r)=>json!({"stdout":String::from_utf8_lossy(&r.stdout),"stderr":String::from_utf8_lossy(&r.stderr),"exit":r.status.code()}),Err(error)=>json!({"capture_error":error.to_string(),"partial_output_unavailable":true})};
            fs::write(raw_path,serde_json::to_vec_pretty(&raw).map_err(std::io::Error::other)?)?;let _=send.send(());result
        });
        let until = Instant::now() + Duration::from_secs(75);
        let mut checked = false;
        loop {
            if done.try_recv().is_ok() {
                break;
            }
            if Instant::now() >= until {
                return Err("Go client supervision deadline".into());
            }
            if !checked {
                let client = lab.inspect(service)?;
                if client["State"]["Running"] == true {
                    isolate(lab, &running_gate, &backend, &client)?;
                    egress(lab, &client, &canary_ip, "client")?;
                    lab.docker.call(&[
                        "exec",
                        &client_id,
                        "/usr/local/bin/humanymous-runtime-pilot",
                        "--go-client-release",
                    ])?;
                    checked = true;
                }
            }
            match receive.recv_timeout(Duration::from_millis(20)) {
                Ok(value) => sign(
                    &mut broker,
                    lab,
                    &public,
                    &decode_strict_json(&value?, 65536)?,
                    &mut input,
                )?,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => return Err("Go Gate private control closed during case".into()),
            }
        }
        if !checked {
            return Err("Go actual running client namespace was not observed".into());
        }
        Ok(handle
            .join()
            .map_err(|_| "Go client capture worker failed")??)
    })?;
    if !captured.status.success() {
        return Err("actual Go client process failed".into());
    }
    let mut observation = decode_strict_json(&captured.stdout, 1048576)?;
    if observation["schema"] != "humanymous.go-client-observation/1"
        || observation["observation"]["case"] != id
    {
        return Err("Go case observation identity mismatch".into());
    }
    std::thread::sleep(Duration::from_millis(75));
    let counter = decode_strict_json(
        lab.docker
            .call(&[
                "exec",
                &gate_id,
                "/usr/local/bin/humanymous-runtime-pilot",
                "--go-count",
            ])?
            .as_bytes(),
        16384,
    )?;
    if counter["case"] != id {
        return Err("Go contact snapshot case mismatch".into());
    }
    let contacts = counter["origin"]["contacts"]
        .as_u64()
        .ok_or("Go independent origin counter absent")?;
    let live_backend = lab.inspect("go-backend")?;
    lab.case(
        "go-backend-live-after-case",
        live_backend["State"]["Running"] == true && live_backend["Image"] == backend["Image"],
        json!("actual upstream actor remains running after probe"),
    )?;
    observation["observation"]["origin_contacts"] = json!(contacts);
    observation["observation"]["actual_host_verified"] = json!(true);
    let dns_after = dns_snapshot(lab, &canary_id)?;
    lab.case(
        "go-no-external-dns-arrival",
        dns_after == dns_before,
        json!("healthy independent canary saw no new queries from Gate/backend/attacker"),
    )?;
    snapshot::write(&lab.evidence, "go-dns-canary-after.json", &dns_after)?;
    snapshot::write(&lab.evidence, "go-measured-observation.json", &observation)?;
    let evaluation = evaluate(lab, &observation["observation"])?;
    let pin = broker
        .head
        .as_ref()
        .ok_or("Go independent latest witness head absent")?
        .to_string();
    lab.docker.call(&[
        "exec",
        &gate_id,
        "/usr/local/bin/humanymous-runtime-pilot",
        "--browser-stop",
        &pin,
    ])?;
    let report = loop {
        let value = frame(&receive)?;
        if value["role"] == "go_gate_result" {
            break value;
        }
        sign(&mut broker, lab, &public, &value, &mut input)?;
    };
    drop(input);
    let status = child.wait(Duration::from_secs(15))?;
    let coverage = &report["witness_coverage"];
    lab.case(
        "go-runtime-witness-clean-drain",
        status.success()
            && report["stopped"] == true
            && report["origin"]["contacts"] == contacts
            && coverage["fully_witnessed"] == true
            && coverage["ambiguous_tail_len"] == 0
            && coverage["authenticated_external_head"]
                == broker.head.as_ref().ok_or("Go pinned head")?.clone()
            && report["checkpoint_repair_performed"] == false,
        json!("actual WAL authenticated against independently retained latest head"),
    )?;
    let after = lab.inspect("go-gate")?;
    lab.case(
        "go-gate-image-stable",
        after["Image"] == image,
        json!(image),
    )?;
    lab.docker.call(&[
        "exec",
        &backend_id,
        "/usr/local/bin/humanymous-runtime-pilot",
        "--go-backend-stop",
    ])?;
    let until = Instant::now() + Duration::from_secs(15);
    let state = loop {
        let value: Value = serde_json::from_str(&lab.docker.call(&[
            "inspect",
            "--format",
            "{{json .State}}",
            &backend_id,
        ])?)?;
        if value["Running"] == false {
            break value;
        }
        if Instant::now() >= until {
            return Err("Go backend actor stop/reap deadline".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let logs = lab.docker.call(&["logs", "--tail", "3", &backend_id])?;
    let stopped: Vec<_> = logs
        .lines()
        .filter_map(|line| decode_strict_json(line.as_bytes(), 65536).ok())
        .filter(|v| v["schema"] == "humanymous.go-backend-stop/1")
        .collect();
    lab.case(
        "go-backend-child-reaped",
        state["ExitCode"] == 0
            && stopped.len() == 1
            && stopped[0]["child_reaped"] == true
            && stopped[0]["graceful_shutdown_qualified"] == false
            && stopped[0]["contacts"]["contacts"] == contacts,
        json!(
            "actual actor exit plus shared ManagedChild reap receipt; no graceful upstream claim"
        ),
    )?;
    snapshot::write(
        &lab.evidence,
        "go-backend-stop.json",
        &json!({"state":state,"receipt":stopped[0]}),
    )?;
    snapshot::write(&lab.evidence, "go-gate-result.json", &report)?;
    Ok(
        json!({"case":id,"evaluation":evaluation,"configuration":expected,"runtime_image":image,"backend_image":backend["Image"],"engine_sha256":engine,"actual_upstream_binary_sha256":actual_binary,"full_product_qualified":false}),
    )
}

pub(super) fn run(lab: &mut Lab) -> Result<()> {
    let contract = decode_strict_json(
        &fs::read(lab.config_file("adapters/go/pilot-contract.json")?)?,
        65536,
    )?;
    let cases = contract["cases"]
        .as_array()
        .ok_or("Go fixed case inventory")?;
    if cases.len() != 8 {
        return Err("Go eight-case inventory required".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut ledger = Vec::new();
    for case in cases {
        let id = case["id"].as_str().ok_or("Go case ID")?;
        if id.is_empty()
            || id.len() > 40
            || !id.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
            || !seen.insert(id)
        {
            return Err("Go case inventory unsafe or duplicated".into());
        }
        ledger.push(json!({"case":id,"status":"not_run","evaluation":null,"origin_contacts":null,"full_product_qualified":false}));
    }
    snapshot::write(
        &lab.evidence,
        "go-product-scheduled.json",
        &json!({"schema":"humanymous.go-product-ledger/1","scheduled":8,"evaluated":0,"cases":ledger,"status":"not_run","full_product_qualified":false}),
    )?;
    let result = execute(lab, &contract, &mut ledger);
    if let Err(error) = &result {
        let evaluated = ledger
            .iter()
            .filter(|v| {
                v["evaluation"]["status"] == "passed" || v["evaluation"]["status"] == "failed"
            })
            .count();
        let report = json!({"schema":"humanymous.go-product-ledger/1","scheduled":8,"evaluated":evaluated,"cases":ledger,"status":"failed","setup_or_qualification_error":{"stage":lab.stage,"detail":error.to_string()},"full_product_qualified":false});
        snapshot::write(&lab.evidence, "go-product-report.json", &report)?;
        lab.components["go_product"] = report;
    }
    let projection = (|| -> Result<Value> {
        let raw = fs::read(lab.evidence.join("go-product-report.json"))?;
        let source_raw = fs::read(lab.evidence.join("source-manifest.json"))?;
        let source = decode_strict_json(&source_raw, 16 * 1024 * 1024)?;
        super::go_public::project(
            &lab.components["go_product"],
            &raw,
            &source,
            &snapshot::digest(&source_raw),
            &super::go_public::run_context()?,
        )
    })();
    match projection {
        Ok(value) => {
            snapshot::write(&lab.evidence, "go-public-receipt.json", &value)?;
            lab.go_component_verified = value["qualified_component"] == true;
            lab.public_receipts["go"] = value;
        }
        Err(error) => {
            if result.is_ok() {
                return Err(error);
            }
            snapshot::write(
                &lab.evidence,
                "go-public-projection-failure.private.json",
                &json!({"detail":error.to_string(),"original_execution_error_preserved":true}),
            )?;
        }
    }
    result
}
fn execute(lab: &mut Lab, contract: &Value, ledger: &mut [Value]) -> Result<()> {
    // Same-YAML local act is not actual GitHub hosted evidence. Reject before
    // building this strict-hosted profile; the outer eight NOT_RUN rows survive.
    go_fixture::require_hosted()?;
    lab.stage = "runtime_build";
    // No fixture-dependent interpolation is needed for the initial build.
    let placeholder = lab.config_file("test/lab")?;
    set(lab, "HM_GO_FIXTURE_EXPORT", placeholder);
    lab.compose(&["build", "go-gate"], true)?;
    lab.compose(&["build", "go-egress-canary"], true)?;
    lab.stage = "go_fixture";
    let fixture = go_fixture::prepare(lab)?;
    set(lab, "HM_GO_FIXTURE_EXPORT", fixture.export.clone());
    lab.compose(&["build", "go-backend"], true)?;
    let cases = contract["cases"]
        .as_array()
        .ok_or("Go fixed case inventory")?;
    if cases.len() != 8 {
        return Err("Go eight-case inventory required".into());
    }
    for (index, case) in cases.iter().enumerate() {
        lab.stage = "go_case";
        let id = case["id"].as_str().ok_or("Go case ID")?;
        ledger[index]["status"] = json!("attempted");
        let directory = lab.evidence.join(format!("go-case-{id}"));
        fs::create_dir(&directory)?;
        let mut random = [0; 8];
        getrandom::fill(&mut random)?;
        let project = format!(
            "hm-rust-lab-{}",
            random
                .iter()
                .map(|v| format!("{v:02x}"))
                .collect::<String>()
        );
        let mut child = Lab {
            docker: Docker::new(&lab.docker.root),
            project,
            environment: lab.environment.clone(),
            evidence: directory,
            collector: false,
            topology_only: false,
            browser_product: false,
            source_binding: None,
            go_product: true,
            mount_probe: false,
            authority_node_ipc: false,
            w2_h2_product_binding: false,
            w2_h2_hosted_venue: false,
            n06_owner_join_readiness: false,
            g2_collector_product_binding: false,
            g2_collector_product_binding_readiness: false,
            g5_witness_transport_honesty: false,
            g5_witness_transport: false,
            clean_room_product: false,
            g6_origin_audit_binding: false,
            red_blue_product: false,
            clean_room_product_matrix: false,
            require_full: false,
            browser_ready_interactive_at_honesty: false,
            n06_owner_auth_handoff_honesty: false,
            g4_cef_at: false,
            go_component_verified: false,
            stage: "go_case",
            cases: vec![],
            components: json!({}),
            public_receipts: json!({"schema":"humanymous.lab-public-receipts/1"}),
        };
        let result = one(&mut child, id, contract, &fixture.identity);
        let cleanup = child.cleanup();
        let item = match (result, cleanup) {
            (Ok(result), Ok(())) => result,
            (Ok(mut result), Err(error)) => {
                result["status"] = json!("error");
                result["error_code"] = json!("cleanup_failed");
                result["cleanup_error"] = json!(error.to_string());
                result
            }
            (Err(error), cleanup) => {
                let path = child.evidence.join("go-case-evaluation.raw.json");
                let evaluation = if path.is_file() {
                    decode_strict_json(&fs::read(path)?, 65536)?
                } else {
                    Value::Null
                };
                json!({"case":id,"status":"error","error_code":child.stage,"evaluation":evaluation,"detail":error.to_string(),"cleanup_error":cleanup.err().map(|e|e.to_string()),"full_product_qualified":false})
            }
        };
        snapshot::write(
            &child.evidence,
            "case-ledger.json",
            &json!({"result":item,"assertions":child.cases,"engine_affinity":child.docker.engine_identity}),
        )?;
        snapshot::write(
            &child.evidence,
            "docker-transcript.json",
            &json!(child.docker.transcript),
        )?;
        ledger[index] = item;
    }
    let passed = ledger
        .iter()
        .all(|v| v["status"] != "error" && v["evaluation"]["status"] == "passed");
    let mut qualification =
        json!({"status":"not_run","reason":"complete successful measured ledger required"});
    if passed {
        let source = decode_strict_json(
            &fs::read(lab.evidence.join("source-manifest.json"))?,
            16 * 1024 * 1024,
        )?;
        if source["dirty"] != "" {
            return Err("hosted Go qualification requires clean immutable source".into());
        }
        let first = ledger.first().ok_or("Go complete ledger absent")?;
        let image = first["runtime_image"]
            .as_str()
            .and_then(|s| s.strip_prefix("sha256:"))
            .ok_or("actual Go runtime image SHA")?;
        let engine = first["engine_sha256"]
            .as_str()
            .ok_or("actual Go engine artifact SHA")?;
        if !ledger.iter().all(|v| {
            v["runtime_image"] == first["runtime_image"]
                && v["backend_image"] == first["backend_image"]
                && v["engine_sha256"] == engine
                && v["actual_upstream_binary_sha256"] == fixture.identity["binary_sha256"]
        }) {
            return Err("Go artifact identities drifted between cases".into());
        }
        let configs: Vec<_> = ledger
            .iter()
            .map(|v| json!({"case":v["case"],"configuration":v["configuration"]}))
            .collect();
        let configuration = snapshot::digest(&serde_json::to_vec(&configs)?);
        let artifact = json!({"full_source_sha256":fixture.identity["full_fixture_sha256"],"binary_sha256":fixture.identity["binary_sha256"],"compiler_index":fixture.identity["compiler"]["manifest_index_digest"],"compiler_platform":fixture.identity["compiler"]["linux_amd64_manifest_digest"],"actual_go_version":fixture.identity["actual_go_version"]});
        let fixture_digest = snapshot::digest(&serde_json::to_vec(&artifact)?);
        let host_version = format!(
            "go-httpbin-{}",
            fixture.identity["fixture_revision"]
                .as_str()
                .ok_or("Go upstream source revision")?
        );
        // This trusted tuple is selected from the frozen source/build/OS and
        // independently inspected artifacts, before constructing the receipt.
        let expected = json!({"source":source["head"],"image":image,"fixture":fixture_digest,"configuration":configuration,"engine":engine,"host_version":host_version,"go_version":fixture.identity["compiler"]["expected_go_version"],"os":std::env::consts::OS,"arch":std::env::consts::ARCH});
        let outcomes: Vec<_> = ledger
            .iter()
            .map(|v| v["evaluation"]["outcome"].clone())
            .collect();
        let receipt = json!({"profile":contract["qualification_profile"],"host":"net-http","source_sha":expected["source"],"image_sha256":expected["image"],"fixture_sha256":expected["fixture"],"configuration_sha256":expected["configuration"],"engine_sha256":expected["engine"],"host_version":expected["host_version"],"go_version":expected["go_version"],"os":expected["os"],"arch":expected["arch"],"isolated_docker":true,"hosted":true,"outcomes":outcomes});
        snapshot::write(
            &lab.evidence,
            "go-independent-expected-tuple.json",
            &json!({"expected":expected,"configurations":configs,"fixture_artifact_binding":artifact}),
        )?;
        snapshot::write(&lab.evidence, "go-measured-receipt.json", &receipt)?;
        let command = lab.compose_args(&[
            "run",
            "--rm",
            "-T",
            "--no-deps",
            "runtime-pilot",
            "--go-qualify",
        ])?;
        let output = lab.docker.invoke_input(
            &command,
            &lab.environment,
            true,
            &serde_json::to_vec(&json!({"expected":expected,"receipt":receipt}))?,
        )?;
        qualification = decode_strict_json(output.stdout.as_bytes(), 65536)?;
        if qualification["qualified_component"] != true
            || qualification["scheduled"] != 8
            || qualification["full_product_qualified"] != false
        {
            return Err("Go bounded qualifier receipt mismatch".into());
        }
    }
    let report = json!({"schema":"humanymous.go-product-ledger/1","scheduled":8,"cases":ledger,"status":if passed{"passed"}else{"failed"},"qualification":qualification,"full_product_qualified":false});
    snapshot::write(&lab.evidence, "go-product-report.json", &report)?;
    lab.components["go_product"] = report;
    // Keep the fixture context alive until every case image/container is gone.
    drop(fixture.context);
    lab.case(
        "go-eight-measured-cases",
        passed,
        json!("all eight source-bound observations and cleanup required"),
    )?;
    Ok(())
}
