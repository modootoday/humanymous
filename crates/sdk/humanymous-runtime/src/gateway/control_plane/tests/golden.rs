//! Differential parity against outputs captured from the frozen Go oracle at
//! 981d25c (see the fixture's `provenance`). Expectations are never
//! regenerated to hide drift. Every captured value is compared exactly except
//! the reported differences: the loader body (Go inline script, not carried),
//! the flood breach verdict spelling (`DENY`, as `gateway::control_flood`) and
//! the Go rate defaults (refused, owner decision 2026-11-16).
use super::*;
use crate::gateway::control_flood;
use crate::gateway::gate_config::{self, GateConfig};
use crate::network::history::rate::Limits;
use humanymous_core::signals::observation::Signal as Observed;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::OnceLock;

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
            // The flood breach keeps the gateway::control_flood spelling.
            if record["event_type"] == control_flood::EVENT {
                assert_eq!(record["verdict"], "deny");
                record["verdict"] = json!("DENY");
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
        network: NetworkReport {
            signals: Some(stored_signals()),
            ..NetworkReport::default()
        },
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
    path: String,
    label: Option<String>,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

fn exchange(e: &Value) -> Exchange {
    let raw = text(&e["path"]);
    let (path, label) = match raw.split_once("?label=") {
        Some((path, label)) => (path.to_owned(), Some(label.to_owned())),
        None => (raw.to_owned(), None),
    };
    let headers = e["request_headers"]
        .as_object()
        .map(|h| {
            h.iter()
                .map(|(k, v)| (k.to_ascii_lowercase(), text(v).to_owned()))
                .collect()
        })
        .unwrap_or_default();
    let body = match e["request_body_len"].as_u64() {
        Some(len) => vec![b'a'; usize::try_from(len).unwrap()],
        None => text(&e["request_body"]).as_bytes().to_vec(),
    };
    Exchange {
        method: text(&e["method"]).into(),
        path,
        label,
        headers,
        body,
    }
}

impl Exchange {
    fn header(&self, name: &str) -> &str {
        self.headers.get(name).map_or("", String::as_str)
    }
    fn request(&self) -> Request<'_> {
        let ua = self.header("user-agent");
        Request {
            method: &self.method,
            path: &self.path,
            label: self.label.as_deref(),
            peer: "127.0.0.1".parse().unwrap(),
            session: self.header("cookie").strip_prefix("hsid="),
            // Go's transport sends its default User-Agent when none is set.
            user_agent: if ua.is_empty() {
                "Go-http-client/1.1"
            } else {
                ua
            },
            accept_language: self.header("accept-language"),
            sec_ch_ua: "",
            nonce: self.header("x-hmn-nonce"),
            receipt: self.header("x-hmn-stepup-receipt"),
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

fn domain_request(sid: &str) -> Request<'_> {
    Request {
        peer: "203.0.113.7".parse().unwrap(),
        session: Some(sid),
        sec_ch_ua: r#""Chromium";v="140""#,
        body: b"{}",
        ..request("POST", "/collect")
    }
}

#[test]
fn verdict_token_issuance_matches_go() {
    let want = &golden()["verdict_token"];
    let probe = domain_request("sid-token-0001");
    assert_eq!(
        token::bind_key(probe.user_agent, probe.accept_language, probe.sec_ch_ua).as_deref(),
        Some(text(&want["bind_key"]))
    );
    assert_eq!(token_bind(&probe), text(&want["token_bind"]));
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
        assert_eq!(
            response.records.last().unwrap().event_type,
            "scoring.evaluated"
        );
    }
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
    let records: Vec<Value> = response.records.iter().map(reduce).collect();
    assert_eq!(records, expected_records(&golden()["link_and_audit"]));
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
