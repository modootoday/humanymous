//! Differential parity against outputs captured from the frozen Go oracle at
//! 981d25c (see the fixture's `provenance`). Expectations are never
//! regenerated to hide drift. Every captured value is compared exactly except
//! the reported differences: the loader body (Go inline script, not carried)
//! and the Go rate defaults (refused, owner decision 2026-11-16).
//!
//! The network report the kernel stores is computed by the kernel from the
//! request's own header multimap, so the stored signals and the
//! `header_projection` captures are Go outputs checked against Rust work,
//! never values the test feeds in.
use super::*;
use crate::gateway::control_flood;
use crate::gateway::control_plane::headers;
use crate::gateway::gate_config::{self, GateConfig};
use crate::network::history::rate::Limits;
use crate::network::http::HeaderInfo;
use crate::network::report::{self, Observation};
use humanymous_core::signals::observation::Signal as Observed;
use humanymous_core::signals::report::NetworkReport;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{LazyLock, OnceLock};

fn golden() -> &'static Value {
    static GOLDEN: OnceLock<Value> = OnceLock::new();
    GOLDEN.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../tests/fixtures/gateway/control-plane-go-golden.json"
        ))
        .expect("golden fixture")
    })
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

/// The reduced record shape the capture kept.
fn reduce(record: &Record) -> Value {
    json!({
        "event_type": record.event_type,
        "actor_kind": record.actor.kind,
        "actor_pseudonym_present": !record.actor.id_pseudonym.is_empty(),
        "tenant_id": record.tenant_id,
        "session_pseudonym_present": !record.session_pseudonym.is_empty(),
        "route_class": record.route_class,
        "verdict": record.verdict,
        "risk_score": record.risk_score,
        "triggered_rules": record.triggered_rules.clone().unwrap_or_default(),
        "top_signals": record.top_signals.clone().unwrap_or_default().iter()
            .map(|s| json!({"id": s.id, "verdict": s.verdict, "conf": s.conf})).collect::<Vec<_>>(),
        "enforcement_action": record.enforcement_action,
        "enforcement_mode": record.enforcement_mode,
        "fail_reason": record.fail_reason,
        "key_id": record.key_id,
    })
}

fn expected_records(records: &Value) -> Vec<Value> {
    records
        .as_array()
        .unwrap()
        .iter()
        .map(|record| {
            let mut record = record.clone();
            // Integer-valued confidences compare as JSON floats.
            for signal in record["top_signals"].as_array_mut().unwrap() {
                signal["conf"] = json!(signal["conf"].as_f64().unwrap());
            }
            record
        })
        .collect()
}

fn stored_signals() -> Vec<Observed> {
    serde_json::from_value(golden()["collect_ok_stored"]["network_signals"].clone()).unwrap()
}

