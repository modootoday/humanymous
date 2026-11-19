//! Parity of legacy cmd/server/powhandler.go (baseline 981d25c) against a Go
//! 1.25.3 capture of the real `issuePoWHeader`/`handlePoW` (see the fixture
//! provenance), plus the handler-level port of powhandler_launder_test.go.
use super::*;
use humanymous_core::challenge::proof_of_work::solve;
use humanymous_core::model::go_json;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::Duration;

const GOLDEN: &str = include_str!("../../../../tests/fixtures/gateway/pow-asset-go-golden.json");
const MASTER: &[u8] = b"fm11-residual-15-pow-master-0001";
const HOUR: Duration = Duration::from_secs(3600);

fn golden() -> Value {
    serde_json::from_str::<Value>(GOLDEN).unwrap()["outputs"].clone()
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}

fn at_unix(seconds: i64) -> Timestamp {
    Timestamp(time::OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

fn body_bytes(case: &Value) -> Vec<u8> {
    if let Some(hex_body) = case["body_hex"].as_str() {
        return hex::decode(hex_body).unwrap();
    }
    let mut body = text(case, "body").to_owned();
    for pad in case["pads"].as_array().into_iter().flatten() {
        let fill = text(pad, "byte").repeat(pad["len"].as_u64().unwrap() as usize);
        body = body.replacen("@PAD@", &fill, 1);
    }
    body.into_bytes()
}

/// Test-side reading of the captured `Cookie` header (Go `r.Cookie("hsid")`
/// for these simple inputs); session extraction is the transport's job.
fn hsid(cookie: &str) -> &str {
    cookie
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == "hsid")
        .map_or("", |(_, value)| value)
}

fn assert_response(actual: &Response, expected: &Value, case: &str) {
    assert_eq!(
        u64::from(actual.status),
        expected["status"].as_u64().unwrap(),
        "{case}: status"
    );
    let headers = expected["headers"].as_object().unwrap();
    assert_eq!(actual.headers.len(), headers.len(), "{case}: header set");
    for (name, value) in headers {
        assert_eq!(actual.header(name), value.as_str(), "{case}: header {name}");
    }
    assert_eq!(
        String::from_utf8_lossy(&actual.body),
        text(expected, "body"),
        "{case}: body"
    );
}

fn unscored(_: &mut SessionReport) -> Scored {
    panic!("the engine must not run for this request")
}

#[test]
fn submission_decode_matches_go_golden() {
    let golden = golden();
    let cases = golden["decode"].as_array().unwrap();
    assert!(cases.len() >= 60);
    let now = at_unix(golden["now_unix"].as_i64().unwrap());
    let mut store = CollectorStore::new(HOUR);
    store.ensure("decode-sid", now);
    for case in cases {
        let name = text(case, "name");
        let body = body_bytes(case);
        let expected = &case["decoded"];
        let decoded = decode_submission(&body);
        assert_eq!(
            decoded.is_none(),
            expected["err"].as_bool().unwrap(),
            "{name}: error"
        );
        if let Some(submission) = decoded {
            assert_eq!(
                submission.bucket,
                expected["bucket"].as_u64().unwrap(),
                "{name}: bucket"
            );
            let nonce = submission.nonce.as_bytes();
            assert_eq!(
                nonce.len() as u64,
                expected["nonce_len"].as_u64().unwrap(),
                "{name}: nonce length"
            );
            match expected["nonce_sha256"].as_str() {
                Some(digest) => assert_eq!(
                    hex::encode(Sha256::digest(nonce)),
                    digest,
                    "{name}: nonce digest"
                ),
                None => assert_eq!(
                    hex::encode(nonce),
                    text(expected, "nonce_hex"),
                    "{name}: nonce"
                ),
            }
        }
        let response = handle(
            &mut store,
            MASTER,
            "POST",
            "decode-sid",
            &body,
            now,
            unscored,
        );
        assert_response(&response, &case["response"], name);
    }
}

fn state(store: &CollectorStore, session: &str) -> Value {
    let mut state = serde_json::json!({
        "difficulty": store.pow_difficulty(session),
        "issued_set": store.pow_issued_at(session) != Timestamp::default(),
        "solved": store.pow_solved(session),
        "too_fast": store.pow_too_fast(session),
        "network_signals": 0,
    });
    let signals = store
        .get(session)
        .and_then(|report| report.network.signals)
        .unwrap_or_default();
    if let Some(last) = signals.last() {
        state["network_signals"] = signals.len().into();
        let mut last = last.clone();
        if last.id == TOO_FAST_SIGNAL {
            assert!(last.value.is_i64(), "elapsed milliseconds are an integer");
            last.value = "<elapsed_ms>".into();
        }
        state["last_signal"] = serde_json::from_slice(&go_json::to_vec(&last).unwrap()).unwrap();
    }
    state
}

#[test]
fn issue_header_matches_go_golden() {
    let golden = golden();
    let now = at_unix(golden["now_unix"].as_i64().unwrap());
    assert_eq!(bucket(now), golden["bucket"].as_u64().unwrap());
    let mut store = CollectorStore::new(HOUR);
    store.ensure("hdr-sid", now);
    store.ensure("hdr-solved", now);
    store.set_pow_solved("hdr-solved", now);
    let steps = golden["issue_header"].as_array().unwrap();
    assert_eq!(steps.len(), 9);
    for step in steps {
        let name = text(step, "name");
        let session = text(step, "session");
        assert_eq!(
            store.get(session).is_some(),
            step["known"].as_bool().unwrap(),
            "{name}: known"
        );
        let header = issue_header(
            &mut store,
            MASTER,
            session,
            text(step, "verdict"),
            step["risk"].as_f64().unwrap(),
            now,
        );
        assert_eq!(
            header.as_deref().unwrap_or_default(),
            text(step, "header"),
            "{name}: header"
        );
        assert_eq!(state(&store, session), step["after"], "{name}: state");
    }
}

#[test]
fn handle_matches_go_golden() {
    let golden = golden();
    let now_unix = golden["now_unix"].as_i64().unwrap();
    let now = at_unix(now_unix);
    let an_hour_ago = at_unix(now_unix - 3600);
    let mut store = CollectorStore::new(HOUR);
    let flows = golden["handle"].as_array().unwrap();
    assert_eq!(flows.len(), 14);
    for flow in flows {
        let name = text(flow, "name");
        // Replays the capture program's setup before the named step.
        match name {
            "wrong_nonce" => {
                store.ensure("flow-a", now);
                store.set_pow_issued("flow-a", an_hour_ago, 16);
            }
            "too_fast" => {
                store.ensure("flow-b", now);
                store.set_pow_issued("flow-b", now, 16);
            }
            "sticky_flag_plausible_clock" => {
                store.ensure("flow-c", now);
                store.set_pow_issued("flow-c", an_hour_ago, 17);
                store.set_pow_too_fast("flow-c");
            }
            "folded_keys_solve" => {
                store.ensure("flow-d", now);
                store.set_pow_issued("flow-d", an_hour_ago, 14);
            }
            _ => {}
        }
        let expected = &flow["response"];
        let reply: Value = serde_json::from_str(text(expected, "body")).unwrap_or(Value::Null);
        let mut scored = false;
        let session = hsid(text(flow, "cookie"));
        let response = handle(
            &mut store,
            MASTER,
            text(flow, "method"),
            session,
            text(flow, "body").as_bytes(),
            now,
            |_| {
                scored = true;
                Scored {
                    verdict: text(&reply, "verdict").to_owned(),
                    risk_score: reply["riskScore"].as_f64().unwrap_or_default(),
                }
            },
        );
        assert_eq!(
            scored,
            reply.get("verdict").is_some(),
            "{name}: the engine runs exactly when Go re-scored"
        );
        assert_response(&response, expected, name);
        assert_eq!(state(&store, session), flow["after"], "{name}: state");
    }
}

/// Port of TestPoWTooFastCannotBeLaunderedByDelayedResubmit at the handler.
#[test]
fn too_fast_cannot_be_laundered_by_delayed_resubmit() {
    let sid = "launder-sid";
    let master = b"test-master-key-0123456789abcdef";
    let difficulty = 16;
    let start = at_unix(1_800_000_000);
    let window = bucket(start);
    let challenge = issue(master, sid, difficulty, window);
    let nonce = solve(&challenge.seed, difficulty, 1 << 24).unwrap();
    let mut store = CollectorStore::new(HOUR);
    store.ensure(sid, start);
    store.set_pow_issued(sid, start, difficulty);
    let body = format!(r#"{{"bucket":{window},"nonce":"{nonce}"}}"#);
    let mut post = |now: Timestamp| {
        let response = handle(
            &mut store,
            master,
            "POST",
            sid,
            body.as_bytes(),
            now,
            |_| Scored {
                verdict: "DENY".to_owned(),
                risk_score: 100.0,
            },
        );
        serde_json::from_slice::<Value>(&response.body).unwrap()
    };
    let first = post(Timestamp(start.0 + Duration::from_micros(50)));
    assert_eq!(first["ok"], false, "instant submit is too fast: {first}");
    let floor = plausible_browser_solve(difficulty).unwrap();
    let second = post(Timestamp(start.0 + floor + Duration::from_millis(50)));
    assert_ne!(second["ok"], true, "delayed resubmit laundered: {second}");
    assert!(store.pow_too_fast(sid));
    assert!(!store.pow_solved(sid));
}

#[test]
fn non_finite_risk_score_writes_no_json() {
    let sid = "edge-sid";
    let now = at_unix(1_800_000_000);
    let mut store = CollectorStore::new(HOUR);
    store.ensure(sid, now);
    store.set_pow_issued(sid, at_unix(1_700_000_000), 1);
    let nonce = solve(&issue(MASTER, sid, 1, bucket(now)).seed, 1, 1 << 16).unwrap();
    let body = format!(r#"{{"bucket":{},"nonce":"{nonce}"}}"#, bucket(now));
    let response = handle(
        &mut store,
        MASTER,
        "POST",
        sid,
        body.as_bytes(),
        now,
        |_| Scored {
            verdict: "ALLOW".to_owned(),
            risk_score: f64::NAN,
        },
    );
    assert_eq!(response.status, 200);
    assert_eq!(response.header("Content-Type"), Some("application/json"));
    assert!(response.body.is_empty(), "Go wrote nothing on encode error");
}
