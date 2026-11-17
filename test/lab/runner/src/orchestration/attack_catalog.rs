//! Roadmap 2: the product attack catalog against the shipped runtime binary in
//! the Docker topology.
//!
//! Containers (deployments/lab/compose.rust.yaml): `ac-origin` on `backend`
//! only, `ac-gate` on `ingress` and `backend` (sealed custody and WAL on its
//! private tmpfs) and `ac-attacker` on `ingress` only. The attacker runs the
//! `humanymous-attacker` network catalog with the catalog's own step logic and
//! expectations; the runner is the lab operator in between: it starts,
//! restarts and stops one Gate session per case on the attacker's request and
//! returns the case's effects, measured independently: the audit delta from
//! the Gate's offline-authenticated history, the origin delta from the
//! origin's own contact log.
//!
//! Go parity (981d25c): `deployments/compose/bots.yaml` (catalog container on
//! the internal `lab-a` network only), `deployments/bots/run-attack.sh` and
//! `scripts/assert-attack.mjs` (fail on any bot ALLOW, a DENY of the human
//! baseline, an errored or skipped profile, or an incomplete catalog). The
//! Rust rows carry stricter per-case expectations (exact statuses, origin
//! effects, audit verdicts and rule); the runner also applies the Go escape
//! and false-positive rules directly to the measured effects.
use super::{
    Lab,
    origin_audit::{Lines, address, attached, contacts, next, passphrase, reader},
};
use crate::{docker::Result, snapshot};
use humanymous_platform::testing::process::ManagedChild;
use serde_json::{Value, json};
use std::{
    io::Write,
    process::ChildStdin,
    time::{Duration, Instant},
};

const REPORT_SCHEMA: &str = "humanymous.lab-attack-catalog/1";
/// Every implemented network case (crates/attacker/src/network/cases.rs).
const SUPPORTED: &[&str] = &[
    "synthetic-zero-motor-allow",
    "synthetic-selenium-deny",
    "missing-client-report",
    "synthetic-long-window-challenge",
    "forged-pass-server-signal",
    "rit-replay-after-control",
    "rit-body-mutation",
    "rit-query-target-mutation",
    "rit-header-order-mutation",
    "rit-expired-deadline",
    "rit-missing-token",
    "missing-session",
    "duplicate-content-length",
    "forwarded-local-admin-forgery",
    "required-tls-refusal",
    "required-h2-refusal",
    "required-collector-refusal",
    "restart-stale-session",
    "method-route-mutation",
];
/// Catalog rows this HTTP/1.1 topology cannot execute, and the required lab
/// case that owns each capability instead (None: no required case owns it).
const UNSUPPORTED: &[(&str, Option<&str>)] = &[
    ("ordinary-browser-human", Some("human-control")),
    ("assistive-technology-human", Some("challenge-assistive")),
    ("accessible-pass-human", Some("challenge-assistive")),
    (
        "signed-remote-installation",
        Some("installation-target-binding"),
    ),
    ("coherent-T4-browser", Some("coherent-browser-ceiling")),
    ("full-gate-original-tls-h2", Some("raw-tls-h2")),
    ("audit-outage-network-injection", None),
];

/// A long-lived actor over a private duplex.
struct Actor {
    child: ManagedChild,
    input: ChildStdin,
    lines: Lines,
}
impl Actor {
    fn start(lab: &mut Lab, id: &str, first: &Value) -> Result<Self> {
        let (child, mut input, output) =
            lab.docker
                .duplex(&["start".into(), "-a".into(), "-i".into(), id.into()])?;
        serde_json::to_writer(&mut input, first)?;
        input.write_all(b"\n")?;
        input.flush()?;
        Ok(Self {
            child,
            input,
            lines: reader(output),
        })
    }
    fn send(&mut self, line: &str) -> Result<()> {
        self.input.write_all(line.as_bytes())?;
        self.input.write_all(b"\n")?;
        Ok(self.input.flush()?)
    }
    fn call(&mut self, line: &str, seconds: u64) -> Result<Value> {
        self.send(line)?;
        next(&self.lines, seconds)
    }
    fn finish(self) -> Result<bool> {
        let Self {
            mut child, input, ..
        } = self;
        drop(input);
        Ok(child.wait(Duration::from_secs(60))?.success())
    }
}

