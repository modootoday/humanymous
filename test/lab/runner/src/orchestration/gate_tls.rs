//! Roadmap 4 item 2a: full Gate TLS/H2/capture in the Docker topology.
//!
//! Containers (deployments/lab/compose.rust.yaml): `tg-origin` on `backend`
//! only, `tg-gate` on `ingress` and `backend`, `tg-client` on `ingress` only.
//! The Gate generates an ephemeral lab CA and server pair on its private tmpfs
//! and starts each runtime session with the closed TLS launch envelope; its
//! ingress relay is L4 passthrough, so the shipped runtime binary terminates
//! TLS and captures the original client's ClientHello, ALPN, H1 header order
//! and H2 frames. The client trusts only the published lab CA. The runner
//! operates one Gate session per row and answers each with independently
//! measured effects (offline-authenticated audit delta, origin contact log).
//!
//! Two required cases come out of one ledger:
//! - `gate`: attack-catalog rows with their unchanged catalog expectations and
//!   TLS edge rows (truthful synthetic client challenged, replay refused,
//!   ambiguous framing closed), every row's effects bound to the runner's
//!   measurement, the origin unreachable from ingress, the Gate's certificate
//!   presented on every handshake, the runtime binary bound by digest.
//! - `raw-tls-h2`: capture rows (raw-TLS and original-H2 routes decided only
//!   when the runtime captured the hello / H2 frames; original-H2 refused over
//!   TLS H1; rustls hello with a Chrome claim never forwarded; no plaintext on
//!   the TLS ingress) with every session ready in TLS mode.
//!
//! Go parity (981d25c): `cmd/gate/tls.go` operator certificate,
//! `cmd/server/tlsconn.go`, `h1capture.go`, `h2capture.go`, and
//! `test/gate/e2e.mjs` edge checks (synthetic baseline challenged, bot denied
//! with the origin untouched, smuggling refused, forged forwarding refused,
//! replay refused, origin direct hit blocked).
use super::{
    Lab,
    attack_catalog::{Actor, Operated, go_rules, operate},
    origin_audit::{address, attached, contacts, next, passphrase},
};
use crate::{docker::Result, snapshot};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const REPORT_SCHEMA: &str = "humanymous.lab-gate-tls/1";
/// Attack-catalog rows run through the TLS edge (`gate` case).
const GATE_CATALOG: &[&str] = &[
    "synthetic-selenium-deny",
    "rit-body-mutation",
    "rit-query-target-mutation",
    "rit-header-order-mutation",
    "rit-expired-deadline",
    "rit-missing-token",
    "missing-session",
    "forwarded-local-admin-forgery",
    "required-collector-refusal",
    "restart-stale-session",
    "method-route-mutation",
];
/// TLS edge rows (`gate` case).
const GATE_EDGE: &[&str] = &[
    "tls-edge-rustls-native-challenged",
    "tls-edge-replay-refused",
    "tls-edge-ambiguous-framing-refused",
];
/// Original-hop capture rows (`raw-tls-h2` case).
const CAPTURE: &[&str] = &[
    "tls-h1-required-raw-tls",
    "tls-h2-required-original-h2",
    "tls-h1-required-original-h2-refused",
    "tls-chrome-claim-rustls-hello",
    "plaintext-on-tls-ingress-refused",
];

