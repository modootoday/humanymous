//! The mounted Pass flow served through the Core HTTP path: the Go-captured
//! Pass transcripts (`pass-flow-go-golden.json`, go1.26.3 at oracle 981d25c)
//! replayed as wire requests through `read_request`, `CoreServer::handle`
//! and `encode`, the page, the accessible default and the fail-closed paths.
use super::super::http::{Request, encode, read_request};
use super::super::serve::{bind, serve};
use super::super::tests::{TempDir, limits, options};
use super::super::{Clock, Config, Connection, CoreServer, PassRequest, PassRoutes};
use super::*;
use crate::gateway::pass_store::{PassNonce, Refusal as StoreRefusal};
use crate::gateway::token::TokenKey;
use crate::network::proxy::Cidr;
use humanymous_core::challenge::{pass, proof_of_work};
use humanymous_core::signals::observation::Signal;
use humanymous_core::signals::report::SessionReport;
use humanymous_core::watermark::issuance::Fallback;
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;

const GOLDEN: &str = include_str!("../../../../tests/fixtures/gateway/pass-flow-go-golden.json");
const PAGE: &[u8] = b"<!doctype html><title>Pass</title><main id=\"pass\"></main>\n";

fn golden() -> Value {
    serde_json::from_str::<Value>(GOLDEN).unwrap()["outputs"].clone()
}

fn constants() -> Value {
    golden()["constants"].clone()
}

fn master_key() -> Vec<u8> {
    constants()["masterKey"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec()
}

fn step_up_key() -> TokenKey {
    TokenKey::new(
        constants()["stepUpKey"]
            .as_str()
            .unwrap()
            .as_bytes()
            .to_vec(),
    )
    .unwrap()
}

/// Wall clock the test moves; nanoseconds since the Unix epoch.
#[derive(Clone)]
struct TestClock(Arc<AtomicU64>);

impl TestClock {
    fn at(unix: i64) -> Self {
        let clock = Self(Arc::new(AtomicU64::new(0)));
        clock.set(unix, 0);
        clock
    }
    fn set(&self, unix: i64, millis: u64) {
        let nanos = u64::try_from(unix).unwrap() * 1_000_000_000 + millis * 1_000_000;
        self.0.store(nanos, Ordering::SeqCst);
    }
    fn clock(&self) -> Clock {
        let nanos = Arc::clone(&self.0);
        Arc::new(move || UNIX_EPOCH + Duration::from_nanos(nanos.load(Ordering::SeqCst)))
    }
}

fn web() -> TempDir {
    let dir = TempDir::new("pass-web");
    dir.write("index.html", b"<!doctype html><title>index</title>\n");
    dir.write("pass.html", PAGE);
    dir
}

fn config(web_dir: &str) -> Config {
    Config {
        web_dir: web_dir.to_owned(),
        master_key: Zeroizing::new(master_key()),
        rit_enabled: false,
        ops_token: "optok".to_owned(),
        trusted_proxies: vec![Cidr::parse("10.0.0.0/8").unwrap()],
        version: "dev".to_owned(),
        watermark_fallback: Fallback::OpaqueTrailer,
        limits: limits(),
    }
}

fn queued(nonces: Vec<String>) -> impl FnMut() -> Result<PassNonce, StoreRefusal> + Send + 'static {
    let mut queue: VecDeque<String> = nonces.into();
    move || {
        let hex = queue
            .pop_front()
            .expect("more /new calls than recorded nonces");
        let bytes: [u8; 18] = hex::decode(hex).unwrap().try_into().unwrap();
        Ok(PassNonce::from_entropy(bytes))
    }
}

/// A served Pass mount exactly as `with_pass_flow` builds it, with the
/// recorded challenge nonces injected.
fn mounted(web_dir: &str, clock: &TestClock, step_up: bool, nonces: Vec<String>) -> CoreServer {
    let mount = PassMount::new(pass_flow::Config {
        master_key: master_key(),
        web_dir: std::path::PathBuf::from(web_dir),
        step_up: step_up.then(step_up_key),
        motor_evidence: MotorEvidence::Off,
    })
    .unwrap()
    .with_nonce_source(queued(nonces));
    CoreServer::new(config(web_dir), clock.clock())
        .unwrap()
        .with_pass_routes(Arc::new(mount))
}

/// One response as read back from the wire.
struct Wire {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Wire {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }
}