/// The oracle engine's answer to the minimal capture report.
fn captured_score() -> ScoreResult {
    let body: Value = serde_json::from_str(
        golden()["exchanges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["name"] == "collect_ok")
            .unwrap()["body"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let top = golden()["collect_ok_sticky"]["top"].as_array().unwrap();
    ScoreResult {
        risk_score: body["riskScore"].as_f64().unwrap(),
        verdict: text(&body["verdict"]).into(),
        hard_rule_fired: text(&body["hardRuleFired"]).into(),
        top_contributors: Some(
            top.iter()
                .map(|s| Contributor {
                    id: text(&s["id"]).into(),
                    ..Contributor::default()
                })
                .collect(),
        ),
        ..ScoreResult::default()
    }
}

fn fake(sids: Vec<String>) -> Fake {
    Fake {
        sids,
        result: Some(captured_score()),
        ..Fake::default()
    }
}

/// Random session ids the oracle minted, in exchange order.
fn minted() -> Vec<String> {
    let mut sids: Vec<String> = golden()["exchanges"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|e| e["set_cookie"].as_array().unwrap())
        .filter_map(|c| text(c).strip_prefix("hsid="))
        .map(|c| c.split(';').next().unwrap().to_owned())
        .collect();
    sids.reverse();
    sids
}

struct Exchange {
    method: String,
    proto_major: u8,
    path: String,
    label: Option<String>,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// `(name, value)` pairs as the fixture stores them.
fn pairs(value: &Value) -> Vec<(String, String)> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| (text(&pair[0]).to_owned(), text(&pair[1]).to_owned()))
        .collect()
}

fn proto_major(seen: &Value) -> u8 {
    u8::try_from(seen["proto_major"].as_u64().unwrap()).unwrap()
}

/// What the oracle handler saw for golden exchange `name`.
fn seen(name: &str) -> &'static Value {
    let found = golden()["header_projection"]["exchanges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == name)
        .unwrap();
    &found["seen"]
}

fn exchange(e: &Value) -> Exchange {
    let raw = text(&e["path"]);
    let (path, label) = match raw.split_once("?label=") {
        Some((path, label)) => (path.to_owned(), Some(label.to_owned())),
        None => (raw.to_owned(), None),
    };
    // The handler-visible header Go's transport and server produced for the
    // recorded request (default User-Agent, Accept-Encoding, Content-Length).
    let seen = seen(text(&e["name"]));
    let body = match e["request_body_len"].as_u64() {
        Some(len) => vec![b'a'; usize::try_from(len).unwrap()],
        None => text(&e["request_body"]).as_bytes().to_vec(),
    };
    Exchange {
        method: text(&e["method"]).into(),
        proto_major: proto_major(seen),
        path,
        label,
        headers: pairs(&seen["handler_header"]),
        body,
    }
}

impl Exchange {
    fn request(&self) -> Request<'_> {
        Request {
            method: &self.method,
            proto_major: self.proto_major,
            path: &self.path,
            label: self.label.as_deref(),
            peer: "127.0.0.1".parse().unwrap(),
            session: headers::first_value(&self.headers, "Cookie").strip_prefix("hsid="),
            headers: &self.headers,
            body: &self.body,
        }
    }
}

#[test]
fn control_plane_exchanges_match_go() {
    let mut main = keyed(fake(minted())).with_loader(b"collector-bundle".to_vec());
    let mut unkeyed = plane(fake(Vec::new()));
    let tiny = Guard::with_thresholds(
        Duration::from_secs(60),
        1,
        2,
        control_flood::default_limits().unwrap(),
    )
    .unwrap();
    let mut flood = keyed(fake(Vec::new())).with_guard(tiny);
    let mut checked = 0;
    for (tick, e) in golden()["exchanges"].as_array().unwrap().iter().enumerate() {
        let name = text(&e["name"]);
        // The rebuilt request is the same exchange: the oracle answered it
        // with the same status.
        assert_eq!(seen_status(name), &e["status"], "{name}");
        let cp = if name == "stepup_unconfigured" {
            &mut unkeyed
        } else if name.starts_with("flood_") {
            &mut flood
        } else {
            &mut main
        };
        if name == "readyz_draining" {
            cp.set_ready(false);
        }
        let input = exchange(e);
        let response = cp.handle(&input.request(), now(tick as u64));
        assert_eq!(
            u64::from(response.status),
            e["status"].as_u64().unwrap(),
            "{name}"
        );
        let headers: BTreeMap<&str, &str> = [
            "Content-Type",
            "X-Content-Type-Options",
            "Retry-After",
            "Cache-Control",
        ]
        .into_iter()
        .filter_map(|h| response.header(h).map(|v| (h, v)))
        .collect();
        let want: BTreeMap<&str, &str> = e["headers"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.as_str(), text(v)))
            .collect();
        assert_eq!(headers, want, "{name}");
        let cookies: Vec<&str> = e["set_cookie"]
            .as_array()
            .unwrap()
            .iter()
            .map(text)
            .collect();
        assert_eq!(response.set_cookies(), cookies, "{name}");
        if name == "loader" {
            assert_eq!(response.body, b"collector-bundle", "{name}");
        } else {
            assert_eq!(
                String::from_utf8(response.body.clone()).unwrap(),
                text(&e["body"]),
                "{name}"
            );
        }
        let records: Vec<Value> = response.records.iter().map(reduce).collect();
        assert_eq!(records, expected_records(&e["records"]), "{name}");
        checked += 1;
    }
    assert_eq!(checked, 33);

    let stored = main.store().get("sid-collect-0001").unwrap();
    let want = &golden()["collect_ok_stored"];
    assert_eq!(stored.label, text(&want["label"]));
    assert_eq!(stored.client.session_id, text(&want["client_session_id"]));
    // Computed by the kernel from the request headers, checked against the
    // first capture's stored signals.
    assert_eq!(stored.network.signals.unwrap_or_default(), stored_signals());
    let sticky = main.verdicts().get("sid-collect-0001", Duration::ZERO);
    let want = &golden()["collect_ok_sticky"];
    assert_eq!(sticky.verdict.as_str(), text(&want["verdict"]));
    assert_eq!(sticky.risk, want["risk"].as_f64().unwrap());
    assert_eq!(sticky.rule, text(&want["rule"]));
    let top: Vec<&str> = want["top"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| text(&s["id"]))
        .collect();
    assert_eq!(sticky.top, top);
}

