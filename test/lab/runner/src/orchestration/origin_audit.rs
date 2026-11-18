//! Roadmap 2c cross-namespace origin/audit effect binding.
//!
//! Containers (deployments/lab/compose.rust.yaml): `oa-origin` on `backend`
//! only, `oa-gate` on `ingress` and `backend` with sealed custody and WAL on
//! named volumes, `oa-client` on `ingress` only and a network-none
//! `oa-tamper` that mounts only the WAL volume. The runner is the trusted
//! operator: it supplies the passphrase privately and retains the signed
//! audit head between Gate sessions as the independently protected anchor.
//!
//! Sequence: provision and serve; restart; crash (`docker kill`) and serve
//! again; five tampered copies of the live history (copy, torn, forge,
//! truncate, rollback) each offered to a fresh Gate session; a final positive
//! control on the untouched live history. Origin contacts are counted from the
//! origin's own log, never from the client or Gate.
//!
//! Go parity (981d25c): `deployments/compose/{networks,defenders,bots}.yaml`
//! (origin on `web` only, Gate on every network, bots on a lab subnet only),
//! `internal/gate/gate.go` (single configured upstream) and
//! `internal/audit/wal_test.go` (restart replay with contiguous sequence, torn
//! tail recovered, framed corruption fatal).
use super::Lab;
use crate::{docker::Result, snapshot};
use humanymous_platform::testing::{json::decode_strict_json, process::ManagedChild};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{ChildStdin, ChildStdout},
    sync::mpsc,
    time::{Duration, Instant},
};

/// Exact client ledger (test/lab/runtime/src/origin_audit.rs), every phase.
const CLIENT_ROWS: &[&str] = &[
    "origin-direct-unreachable",
    "gate-loopback-ports-unreachable",
    "session-issued-via-gate",
    "unsigned-refused",
    "absolute-form-refused",
    "signed-effect-forwarded",
    "replay-refused",
];
/// Tampered histories the runtime must still open.
const TAMPER_ACCEPTED: &[&str] = &["copy", "torn"];
/// Tampered histories the runtime must refuse at startup.
const TAMPER_REFUSED: &[&str] = &["forge", "truncate", "rollback"];
/// Client phases, one Gate session each.
const PHASES: &[&str] = &["first", "restarted", "after-crash", "final"];
const CLIENT_SCHEMA: &str = "humanymous.origin-audit-client/1";
const AUDIT_REFUSAL: &str = "audit authenticated recovery";

type Lines = mpsc::Receiver<std::io::Result<Vec<u8>>>;

fn reader(output: ChildStdout) -> Lines {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let mut output = BufReader::new(output);
        loop {
            let mut line = Vec::new();
            match Read::take(&mut output, 1 << 20).read_until(b'\n', &mut line) {
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
        1 << 20,
    )?)
}

struct Session {
    child: ManagedChild,
    input: ChildStdin,
    lines: Lines,
}

struct Operator {
    gate: String,
    passphrase: String,
}

impl Operator {
    /// Start one Gate session; `Ok((None, line))` when the session refused.
    fn start(
        &self,
        lab: &mut Lab,
        provision: bool,
        history: &str,
        trusted_head: &Value,
    ) -> Result<(Option<Session>, Value)> {
        let (mut child, mut input, output) =
            lab.docker
                .duplex(&["start".into(), "-a".into(), "-i".into(), self.gate.clone()])?;
        let control = json!({"provision":provision,"passphrase":self.passphrase,"history":history,
            "allow_new":provision,"trusted_head":trusted_head});
        serde_json::to_writer(&mut input, &control)?;
        input.write_all(b"\n")?;
        input.flush()?;
        let lines = reader(output);
        let first = next(&lines, 120)?;
        if first["role"] == "oa_gate_ready" {
            Ok((
                Some(Session {
                    child,
                    input,
                    lines,
                }),
                first,
            ))
        } else {
            drop(input);
            child.wait(Duration::from_secs(60))?;
            Ok((None, first))
        }
    }

    /// Request the supervised stop and read the offline-verified summary.
    fn stop(&self, mut session: Session) -> Result<Value> {
        session.input.write_all(b"stop\n")?;
        session.input.flush()?;
        let stopped = next(&session.lines, 120)?;
        drop(session.input);
        session.child.wait(Duration::from_secs(60))?;
        Ok(stopped)
    }