fn parse(bytes: &[u8]) -> Wire {
    let end = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = std::str::from_utf8(&bytes[..end]).unwrap();
    let mut lines = head.split("\r\n");
    let status = lines.next().unwrap().split(' ').nth(1).unwrap();
    Wire {
        status: status.parse().unwrap(),
        headers: lines
            .map(|line| {
                let (name, value) = line.split_once(": ").unwrap();
                (name.to_owned(), value.to_owned())
            })
            .collect(),
        body: bytes[end + 4..].to_vec(),
    }
}

/// Frame one request on the wire, read it as the transport does, serve it
/// and encode the reply.
fn exchange(
    server: &CoreServer,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    remote: &str,
) -> Wire {
    let mut raw = format!("{method} {path} HTTP/1.1\r\nHost: example.com\r\n");
    for (name, value) in headers {
        raw.push_str(&format!("{name}: {value}\r\n"));
    }
    if !body.is_empty() {
        raw.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    raw.push_str("\r\n");
    let mut raw = raw.into_bytes();
    raw.extend_from_slice(body);
    let request: Request = read_request(&mut raw.as_slice(), remote, |request| {
        server.body_limit(request)
    })
    .unwrap();
    let response = server.handle(&request, &Connection::plaintext(None));
    parse(&encode(&response, false, "Thu, 19 Nov 2026 00:00:00 GMT"))
}

fn stored(server: &CoreServer, session: &str) -> Option<SessionReport> {
    let (state, _) = server.lock().unwrap();
    state.store.get(session)
}

/// Steps whose Go answer came from `realEventOK` or the behavior tells,
/// which the served `MotorEvidence::Off` never applies, with the accessible
/// answer the Core serves instead (the step-up receipt removed).
const MOTOR_STEPS: [(&str, usize, &str); 7] = [
    // Go: "untrusted events"; the same solve clears.
    (
        "duplicate_trace_and_real_event",
        6,
        r#"{"ok":true,"riskScore":12.5,"verdict":"CHALLENGE"}"#,
    ),
    // Go: "insufficient interaction"; the session already cleared.
    (
        "duplicate_trace_and_real_event",
        7,
        r#"{"alreadySolved":true,"ok":true}"#,
    ),
    // Go: "uniform key timing".
    (
        "duplicate_trace_and_real_event",
        10,
        r#"{"ok":true,"riskScore":12.5,"verdict":"CHALLENGE"}"#,
    ),
    // Go: cleared at riskScore 55.4 with the machine-speed tells folded.
    (
        "duplicate_trace_and_real_event",
        11,
        r#"{"alreadySolved":true,"ok":true}"#,
    ),
    (
        "duplicate_trace_and_real_event",
        12,
        r#"{"botAttempts":0,"bypassRate":0,"humanAttempts":4,"humanPassRate":0.75,"perDifficulty":{"d0":0.75},"perStrategy":{},"raw":{"human|fail|d0":1,"human|pass|d0":3},"unknownPassRate":0}"#,
    ),
    // Go: riskScore 23 and 25.1 with the behavior tells folded.
    (
        "behavior_tells_fused_soft",
        2,
        r#"{"ok":true,"riskScore":0,"verdict":"CHALLENGE"}"#,
    ),
    (
        "behavior_tells_fused_soft",
        5,
        r#"{"ok":true,"riskScore":12.5,"verdict":"CHALLENGE"}"#,
    ),
];

/// Final stored Pass signals and risk of the motor scenarios' sessions under
/// `MotorEvidence::Off`: no `l4.*` tell is ever folded.
const MOTOR_FINALS: [(&str, &[&str], f64); 5] = [
    ("dup-1", &["l7.pass.solved"], 0.0),
    ("dup-2", &["l7.pass.velocity", "l7.pass.solved"], 12.5),
    ("dup-3", &["l7.pass.velocity", "l7.pass.solved"], 12.5),
    ("beh-1", &["l7.pass.solved"], 0.0),
    ("beh-2", &["l7.pass.velocity", "l7.pass.solved"], 12.5),
];

fn signals(report: &SessionReport) -> Vec<Signal> {
    report.network.signals.clone().unwrap_or_default()
}

/// B1 differential through the served path: every recorded Go Pass request
/// is framed on the wire and answered by the Core server with the mounted
/// flow (`MotorEvidence::Off`). The status, the content type, Set-Cookie
/// presence, the security headers and the exact body bytes equal Go for
/// every step that does not depend on motor evidence; the fingerprint comes
/// from the peer address through the Core's trust-gated client subnet, and
/// the final stored signals and frozen-engine scores equal Go. Steps and
/// sessions that Go answered from `realEventOK` or the behavior tells are
/// pinned to the accessible answer, and the signals they share with Go are
/// Go's.
#[test]
fn golden_pass_transcripts_replay_through_the_core_http_path() {
    let golden = golden();
    let web = web();
    let scenarios = golden["scenarios"].as_array().unwrap();
    assert_eq!(scenarios.len(), 8);
    let mut replayed = 0;
    let mut diverged = 0;
    let mut motor_sessions = 0;
    for scenario in scenarios {
        let name = scenario["name"].as_str().unwrap();
        let nonces: Vec<String> = serde_json::from_value(scenario["nonces"].clone()).unwrap();
        let steps = scenario["steps"].as_array().unwrap();
        let clock = TestClock::at(steps[0]["nowUnix"].as_i64().unwrap());
        let server = mounted(
            &web.text(),
            &clock,
            scenario["stepUp"].as_bool().unwrap(),
            nonces,
        );
        for (index, step) in steps.iter().enumerate() {
            clock.set(step["nowUnix"].as_i64().unwrap(), index as u64);
            let context = format!("{name} step {index}: {}", step["step"]);
            let remote = step["step"]["remote"].as_str().unwrap_or("192.0.2.1:1234");
            let cookie = step["cookie"].as_str().unwrap();
            let mut headers = Vec::new();
            if !cookie.is_empty() {
                headers.push(("Cookie", cookie));
            }
            if let Some(label) = step["step"]["redteam"].as_str() {
                headers.push(("X-HM-Redteam", label));
            }
            let wire = exchange(
                &server,
                step["method"].as_str().unwrap(),
                step["path"].as_str().unwrap(),
                &headers,
                step["body"].as_str().unwrap().as_bytes(),
                remote,
            );
            // The Core's passFP for this peer is the one Go recorded.
            assert_eq!(
                crate::gateway::pass_scoring::fingerprint(
                    &crate::gateway::request::ja4_stable(None),
                    &crate::network::http::client_subnet(remote, &[], &[]),
                ),
                step["fingerprint"].as_str().unwrap(),
                "{context}"
            );
            assert_eq!(
                u64::from(wire.status),
                step["status"].as_u64().unwrap(),
                "{context}"
            );
            assert_eq!(
                wire.header("Content-Type"),
                step["contentType"].as_str(),
                "{context}"
            );
            assert_eq!(
                wire.header("Set-Cookie").is_some(),
                step["setCookie"].as_bool().unwrap(),
                "{context}"
            );
            for (header, value) in crate::gateway::core_server::routes::SECURITY_HEADERS {
                assert_eq!(wire.header(header), Some(value), "{context}");
            }
            let go = step["response"].as_str().unwrap();
            match MOTOR_STEPS
                .iter()
                .find(|(scenario, at, _)| *scenario == name && *at == index)
            {
                Some((_, _, accessible)) => {
                    assert_ne!(wire.body, go.as_bytes(), "{context}");
                    let mut ours = wire.json();
                    if ours["ok"] == true && ours.get("riskScore").is_some()
                        || ours.get("alreadySolved").is_some()
                    {
                        assert!(ours["stepUpReceipt"].is_string(), "{context}: {ours}");
                    }
                    if let Some(map) = ours.as_object_mut() {
                        map.remove("stepUpReceipt");
                    }
                    let accessible: Value = serde_json::from_str(accessible).unwrap();
                    assert_eq!(ours, accessible, "{context}");
                    diverged += 1;
                }
                None => assert_eq!(String::from_utf8_lossy(&wire.body), go, "{context}"),
            }
            replayed += 1;
        }
        for (session, go) in scenario["finals"].as_object().unwrap() {
            let context = format!("{name} {session}");
            let report = stored(&server, session);
            assert_eq!(
                report.is_some(),
                go["stored"].as_bool().unwrap(),
                "{context}"
            );
            let report = report.unwrap_or_default();
            let expected: Vec<Signal> = serde_json::from_value::<Option<_>>(go["signals"].clone())
                .unwrap()
                .unwrap_or_default();
            if let Some((_, ids, risk)) = MOTOR_FINALS.iter().find(|(sid, _, _)| sid == session) {
                let ours = signals(&report);
                let found: Vec<&str> = ours.iter().map(|signal| signal.id.as_str()).collect();
                assert_eq!(found, *ids, "{context}");
                for signal in &ours {
                    if let Some(theirs) = expected.iter().find(|go| go.id == signal.id) {
                        assert_eq!(signal, theirs, "{context}");
                    }
                }
                assert_eq!(report.scoring.risk_score, *risk, "{context}");
                assert_eq!(report.scoring.verdict, "CHALLENGE", "{context}");
                motor_sessions += 1;
                continue;
            }
            assert_eq!(signals(&report), expected, "{context}");
            assert_eq!(
                report.scoring.risk_score,
                go["scoring"]["riskScore"].as_f64().unwrap(),
                "{context}"
            );
            assert_eq!(
                report.scoring.verdict,
                go["scoring"]["verdict"].as_str().unwrap(),
                "{context}"
            );
            assert_eq!(
                report.scoring.hard_rule_fired,
                go["scoring"]["hardRuleFired"].as_str().unwrap(),
                "{context}"
            );
        }
    }
    assert_eq!(diverged, MOTOR_STEPS.len());
    assert_eq!(motor_sessions, MOTOR_FINALS.len());
    assert!(replayed >= 70, "replayed {replayed} Go requests");
}

/// The Core golden's `pass-mount` case (GET /api/pass/kpi through Go
/// `app.routes()`) is answered byte for byte once the flow is mounted with
/// `with_pass_flow`; without a mount every Pass path is 404.
#[test]
fn with_pass_flow_serves_the_go_mounted_kpi_and_unmounted_is_404() {
    let web = web();
    let clock = TestClock::at(1_795_087_652);
    let core: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/gateway/core-server-go-golden.json"
    ))
    .unwrap();
    let case = core["capture"]["http"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "pass-mount")
        .unwrap()
        .clone();
    let unmounted = CoreServer::new(config(&web.text()), clock.clock()).unwrap();
    for path in pass_flow::ROUTES {
        assert_eq!(
            exchange(&unmounted, "GET", path, &[], b"", "192.0.2.1:1234").status,
            404,
            "{path}"
        );
    }
    let server = CoreServer::new(config(&web.text()), clock.clock())
        .unwrap()
        .with_pass_flow(None)
        .unwrap();
    let wire = exchange(
        &server,
        "GET",
        case["target"].as_str().unwrap(),
        &[],
        b"",
        "192.0.2.1:1234",
    );
    assert_eq!(u64::from(wire.status), case["status"].as_u64().unwrap());
    let theirs: BTreeMap<String, Vec<String>> =
        serde_json::from_value(case["respHeaders"].clone()).unwrap();
    for (name, values) in theirs {
        assert_eq!(wire.header(&name), Some(values[0].as_str()), "{name}");
    }
    assert_eq!(wire.body, case["respBody"].as_str().unwrap().as_bytes());
}