pub(super) fn run(lab: &mut Lab) -> Result<()> {
    for service in ["tg-origin", "tg-gate", "tg-client"] {
        lab.compose(&["create", service], true)?;
    }
    let origin = lab.inspect("tg-origin")?;
    let gate = lab.inspect("tg-gate")?;
    let client = lab.inspect("tg-client")?;
    let topology = json!({"origin":attached(lab,&origin),"gate":attached(lab,&gate),
        "client":attached(lab,&client)});
    let id = |container: &Value| -> Result<String> {
        Ok(container["Id"]
            .as_str()
            .ok_or("container ID absent")?
            .to_owned())
    };
    let (origin_id, gate_id, client_id) = (id(&origin)?, id(&gate)?, id(&client)?);

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
    // Only public material crosses: the lab CA certificate.
    let ca_pem = provisioned["ca_pem"]
        .as_str()
        .ok_or("lab CA not provisioned")?
        .to_owned();

    lab.stage = "gate_tls_cases";
    let mut client_actor = Actor::start(
        lab,
        &client_id,
        &json!({"gate":gate_ingress,"origin":origin_ip,"ca_pem":ca_pem}),
    )?;
    let Operated {
        report,
        cases,
        sessions,
    } = operate(lab, &mut gate_actor, &mut client_actor, &origin_id)?;
    let client_success = client_actor.finish()?;
    gate_actor.send("exit")?;
    let gate_exit = gate_actor.finish()?;
    let (_, peers) = contacts(lab, &origin_id)?;
    let ledger = json!({"topology":topology,"gate_backend_ip":gate_backend,"origin_ip":origin_ip,
        "provisioned":provisioned,"sessions":sessions,"cases":cases,"contact_peers":peers,
        "report":report,"client_process_success":client_success,"gate_exit_success":gate_exit,
        "full_product_qualified":false});
    snapshot::write(&lab.evidence, "gate-tls-ledger.json", &ledger)?;
    let gate_passed = gate_case(&ledger);
    let capture_passed = capture_case(&ledger);
    lab.components["gate_tls"] = json!({
        "status":if gate_passed && capture_passed {"passed"} else {"failed"},
        "evidence":"gate-tls-ledger.json","full_product_qualified":false,
        "not_qualified":[
            "forwarded origin effect over TLS: the frozen scoring challenges a truthful synthetic rustls client (owner decision)",
            "browser ClientHello / uTLS / real browser H2 (no Rust browser client)",
            "Go gate admin, ban and RBAC e2e checks (not ported)",
            "operator- or ACME-issued certificate (ephemeral lab CA only)"]});
    lab.case(
        "gate",
        gate_passed,
        json!("shipped runtime terminating TLS from the closed launch envelope with a Gate-generated lab pair, L4 passthrough from an ingress-only client to a backend-only origin: catalog rows with unchanged expectations plus TLS edge rows (synthetic rustls client challenged, replay refused, ambiguous framing closed) against independently measured audit and origin effects; origin not reachable from ingress; only the Gate's certificate presented"),
    )?;
    lab.case(
        "raw-tls-h2",
        capture_passed,
        json!("raw-TLS and original-H2 routes decided only from the captured ClientHello and H2 frames, original-H2 refused over TLS H1, a rustls hello with a Chrome claim never forwarded, no plaintext fallback; every session ready in TLS mode"),
    )
}