static DOMAIN_HEADERS: LazyLock<Vec<(String, String)>> = LazyLock::new(|| {
    fields(&[
        ("User-Agent", "Mozilla/5.0 capture"),
        ("Accept-Language", "en-US"),
        ("Sec-CH-UA", r#""Chromium";v="140""#),
    ])
});

fn domain_request(sid: &str) -> Request<'_> {
    Request {
        peer: "203.0.113.7".parse().unwrap(),
        session: Some(sid),
        headers: &DOMAIN_HEADERS,
        body: b"{}",
        ..request("POST", "/collect")
    }
}

#[test]
fn verdict_token_issuance_matches_go() {
    let want = &golden()["verdict_token"];
    let probe = domain_request("sid-token-0001");
    assert_eq!(
        token::bind_key(
            probe.header("User-Agent"),
            probe.header("Accept-Language"),
            probe.header("Sec-CH-UA")
        )
        .as_deref(),
        Some(text(&want["client_fingerprint"]))
    );
    assert_eq!(
        token_bind(&probe),
        text(&want["fingerprint_subnet_binding"])
    );
    for case in want["cases"].as_array().unwrap() {
        let verdict = text(&case["verdict"]);
        let mut cp = keyed(Fake {
            result: Some(ScoreResult {
                verdict: verdict.into(),
                ..ScoreResult::default()
            }),
            ..Fake::default()
        });
        let response = cp.handle(&domain_request("sid-token-0001"), now(0));
        let cookies: Vec<&str> = case["set_cookie"]
            .as_array()
            .unwrap()
            .iter()
            .map(text)
            .collect();
        assert_eq!(response.set_cookies(), cookies, "{verdict}");
        // The token record precedes scoring.evaluated, as in Go.
        let issued: Vec<Value> = response
            .records
            .iter()
            .filter(|r| r.event_type == EVENT_TOKEN_ISSUED)
            .map(reduce)
            .collect();
        assert_eq!(issued, expected_records(&case["records"]), "{verdict}");
        let events: Vec<&str> = scoring_records(&response)
            .iter()
            .map(|r| r.event_type.as_str())
            .collect();
        assert_eq!(events.last(), Some(&"scoring.evaluated"), "{verdict}");
    }
}

/// Records up to `scoring.evaluated`; the kernel-built network residuals that
/// follow it (Go `emitNetworkResidualAudit`) are pinned by
/// `network_residual_and_scoring_records_match_go`.
fn scoring_records(response: &Response) -> &[Record] {
    let end = response
        .records
        .iter()
        .position(|r| r.event_type == "scoring.evaluated")
        .map_or(0, |at| at + 1);
    let (scoring, residuals) = response.records.split_at(end);
    assert!(residuals.iter().all(|r| r.event_type.starts_with("net.")));
    scoring
}

#[test]
fn verdict_body_and_sticky_match_go() {
    for case in golden()["verdicts"].as_array().unwrap() {
        let result: ScoreResult = serde_json::from_value(case["result"].clone()).unwrap();
        assert_eq!(collect::verdict_body("sid-v", &result), text(&case["body"]));
        assert_eq!(text(&case["content_type"]), "application/json");
        let sticky = collect::sticky(&result, now(0));
        let want = &case["sticky"];
        assert_eq!(sticky.verdict.as_str(), text(&want["verdict"]));
        assert_eq!(sticky.risk, want["risk"].as_f64().unwrap());
        assert_eq!(sticky.rule, text(&want["rule"]));
        let top: Vec<&str> = want["top"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| text(&s["id"]))
            .collect();
        assert_eq!(sticky.top, top);
        let reduced: Vec<Value> = collect::top_signals(&result)
            .iter()
            .map(|s| json!({"id": s.id, "verdict": s.verdict, "conf": s.conf}))
            .collect();
        let want_top: Vec<Value> = want["top"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| json!({"id": s["id"], "verdict": s["verdict"], "conf": s["conf"].as_f64().unwrap()}))
            .collect();
        assert_eq!(reduced, want_top);
    }
}