/// Measured effects of one case: the authenticated audit suffix since the
/// previous stop and the origin contacts since the session started.
fn effects(previous: &[Value], stopped: &Value, contacts_delta: usize) -> Option<Value> {
    let records = stopped["records"].as_array()?;
    let sound = stopped["role"] == "ac_gate_stopped"
        && stopped["verified"] == true
        && stopped["contiguous"] == true
        && stopped["sequence"].as_u64() == Some(records.len() as u64)
        && (records.is_empty() || stopped["anchored"] == true)
        && records.len() >= previous.len()
        && records[..previous.len()] == *previous;
    sound.then(|| json!({"origin_effects":contacts_delta,"audit":records[previous.len()..]}))
}

pub(super) fn run(lab: &mut Lab) -> Result<()> {
    for service in ["ac-origin", "ac-gate", "ac-attacker"] {
        lab.compose(&["create", service], true)?;
    }
    let origin = lab.inspect("ac-origin")?;
    let gate = lab.inspect("ac-gate")?;
    let attacker = lab.inspect("ac-attacker")?;
    let topology = json!({"origin":attached(lab,&origin),"gate":attached(lab,&gate),
        "attacker":attached(lab,&attacker)});
    let id = |container: &Value| -> Result<String> {
        Ok(container["Id"]
            .as_str()
            .ok_or("container ID absent")?
            .to_owned())
    };
    let (origin_id, gate_id, attacker_id) = (id(&origin)?, id(&gate)?, id(&attacker)?);

    lab.docker.call(&["start", &origin_id])?;
    let deadline = Instant::now() + Duration::from_secs(60);
    while !contacts(lab, &origin_id)?.0 {
        if Instant::now() > deadline {
            return Err("owned origin not ready".into());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    let mut gate_actor = Actor::start(lab, &gate_id, &json!({"passphrase":passphrase()?}))?;
    let provisioned = next(&gate_actor.lines, 120)?;
    let gate_ingress = address(lab, &gate_id, "ingress")?;
    let gate_backend = address(lab, &gate_id, "backend")?;
    let origin_ip = address(lab, &origin_id, "backend")?;

    lab.stage = "attack_catalog_cases";
    let mut client = Actor::start(lab, &attacker_id, &json!({"gate":gate_ingress}))?;
    let mut cases = Vec::new();
    let mut sessions = Vec::new();
    let mut previous: Vec<Value> = Vec::new();
    let mut contacts_at_start = 0;
    let mut current = Value::Null;
    let report = loop {
        let message = next(&client.lines, 300)?;
        match message["op"].as_str() {
            Some("start") => {
                contacts_at_start = contacts(lab, &origin_id)?.1.len();
                let line = gate_actor.call("start", 120)?;
                let ok = line["role"] == "ac_gate_ready";
                current = message["case"].clone();
                sessions.push(json!({"case":current,"gate":line}));
                client.send(&json!({"op":"started","ok":ok}).to_string())?;
            }
            Some("restart") => {
                let line = gate_actor.call("restart", 120)?;
                let ok = line["role"] == "ac_gate_ready" && line["restarted"] == true;
                sessions.push(json!({"case":current,"gate":line}));
                client.send(&json!({"op":"restarted","ok":ok}).to_string())?;
            }
            Some("stop") if message["case"] == current => {
                let stopped = gate_actor.call("stop", 120)?;
                let contacts_delta = contacts(lab, &origin_id)?
                    .1
                    .len()
                    .checked_sub(contacts_at_start)
                    .ok_or("origin contact log shrank")?;
                let measured = effects(&previous, &stopped, contacts_delta);
                if let Some(records) = stopped["records"].as_array().filter(|_| measured.is_some())
                {
                    previous = records.clone();
                }
                cases.push(
                    json!({"case":current,"effects":measured,"stopped_sequence":stopped["sequence"],
                    "verified":stopped["verified"]}),
                );
                client.send(&json!({"op":"effects","effects":measured}).to_string())?;
                current = Value::Null;
            }
            Some("report") => break message["report"].clone(),
            _ => return Err("unexpected attacker control line".into()),
        }
    };
    let attacker_success = client.finish()?;
    gate_actor.send("exit")?;
    let gate_exit = gate_actor.finish()?;
    let (_, peers) = contacts(lab, &origin_id)?;
    let ledger = json!({"topology":topology,"gate_backend_ip":gate_backend,"origin_ip":origin_ip,
        "provisioned":provisioned,"sessions":sessions,"cases":cases,"contact_peers":peers,
        "report":report,"attacker_process_success":attacker_success,"gate_exit_success":gate_exit,
        "full_product_qualified":false});
    snapshot::write(&lab.evidence, "attack-catalog-ledger.json", &ledger)?;
    let passed = catalog(&ledger);
    lab.components["attack_catalog"] = json!({"status":if passed {"passed"} else {"failed"},
        "evidence":"attack-catalog-ledger.json","full_product_qualified":false,
        "not_qualified":["TLS/H2 ingress and raw capture (plain HTTP/1.1 relay)",
            "real browser, human and assistive-technology profiles (separate required cases)",
            "Go browser/uTLS/HTTP2/proxy/correlation profiles (no Rust client; see INTEGRATION.md)",
            "swarm / cross-subnet correlation (not evaluated)"]});
    lab.case(
        "attack-catalog",
        passed,
        json!("Rust attacker network catalog from an ingress-only container through the Gate to a backend-only origin: every supported case evaluated with its exact expectation against independently measured audit and origin effects; no attack step forwarded; controls never DENY; unsupported rows only those owned by other required cases"),
    )
}

/// Go escape and false-positive rules applied to one evaluated row.
fn go_rules(row: &Value) -> bool {
    let steps = row["steps"].as_array().map_or(&[][..], Vec::as_slice);
    let audit = row["effects"]["audit"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let decisions = || {
        audit
            .iter()
            .filter(|e| e["event_type"] == "runtime.decision")
    };
    let controls = steps
        .iter()
        .filter(|s| s["role"] == "synthetic_control")
        .count() as u64;
    let origin = row["effects"]["origin_effects"].as_u64();
    // No attack step is ever forwarded (2xx) by the Gate.
    let refused = steps.iter().all(|s| {
        s["role"] != "attack"
            || s["http_status"]
                .as_u64()
                .is_some_and(|c| !(200..300).contains(&c))
    });
    match row["class"].as_str() {
        // Go: a bot reaching ALLOW escaped. Only the synthetic control step of
        // a replay case may reach the origin; the attack never does.
        Some("attack" | "capability_control") => {
            refused
                && origin == Some(controls)
                && decisions().filter(|e| e["verdict"] == "ALLOW").count() as u64 == controls
        }
        // Go: the human baseline must never be denied.
        Some("synthetic_control" | "synthetic_accessibility") => {
            decisions().all(|e| e["verdict"] != "DENY") && decisions().count() == 1
        }
        _ => false,
    }
}

/// The attack-catalog case: exact topology, complete catalog, every supported
/// row evaluated and matched, effects bound to the runner's measurement, Go
/// rules on every row, and every origin contact from the Gate backend.
fn catalog(ledger: &Value) -> bool {
    let topology = &ledger["topology"];
    let report = &ledger["report"];
    let rows = report["rows"].as_array().map_or(&[][..], Vec::as_slice);
    let cases = ledger["cases"].as_array().map_or(&[][..], Vec::as_slice);
    let peers = ledger["contact_peers"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let gate_ip = ledger["gate_backend_ip"].as_str().unwrap_or("");
    let row = |id: &str| rows.iter().find(|row| row["id"] == id);
    let measured = |id: &str| {
        let mut found = cases.iter().filter(|case| case["case"] == id);
        match (found.next(), found.next()) {
            (Some(case), None) => Some(&case["effects"]),
            _ => None,
        }
    };
    let supported = SUPPORTED.iter().all(|id| {
        row(id).is_some_and(|row| {
            row["status"] == "evaluated"
                && row["expectation_matches"] == true
                && row["reason"].is_null()
                && measured(id)
                    .is_some_and(|effects| !effects.is_null() && *effects == row["effects"])
                && go_rules(row)
        })
    });
    let unsupported = UNSUPPORTED.iter().all(|(id, _)| {
        row(id).is_some_and(|row| {
            row["status"] == "unsupported"
                && row["steps"] == json!([])
                && row["effects"].is_null()
                && row["reason"].as_str().is_some_and(|r| !r.trim().is_empty())
        })
    });
    let origin_total: u64 = rows
        .iter()
        .filter_map(|row| row["effects"]["origin_effects"].as_u64())
        .sum();
    let runtime = ledger["provisioned"]["runtime_sha256"]
        .as_str()
        .unwrap_or("");
    let sessions = ledger["sessions"].as_array().map_or(&[][..], Vec::as_slice);
    topology["origin"] == json!(["backend"])
        && topology["gate"] == json!(["backend", "ingress"])
        && topology["attacker"] == json!(["ingress"])
        && !gate_ip.is_empty()
        && ledger["origin_ip"] != ledger["gate_backend_ip"]
        && report["schema"] == REPORT_SCHEMA
        && report["full_product_qualified"] == false
        && report["human_executed"] == false
        && report["scoped_passed"] == true
        && report["totals"]["scheduled"] == SUPPORTED.len() + UNSUPPORTED.len()
        && report["totals"]["evaluated"] == SUPPORTED.len()
        && report["totals"]["errors"] == 0
        && report["totals"]["expectation_mismatches"] == 0
        && rows.len() == SUPPORTED.len() + UNSUPPORTED.len()
        && cases.len() == SUPPORTED.len()
        && supported
        && unsupported
        && ledger["attacker_process_success"] == true
        && ledger["gate_exit_success"] == true
        && runtime.len() == 64
        && !sessions.is_empty()
        && sessions.iter().all(|s| {
            s["gate"]["role"] == "ac_gate_ready"
                && s["gate"]["runtime_sha256"] == runtime
                && s["gate"]["process_sha256"] == runtime
        })
        && peers.len() as u64 == origin_total
        && peers.iter().all(|peer| peer == gate_ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(seq: u64, verdict: &str) -> Value {
        json!({"seq":seq,"event_type":if verdict == "COLLECTED" {"runtime.collect"} else {"runtime.decision"},
            "verdict":verdict,"risk_score":0,"rules":[],"record_hash":"a".repeat(64)})
    }

    /// A complete passing ledger with per-class shapes taken from the catalog.
    fn ledger() -> Value {
        let mut seq = 0;
        let mut rows = Vec::new();
        let mut cases = Vec::new();
        let mut origin = 0;
        for id in SUPPORTED {
            let (class, steps, verdicts, effects): (&str, Vec<(&str, u16)>, Vec<&str>, u64) =
                match *id {
                    "synthetic-zero-motor-allow" => (
                        "synthetic_control",
                        vec![("setup", 200), ("synthetic_control", 200)],
                        vec!["COLLECTED", "ALLOW"],
                        1,
                    ),
                    "synthetic-long-window-challenge" => (
                        "synthetic_accessibility",
                        vec![("setup", 200), ("synthetic_control", 403)],
                        vec!["COLLECTED", "CHALLENGE"],
                        0,
                    ),
                    "rit-replay-after-control" => (
                        "attack",
                        vec![("setup", 200), ("synthetic_control", 200), ("attack", 403)],
                        vec!["COLLECTED", "ALLOW"],
                        1,
                    ),
                    "required-tls-refusal"
                    | "required-h2-refusal"
                    | "required-collector-refusal" => {
                        ("capability_control", vec![("attack", 422)], vec![], 0)
                    }
                    _ => (
                        "attack",
                        vec![("setup", 200), ("attack", 403)],
                        vec!["COLLECTED"],
                        0,
                    ),
                };
            let audit: Vec<_> = verdicts
                .iter()
                .map(|v| {
                    seq += 1;
                    event(seq, v)
                })
                .collect();
            origin += effects;
            let delta = json!({"origin_effects":effects,"audit":audit});
            let steps: Vec<_> = steps
                .iter()
                .map(|(role, status)| json!({"id":"s","role":role,"http_status":status,"response_verdict":null,"error":null}))
                .collect();
            rows.push(
                json!({"id":id,"tier":"T0","class":class,"status":"evaluated","steps":steps,
                "processes":[],"effects":delta,"expectation_matches":true,"reason":null}),
            );
            cases.push(json!({"case":id,"effects":delta}));
        }
        for (id, _) in UNSUPPORTED {
            rows.push(json!({"id":id,"tier":"runtime","class":"human","status":"unsupported","steps":[],
                "processes":[],"effects":null,"expectation_matches":null,"reason":"missing capability"}));
        }
        let runtime = "b".repeat(64);
        json!({"topology":{"origin":["backend"],"gate":["backend","ingress"],"attacker":["ingress"]},
            "gate_backend_ip":"172.20.0.3","origin_ip":"172.20.0.2",
            "provisioned":{"role":"ac_gate_provisioned","runtime_sha256":runtime},
            "sessions":[{"case":"x","gate":{"role":"ac_gate_ready","runtime_sha256":runtime,"process_sha256":runtime}}],
            "cases":cases,"contact_peers":vec!["172.20.0.3"; origin as usize],
            "report":{"schema":REPORT_SCHEMA,"full_product_qualified":false,"human_executed":false,
                "scoped_passed":true,"totals":{"scheduled":26,"evaluated":19,"errors":0,"unsupported":7,
                "expectation_mismatches":0},"rows":rows},
            "attacker_process_success":true,"gate_exit_success":true})
    }

    fn index(ledger: &Value, id: &str) -> usize {
        ledger["report"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .position(|row| row["id"] == id)
            .unwrap()
    }

    #[test]
    fn complete_ledger_passes() {
        assert!(catalog(&ledger()));
    }

    #[test]
    fn any_deviation_fails_the_catalog() {
        let good = ledger();
        let replay = index(&good, "rit-replay-after-control");
        let human = index(&good, "synthetic-zero-motor-allow");
        let selenium = index(&good, "synthetic-selenium-deny");
        let tls = index(&good, "required-tls-refusal");
        let unsupported = index(&good, "coherent-T4-browser");
        let mutations: Vec<(String, Value)> = vec![
            ("/topology/attacker".into(), json!(["backend", "ingress"])),
            ("/topology/origin".into(), json!(["backend", "ingress"])),
            ("/gate_backend_ip".into(), json!("")),
            ("/report/scoped_passed".into(), json!(false)),
            ("/report/full_product_qualified".into(), json!(true)),
            ("/report/totals/errors".into(), json!(1)),
            ("/report/totals/scheduled".into(), json!(25)),
            ("/attacker_process_success".into(), json!(false)),
            ("/gate_exit_success".into(), json!(false)),
            ("/contact_peers/0".into(), json!("172.20.0.9")),
            (
                "/sessions/0/gate/process_sha256".into(),
                json!("c".repeat(64)),
            ),
            ("/sessions/0/gate/role".into(), json!("ac_gate_refused")),
            (format!("/report/rows/{selenium}/status"), json!("error")),
            (
                format!("/report/rows/{selenium}/expectation_matches"),
                json!(false),
            ),
            // Go escape rule: a forwarded attack step or an attack ALLOW.
            (
                format!("/report/rows/{selenium}/steps/1/http_status"),
                json!(200),
            ),
            (
                format!("/report/rows/{replay}/steps/2/http_status"),
                json!(200),
            ),
            (
                format!("/report/rows/{tls}/steps/0/http_status"),
                json!(204),
            ),
            // Go false-positive rule: the human baseline denied.
            (
                format!("/report/rows/{human}/effects/audit/1/verdict"),
                json!("DENY"),
            ),
            // Effects not bound to the runner's measurement.
            (
                format!("/report/rows/{human}/effects/origin_effects"),
                json!(0),
            ),
            (
                format!("/report/rows/{unsupported}/status"),
                json!("skipped"),
            ),
            (format!("/report/rows/{unsupported}/reason"), json!(" ")),
        ];
        for (pointer, value) in mutations {
            let mut bad = ledger();
            *bad.pointer_mut(&pointer).unwrap() = value;
            assert!(!catalog(&bad), "{pointer}");
        }
        let mut missing = ledger();
        missing["report"]["rows"]
            .as_array_mut()
            .unwrap()
            .remove(human);
        assert!(!catalog(&missing));
        let mut duplicated = ledger();
        let first = duplicated["cases"][0].clone();
        duplicated["cases"].as_array_mut().unwrap().push(first);
        assert!(!catalog(&duplicated));
        let mut extra_contact = ledger();
        extra_contact["contact_peers"]
            .as_array_mut()
            .unwrap()
            .push(json!("172.20.0.3"));
        assert!(!catalog(&extra_contact));
    }

    #[test]
    fn effects_need_a_verified_extension_of_the_previous_history() {
        let stopped = |records: Vec<Value>| {
            json!({"role":"ac_gate_stopped","verified":true,"contiguous":true,"anchored":true,
                "sequence":records.len(),"records":records})
        };
        let first = vec![event(1, "COLLECTED")];
        let second = vec![event(1, "COLLECTED"), event(2, "ALLOW")];
        let delta = effects(&first, &stopped(second.clone()), 1).unwrap();
        assert_eq!(delta["audit"], json!([event(2, "ALLOW")]));
        assert_eq!(delta["origin_effects"], 1);
        assert!(effects(&second, &stopped(first.clone()), 0).is_none());
        let rewritten = vec![event(1, "DENY"), event(2, "ALLOW")];
        assert!(effects(&first, &stopped(rewritten), 0).is_none());
        let mut unverified = stopped(second.clone());
        unverified["verified"] = json!(false);
        assert!(effects(&first, &unverified, 0).is_none());
        let mut unanchored = stopped(second);
        unanchored["anchored"] = json!(false);
        assert!(effects(&first, &unanchored, 0).is_none());
        assert!(effects(&[], &stopped(Vec::new()), 0).is_some());
    }

    /// The runner's catalog lists are the attacker catalog, verbatim.
    #[test]
    fn lists_match_the_attacker_catalog_source() {
        let source = include_str!("../../../../../crates/attacker/src/network/cases.rs");
        let catalog = &source[..source.find("fn expected(").unwrap()];
        let quoted: Vec<&str> = catalog
            .split('"')
            .skip(1)
            .step_by(2)
            .filter(|s| !s.contains(' ') && s.contains('-'))
            .collect();
        let mut listed: Vec<&str> = SUPPORTED
            .iter()
            .copied()
            .chain(UNSUPPORTED.iter().map(|(id, _)| *id))
            .collect();
        let mut ids = quoted;
        listed.sort_unstable();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids, listed);
        let client = include_str!("../../../runtime/src/attack_catalog.rs");
        assert!(client.contains(&format!("\"{REPORT_SCHEMA}\"")));
        assert!(
            UNSUPPORTED
                .iter()
                .filter(|(_, owner)| owner.is_none())
                .count()
                == 1
        );
        for (_, owner) in UNSUPPORTED {
            if let Some(owner) = owner {
                assert!(
                    super::super::qualification::CAPABILITIES
                        .iter()
                        .any(|(_, ids)| ids.contains(owner)),
                    "{owner}"
                );
            }
        }
    }
}
