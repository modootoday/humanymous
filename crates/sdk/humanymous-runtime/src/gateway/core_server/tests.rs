//! Core server parity against the captured Go cmd/server behaviour (fixture
//! `core-server-go-golden.json`, Go 1.26.3 at oracle 981d25c) plus the
//! deliberate differences, framing, and actual loopback/TLS socket serving.
use super::http::{Refused, Request, encode, query_value, read_request, unescape};
use super::serve::{Options, bind, serve};
use super::*;
use crate::gateway::connection_cap::ConnectionCap;
use crate::gateway::logging::{Identity, RawSettings, Role, Settings};
use crate::gateway::resource;
use crate::network::history::Unavailable;
use crate::network::http::cookie_value;
use humanymous_core::integrity::request::time_bucket;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::PathBuf;

const GOLDEN: &str = include_str!("../../../tests/fixtures/gateway/core-server-go-golden.json");
const NOW: u64 = 1_790_000_000;

fn golden() -> Value {
    serde_json::from_str::<Value>(GOLDEN).unwrap()["capture"].clone()
}

fn key() -> Vec<u8> {
    (0u8..32).collect()
}

struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> Self {
        let mut nonce = [0u8; 8];
        getrandom::fill(&mut nonce).unwrap();
        let path = std::env::temp_dir().join(format!("hmn-core-{tag}-{}", hex::encode(nonce)));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, body: &[u8]) {
        let path = self.0.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }
    fn text(&self) -> String {
        self.0.to_str().unwrap().to_owned()
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn web() -> TempDir {
    let dir = TempDir::new("web");
    dir.write("index.html", b"<!doctype html><title>index</title>\n");
    dir.write("demo.html", b"<!doctype html><title>demo</title>\n");
    dir.write("js/app.js", b"console.log(1);\n");
    dir.write("css/site.css", b"body{}\n");
    dir.write("data/x.unknownext", b"raw\n");
    dir
}

fn limits() -> Limits {
    Limits {
        traffic: traffic::Limits::new(4096, 4096, 4096, 64 << 20).unwrap(),
        correlation: correlation::Limits::new(4096, 4096, 256, 4096, 64 << 20).unwrap(),
        correlation_population: anomaly::Limits::new(16, 256, 256, 1_000_000).unwrap(),
        rate: rate::Limits::new(4096, 4096, 4096, Duration::from_secs(60), 64 << 20, 4096).unwrap(),
        watermark_capacity: 4096,
        watermark_codec: frame::Limits::new(1 << 20, 2 << 20, 8).unwrap(),
        fingerprint_sessions: 4096,
    }
}

fn config(web_dir: &str, rit: bool) -> Config {
    Config {
        web_dir: web_dir.to_owned(),
        master_key: Zeroizing::new(key()),
        rit_enabled: rit,
        ops_token: "optok".to_owned(),
        // Go trusted private peers implicitly; C1 needs the explicit list.
        trusted_proxies: vec![Cidr::parse("10.0.0.0/8").unwrap()],
        version: "dev".to_owned(),
        watermark_fallback: Fallback::OpaqueTrailer,
        limits: limits(),
    }
}

fn fixed_clock() -> Clock {
    Arc::new(|| UNIX_EPOCH + Duration::from_secs(NOW))
}

fn server(web_dir: &str, rit: bool) -> CoreServer {
    CoreServer::new(config(web_dir, rit), fixed_clock()).unwrap()
}

fn current_tb() -> u64 {
    time_bucket(NOW as i64, rit::WINDOW as i64)
}

fn str_of(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

fn signal_json(signal: &humanymous_core::signals::observation::Signal) -> Value {
    json!({
        "id": signal.id,
        "value": signal.value,
        "verdict": signal.verdict.as_str(),
        "notes": signal.notes,
    })
}

#[test]
fn rit_canonical_seed_and_mac_match_go() {
    let key = key();
    for case in golden()["ritCanonical"].as_array().unwrap() {
        let body = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            str_of(&case["body"]),
        )
        .unwrap();
        let sid = str_of(&case["sid"]);
        let n = case["n"].as_u64().unwrap();
        let tb = case["tb"].as_u64().unwrap();
        let canonical = rit::canonical(sid, n, tb, &body);
        assert_eq!(canonical, str_of(&case["canonical"]));
        let prior = n.wrapping_sub(1);
        assert_eq!(rit::seed_text(&key, sid, prior), str_of(&case["seed"]));
        assert_eq!(
            rit::token(&key, sid, prior, &canonical),
            str_of(&case["mac"])
        );
    }
}

#[test]
fn within_tb_and_parse_uint_match_go() {
    let golden = golden();
    for case in golden["withinTB"].as_array().unwrap() {
        let (a, b, tol) = (
            case["A"].as_u64().unwrap(),
            case["B"].as_u64().unwrap(),
            case["Tol"].as_u64().unwrap(),
        );
        assert_eq!(rit::within_tb(a, b, tol), case["Within"].as_bool().unwrap());
    }
    for case in golden["parseUint"].as_array().unwrap() {
        assert_eq!(
            rit::parse_uint(str_of(&case["in"])),
            case["out"].as_u64().unwrap(),
            "{case}"
        );
    }
}

#[test]
fn verify_rit_sequence_matches_go() {
    let key = key();
    let mut store = CollectorStore::new(SESSION_TTL);
    let now = timestamp(Duration::from_secs(NOW));
    for step in golden()["verifyRIT"].as_array().unwrap() {
        let sid = str_of(&step["sid"]);
        let n = step["n"].as_u64().unwrap();
        let tb = current_tb()
            .checked_add_signed(step["tbOffset"].as_i64().unwrap())
            .unwrap();
        let body = str_of(&step["body"]).as_bytes();
        let before = store.rit_counter(sid);
        assert_eq!(before, step["counterBefore"].as_u64().unwrap(), "{step}");
        let token = match str_of(&step["token"]) {
            "valid" => rit::token(&key, sid, before, &rit::canonical(sid, n, tb, body)),
            "bad" => "AAAA".to_owned(),
            _ => String::new(),
        };
        let n_header = match str_of(&step["nHeader"]) {
            "" => n.to_string(),
            set => set.to_owned(),
        };
        let tb_header = tb.to_string();
        let signal = rit::verify(
            &mut store,
            &key,
            sid,
            rit::Presented {
                token: &token,
                n: &n_header,
                tb: &tb_header,
            },
            body,
            now,
        );
        let mut expected = step["signal"].clone();
        if str_of(&expected["notes"]).contains("time bucket") {
            // The stale-bucket value is the presented bucket, which Go
            // derived from its wall clock at capture: rebase it on ours.
            let go_now = expected["value"].as_i64().unwrap() - step["tbOffset"].as_i64().unwrap();
            assert!(go_now > 0);
            expected["value"] = Value::from(tb);
        }
        assert_eq!(signal_json(&signal), expected, "{step}");
        assert_eq!(signal.confidence, 1.0);
        assert_eq!(signal.collected.as_str(), "server");
        assert_eq!(
            store.rit_counter(sid),
            step["counterAfter"].as_u64().unwrap()
        );
        assert_eq!(
            rit::issue_seed(&store, &key, sid).0,
            str_of(&step["seedAfter"])
        );
    }
}

/// Go's encoder writes `0` where serde_json keeps `0.0`: compare numbers as
/// f64 values.
fn numbers(value: Value) -> Value {
    match value {
        Value::Number(number) => number.as_f64().map_or(Value::Number(number), Value::from),
        Value::Array(list) => Value::Array(list.into_iter().map(numbers).collect()),
        Value::Object(map) => {
            Value::Object(map.into_iter().map(|(k, v)| (k, numbers(v))).collect())
        }
        other => other,
    }
}

fn empty_signal(signal: Option<humanymous_core::signals::observation::Signal>) -> Value {
    match signal {
        Some(signal) => signal_json(&signal),
        // Go's zero Signal.
        None => json!({"id": "", "value": null, "verdict": "", "notes": ""}),
    }
}

#[test]
fn fingerprint_churn_and_webrtc_leak_match_go() {
    let golden = golden();
    let mut tracker = FingerprintTracker::new(16);
    for case in golden["fpChurn"].as_array().unwrap() {
        let signal = tracker
            .observe(str_of(&case["Sid"]), str_of(&case["FP"]), Duration::ZERO)
            .unwrap();
        assert_eq!(empty_signal(signal), case["Sig"], "{case}");
    }
    for case in golden["webrtcLeak"].as_array().unwrap() {
        let addrs: Vec<String> = case["addrs"]
            .as_array()
            .map(|list| list.iter().map(|a| str_of(a).to_owned()).collect())
            .unwrap_or_default();
        let signal = signals::webrtc_leak(&addrs, str_of(&case["tcp"]));
        assert_eq!(empty_signal(signal), case["sig"], "{case}");
    }
}

#[test]
fn fingerprint_memory_is_bounded_and_expires() {
    let mut tracker = FingerprintTracker::new(1);
    assert!(tracker.observe("a", "x", Duration::ZERO).unwrap().is_none());
    assert_eq!(
        tracker.observe("b", "x", Duration::ZERO).unwrap_err(),
        Unavailable::Budget
    );
    tracker.gc(SESSION_TTL + Duration::from_secs(1), SESSION_TTL);
    assert!(tracker.is_empty());
    assert!(tracker.observe("b", "x", Duration::ZERO).unwrap().is_none());
    assert_eq!(tracker.len(), 1);
}

fn mask_cookie(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("hsid=") {
        out.push_str(&rest[..at + 5]);
        let tail = &rest[at + 5..];
        if tail.len() >= 32 && tail[..32].bytes().all(|b| b.is_ascii_hexdigit()) {
            out.push_str("<random>");
            rest = &tail[32..];
        } else {
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

fn fresh_sid(response: &Response) -> Option<String> {
    let cookie = response.header("Set-Cookie")?;
    Some(cookie.strip_prefix("hsid=")?.split(';').next()?.to_owned())
}

fn build_request(case: &Value) -> Request {
    let mut headers = vec![("Host".to_owned(), "example.com".to_owned())];
    let mut sid = String::new();
    for pair in case["headers"].as_array().into_iter().flatten() {
        let (name, value) = (str_of(&pair[0]), str_of(&pair[1]));
        if name == "Cookie" {
            sid = value.trim_start_matches("hsid=").to_owned();
        }
        headers.push((name.to_owned(), value.to_owned()));
    }
    let body = str_of(&case["body"]).as_bytes().to_vec();
    if let Some(at) = case["tokenAt"].as_u64() {
        let tb = current_tb();
        let canonical = rit::canonical(&sid, at, tb, &body);
        headers.push((
            "X-HM-Token".into(),
            rit::token(&key(), &sid, at - 1, &canonical),
        ));
        headers.push(("X-HM-N".into(), at.to_string()));
        headers.push(("X-HM-TB".into(), tb.to_string()));
    }
    let remote = match str_of(&case["remote"]) {
        "" => "192.0.2.1:1234",
        set => set,
    };
    Request::new(
        str_of(&case["method"]),
        str_of(&case["target"]),
        headers,
        body,
        remote,
    )
}

/// Go headers that this port deliberately does not send (no conditional or
/// range serving), plus `Content-Length`, which is added when encoding.
const NOT_IN_HANDLER: [&str; 3] = ["Content-Length", "Last-Modified", "Accept-Ranges"];

#[test]
fn http_routes_match_go() {
    let golden = golden();
    let web = web();
    let server = server(&web.text(), true);
    let key = key();
    for case in golden["http"].as_array().unwrap() {
        let name = str_of(&case["name"]);
        let request = build_request(case);
        let response = server.handle(&request, &Connection::default());
        if name == "pass-mount" {
            // Go mounted its Pass store; here the Pass owner mounts PassRoutes.
            assert_eq!(response.status, 404, "{name}");
            continue;
        }
        assert_eq!(
            u64::from(response.status),
            case["status"].as_u64().unwrap(),
            "{name}"
        );
        let fresh = fresh_sid(&response);
        let mut ours: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (header, values) in &response.headers {
            let values = match header.as_str() {
                "X-Hm-Pow" => vec!["<present>".to_owned()],
                "X-Hm-Seed" if fresh.is_some() => {
                    assert_eq!(
                        values[0],
                        rit::seed_text(&key, fresh.as_deref().unwrap(), 0)
                    );
                    continue;
                }
                _ => values.iter().map(|v| mask_cookie(v)).collect(),
            };
            ours.insert(header.clone(), values);
        }
        let mut theirs: BTreeMap<String, Vec<String>> =
            serde_json::from_value(case["respHeaders"].clone()).unwrap();
        for header in NOT_IN_HANDLER {
            theirs.remove(header);
        }
        if fresh.is_some() {
            theirs.remove("X-Hm-Seed");
        }
        if name == "static-unknown" {
            // Go sniffed text/plain; no sniffing here (deviation).
            assert_eq!(ours["Content-Type"], ["application/octet-stream"]);
            theirs.insert(
                "Content-Type".into(),
                vec!["application/octet-stream".into()],
            );
        }
        assert_eq!(ours, theirs, "{name}");
        let body = if request.method == "HEAD" {
            String::new()
        } else {
            String::from_utf8_lossy(&response.body).into_owned()
        };
        let expected = str_of(&case["respBody"]);
        if name == "report-own" {
            let report: Value = serde_json::from_str(&body).unwrap();
            assert_eq!(report["sessionId"], "s2");
        } else if let Some(sid) = &fresh
            && body.starts_with('{')
        {
            let mut mine: Value = serde_json::from_str(&body).unwrap();
            let mut go: Value = serde_json::from_str(expected).unwrap();
            assert_eq!(mine["sessionId"], sid.as_str());
            if mine.get("ritSeed").is_some() {
                assert_eq!(mine["ritSeed"], rit::seed_text(&key, sid, 0).as_str());
                mine["ritSeed"] = Value::Null;
                go["ritSeed"] = Value::Null;
            }
            mine["sessionId"] = Value::Null;
            go["sessionId"] = Value::Null;
            assert_eq!(mine, go, "{name}");
        } else {
            assert_eq!(body, expected, "{name}");
        }
    }

    // Final stored state per session.
    let (state, _) = server.lock().unwrap();
    let mut random = Vec::new();
    let stored: BTreeMap<String, Value> = golden["stored"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| (str_of(&entry["sid"]).to_owned(), entry.clone()))
        .collect();
    for report in state.store.list(1000) {
        let sid = report.session_id.clone();
        let signals: Vec<Value> = report
            .network
            .signals
            .iter()
            .flatten()
            .map(signal_json)
            .collect();
        let summary = json!({
            "label": report.label,
            "signals": if signals.is_empty() { Value::Null } else { Value::from(signals) },
            "verdict": report.scoring.verdict,
            "riskScore": report.scoring.risk_score,
            "ritCounter": state.store.rit_counter(&sid),
            "hardRule": report.scoring.hard_rule_fired,
        });
        match stored.get(&sid) {
            Some(go) => {
                let mut go = go.clone();
                go.as_object_mut().unwrap().remove("sid");
                assert_eq!(numbers(summary), numbers(go), "{sid}");
            }
            None => random.push(numbers(summary)),
        }
    }
    let go_random: Vec<Value> = stored
        .iter()
        .filter(|(sid, _)| sid.len() == 32)
        .map(|(_, go)| {
            let mut go = go.clone();
            go.as_object_mut().unwrap().remove("sid");
            numbers(go)
        })
        .collect();
    let canonical =
        |list: &[Value]| -> BTreeSet<String> { list.iter().map(Value::to_string).collect() };
    assert_eq!(random.len(), go_random.len());
    assert_eq!(canonical(&random), canonical(&go_random));

    // Traffic log records for s2 and x1 (timestamps masked).
    let mut records = Vec::new();
    for sid in ["s2", "x1"] {
        let reply = routes::traffic_json(&state.traffic.session(sid));
        for mut record in serde_json::from_slice::<Vec<Value>>(&reply.body).unwrap() {
            record["ts"] = Value::from("<masked>");
            records.push(record);
        }
    }
    assert_eq!(Value::from(records), golden["traffic"]);
}

#[test]
fn rate_limiter_matches_go() {
    let golden = golden();
    let web = web();
    let server = server(&web.text(), false);
    let mut verdicts = Vec::new();
    for _ in 0..85 {
        let request = Request::new(
            "POST",
            "/api/collect",
            vec![
                ("Host".into(), "example.com".into()),
                ("Cookie".into(), "hsid=rl".into()),
            ],
            b"{}".to_vec(),
            "192.0.2.1:1234",
        );
        let response = server.handle(&request, &Connection::default());
        let reply: Value = serde_json::from_slice(&response.body).unwrap();
        verdicts.push(reply["verdict"].clone());
    }
    assert_eq!(Value::from(verdicts), golden["rateVerdicts"]);
    let (state, _) = server.lock().unwrap();
    let ids: Vec<Value> = state
        .store
        .get("rl")
        .unwrap()
        .network
        .signals
        .iter()
        .flatten()
        .map(|signal| Value::from(signal.id.clone()))
        .collect();
    assert_eq!(Value::from(ids), golden["rateSignals"]);
}

fn get(server: &CoreServer, target: &str, headers: &[(&str, &str)]) -> Response {
    let mut fields = vec![("Host".to_owned(), "example.com".to_owned())];
    fields.extend(
        headers
            .iter()
            .map(|(n, v)| ((*n).to_owned(), (*v).to_owned())),
    );
    server.handle(
        &Request::new("GET", target, fields, Vec::new(), "192.0.2.1:1234"),
        &Connection::default(),
    )
}

#[test]
fn trusted_proxy_headers_are_ignored_without_an_allowlist() {
    let web = web();
    let mut config = config(&web.text(), false);
    config.trusted_proxies.clear();
    let server = CoreServer::new(config, fixed_clock()).unwrap();
    let request = Request::new(
        "POST",
        "/api/collect",
        vec![
            ("Host".into(), "example.com".into()),
            ("Cookie".into(), "hsid=x".into()),
            ("X-Forwarded-For".into(), "198.51.100.10".into()),
        ],
        b"{}".to_vec(),
        "10.0.0.5:4000",
    );
    assert_eq!(server.handle(&request, &Connection::default()).status, 200);
    let (state, _) = server.lock().unwrap();
    assert_eq!(state.traffic.session("x")[0].remote_addr, "10.0.0.5");
}

#[test]
fn ops_views_are_absent_and_operator_views_need_the_token() {
    let web = web();
    let server = server(&web.text(), false);
    for path in ["/api/explain/s", "/api/counters", "/api/mlcorrect"] {
        let response = get(&server, path, &[("Authorization", "Bearer optok")]);
        assert_eq!(response.status, 404, "{path}");
    }
    let list = get(&server, "/api/report", &[("Authorization", "Bearer optok")]);
    assert_eq!(
        (list.status, list.body.as_slice()),
        (200, b"[]\n".as_slice())
    );
    let missing = get(
        &server,
        "/api/traffic/none",
        &[("Authorization", "Bearer optok")],
    );
    assert_eq!(missing.body, b"null\n");
    let mut config = config(&web.text(), false);
    config.ops_token.clear();
    let unconfigured = CoreServer::new(config, fixed_clock()).unwrap();
    let response = get(
        &unconfigured,
        "/api/report",
        &[("Authorization", "Bearer ")],
    );
    assert_eq!(response.status, 404);
}

struct StubPass;
impl PassRoutes for StubPass {
    fn handle(&self, request: &PassRequest<'_>) -> resource::Response {
        resource::Response::json(&json!({"path": request.path, "session": request.session}))
    }
}

struct PanicPass;
impl PassRoutes for PanicPass {
    fn handle(&self, _: &PassRequest<'_>) -> resource::Response {
        panic!("pass handler failure");
    }
}

#[test]
fn pass_routes_mount_point_dispatches_every_pass_path() {
    let web = web();
    let server = server(&web.text(), false).with_pass_routes(Arc::new(StubPass));
    for path in [
        "/pass",
        "/api/pass/new",
        "/api/pass/pow",
        "/api/pass/attest",
        "/api/pass/solve",
        "/api/pass/kpi",
    ] {
        let response = get(&server, path, &[("Cookie", "hsid=p")]);
        assert_eq!(response.status, 200, "{path}");
        assert_eq!(
            serde_json::from_slice::<Value>(&response.body).unwrap(),
            json!({"path": path, "session": "p"})
        );
        assert!(
            response
                .header("Content-Security-Policy-Report-Only")
                .is_some()
        );
    }
    assert_eq!(get(&server, "/api/pass/other", &[]).status, 404);
}

#[derive(Clone, Default)]
struct Shared(Arc<Mutex<Vec<u8>>>);
impl Write for Shared {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn logger(out: &Shared) -> OperationalLogger {
    let settings = Settings::validate(
        Role::Core,
        &RawSettings {
            level: "info",
            console_format: "jsonl",
            console_stream: "stdout",
            plain_file: "",
            jsonl_file: "",
        },
    )
    .unwrap();
    OperationalLogger::open(
        Identity::new(Role::Core, "v-test", "node-test"),
        &settings,
        Box::new(out.clone()),
        Box::new(Shared::default()),
    )
    .unwrap()
}

#[test]
fn observability_records_requests_and_recovers_panics() {
    let web = web();
    let out = Shared::default();
    let server = server(&web.text(), false)
        .with_operational_logger(logger(&out))
        .with_pass_routes(Arc::new(PanicPass));
    assert_eq!(get(&server, "/healthz", &[]).status, 200);
    let panicked = get(&server, "/api/pass/kpi", &[]);
    assert_eq!(panicked.status, 500);
    assert!(panicked.body.is_empty());
    assert!(panicked.header("X-Content-Type-Options").is_some());
    let text = String::from_utf8(out.0.lock().unwrap().clone()).unwrap();
    assert!(text.contains("http.request_completed"), "{text}");
    assert!(text.contains("http.panic_recovered"), "{text}");
    assert!(!text.contains("pass handler failure"));

    // Logging off: Go's net/http dropped the connection (status 0 here).
    let quiet = self::server(&web.text(), false).with_pass_routes(Arc::new(PanicPass));
    assert_eq!(get(&quiet, "/api/pass/kpi", &[]).status, 0);
}

#[test]
fn collect_publishes_live_feed_and_writes_external_receipts() {
    let web = web();
    web.write("detector.wasm", b"\x00asm-core-test");
    let receipts = TempDir::new("receipts");
    let writer = external_input::Writer::configure(Some(&receipts.0), &web.0)
        .unwrap()
        .unwrap();
    let hub = Hub::new(16, 16);
    let server = server(&web.text(), false)
        .with_live_feed(Arc::clone(&hub))
        .with_external_input_receipts(writer);
    let wasm_report = serde_json::to_vec(&json!({
        "engineVersion": "rust-wasm-0.1.0",
        "signals": [{"id": "l1.navigator.webdriver", "value": false, "verdict": "OK", "confidence": 1, "collected": "wasm"}]
    }))
    .unwrap();
    let collect = |label: &str, body: &[u8]| {
        let target = format!("/api/collect?label={label}");
        server.handle(
            &Request::new(
                "POST",
                &target,
                vec![
                    ("Host".into(), "example.com".into()),
                    ("Cookie".into(), "hsid=e1".into()),
                ],
                body.to_vec(),
                "192.0.2.1:1234",
            ),
            &Connection::default(),
        )
    };
    let label = "external-input/run-1/core";
    // Without WASM evidence the receipt is refused and the reply fails closed.
    let refused = collect(label, b"{}");
    assert_eq!(
        (refused.status, refused.body.as_slice()),
        (500, b"measurement receipt failed\n".as_slice())
    );
    assert!(!receipts.0.join(external_input::RECEIPT_FILE).exists());
    assert_eq!(collect(label, &wasm_report).status, 200);
    assert!(receipts.0.join(external_input::RECEIPT_FILE).exists());
    let (events, _subscription) = hub.subscribe(0);
    assert_eq!(events[0].kind, "session.scored");
    let scored: Value = serde_json::from_str(&events[0].data).unwrap();
    assert_eq!(scored["source"], format!("profile:{label}"));
    // Go published before the receipt, so the refused collect is on the feed too.
    let scored = events
        .iter()
        .filter(|event| event.kind == "session.scored")
        .count();
    assert_eq!(scored, 2);
    assert!(
        events
            .iter()
            .all(|event| event.kind == "session.scored" || event.kind == "network.residual")
    );
    assert!(events.len() > 2);
    // The receipt is exclusive: a second publication fails closed.
    let refused = collect(label, &wasm_report);
    assert_eq!(refused.status, 500);
    assert_eq!(refused.body, b"measurement receipt failed\n");
}

#[test]
fn resource_is_gated_watermarked_and_signals_recorded() {
    let web = web();
    let server = server(&web.text(), false);
    let fresh = get(&server, "/res/js/app.js", &[]);
    // Unknown trust is CHALLENGE: a script is light enough to serve.
    assert!(fresh.header("Set-Cookie").is_some());
    assert!(fresh.header("X-HM-Gate").is_some());
    if fresh.status == 200 {
        assert_eq!(fresh.header("Cache-Control"), Some("no-store"));
        assert!(fresh.header("X-HM-Watermark").is_some());
    }
    let bad = get(&server, "/res/a/../../../x", &[("Cookie", "hsid=r")]);
    assert_eq!(bad.status, 307); // the mux cleans the escaped path first
    // Escaped traversal never leaves the root (Go res-bad: 404).
    let traversal = get(&server, "/res/..%2f..%2fx", &[("Cookie", "hsid=r")]);
    assert_eq!(traversal.status, 404);
}

#[test]
fn trace_reads_only_for_an_operator_post() {
    let web = web();
    let server = server(&web.text(), false);
    let head = |method: &str, auth: &str| {
        let mut headers = vec![("Host".to_owned(), "x".to_owned())];
        if !auth.is_empty() {
            headers.push(("Authorization".into(), auth.into()));
        }
        Request::new(method, "/api/trace", headers, Vec::new(), "127.0.0.1:1")
    };
    assert_eq!(server.body_limit(&head("POST", "")), 0);
    assert_eq!(server.body_limit(&head("GET", "Bearer optok")), 0);
    assert_eq!(
        server.body_limit(&head("POST", "Bearer optok")),
        resource::MAX_TRACE_BODY
    );
    assert_eq!(
        server.body_limit(&Request::new("POST", "/api/collect", vec![], vec![], "x")),
        COLLECT_BODY
    );
    let mut traced = head("POST", "Bearer optok");
    traced.body = b"no carrier".to_vec();
    let reply = server.handle(&traced, &Connection::default());
    assert_eq!(reply.status, 200);
    assert_eq!(
        serde_json::from_slice::<Value>(&reply.body).unwrap()["traced"],
        false
    );
}

#[test]
fn static_file_server_follows_go_redirects_without_listing() {
    let web = web();
    web.write("sub/index.html", b"<p>sub</p>");
    std::fs::create_dir(web.0.join("empty")).unwrap();
    let server = server(&web.text(), false);
    let sub = get(&server, "/static/sub", &[]);
    assert_eq!((sub.status, sub.header("Location")), (301, Some("sub/")));
    let index = get(&server, "/static/sub/", &[]);
    assert_eq!(
        (index.status, index.body.as_slice()),
        (200, b"<p>sub</p>".as_slice())
    );
    let file_slash = get(&server, "/static/js/app.js/?q=1", &[]);
    assert_eq!(
        (file_slash.status, file_slash.header("Location")),
        (301, Some("../app.js?q=1"))
    );
    assert_eq!(get(&server, "/static/empty/", &[]).status, 404);
    let dotted = get(&server, "/static/%2e%2e/index.html", &[]);
    assert_eq!(dotted.status, 301); // Go's index redirect, never outside root
    let escaped = get(&server, "/static/%2e%2e/demo.html", &[]);
    assert_eq!(escaped.status, 200); // path.Clean keeps it inside the root
    assert_eq!(escaped.body, b"<!doctype html><title>demo</title>\n");
}

#[test]
fn static_local_name_refuses_os_path_syntax() {
    use super::routes::local_name;
    for name in [
        "..\\..\\secret.txt",
        "js\\app.js",
        "C:/Windows/win.ini",
        "C:",
        "\\\\server\\share\\x",
        "a\0b",
        "..",
        "../x",
        "./x",
    ] {
        assert!(local_name(name, false).is_none(), "{name:?}");
        assert!(local_name(name, true).is_none(), "{name:?}");
    }
    for device in [
        "CON",
        "nul",
        "con.txt",
        "COM1",
        "lpt9.log",
        "COM\u{b9}",
        "CONIN$",
        "aux .x",
    ] {
        assert!(local_name(device, true).is_none(), "{device:?}");
        assert!(
            local_name(&format!("js/{device}"), true).is_none(),
            "{device:?}"
        );
        assert!(local_name(device, false).is_some(), "{device:?}");
    }
    for name in ["", "js/app.js", "COM0", "COM10", "console.js", "a..b/c.d"] {
        assert!(local_name(name, true).is_some(), "{name:?}");
        assert!(local_name(name, false).is_some(), "{name:?}");
    }
}

#[test]
fn static_file_server_refuses_backslash_and_drive_targets() {
    let web = web();
    let server = server(&web.text(), false);
    // A real file one level above the web root.
    let parent = web.0.parent().unwrap();
    let secret_name = format!(
        "{}-secret.txt",
        web.0.file_name().unwrap().to_str().unwrap()
    );
    let secret = parent.join(&secret_name);
    std::fs::write(&secret, b"secret").unwrap();
    let mut targets = vec![
        format!("/static/..%5C{secret_name}"),
        format!("/static/..%5c..%5C{secret_name}"),
        format!("/static/..\\{secret_name}"),
        "/static/C:/Windows/win.ini".to_owned(),
        "/static/C%3A/Windows/win.ini".to_owned(),
        "/static/js%5Capp.js".to_owned(),
    ];
    if cfg!(windows) {
        let absolute = secret.to_str().unwrap().replace('\\', "/");
        targets.push(format!("/static/{absolute}"));
    }
    if cfg!(unix) {
        // Unix names may hold `\` and `:`; they are still refused.
        web.write("js\\app.js", b"backslash");
        web.write("c:d.js", b"colon");
        targets.push("/static/c:d.js".to_owned());
    }
    for target in &targets {
        let reply = get(&server, target, &[]);
        assert_eq!(
            (reply.status, reply.body.as_slice()),
            (404, b"404 page not found\n".as_slice()),
            "{target}"
        );
    }
    let _ = std::fs::remove_file(&secret);
    let normal = get(&server, "/static/js/app.js", &[]);
    assert_eq!(
        (normal.status, normal.body.as_slice()),
        (200, b"console.log(1);\n".as_slice())
    );
}

#[test]
fn redirect_location_escapes_like_go_url_string() {
    let web = web();
    let server = server(&web.text(), false);
    // Locations captured from Go 1.26 net/http ServeMux for the same targets.
    for (target, location) in [
        ("/./\\evil.com", "/%255Cevil.com"),
        ("/./%5Cevil.com", "/%255Cevil.com"),
        ("/a/../b%20c", "/b%2520c"),
        ("/x/../d%C3%A9mo", "/d%25C3%25A9mo"),
        ("/x/../a<b>", "/a%253Cb%253E"),
        ("/x/../\"q\"", "/%2522q%2522"),
        ("/api/tra%66fic", "/api/traffic/"),
        ("/api/traffic?x=<1>", "/api/traffic/?x=<1>"),
        ("/static/../demo", "/demo"),
        ("//demo", "/demo"),
    ] {
        let reply = get(&server, target, &[]);
        assert_eq!(
            (reply.status, reply.header("Location")),
            (307, Some(location)),
            "{target}"
        );
        assert!(!reply.header("Location").unwrap().contains('\\'));
    }
}

#[test]
fn redirects_preserve_query_and_escape_html() {
    let web = web();
    let server = server(&web.text(), false);
    let bare = get(&server, "/api/traffic?x=<1>", &[]);
    assert_eq!(bare.status, 307);
    assert_eq!(bare.header("Location"), Some("/api/traffic/?x=<1>"));
    assert_eq!(
        String::from_utf8(bare.body).unwrap(),
        "<a href=\"/api/traffic/?x=&lt;1&gt;\">Temporary Redirect</a>.\n\n"
    );
    let post = server.handle(
        &Request::new("POST", "/static", vec![], vec![], "1.2.3.4:5"),
        &Connection::default(),
    );
    assert_eq!(post.status, 307);
    assert!(post.body.is_empty() && post.header("Content-Type").is_none());
    // Segments are matched unescaped, as Go 1.22+ patterns are.
    assert_eq!(get(&server, "/de%6do", &[]).status, 200);
    assert_eq!(get(&server, "/api%2Fsession", &[]).status, 404);
}

#[test]
fn framing_is_content_length_only_and_bounded() {
    let parse = |raw: &[u8]| read_request(&mut &raw[..], "127.0.0.1:9", |_| 16);
    let ok =
        parse(b"POST /api/collect?label=a+b HTTP/1.1\r\nHost: x\r\nContent-Length: 4\r\n\r\nbody")
            .unwrap();
    assert_eq!(
        (ok.body.as_slice(), ok.query("label")),
        (b"body".as_slice(), "a b".to_owned())
    );
    let bare_lf = parse(b"GET / HTTP/1.0\n\n").unwrap();
    assert_eq!(bare_lf.minor, 0);
    for (raw, status) in [
        (
            &b"POST / HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n"[..],
            400,
        ),
        (b"GET / HTTP/1.1\r\n\r\n", 400),
        (b"GET / HTTP/2.0\r\nHost: x\r\n\r\n", 505),
        (
            b"GET / HTTP/1.1\r\nHost: x\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\n",
            400,
        ),
        (b"GET / HTTP/1.1\r\nHost: x\r\nA: b\r\n folded\r\n\r\n", 400),
        (b"GET /%zz HTTP/1.1\r\nHost: x\r\n\r\n", 400),
        (b"GET * HTTP/1.1\r\nHost: x\r\n\r\n", 400),
        (b"GET / HTTP/1.1\r\nHost: x\r\nBad Name: v\r\n\r\n", 400),
        (
            b"GET / HTTP/1.1\r\nHost: x\r\nContent-Length: 9\r\n\r\nshort",
            400,
        ),
    ] {
        assert_eq!(
            parse(raw).unwrap_err(),
            Refused(status),
            "{:?}",
            String::from_utf8_lossy(raw)
        );
    }
    let mut huge = b"GET / HTTP/1.1\r\nHost: x\r\nX: ".to_vec();
    huge.resize(http::MAX_HEAD + 10, b'a');
    assert_eq!(parse(&huge).unwrap_err(), Refused(431));
    // Only the route's limit plus one byte is read.
    let long = parse(b"POST / HTTP/1.1\r\nHost: x\r\nContent-Length: 40\r\n\r\n0123456789012345678901234567890123456789")
        .unwrap();
    assert_eq!(long.body.len(), 17);
    let target = Request::new("GET", "http://h.example/a%20b?x=1", vec![], vec![], "x");
    assert_eq!(target.path().as_deref(), Some("/a b"));
    assert_eq!(query_value("a=1;b&label=x&label=y", "label"), "x");
    assert_eq!(query_value("label=%zz&label=ok", "label"), "ok");
    assert_eq!(unescape("%41+", false).as_deref(), Some("A+"));
}

#[test]
fn encoding_matches_go_server_framing() {
    let mut headers = BTreeMap::new();
    headers.insert("X-B".to_owned(), vec!["2\r\nInjected: 1".to_owned()]);
    headers.insert("A".to_owned(), vec!["1".to_owned()]);
    let response = Response {
        status: 200,
        headers,
        body: b"hi".to_vec(),
    };
    let date = http::http_date(784_111_777);
    assert_eq!(date, "Sun, 06 Nov 1994 08:49:37 GMT");
    assert_eq!(
        String::from_utf8(encode(&response, false, &date)).unwrap(),
        format!(
            "HTTP/1.1 200 OK\r\nA: 1\r\nConnection: close\r\nContent-Length: 2\r\nDate: {date}\r\nX-B: 2  Injected: 1\r\n\r\nhi"
        )
    );
    let head = encode(&response, true, &date);
    assert!(
        head.ends_with(b"\r\n\r\n") && String::from_utf8_lossy(&head).contains("Content-Length: 2")
    );
    let none = Response {
        status: 204,
        headers: BTreeMap::new(),
        body: b"x".to_vec(),
    };
    let none = String::from_utf8(encode(&none, false, &date)).unwrap();
    assert!(!none.contains("Content-Length") && none.ends_with("\r\n\r\n"));
}

#[test]
fn cookie_value_follows_go_read_cookies() {
    let fields = |line: &str| vec![("Cookie".to_owned(), line.to_owned())];
    assert_eq!(cookie_value(&fields("a=1; hsid=\"q\""), "hsid"), "q");
    assert_eq!(
        cookie_value(&fields("hsid=bad\\v; hsid=good"), "hsid"),
        "good"
    );
    // Go trims the name but keeps the value's leading space.
    assert_eq!(cookie_value(&fields(" hsid = x "), "hsid"), " x");
    assert_eq!(cookie_value(&fields("Hsid=x"), "hsid"), "");
    assert_eq!(cookie_value(&[], "hsid"), "");
}

#[test]
fn construction_refuses_weak_keys_and_empty_roots() {
    let web = web();
    let mut short = config(&web.text(), false);
    short.master_key = Zeroizing::new(vec![1; 31]);
    assert_eq!(
        CoreServer::new(short, fixed_clock()).err(),
        Some(Refusal::MasterKey)
    );
    assert_eq!(
        CoreServer::new(config("", false), fixed_clock()).err(),
        Some(Refusal::WebDir)
    );
}

#[test]
fn history_budget_exhaustion_fails_closed() {
    let web = web();
    let mut config = config(&web.text(), false);
    config.limits.traffic = traffic::Limits::new(1, 16, 4096, 1 << 20).unwrap();
    let server = CoreServer::new(config, fixed_clock()).unwrap();
    assert_eq!(
        get(&server, "/healthz", &[("Cookie", "hsid=a")]).status,
        200
    );
    let refused = get(&server, "/healthz", &[("Cookie", "hsid=b")]);
    assert_eq!(refused.status, 503);
    assert_eq!(refused.body, b"history unavailable\n");
}

#[test]
fn clock_regression_is_absorbed_by_monotone_ticks() {
    let web = web();
    let now = Arc::new(Mutex::new(NOW));
    let shared = Arc::clone(&now);
    let clock: Clock = Arc::new(move || UNIX_EPOCH + Duration::from_secs(*shared.lock().unwrap()));
    let server = CoreServer::new(config(&web.text(), false), clock).unwrap();
    assert_eq!(
        get(&server, "/api/session", &[("Cookie", "hsid=c")]).status,
        200
    );
    *now.lock().unwrap() = NOW - 100;
    assert_eq!(
        get(&server, "/api/session", &[("Cookie", "hsid=c")]).status,
        200
    );
    *now.lock().unwrap() = NOW + 7200;
    // A sweep runs and expires the idle session.
    assert_eq!(get(&server, "/healthz", &[]).status, 200);
    let (state, _) = server.lock().unwrap();
    assert!(state.store.get("c").is_none());
}

async fn exchange(address: std::net::SocketAddr, bytes: &'static [u8]) -> String {
    tokio::task::spawn_blocking(move || {
        let mut socket = std::net::TcpStream::connect(address).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        socket.write_all(bytes).unwrap();
        let mut out = Vec::new();
        let _ = socket.read_to_end(&mut out);
        String::from_utf8_lossy(&out).into_owned()
    })
    .await
    .unwrap()
}

fn options(tls: Option<Arc<rustls::ServerConfig>>, cap: usize) -> Options {
    Options {
        tls,
        budget: Duration::from_secs(5),
        cap: ConnectionCap::new(cap).unwrap(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bind_refuses_non_loopback_addresses() {
    for address in ["0.0.0.0:0", "192.0.2.1:0", "[::]:0"] {
        let error = bind(address.parse().unwrap()).await.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput, "{address}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loopback_socket_serves_one_request_with_wire_order() {
    let web = web();
    let server =
        Arc::new(CoreServer::new(config(&web.text(), false), Arc::new(SystemTime::now)).unwrap());
    let listener = bind("127.0.0.1:0".parse().unwrap()).await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(serve(
        Arc::clone(&server),
        listener,
        options(None, 4),
        async {
            let _ = stopped.await;
        },
    ));
    let health = exchange(address, b"GET /healthz HTTP/1.1\r\nHost: x\r\n\r\n").await;
    assert!(health.starts_with("HTTP/1.1 200 OK\r\n"), "{health}");
    for header in [
        "Connection: close\r\n",
        "Content-Type: application/json\r\n",
        "Referrer-Policy: strict-origin-when-cross-origin\r\n",
        "Date: ",
    ] {
        assert!(health.contains(header), "{header}: {health}");
    }
    assert!(health.ends_with("\"version\":\"dev\"}\n"), "{health}");
    let refused = exchange(
        address,
        b"POST / HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
    )
    .await;
    assert!(
        refused.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "{refused}"
    );
    let collected = exchange(
        address,
        b"POST /api/collect HTTP/1.1\r\nUser-Agent: t\r\nHost: x\r\nCookie: hsid=w\r\nContent-Length: 2\r\n\r\n{}",
    )
    .await;
    assert!(collected.starts_with("HTTP/1.1 200 OK\r\n"), "{collected}");
    let head = exchange(
        address,
        b"HEAD / HTTP/1.1\r\nHost: x\r\nCookie: hsid=w\r\n\r\n",
    )
    .await;
    assert!(
        head.contains("Content-Length: 36\r\n") && head.ends_with("\r\n\r\n"),
        "{head}"
    );
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    let (state, _) = server.lock().unwrap();
    let report = state.store.get("w").unwrap();
    // The plaintext peek supplied the wire header order (order-reliable),
    // which the header-order consumer reads; the ClientHello is absent.
    assert!(
        report
            .network
            .signals
            .iter()
            .flatten()
            .any(|s| s.id == "l5.tls.not_observed")
    );
    assert_eq!(state.traffic.session("w").len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn connection_cap_holds_the_next_peer_in_the_backlog() {
    let web = web();
    let server =
        Arc::new(CoreServer::new(config(&web.text(), false), Arc::new(SystemTime::now)).unwrap());
    let listener = bind("127.0.0.1:0".parse().unwrap()).await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(serve(
        Arc::clone(&server),
        listener,
        options(None, 1),
        std::future::pending(),
    ));
    let idle = std::net::TcpStream::connect(address).unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let waiting = tokio::task::spawn_blocking(move || {
        let mut socket = std::net::TcpStream::connect(address).unwrap();
        socket
            .write_all(b"GET /healthz HTTP/1.1\r\nHost: x\r\n\r\n")
            .unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(300)))
            .unwrap();
        let mut byte = [0u8; 1];
        let early = socket.read(&mut byte);
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut out = byte.to_vec();
        socket.read_to_end(&mut out).unwrap();
        (early.is_err(), String::from_utf8_lossy(&out).into_owned())
    });
    tokio::time::sleep(Duration::from_millis(500)).await;
    drop(idle);
    let (held, reply) = waiting.await.unwrap();
    assert!(held, "second peer was served while the cap was full");
    assert!(reply.contains("200 OK"), "{reply}");
    task.abort();
}

#[test]
fn plaintext_budget_is_one_deadline_for_a_trickling_peer() {
    let web = web();
    let server = CoreServer::new(config(&web.text(), false), Arc::new(SystemTime::now)).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let trickle = {
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut socket = std::net::TcpStream::connect(address).unwrap();
            let _ = socket.write_all(b"GET /healthz HTTP/1.1\r\nHost: x\r\nX-Pad: ");
            // One byte well inside every per-read window, for far longer
            // than the whole budget.
            for _ in 0..200 {
                if stop.load(std::sync::atomic::Ordering::Acquire)
                    || socket.write_all(b"a").is_err()
                {
                    break;
                }
                std::thread::sleep(Duration::from_millis(40));
            }
        })
    };
    let (stream, _) = listener.accept().unwrap();
    let budget = Duration::from_millis(1200);
    let options = Options {
        tls: None,
        budget,
        cap: ConnectionCap::new(1).unwrap(),
    };
    let started = std::time::Instant::now();
    let _ = super::serve::serve_connection(&server, stream, &options);
    let elapsed = started.elapsed();
    stop.store(true, std::sync::atomic::Ordering::Release);
    trickle.join().unwrap();
    // Budget, plus the bounded post-response drain, plus scheduling slack;
    // a per-read timeout would have kept reading for the full 8 s trickle.
    assert!(elapsed >= budget, "{elapsed:?}");
    assert!(elapsed < Duration::from_secs(4), "{elapsed:?}");
}

fn tls_configurations() -> (Arc<rustls::ServerConfig>, Arc<rustls::ClientConfig>) {
    let certified = rcgen::generate_simple_self_signed(vec!["core.test".into()]).unwrap();
    let certificate = certified.cert.der().clone();
    let key = rustls::pki_types::PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    let mut server = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![certificate.clone()], key.into())
        .unwrap();
    server.alpn_protocols = vec![b"http/1.1".to_vec()];
    let mut roots = rustls::RootCertStore::empty();
    roots.add(certificate).unwrap();
    let mut client = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    client.alpn_protocols = vec![b"http/1.1".to_vec()];
    (Arc::new(server), Arc::new(client))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tls_connection_owns_its_client_hello() {
    let web = web();
    let server =
        Arc::new(CoreServer::new(config(&web.text(), false), Arc::new(SystemTime::now)).unwrap());
    let listener = bind("127.0.0.1:0".parse().unwrap()).await.unwrap();
    let address = listener.local_addr().unwrap();
    let (server_tls, client_tls) = tls_configurations();
    let task = tokio::spawn(serve(
        Arc::clone(&server),
        listener,
        options(Some(server_tls), 4),
        std::future::pending(),
    ));
    let reply = tokio::task::spawn_blocking(move || {
        let socket = std::net::TcpStream::connect(address).unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let connection = rustls::ClientConnection::new(
            client_tls,
            rustls::pki_types::ServerName::try_from("core.test").unwrap(),
        )
        .unwrap();
        let mut stream = rustls::StreamOwned::new(connection, socket);
        stream
            .write_all(b"POST /api/collect HTTP/1.1\r\nHost: core.test\r\nCookie: hsid=t1\r\nContent-Length: 2\r\n\r\n{}")
            .unwrap();
        let mut out = Vec::new();
        let _ = stream.read_to_end(&mut out);
        String::from_utf8_lossy(&out).into_owned()
    })
    .await
    .unwrap();
    assert!(reply.starts_with("HTTP/1.1 200 OK\r\n"), "{reply}");
    task.abort();
    let (state, _) = server.lock().unwrap();
    let records = state.traffic.session("t1");
    // The HTTP record was backfilled from this connection's own ClientHello.
    assert!(!records[0].ja4.is_empty(), "{records:?}");
    let report = state.store.get("t1").unwrap();
    assert!(
        !report
            .network
            .signals
            .iter()
            .flatten()
            .any(|s| s.id == "l5.tls.not_observed")
    );
}