/// Go `handlePassPage`: the page issues the session cookie only when absent,
/// ensures the session in the Core store and serves the web root's
/// `pass.html`; a missing page is 404 with the issued cookie kept.
#[test]
fn page_serves_pass_html_and_ensures_the_core_session() {
    let web = web();
    let clock = TestClock::at(1_795_087_652);
    let server = CoreServer::new(config(&web.text()), clock.clock())
        .unwrap()
        .with_pass_flow(None)
        .unwrap();
    let fresh = exchange(&server, "GET", "/pass", &[], b"", "192.0.2.1:1234");
    assert_eq!(fresh.status, 200);
    assert_eq!(
        fresh.header("Content-Type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(fresh.body, PAGE);
    let cookie = fresh.header("Set-Cookie").unwrap();
    let sid = cookie
        .strip_prefix("hsid=")
        .unwrap()
        .split(';')
        .next()
        .unwrap();
    assert!(stored(&server, sid).is_some());
    let again = exchange(
        &server,
        "GET",
        "/pass",
        &[("Cookie", &format!("hsid={sid}"))],
        b"",
        "192.0.2.1:1234",
    );
    assert_eq!(again.status, 200);
    assert!(again.header("Set-Cookie").is_none());
    assert_eq!(again.body, PAGE);

    let bare = TempDir::new("pass-bare");
    bare.write("index.html", b"x");
    let missing = CoreServer::new(config(&bare.text()), clock.clock())
        .unwrap()
        .with_pass_flow(None)
        .unwrap();
    let absent = exchange(&missing, "GET", "/pass", &[], b"", "192.0.2.1:1234");
    assert_eq!(absent.status, 404);
    assert!(absent.header("Set-Cookie").is_some());
}

/// Accessibility hard rule: the served mount refuses `GoParity`; fast,
/// perfectly regular, untrusted-flagged keyboard input (a switch or other
/// assistive device) clears Pass through the Core with no behavior tell.
#[test]
fn served_mount_never_gates_pass_on_motor_evidence() {
    let refused = PassMount::new(pass_flow::Config {
        master_key: master_key(),
        web_dir: std::path::PathBuf::from("web"),
        step_up: None,
        motor_evidence: MotorEvidence::GoParity,
    });
    assert_eq!(refused.err(), Some(MountRefusal::MotorEvidence));

    let web = web();
    let clock = TestClock::at(1_795_087_652);
    let server = CoreServer::new(config(&web.text()), clock.clock())
        .unwrap()
        .with_pass_flow(Some(step_up_key()))
        .unwrap();
    let cookie = [("Cookie", "hsid=assistive-1")];
    let peer = "198.51.100.9:7000";
    let issued = exchange(&server, "GET", "/api/pass/new", &cookie, b"", peer);
    assert_eq!(issued.status, 200);
    let issued = issued.json();
    // The mount uses the Core master key, as Go's handlers used a.masterKey.
    let bucket = issued["bucket"].as_u64().unwrap();
    let challenge = pass::generate(&master_key(), "assistive-1", bucket, 1, 0);
    assert_eq!(
        issued["challenge"],
        serde_json::to_value(&challenge).unwrap()
    );
    let pow = &issued["preflight"]["pow"];
    let paid = proof_of_work::solve(
        pow["seed"].as_str().unwrap(),
        i32::try_from(pow["difficulty"].as_i64().unwrap()).unwrap(),
        1 << 26,
    )
    .unwrap();
    let paid = json!({
        "bucket": pow["bucket"],
        "nonce": paid,
        "challengeNonce": issued["challengeNonce"],
    })
    .to_string();
    let paid = exchange(
        &server,
        "POST",
        "/api/pass/pow",
        &cookie,
        paid.as_bytes(),
        peer,
    );
    assert_eq!(paid.json(), json!({"ok": true}));
    let n = issued["challenge"]["n"].as_i64().unwrap();
    let center = issued["challenge"]["center"].as_i64().unwrap();
    let offsets: Vec<i64> = issued["challenge"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (center - row["keyIndex"].as_i64().unwrap()).rem_euclid(n))
        .collect();
    let proof = json!({
        "challengeNonce": issued["challengeNonce"],
        "offsets": offsets,
        "trusted": false,
        "keys": 3,
        "keyDurs": [5, 5, 5, 5],
    })
    .to_string();
    let solved = exchange(
        &server,
        "POST",
        "/api/pass/solve",
        &cookie,
        proof.as_bytes(),
        peer,
    );
    let solved = solved.json();
    assert_eq!(solved["ok"], true, "{solved}");
    assert!(solved["stepUpReceipt"].is_string(), "{solved}");
    let ids: Vec<String> = stored(&server, "assistive-1")
        .unwrap()
        .network
        .signals
        .iter()
        .flatten()
        .map(|signal| signal.id.clone())
        .collect();
    assert_eq!(ids, ["l7.pass.solved"]);
}

struct PanicPass;
impl PassRoutes for PanicPass {
    fn handle(&self, _: &PassRequest<'_>, _: &mut dyn Backend) -> resource::Response {
        panic!("pass handler failure");
    }
}

/// Fail closed: a limiter refusal (an over-long trusted-proxy client
/// address in the fingerprint), an unavailable Core history and a poisoned
/// Pass lock each answer 503 instead of an unmetered Pass; a panicking Pass
/// handler cannot poison the Core state, so the other routes keep serving.
#[test]
fn pass_failures_fail_closed_and_leave_the_core_serving() {
    let web = web();
    let clock = TestClock::at(1_795_087_652);
    let server = CoreServer::new(config(&web.text()), clock.clock())
        .unwrap()
        .with_pass_flow(None)
        .unwrap();
    // No three dots: the whole forwarded value is the subnet.
    let long = "f".repeat(600);
    let refused = exchange(
        &server,
        "GET",
        "/api/pass/new",
        &[("Cookie", "hsid=lim-1"), ("X-Forwarded-For", &long)],
        b"",
        "10.0.0.2:4000",
    );
    assert_eq!(refused.status, 503);
    assert_eq!(refused.body, b"pass unavailable\n");

    // A poisoned Pass lock refuses Pass only.
    let mount = Arc::new(
        PassMount::new(pass_flow::Config {
            master_key: master_key(),
            web_dir: web.0.clone(),
            step_up: None,
            motor_evidence: MotorEvidence::Off,
        })
        .unwrap(),
    );
    let poison = Arc::clone(&mount);
    let _ = std::thread::spawn(move || {
        let _guard = poison.flow.lock().unwrap();
        panic!("poison the pass lock");
    })
    .join();
    let poisoned = CoreServer::new(config(&web.text()), clock.clock())
        .unwrap()
        .with_pass_routes(mount);
    let answer = exchange(
        &poisoned,
        "GET",
        "/api/pass/kpi",
        &[],
        b"",
        "192.0.2.1:1234",
    );
    assert_eq!(answer.status, 503);
    assert_eq!(answer.body, b"pass unavailable\n");
    let session = exchange(&poisoned, "GET", "/api/session", &[], b"", "192.0.2.1:1234");
    assert_eq!(session.status, 200);

    // A panicking handler leaves the Core state usable.
    let panicking = CoreServer::new(config(&web.text()), clock.clock())
        .unwrap()
        .with_pass_routes(Arc::new(PanicPass));
    assert_eq!(
        exchange(
            &panicking,
            "GET",
            "/api/pass/kpi",
            &[],
            b"",
            "192.0.2.1:1234"
        )
        .status,
        0
    );
    assert!(panicking.lock().is_some());
    let session = exchange(
        &panicking,
        "GET",
        "/api/session",
        &[],
        b"",
        "192.0.2.1:1234",
    );
    assert_eq!(session.status, 200);

    // An unavailable Core history refuses Pass before the flow runs.
    let broken = CoreServer::new(config(&web.text()), clock.clock())
        .unwrap()
        .with_pass_flow(None)
        .unwrap();
    let _ = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let _guard = broken.state.lock().unwrap();
                panic!("poison the core state");
            })
            .join()
    });
    let history = exchange(&broken, "GET", "/api/pass/kpi", &[], b"", "192.0.2.1:1234");
    assert_eq!(history.status, 503);
    assert_eq!(history.body, b"history unavailable\n");
}