#[test]
fn network_residual_and_scoring_records_match_go() {
    let mut cp = plane(Fake::default());
    let signals: Vec<Observed> = serde_json::from_value(json!([
        {"id": "l5.header.proxy_hop", "verdict": "BOT", "notes": "via: squid"},
        {"id": "l5.tcp.not_observed", "verdict": "UNKNOWN"},
        {"id": "l5.http.unmapped", "verdict": "BOT"}
    ]))
    .unwrap();
    let records: Vec<Value> = cp
        .network_residual_records("sid-net", "psn-net", &signals)
        .iter()
        .map(reduce)
        .collect();
    assert_eq!(records, expected_records(&golden()["network_residuals"]));

    let mut cp = plane(Fake {
        result: Some(ScoreResult {
            risk_score: 77.6,
            verdict: "DENY".into(),
            hard_rule_fired: "HR-02".into(),
            top_contributors: Some(
                ["b", "a"]
                    .map(|id| Contributor {
                        id: id.into(),
                        ..Contributor::default()
                    })
                    .to_vec(),
            ),
            ..ScoreResult::default()
        }),
        ..Fake::default()
    });
    let response = cp.handle(&domain_request("sid-link"), now(0));
    let records: Vec<Value> = scoring_records(&response).iter().map(reduce).collect();
    assert_eq!(records, expected_records(&golden()["link_and_audit"]));
    // The residuals after it are those of the network report the kernel built.
    let stored = cp.store().get("sid-link").unwrap();
    let signals = stored.network.signals.unwrap_or_default();
    let residuals: Vec<Value> = cp
        .network_residual_records("sid-link", "psn:sid-link:sid-link", &signals)
        .iter()
        .map(reduce)
        .collect();
    let emitted: Vec<Value> = response.records[records.len()..]
        .iter()
        .map(reduce)
        .collect();
    assert_eq!(emitted, residuals);
    assert!(!emitted.is_empty());
}

#[test]
fn gate_config_defaults_match_go() {
    let want = &golden()["config"];
    let ns = |key: &str| Duration::from_nanos(want[key].as_u64().unwrap());
    let int = |key: &str| usize::try_from(want[key].as_u64().unwrap()).unwrap();
    let zero = GateConfig::default();
    assert_eq!(zero.erasure_hold(), ns("erasure_hold_ns_zero"));
    assert_eq!(zero.cred_fanout_cap(), int("default_cred_fanout_cap"));
    assert_eq!(
        gate_config::DEFAULT_CRED_FANOUT_CAP,
        int("default_cred_fanout_cap")
    );
    let set = GateConfig {
        erasure_hold: Duration::from_secs(2),
        rate_window: Duration::from_secs(3),
        rate_soft: 7,
        rate_hard: 9,
        ..GateConfig::default()
    };
    assert_eq!(set.erasure_hold(), ns("erasure_hold_ns_set"));
    let limits = || control_flood::default_limits().unwrap();
    let rate = set.rate(limits()).unwrap();
    assert_eq!(rate.window(), ns("rl_window_ns_set"));
    assert_eq!(
        (set.rate_soft, set.rate_hard),
        (int("rl_soft_set"), int("rl_hard_set"))
    );
    // Go's zero-value defaults are the documented legacy values ...
    assert_eq!(gate_config::LEGACY_RATE_WINDOW, ns("rl_window_ns_zero"));
    assert_eq!(gate_config::LEGACY_RATE_SOFT, int("rl_soft_zero"));
    assert_eq!(gate_config::LEGACY_RATE_HARD, int("rl_hard_zero"));
    // ... but Rust refuses a zero setting (owner decision 2026-11-16).
    assert!(zero.rate(limits()).is_err());
    let _: Limits = limits();
}

fn seen_status(name: &str) -> &'static Value {
    &golden()["header_projection"]["exchanges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == name)
        .unwrap()["status"]
}

/// One request the oracle handler observed, with the input Rust receives.
struct Case {
    name: String,
    method: String,
    proto_major: u8,
    headers: Vec<(String, String)>,
    seen: &'static Value,
}