/// Facts both cases rest on: topology, report shape, process outcomes, the
/// runtime digest on every session, TLS provisioning and certificate binding,
/// and every origin contact from the Gate backend.
fn common(ledger: &Value) -> bool {
    let topology = &ledger["topology"];
    let report = &ledger["report"];
    let provisioned = &ledger["provisioned"];
    let rows = report["rows"].as_array().map_or(&[][..], Vec::as_slice);
    let sessions = ledger["sessions"].as_array().map_or(&[][..], Vec::as_slice);
    let peers = ledger["contact_peers"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let gate_ip = ledger["gate_backend_ip"].as_str().unwrap_or("");
    let runtime = provisioned["runtime_sha256"].as_str().unwrap_or("");
    let leaf = provisioned["leaf_sha256"].as_str().unwrap_or("");
    let scheduled = GATE_CATALOG.len() + GATE_EDGE.len() + CAPTURE.len();
    let origin_total: u64 = rows
        .iter()
        .filter_map(|row| row["effects"]["origin_effects"].as_u64())
        .sum();
    topology["origin"] == json!(["backend"])
        && topology["gate"] == json!(["backend", "ingress"])
        && topology["client"] == json!(["ingress"])
        && !gate_ip.is_empty()
        && ledger["origin_ip"] != ledger["gate_backend_ip"]
        && report["schema"] == REPORT_SCHEMA
        && report["full_product_qualified"] == false
        && report["human_executed"] == false
        && report["totals"]["scheduled"] == scheduled
        && rows.len() == scheduled
        && report["origin_direct_refused"] == true
        && leaf.len() == 64
        && report["presented_certificates"] == json!([leaf])
        && report["tls_handshakes"].as_u64().is_some_and(|n| n > 0)
        && provisioned["role"] == "ac_gate_provisioned"
        && provisioned["tls"] == true
        && provisioned["ca_pem"]
            .as_str()
            .is_some_and(|pem| pem.starts_with("-----BEGIN CERTIFICATE-----\n"))
        && ledger["client_process_success"] == true
        && ledger["gate_exit_success"] == true
        && runtime.len() == 64
        && !sessions.is_empty()
        && sessions.iter().all(|s| {
            s["gate"]["role"] == "ac_gate_ready"
                && s["gate"]["tls"] == true
                && s["gate"]["runtime_sha256"] == runtime
                && s["gate"]["process_sha256"] == runtime
        })
        && peers.len() as u64 == origin_total
        && peers.iter().all(|peer| peer == gate_ip)
}

/// Every listed row evaluated, matched and bound to exactly one measurement.
fn rows_pass(ledger: &Value, ids: &[&str], catalog_rules: bool) -> bool {
    let rows = ledger["report"]["rows"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let cases = ledger["cases"].as_array().map_or(&[][..], Vec::as_slice);
    ids.iter().all(|id| {
        let mut found = cases.iter().filter(|case| case["case"] == *id);
        let measured = match (found.next(), found.next()) {
            (Some(case), None) => Some(&case["effects"]),
            _ => None,
        };
        rows.iter().filter(|row| row["id"] == *id).count() == 1
            && rows.iter().any(|row| {
                row["id"] == *id
                    && row["status"] == "evaluated"
                    && row["expectation_matches"] == true
                    && row["reason"].is_null()
                    && measured.is_some_and(|e| !e.is_null() && *e == row["effects"])
                    // No step of a TLS row is ever forwarded to the origin.
                    && row["effects"]["origin_effects"] == 0
                    && (!catalog_rules || go_rules(row))
            })
    })
}

fn gate_case(ledger: &Value) -> bool {
    common(ledger) && rows_pass(ledger, GATE_CATALOG, true) && rows_pass(ledger, GATE_EDGE, false)
}

fn capture_case(ledger: &Value) -> bool {
    let sessions = ledger["sessions"].as_array().map_or(&[][..], Vec::as_slice);
    common(ledger)
        && rows_pass(ledger, CAPTURE, false)
        && sessions
            .iter()
            .all(|s| s["gate"]["readiness"]["profile"]["raw_tls"] == true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(seq: u64, verdict: &str) -> Value {
        json!({"seq":seq,"event_type":if verdict == "COLLECTED" {"runtime.collect"} else {"runtime.decision"},
            "verdict":verdict,"risk_score":0,"rules":[],"record_hash":"a".repeat(64)})
    }

    fn ledger() -> Value {
        let mut seq = 0;
        let mut rows = Vec::new();
        let mut cases = Vec::new();
        for id in GATE_CATALOG.iter().chain(GATE_EDGE).chain(CAPTURE) {
            let (class, steps, verdicts): (&str, Vec<(&str, u16)>, Vec<&str>) = match *id {
                "required-collector-refusal" => {
                    ("capability_control", vec![("attack", 422)], vec![])
                }
                "tls-edge-replay-refused" => (
                    "tls_edge",
                    vec![("setup", 200), ("synthetic_control", 403), ("attack", 403)],
                    vec!["COLLECTED", "CHALLENGE"],
                ),
                id if GATE_CATALOG.contains(&id) => (
                    "attack",
                    vec![("setup", 200), ("attack", 403)],
                    vec!["COLLECTED"],
                ),
                _ => (
                    "tls_capture",
                    vec![("setup", 200), ("probe", 403)],
                    vec!["COLLECTED", "CHALLENGE"],
                ),
            };
            let audit: Vec<_> = verdicts
                .iter()
                .map(|v| {
                    seq += 1;
                    event(seq, v)
                })
                .collect();
            let delta = json!({"origin_effects":0,"audit":audit});
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
        let runtime = "b".repeat(64);
        let leaf = "c".repeat(64);
        let session = json!({"case":"x","gate":{"role":"ac_gate_ready","tls":true,
            "runtime_sha256":runtime,"process_sha256":runtime,
            "readiness":{"profile":{"raw_tls":true}}}});
        json!({"topology":{"origin":["backend"],"gate":["backend","ingress"],"client":["ingress"]},
            "gate_backend_ip":"172.20.0.3","origin_ip":"172.20.0.2",
            "provisioned":{"role":"ac_gate_provisioned","runtime_sha256":runtime,"tls":true,
                "ca_pem":"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n",
                "leaf_sha256":leaf},
            "sessions":[session],"cases":cases,"contact_peers":[],
            "report":{"schema":REPORT_SCHEMA,"full_product_qualified":false,"human_executed":false,
                "scoped_passed":true,"origin_direct_refused":true,"presented_certificates":[leaf],
                "tls_handshakes":40,"totals":{"scheduled":19,"evaluated":19,"errors":0,
                "expectation_mismatches":0},"rows":rows},
            "client_process_success":true,"gate_exit_success":true})
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
    fn complete_ledger_passes_both_cases() {
        assert!(gate_case(&ledger()));
        assert!(capture_case(&ledger()));
    }

    #[test]
    fn any_deviation_fails() {
        let good = ledger();
        let selenium = index(&good, "synthetic-selenium-deny");
        let edge = index(&good, "tls-edge-rustls-native-challenged");
        let h2 = index(&good, "tls-h2-required-original-h2");
        let common_mutations: Vec<(String, Value)> = vec![
            ("/topology/client".into(), json!(["backend", "ingress"])),
            ("/topology/origin".into(), json!(["backend", "ingress"])),
            ("/gate_backend_ip".into(), json!("")),
            ("/report/full_product_qualified".into(), json!(true)),
            ("/report/totals/scheduled".into(), json!(20)),
            ("/report/origin_direct_refused".into(), json!(false)),
            (
                "/report/presented_certificates".into(),
                json!(["d".repeat(64)]),
            ),
            ("/report/tls_handshakes".into(), json!(0)),
            ("/provisioned/tls".into(), json!(false)),
            ("/provisioned/ca_pem".into(), json!(null)),
            ("/client_process_success".into(), json!(false)),
            ("/gate_exit_success".into(), json!(false)),
            ("/sessions/0/gate/tls".into(), json!(false)),
            (
                "/sessions/0/gate/process_sha256".into(),
                json!("e".repeat(64)),
            ),
            ("/contact_peers".into(), json!(["172.20.0.3"])),
            // An origin effect on any row (no TLS row is ever forwarded).
            (
                format!("/report/rows/{edge}/effects/origin_effects"),
                json!(1),
            ),
        ];
        for (pointer, value) in common_mutations {
            let mut bad = ledger();
            *bad.pointer_mut(&pointer).unwrap() = value;
            assert!(!gate_case(&bad), "{pointer}");
            assert!(!capture_case(&bad), "{pointer}");
        }
        let gate_only: Vec<(String, Value)> = vec![
            (format!("/report/rows/{selenium}/status"), json!("blocked")),
            (
                format!("/report/rows/{selenium}/steps/1/http_status"),
                json!(200),
            ),
            (
                format!("/report/rows/{edge}/expectation_matches"),
                json!(false),
            ),
            (format!("/report/rows/{edge}/effects/audit"), json!([])),
        ];
        for (pointer, value) in gate_only {
            let mut bad = ledger();
            *bad.pointer_mut(&pointer).unwrap() = value;
            assert!(!gate_case(&bad), "{pointer}");
            assert!(capture_case(&bad), "{pointer}");
        }
        let capture_only: Vec<(String, Value)> = vec![
            (format!("/report/rows/{h2}/status"), json!("error")),
            (format!("/report/rows/{h2}/effects/audit"), json!([])),
            (
                "/sessions/0/gate/readiness/profile/raw_tls".into(),
                json!(false),
            ),
        ];
        for (pointer, value) in capture_only {
            let mut bad = ledger();
            *bad.pointer_mut(&pointer).unwrap() = value;
            assert!(gate_case(&bad), "{pointer}");
            assert!(!capture_case(&bad), "{pointer}");
        }
        let mut duplicated = ledger();
        let first = duplicated["cases"][0].clone();
        duplicated["cases"].as_array_mut().unwrap().push(first);
        assert!(!gate_case(&duplicated));
        let mut missing = ledger();
        missing["report"]["rows"].as_array_mut().unwrap().remove(h2);
        assert!(!capture_case(&missing));
    }

    /// The runner's row lists are the pilot's lists, verbatim.
    #[test]
    fn lists_match_the_runtime_pilot_source() {
        let source = include_str!("../../../runtime/src/gate_tls.rs");
        assert!(source.contains(&format!("\"{REPORT_SCHEMA}\"")));
        for (name, ids) in [
            ("GATE_CATALOG", GATE_CATALOG),
            ("GATE_EDGE", GATE_EDGE),
            ("CAPTURE", CAPTURE),
        ] {
            let start = source
                .find(&format!("pub const {name}: &[&str] = &["))
                .unwrap();
            let body = &source[start..start + source[start..].find("];").unwrap()];
            let quoted: Vec<&str> = body.split('"').skip(1).step_by(2).collect();
            assert_eq!(quoted, ids, "{name}");
        }
        assert!(
            super::super::qualification::CAPABILITIES
                .iter()
                .any(|(_, ids)| ids.contains(&"gate") && ids.contains(&"raw-tls-h2"))
        );
    }
}