/// Over an actual loopback socket: the Core accept loop serves the mounted
/// Pass page and a fresh instance, with the connection's own evidence.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loopback_socket_serves_the_mounted_pass_flow() {
    let web = web();
    let server = Arc::new(
        CoreServer::new(config(&web.text()), Arc::new(SystemTime::now))
            .unwrap()
            .with_pass_flow(None)
            .unwrap(),
    );
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
    let send = |bytes: &'static [u8]| {
        tokio::task::spawn_blocking(move || {
            let mut socket = std::net::TcpStream::connect(address).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            socket.write_all(bytes).unwrap();
            let mut out = Vec::new();
            let _ = socket.read_to_end(&mut out);
            out
        })
    };
    let page = parse(
        &send(b"GET /pass HTTP/1.1\r\nHost: x\r\nCookie: hsid=sock-1\r\n\r\n")
            .await
            .unwrap(),
    );
    assert_eq!(page.status, 200);
    assert_eq!(page.body, PAGE);
    let issued = parse(
        &send(b"GET /api/pass/new HTTP/1.1\r\nHost: x\r\nCookie: hsid=sock-1\r\n\r\n")
            .await
            .unwrap(),
    );
    assert_eq!(issued.status, 200);
    assert_eq!(issued.header("Connection"), Some("close"));
    let body = issued.json();
    assert_eq!(body["ok"], true);
    assert_eq!(body["triesLeft"], 4);
    assert_eq!(body["preflight"]["attestRequired"], false);
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert!(stored(&server, "sock-1").is_some());
}