fn cases() -> Vec<Case> {
    let projection = &golden()["header_projection"];
    let mut cases = Vec::new();
    for e in golden()["exchanges"].as_array().unwrap() {
        let seen = seen(text(&e["name"]));
        cases.push(Case {
            name: text(&e["name"]).into(),
            method: text(&e["method"]).into(),
            proto_major: proto_major(seen),
            headers: pairs(&seen["handler_header"]),
            seen,
        });
    }
    for raw in projection["raw_http1"].as_array().unwrap() {
        let line = text(&raw["request_line"]);
        let seen = &raw["seen"];
        let version = line.rsplit(' ').next().unwrap();
        assert_eq!(
            format!("HTTP/{}.{}", seen["proto_major"], seen["proto_minor"]),
            version
        );
        // The wire fields exactly as written (any casing, repeats, Host) ...
        cases.push(Case {
            name: format!("{}:wire", text(&raw["name"])),
            method: line.split(' ').next().unwrap().into(),
            proto_major: proto_major(seen),
            headers: pairs(&raw["fields"]),
            seen,
        });
        // ... and the handler-visible header Go's server made of them.
        cases.push(Case {
            name: format!("{}:handler", text(&raw["name"])),
            method: line.split(' ').next().unwrap().into(),
            proto_major: proto_major(seen),
            headers: pairs(&seen["handler_header"]),
            seen,
        });
    }
    for h2 in projection["http2"].as_array().unwrap() {
        let seen = &h2["seen"];
        cases.push(Case {
            name: text(&h2["name"]).into(),
            method: "POST".into(),
            proto_major: proto_major(seen),
            headers: pairs(&seen["handler_header"]),
            seen,
        });
    }
    cases
}

impl Case {
    fn request(&self) -> Request<'_> {
        Request {
            method: &self.method,
            proto_major: self.proto_major,
            headers: &self.headers,
            ..request("POST", "/collect")
        }
    }
}

/// Go `network.Build` output; header order is sorted in both (Go's was map
/// iteration order).
fn go_build(seen: &Value) -> NetworkReport {
    serde_json::from_value(seen["build"].clone()).unwrap()
}

/// Go compares JSON numbers as float64: `2` and `2.0` are one wire value.
fn go_numbers(value: Value) -> Value {
    match value {
        Value::Number(n) => json!(n.as_f64().unwrap()),
        Value::Array(items) => Value::Array(items.into_iter().map(go_numbers).collect()),
        Value::Object(map) => {
            Value::Object(map.into_iter().map(|(k, v)| (k, go_numbers(v))).collect())
        }
        other => other,
    }
}

fn wire<T: serde::Serialize>(value: &T) -> Value {
    go_numbers(serde_json::to_value(value).unwrap())
}

#[test]
fn gate_header_projection_matches_go() {
    let cases = cases();
    // 33 exchanges, 6 raw HTTP/1.x requests twice, one HTTP/2 request.
    assert_eq!(cases.len(), 33 + 12 + 1);
    let mut versions = BTreeMap::new();
    for case in &cases {
        let name = &case.name;
        let want: HeaderInfo = serde_json::from_value(case.seen["header_info"].clone()).unwrap();
        let got = headers::header_info(&case.request());
        assert_eq!(got, want, "{name}");
        *versions.entry(got.version.clone()).or_insert(0) += 1;
        let built = report::build(&Observation {
            header: got,
            ..Observation::default()
        });
        assert_eq!(wire(&built), wire(&go_build(case.seen)), "{name}");
    }
    // HTTP/1.0 reports "11" in the gate (the Core adapter said "10").
    let h10 = cases.iter().find(|c| c.name == "h10_minimal:wire").unwrap();
    assert_eq!(h10.seen["proto_minor"], 0);
    assert_eq!(headers::header_info(&h10.request()).version, "11");
    assert_eq!(versions.get("20"), Some(&1));
    assert_eq!(headers::proto_ver(3), "11");
}

#[test]
fn collect_scores_the_projection_go_built() {
    // The proxy-residue request through the kernel: the stored network report
    // equals the one the oracle built from the same wire fields.
    let raw = golden()["header_projection"]["raw_http1"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "h11_proxy_residue")
        .unwrap();
    let wire_fields = pairs(&raw["fields"]);
    let mut cp = plane(Fake {
        result: Some(captured_score()),
        ..Fake::default()
    });
    let response = cp.handle(
        &Request {
            session: Some("sid-raw-0001"),
            headers: &wire_fields,
            body: b"{}",
            ..request("POST", "/collect")
        },
        now(0),
    );
    assert_eq!(response.status, 200);
    let stored = cp.store().get("sid-raw-0001").unwrap();
    let want = go_build(&raw["seen"]);
    assert_eq!(wire(&stored.network.signals), wire(&want.signals));
    let ids: Vec<&str> = want
        .signals
        .iter()
        .flatten()
        .map(|s| s.id.as_str())
        .collect();
    assert!(ids.contains(&"l5.header.proxy_hop"), "{ids:?}");
    assert!(ids.contains(&"l5.header.client_ip_spoof"), "{ids:?}");
}