    /// Crash the whole Gate container; no summary is possible.
    fn kill(&self, lab: &mut Lab, mut session: Session) -> Result<bool> {
        lab.docker.call(&["kill", &self.gate])?;
        drop(session.input);
        let status = session.child.wait(Duration::from_secs(60))?;
        Ok(!status.success())
    }
}

fn attached(lab: &Lab, container: &Value) -> Vec<String> {
    let prefix = format!("{}_", lab.project);
    let mut names: Vec<_> = container["NetworkSettings"]["Networks"]
        .as_object()
        .map(|networks| {
            networks
                .keys()
                .map(|name| name.strip_prefix(&prefix).unwrap_or(name).to_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn address(lab: &mut Lab, id: &str, network: &str) -> Result<String> {
    let state: Value = serde_json::from_str(&lab.docker.call(&["inspect", id])?)?;
    let value =
        state[0]["NetworkSettings"]["Networks"][format!("{}_{network}", lab.project)]["IPAddress"]
            .as_str()
            .ok_or("owned network address missing")?;
    if !value.parse::<std::net::Ipv4Addr>()?.is_private() {
        return Err("owned internal IPv4 required".into());
    }
    Ok(value.into())
}

/// Every contact the origin logged, as peer addresses, in order.
fn contacts(lab: &mut Lab, origin: &str) -> Result<(bool, Vec<String>)> {
    let logs = lab.docker.call(&["logs", origin])?;
    let mut ready = false;
    let mut peers = Vec::new();
    for line in logs.lines().filter(|line| !line.trim().is_empty()) {
        let value = decode_strict_json(line.as_bytes(), 4096)?;
        match value["role"].as_str() {
            Some("oa_origin_ready") => ready = true,
            Some("oa_origin_contact") => peers.push(
                value["peer"]
                    .as_str()
                    .ok_or("origin contact peer")?
                    .to_owned(),
            ),
            _ => return Err("unknown origin log line".into()),
        }
    }
    Ok((ready, peers))
}

/// One-shot actor over a private duplex: stdin stays open until the single
/// report line arrives, because `docker start -a -i` may detach early when
/// its input closes before the attach completes.
fn oneshot(lab: &mut Lab, id: &str, control: &Value, limit: usize) -> Result<Value> {
    let (mut child, mut input, output) =
        lab.docker
            .duplex(&["start".into(), "-a".into(), "-i".into(), id.into()])?;
    serde_json::to_writer(&mut input, control)?;
    input.write_all(b"\n")?;
    input.flush()?;
    let lines = reader(output);
    let line = lines.recv_timeout(Duration::from_secs(120))??;
    drop(input);
    let status = child.wait(Duration::from_secs(60))?;
    let mut report = decode_strict_json(&line, limit)?;
    report["process_success"] = json!(status.success());
    Ok(report)
}

fn client(lab: &mut Lab, id: &str, gate: &str, origin: &str, phase: &str) -> Result<Value> {
    oneshot(
        lab,
        id,
        &json!({"gate":gate,"origin":origin,"phase":phase}),
        1 << 20,
    )
}

fn tamper(lab: &mut Lab, id: &str, mode: &str) -> Result<Value> {
    oneshot(lab, id, &json!({"mode":mode}), 65536)
}

fn passphrase() -> Result<String> {
    let mut secret = [0_u8; 24];
    getrandom::fill(&mut secret)?;
    Ok(secret.iter().map(|b| format!("{b:02x}")).collect())
}

pub(super) fn run(lab: &mut Lab) -> Result<()> {
    for service in ["oa-origin", "oa-gate", "oa-client", "oa-tamper"] {
        lab.compose(&["create", service], true)?;
    }
    let origin = lab.inspect("oa-origin")?;
    let gate = lab.inspect("oa-gate")?;
    let client_container = lab.inspect("oa-client")?;
    let tamper_container = lab.inspect("oa-tamper")?;
    let topology = json!({"origin":attached(lab,&origin),"gate":attached(lab,&gate),
        "client":attached(lab,&client_container),
        "tamper_network_mode":tamper_container["HostConfig"]["NetworkMode"]});
    let id = |container: &Value| -> Result<String> {
        Ok(container["Id"]
            .as_str()
            .ok_or("container ID absent")?
            .to_owned())
    };
    let (origin_id, gate_id) = (id(&origin)?, id(&gate)?);
    let (client_id, tamper_id) = (id(&client_container)?, id(&tamper_container)?);

    lab.docker.call(&["start", &origin_id])?;
    let deadline = Instant::now() + Duration::from_secs(60);
    while !contacts(lab, &origin_id)?.0 {
        if Instant::now() > deadline {
            return Err("owned origin not ready".into());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    let origin_ip = address(lab, &origin_id, "backend")?;
    let operator = Operator {
        gate: gate_id.clone(),
        passphrase: passphrase()?,
    };

    let mut phases = Vec::new();
    let mut sessions = serde_json::Map::new();
    let mut gate_backend_ip = String::new();
    let mut head = Value::Null;
    let mut after_crash_head = Value::Null;
    for (index, phase) in PHASES.iter().enumerate() {
        if *phase == "final" {
            lab.stage = "origin_audit_tamper";
            let mut tampers = Vec::new();
            for mode in TAMPER_ACCEPTED.iter().chain(TAMPER_REFUSED) {
                let report = tamper(lab, &tamper_id, mode)?;
                let (session, line) =
                    operator.start(lab, false, &format!("case-{mode}"), &after_crash_head)?;
                let stopped = match session {
                    Some(session) => operator.stop(session)?,
                    None => Value::Null,
                };
                tampers.push(json!({"mode":mode,"tamper":report,"gate":line,"stopped":stopped}));
            }
            sessions.insert("tampers".into(), json!(tampers));
        }
        lab.stage = "origin_audit_session";
        let provision = index == 0;
        let (session, line) = operator.start(lab, provision, "live", &head)?;
        let Some(session) = session else {
            sessions.insert((*phase).into(), json!({"ready":false,"gate":line}));
            break;
        };
        let gate_ip = address(lab, &gate_id, "ingress")?;
        gate_backend_ip = address(lab, &gate_id, "backend")?;
        let before = contacts(lab, &origin_id)?.1.len();
        let report = client(lab, &client_id, &gate_ip, &origin_ip, phase)?;
        let after = contacts(lab, &origin_id)?.1.len();
        phases.push(
            json!({"name":phase,"client":report,"contacts_before":before,"contacts_after":after}),
        );
        if *phase == "restarted" {
            let killed = operator.kill(lab, session)?;
            sessions.insert(
                (*phase).into(),
                json!({"ready":true,"readiness":line["readiness"],"killed":killed}),
            );
            continue;
        }
        let stopped = operator.stop(session)?;
        if index == 0 {
            head = stopped["head"].clone();
        }
        if *phase == "after-crash" {
            after_crash_head = stopped["head"].clone();
        }
        sessions.insert(
            (*phase).into(),
            json!({"ready":true,"readiness":line["readiness"],"stopped":stopped}),
        );
        if *phase == "after-crash" {
            head = after_crash_head.clone();
        }
    }
    let (_, peers) = contacts(lab, &origin_id)?;
    let ledger = json!({"topology":topology,"gate_backend_ip":gate_backend_ip,"origin_ip":origin_ip,
        "phases":phases,"contact_peers":peers,"sessions":sessions,"full_product_qualified":false});
    snapshot::write(&lab.evidence, "origin-audit-ledger.json", &ledger)?;
    let isolated = isolation(&ledger);
    let restarted = restart(&ledger);
    lab.components["origin_audit"] = json!({"status":if isolated && restarted {"passed"} else {"failed"},
        "evidence":"origin-audit-ledger.json","full_product_qualified":false,
        "not_qualified":["TLS/H2 ingress (plain HTTP/1.1 relay)","independent witness co-signing on this Gate",
            "per-container egress beyond the internal bridges","origin effects beyond one owned recording origin"]});
    let first = lab.case(
        "origin-isolation",
        isolated,
        json!("ingress-only client cannot reach the backend-only origin or the Gate's loopback ports; signed effects reach the origin only through the Gate, once per ALLOW, and every origin contact comes from the Gate's backend address"),
    );
    let second = lab.case(
        "audit-restart",
        restarted,
        json!("custody and WAL on volumes; restart and docker-kill crash continue one contiguous verified chain anchored on the retained head; ALLOW decisions equal origin effects; copy and torn tail open; forged, truncated and rolled-back copies are refused by runtime startup authentication"),
    );
    first.and(second)
}

fn client_passed(report: &Value, phase: &str) -> bool {
    report["schema"] == CLIENT_SCHEMA
        && report["phase"] == phase
        && report["status"] == "passed"
        && report["process_success"] == true
        && report["full_product_qualified"] == false
        && report["cases"].as_array().is_some_and(|rows| {
            rows.len() == CLIENT_ROWS.len()
                && rows
                    .iter()
                    .zip(CLIENT_ROWS)
                    .all(|(row, id)| row["id"] == *id && row["status"] == "passed")
        })
}

/// Origin isolation: exact topology, every client phase passed, exactly one
/// origin contact per phase and every contact from the Gate backend address.
fn isolation(ledger: &Value) -> bool {
    let topology = &ledger["topology"];
    let gate_ip = ledger["gate_backend_ip"].as_str().unwrap_or("");
    let phases = ledger["phases"].as_array().map_or(&[][..], Vec::as_slice);
    let peers = ledger["contact_peers"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    topology["origin"] == json!(["backend"])
        && topology["gate"] == json!(["backend", "ingress"])
        && topology["client"] == json!(["ingress"])
        && topology["tamper_network_mode"] == "none"
        && !gate_ip.is_empty()
        && ledger["origin_ip"] != ledger["gate_backend_ip"]
        && phases.len() == PHASES.len()
        && phases.iter().zip(PHASES).enumerate().all(|(i, (p, name))| {
            p["name"] == *name
                && client_passed(&p["client"], name)
                && p["contacts_before"].as_u64() == Some(i as u64)
                && p["contacts_after"].as_u64() == Some(i as u64 + 1)
        })
        && peers.len() == PHASES.len()
        && peers.iter().all(|peer| peer == gate_ip)
}

fn allow_decisions(stopped: &Value) -> usize {
    stopped["records"].as_array().map_or(0, |records| {
        records
            .iter()
            .filter(|r| r["event_type"] == "runtime.decision" && r["enforcement_action"] == "ALLOW")
            .count()
    })
}

/// A verified contiguous history whose signed head covers every record.
fn sound(stopped: &Value) -> bool {
    let sequence = stopped["sequence"].as_u64().unwrap_or(0);
    stopped["role"] == "oa_gate_stopped"
        && stopped["verified"] == true
        && stopped["contiguous"] == true
        && sequence > 0
        && stopped["records"]
            .as_array()
            .is_some_and(|r| r.len() as u64 == sequence)
        && stopped["head"]["tree_size"].as_u64() == Some(sequence)
}

/// `later` continues `earlier`: strictly longer with an identical prefix.
fn continues(earlier: &Value, later: &Value) -> bool {
    match (earlier["records"].as_array(), later["records"].as_array()) {
        (Some(a), Some(b)) => b.len() > a.len() && b[..a.len()] == a[..],
        _ => false,
    }
}

/// Audit restart: one chain across restart, crash and final session; exact
/// tamper outcomes; ALLOW decisions equal origin effects at each summary.
fn restart(ledger: &Value) -> bool {
    let sessions = &ledger["sessions"];
    let first = &sessions["first"]["stopped"];
    let crash = &sessions["after-crash"]["stopped"];
    let last = &sessions["final"]["stopped"];
    let tampers = sessions["tampers"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let modes: Vec<&str> = TAMPER_ACCEPTED
        .iter()
        .chain(TAMPER_REFUSED)
        .copied()
        .collect();
    let tampered = tampers.len() == modes.len()
        && tampers.iter().zip(&modes).all(|(t, mode)| {
            let prepared = t["mode"] == *mode
                && t["tamper"]["role"] == "oa_tamper"
                && t["tamper"]["mode"] == *mode
                && t["tamper"]["live_modified"] == false
                && t["tamper"]["process_success"] == true;
            let outcome = if TAMPER_ACCEPTED.contains(mode) {
                t["gate"]["role"] == "oa_gate_ready"
                    && sound(&t["stopped"])
                    && t["stopped"]["history"] == format!("case-{mode}")
                    && t["stopped"]["records"] == crash["records"]
            } else {
                t["gate"]["role"] == "oa_gate_refused"
                    && t["gate"]["runtime_started"] == true
                    && t["gate"]["runtime_exit"].as_i64().is_some_and(|c| c != 0)
                    && t["gate"]["audit_refusal"] == AUDIT_REFUSAL
                    && t["gate"]["history"] == format!("case-{mode}")
            };
            prepared && outcome
        });
    sessions["first"]["ready"] == true
        && sessions["restarted"]["ready"] == true
        && sessions["restarted"]["killed"] == true
        && sessions["after-crash"]["ready"] == true
        && sessions["final"]["ready"] == true
        && [first, crash, last]
            .iter()
            .all(|s| sound(s) && s["history"] == "live")
        && continues(first, crash)
        && continues(crash, last)
        && allow_decisions(first) == 1
        && allow_decisions(crash) == 3
        && allow_decisions(last) == PHASES.len()
        && ledger["contact_peers"].as_array().map(Vec::len) == Some(PHASES.len())
        && tampered
}

#[cfg(test)]
mod tests {
    use super::*;

    fn records(allows: usize, extra: usize) -> Value {
        let mut rows = Vec::new();
        for _ in 0..allows {
            rows.push(json!({"event_type":"runtime.collect","enforcement_action":"collect"}));
            rows.push(json!({"event_type":"runtime.decision","enforcement_action":"ALLOW"}));
        }
        for _ in 0..extra {
            rows.push(json!({"event_type":"runtime.decision","enforcement_action":"DENY"}));
        }
        for (i, row) in rows.iter_mut().enumerate() {
            row["seq"] = json!(i + 1);
        }
        json!(rows)
    }

    fn stopped(history: &str, allows: usize) -> Value {
        let records = records(allows, 0);
        let n = records.as_array().unwrap().len();
        json!({"role":"oa_gate_stopped","verified":true,"contiguous":true,"sequence":n,
            "records":records,"head":{"tree_size":n},"history":history})
    }

    fn ledger() -> Value {
        let phases: Vec<_> = PHASES
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let rows: Vec<_> = CLIENT_ROWS
                    .iter()
                    .map(|id| json!({"id":id,"status":"passed"}))
                    .collect();
                json!({"name":name,"contacts_before":i,"contacts_after":i+1,
                    "client":{"schema":CLIENT_SCHEMA,"phase":name,"status":"passed","process_success":true,
                        "full_product_qualified":false,"cases":rows}})
            })
            .collect();
        let tampers: Vec<_> = TAMPER_ACCEPTED
            .iter()
            .map(|mode| json!({"mode":mode,"tamper":{"role":"oa_tamper","mode":mode,"live_modified":false,"process_success":true},
                "gate":{"role":"oa_gate_ready"},"stopped":stopped(&format!("case-{mode}"),3)}))
            .chain(TAMPER_REFUSED.iter().map(|mode| json!({"mode":mode,
                "tamper":{"role":"oa_tamper","mode":mode,"live_modified":false,"process_success":true},
                "gate":{"role":"oa_gate_refused","runtime_started":true,"runtime_exit":1,
                    "audit_refusal":AUDIT_REFUSAL,"history":format!("case-{mode}")},"stopped":null})))
            .collect();
        json!({"topology":{"origin":["backend"],"gate":["backend","ingress"],"client":["ingress"],"tamper_network_mode":"none"},
            "gate_backend_ip":"172.20.0.3","origin_ip":"172.20.0.2","contact_peers":vec!["172.20.0.3"; 4],
            "phases":phases,
            "sessions":{"first":{"ready":true,"stopped":stopped("live",1)},
                "restarted":{"ready":true,"killed":true},
                "after-crash":{"ready":true,"stopped":stopped("live",3)},
                "final":{"ready":true,"stopped":stopped("live",4)},
                "tampers":tampers}})
    }

    #[test]
    fn complete_ledger_passes_both_cases() {
        let good = ledger();
        assert!(isolation(&good));
        assert!(restart(&good));
    }

    #[test]
    fn any_isolation_breach_fails_origin_isolation() {
        let mutations: Vec<(&str, Value)> = vec![
            ("/topology/client", json!(["backend", "ingress"])),
            ("/topology/origin", json!(["backend", "ingress"])),
            ("/topology/tamper_network_mode", json!("bridge")),
            ("/contact_peers/2", json!("172.20.0.9")),
            ("/phases/1/contacts_after", json!(3)),
            ("/phases/0/client/cases/0/status", json!("failed")),
            ("/phases/2/client/process_success", json!(false)),
            ("/phases/3/client/phase", json!("first")),
            ("/gate_backend_ip", json!("")),
        ];
        for (pointer, value) in mutations {
            let mut bad = ledger();
            *bad.pointer_mut(pointer).unwrap() = value;
            assert!(!isolation(&bad), "{pointer}");
        }
        let mut short = ledger();
        short["phases"].as_array_mut().unwrap().pop();
        assert!(!isolation(&short));
        let mut extra = ledger();
        extra["contact_peers"]
            .as_array_mut()
            .unwrap()
            .push(json!("172.20.0.3"));
        assert!(!isolation(&extra));
    }

    #[test]
    fn any_chain_or_tamper_deviation_fails_audit_restart() {
        let mutations: Vec<(&str, Value)> = vec![
            ("/sessions/restarted/killed", json!(false)),
            ("/sessions/first/stopped/verified", json!(false)),
            ("/sessions/after-crash/stopped/contiguous", json!(false)),
            ("/sessions/final/stopped/head/tree_size", json!(1)),
            ("/sessions/after-crash/stopped/records/0/seq", json!(9)),
            ("/sessions/tampers/2/gate/role", json!("oa_gate_ready")),
            ("/sessions/tampers/3/gate/audit_refusal", json!("other")),
            ("/sessions/tampers/4/gate/runtime_exit", json!(0)),
            ("/sessions/tampers/0/gate/role", json!("oa_gate_refused")),
            ("/sessions/tampers/1/tamper/live_modified", json!(true)),
            ("/sessions/tampers/1/stopped/records", json!([])),
        ];
        for (pointer, value) in mutations {
            let mut bad = ledger();
            *bad.pointer_mut(pointer).unwrap() = value;
            assert!(!restart(&bad), "{pointer}");
        }
        // An ALLOW decision without an origin effect, or a lost record.
        let mut unbound = ledger();
        unbound["sessions"]["final"]["stopped"] = stopped("live", 5);
        assert!(!restart(&unbound));
        let mut lost = ledger();
        lost["sessions"]["after-crash"]["stopped"] = stopped("live", 2);
        assert!(!restart(&lost));
        let mut missing = ledger();
        missing["sessions"]["tampers"].as_array_mut().unwrap().pop();
        assert!(!restart(&missing));
        let mut absent = ledger();
        absent["sessions"]
            .as_object_mut()
            .unwrap()
            .remove("after-crash");
        assert!(!restart(&absent));
    }

    /// The runner ledgers are the runtime pilot ledgers, verbatim and in order.
    #[test]
    fn ledgers_match_the_runtime_pilot_source() {
        let source = include_str!("../../../runtime/src/origin_audit.rs");
        for (name, ids) in [
            ("CLIENT_ROWS", CLIENT_ROWS),
            ("TAMPER_ACCEPTED", TAMPER_ACCEPTED),
            ("TAMPER_REFUSED", TAMPER_REFUSED),
        ] {
            let start = source
                .find(&format!("pub const {name}: &[&str] = &["))
                .unwrap();
            let block = &source[start..];
            let block = &block[block.find("= &[").unwrap() + 4..block.find("];").unwrap()];
            let declared: Vec<&str> = block
                .split(',')
                .filter_map(|item| item.trim().strip_prefix('"')?.strip_suffix('"'))
                .collect();
            assert_eq!(declared, ids, "{name}");
        }
        assert!(source.contains(&format!("\"{CLIENT_SCHEMA}\"")));
    }
}
