//! Bounded fixture controls and real TCP ingress/origin counters, not browser e2e.
mod browser;
use crate::{
    application::{Application, Decision},
    configuration::Configuration,
    http::Request,
    session::{Sessions, canonical},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use humanymous_audit::record::Record;
use humanymous_authority::decision::DurableAudit;
use humanymous_integrity::request::{compute_token, seed};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

fn configuration(origin: std::net::SocketAddr) -> Configuration {
    serde_json::from_value(json!({"listen":"127.0.0.1:0","origin":origin.to_string(),
        "scope":{"installation":"install","tenant":"tenant","origin":"origin","revision":"r1","authority":"owned.test"},
        "routes":[{"method":"POST","path":"/effect","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}}],
        "audit":{"directory":"unused","node_id":"runtime-test","keys":{"directory":"unused-keys","passphrase":"unit-fixture"},"allow_new":true,"trusted_head":null},
        "owner":if cfg!(windows){json!({"kind":"windows","sid":"S-1-5-21-1","pid":1})}else{json!({"kind":"unix","uid":0})},
        "session_capacity":16,"session_ttl_ms":60_000,"timeout_ms":1000,"rate_per_second":1000})).unwrap()
}
fn config() -> Configuration {
    configuration("127.0.0.1:32123".parse().unwrap())
}
#[test]
fn required_external_witness_refuses_writer_only_before_opening_state() {
    let mut configuration = config();
    configuration.audit.require_external_witness = true;
    assert!(matches!(
        crate::startup::open_audit(&mut configuration),
        Err("required external witness unavailable")
    ));
    // Missing injection does not consume the private unlock credential.
    assert_eq!(configuration.audit.keys.passphrase.as_str(), "unit-fixture");
}
#[derive(Clone, Default)]
struct Audit {
    records: Arc<Mutex<Vec<Record>>>,
    fail: Arc<AtomicBool>,
}
impl DurableAudit for Audit {
    fn append(&mut self, record: Record) -> std::io::Result<()> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(std::io::Error::other("injected disk failure"));
        }
        self.records.lock().unwrap().push(record);
        Ok(())
    }
}
fn request(method: &str, target: &str, body: Vec<u8>) -> Request {
    Request {
        method: method.into(),
        target: target.into(),
        headers: vec![
            ("Host".into(), "owned.test".into()),
            ("Content-Length".into(), body.len().to_string()),
            ("Content-Type".into(), "application/json".into()),
            ("sec-ch-ua".into(), "Chromium".into()),
            (
                "User-Agent".into(),
                "Mozilla/5.0 Chrome/126.0 Safari/537.36".into(),
            ),
            ("Accept".into(), "*/*".into()),
            ("Sec-Fetch-Site".into(), "same-origin".into()),
            ("Accept-Encoding".into(), "gzip, deflate, br, zstd".into()),
        ],
        body,
    }
}
fn sign(config: &Configuration, req: &mut Request, id: &str, key: &str, n: u64, now: u64) {
    let deadline = now + 900;
    req.headers.retain(|(name, _)| {
        !name.starts_with("x-humanymous-") && !name.eq_ignore_ascii_case("authorization")
    });
    req.headers
        .push(("Authorization".into(), format!("Bearer {id}")));
    req.headers
        .push(("x-humanymous-counter".into(), n.to_string()));
    req.headers
        .push(("x-humanymous-bucket".into(), (now / 10_000).to_string()));
    req.headers
        .push(("x-humanymous-deadline".into(), deadline.to_string()));
    let text = canonical(config, req, id, deadline);
    let key = URL_SAFE_NO_PAD.decode(key).unwrap();
    let token = compute_token(&seed(&key, n - 1), &text, n, now / 10_000);
    req.headers.push(("x-humanymous-token".into(), token));
}
fn issue(app: &mut Application, now: u64) -> Value {
    match app
        .decide(&request("POST", "/_humanymous/session", vec![]), || Ok(now))
        .unwrap()
    {
        Decision::Local(200, bytes) => serde_json::from_slice(&bytes).unwrap(),
        _ => panic!("issue"),
    }
}
fn client_report(id: &str) -> Vec<u8> {
    // Client assertion with zero motor/typing richness. This is a fixture,
    // explicitly not evidence that an ordinary browser or AT ran.
    serde_json::to_vec(&json!({"sessionId":id,"userAgent":"Mozilla/5.0 Chrome/126.0 Safari/537.36",
        "environment":{"probed":true},"behavior":{},"uaClientHints":{"brands":[{"brand":"Chromium","version":"126"}],"platform":"Windows"},
        "signals":[{"id":"l1.navigator.webdriver","value":false,"verdict":"OK","confidence":1,"collected":"wasm"}]})).unwrap()
}
fn collected(app: &mut Application, cfg: &Configuration, issued: &Value, now: u64) {
    let mut req = request(
        "POST",
        "/_humanymous/collect",
        client_report(issued["session"].as_str().unwrap()),
    );
    sign(
        cfg,
        &mut req,
        issued["session"].as_str().unwrap(),
        issued["key"].as_str().unwrap(),
        1,
        now,
    );
    assert!(matches!(
        app.decide(&req, || Ok(now)).unwrap(),
        Decision::Local(200, _)
    ));
}
fn protected(cfg: &Configuration, issued: &Value, n: u64, now: u64) -> Request {
    let mut req = request("POST", "/effect?exact=1", b"original bytes".to_vec());
    sign(
        cfg,
        &mut req,
        issued["session"].as_str().unwrap(),
        issued["key"].as_str().unwrap(),
        n,
        now,
    );
    req
}

#[test]
fn configuration_and_readiness_are_honest() {
    let mut cfg = config();
    assert!(cfg.validate().is_ok());
    assert_eq!(cfg.readiness()["product_qualified"], false);
    cfg.routes[0].required.collector = true;
    assert_eq!(cfg.readiness()["routes"][0]["ready"], false);
    cfg.listen = "0.0.0.0:1234".parse().unwrap();
    assert!(cfg.validate().is_err());
    cfg.listen = "127.0.0.1:0".parse().unwrap();
    cfg.routes[0].required.rit = false;
    assert!(cfg.validate().is_err());
}
#[test]
fn sessions_capacity_expiry_restart_and_rollback() {
    let mut cfg = config();
    cfg.session_capacity = 1;
    cfg.session_ttl_ms = 1000;
    let mut sessions = Sessions::new();
    let issued = sessions.issue(&cfg, 1000).unwrap();
    assert_eq!(sessions.issue(&cfg, 1001).err().unwrap().status, 429);
    assert!(sessions.issue(&cfg, 2000).is_ok());
    assert!(sessions.observe(1999).is_err());
    assert!(sessions.issue(&cfg, 2001).is_err());
    let mut request = request("POST", "/effect", vec![]);
    sign(&cfg, &mut request, &issued.session, &issued.key, 1, 1000);
    assert_eq!(
        Sessions::new()
            .authorize(&cfg, &request, 1000)
            .err()
            .unwrap()
            .status,
        401
    );
}
#[test]
fn signed_method_target_body_scope_policy_and_deadline_cannot_change() {
    let cfg = config();
    for mutation in 0..12 {
        let mut sessions = Sessions::new();
        let issued = sessions.issue(&cfg, 10_000).unwrap();
        let mut req = request("POST", "/effect", b"body".to_vec());
        sign(&cfg, &mut req, &issued.session, &issued.key, 1, 10_000);
        let mut other = config();
        match mutation {
            0 => req.method = "PUT".into(),
            1 => req.target = "/effect?changed=1".into(),
            2 => req.body.push(0),
            3 => other.scope.tenant = "other".into(),
            4 => other.scope.revision = "r2".into(),
            5 => other.scope.authority = "other.test".into(),
            6 => other.scope.installation = "other".into(),
            7 => {
                req.headers
                    .iter_mut()
                    .find(|(n, _)| n == "x-humanymous-deadline")
                    .unwrap()
                    .1 = "10901".into()
            }
            8 => req.headers.push(("Cookie".into(), "account=other".into())),
            9 => req.headers.swap(0, 1),
            10 => req
                .headers
                .push(("X-Forwarded-For".into(), "127.0.0.1".into())),
            _ => other.scope.origin = "other-origin".into(),
        }
        assert!(
            sessions.authorize(&other, &req, 10_000).is_err(),
            "mutation {mutation}"
        );
    }
}
#[test]
fn replay_and_collect_cross_session_cannot_gain_origin_ticket() {
    let cfg = config();
    let mut app =
        Application::new(config(), Default::default(), Box::new(Audit::default())).unwrap();
    let issued = issue(&mut app, 10_000);
    collected(&mut app, &cfg, &issued, 10_000);
    let req = protected(&cfg, &issued, 2, 10_000);
    assert!(matches!(
        app.decide(&req, || Ok(10_000)).unwrap(),
        Decision::Origin { .. }
    ));
    assert_eq!(
        app.decide(&req, || Ok(10_000)).err().unwrap().code,
        "request_integrity"
    );
    let other = issue(&mut app, 10_000);
    let mut req = request(
        "POST",
        "/_humanymous/collect",
        client_report(other["session"].as_str().unwrap()),
    );
    sign(
        &cfg,
        &mut req,
        issued["session"].as_str().unwrap(),
        issued["key"].as_str().unwrap(),
        3,
        10_000,
    );
    assert_eq!(
        app.decide(&req, || Ok(10_000)).err().unwrap().code,
        "report_session"
    );
    assert_eq!(
        app.decide(&req, || Ok(10_000)).err().unwrap().code,
        "request_integrity"
    );
}
#[test]
fn missing_client_is_challenge_and_missing_planes_are_not_ready() {
    let cfg = config();
    let mut app =
        Application::new(config(), Default::default(), Box::new(Audit::default())).unwrap();
    let issued = issue(&mut app, 10_000);
    let req = protected(&cfg, &issued, 1, 10_000);
    let Decision::Local(status, body) = app.decide(&req, || Ok(10_000)).unwrap() else {
        panic!("must not allow");
    };
    assert_eq!(status, 403);
    assert_eq!(
        serde_json::from_slice::<Value>(&body).unwrap()["verdict"],
        "CHALLENGE"
    );
    for field in 0..3 {
        let mut cfg = config();
        match field {
            0 => cfg.routes[0].required.raw_tls = true,
            1 => cfg.routes[0].required.original_h2 = true,
            _ => cfg.routes[0].required.collector = true,
        };
        let mut app =
            Application::new(cfg, Default::default(), Box::new(Audit::default())).unwrap();
        let issued = issue(&mut app, 10_000);
        let req = protected(&config(), &issued, 1, 10_000);
        assert_eq!(app.decide(&req, || Ok(10_000)).err().unwrap().status, 422);
    }
}
#[test]
fn audit_is_minimized_and_failure_poison_is_permanent() {
    let cfg = config();
    let audit = Audit::default();
    let mut app = Application::new(config(), Default::default(), Box::new(audit.clone())).unwrap();
    let issued = issue(&mut app, 10_000);
    collected(&mut app, &cfg, &issued, 10_000);
    let req = protected(&cfg, &issued, 2, 10_000);
    assert!(matches!(
        app.decide(&req, || Ok(10_000)).unwrap(),
        Decision::Origin { .. }
    ));
    let serialized = serde_json::to_string(&*audit.records.lock().unwrap()).unwrap();
    for secret in [
        issued["session"].as_str().unwrap(),
        issued["key"].as_str().unwrap(),
        "original bytes",
        "Mozilla",
    ] {
        assert!(!serialized.contains(secret));
    }
    audit.fail.store(true, Ordering::SeqCst);
    let req = protected(&cfg, &issued, 3, 10_000);
    assert_eq!(
        app.decide(&req, || Ok(10_000)).err().unwrap().code,
        "durable_audit"
    );
    audit.fail.store(false, Ordering::SeqCst);
    assert!(app.decide(&req, || Ok(10_000)).is_err());
    assert!(
        app.decide(&request("POST", "/_humanymous/session", vec![]), || Ok(
            10_000
        ))
        .is_err()
    );
}
#[test]
fn deadline_after_append_and_clock_rollback_never_release_origin() {
    use std::cell::Cell;
    let cfg = config();
    let audit = Audit::default();
    let mut app = Application::new(config(), Default::default(), Box::new(audit.clone())).unwrap();
    let issued = issue(&mut app, 10_000);
    collected(&mut app, &cfg, &issued, 10_000);
    let req = protected(&cfg, &issued, 2, 10_000);
    let calls = Cell::new(0);
    assert_eq!(
        app.decide(&req, || {
            let n = calls.get();
            calls.set(n + 1);
            Ok(if n < 2 { 10_000 } else { 10_900 })
        })
        .err()
        .unwrap()
        .code,
        "deadline"
    );
    assert_eq!(audit.records.lock().unwrap().len(), 2);
    assert!(app.decide(&req, || Ok(9_999)).is_err());
}
#[test]
fn report_rejects_server_planes_and_uses_registry_scores() {
    let cfg = config();
    let mut sessions = Sessions::new();
    let issued = sessions.issue(&cfg, 10_000).unwrap();
    for signal in [
        json!({"id":"l7.pass.solved","collected":"server","verdict":"OK","confidence":1}),
        json!({"id":"l1.navigator.webdriver","collected":"server","verdict":"OK","confidence":1}),
        json!({"id":"unknown","collected":"wasm","verdict":"OK","confidence":1}),
    ] {
        assert!(
            sessions
                .collect(
                    &issued.session,
                    &serde_json::to_vec(&json!({"sessionId":issued.session,"signals":[signal]}))
                        .unwrap()
                )
                .is_err()
        );
    }
    sessions.collect(&issued.session,&serde_json::to_vec(&json!({"sessionId":issued.session,"signals":[{
        "id":"l1.navigator.webdriver","collected":"wasm","verdict":"BOT","confidence":1,"score":-1000,"weight":-1000}]})).unwrap()).unwrap();
    assert_eq!(
        sessions.report(&issued.session).signals.unwrap()[0].score,
        15.0
    );
}

fn encode(req: &Request) -> Vec<u8> {
    let mut bytes = format!("{} {} HTTP/1.1\r\n", req.method, req.target).into_bytes();
    for (n, v) in &req.headers {
        bytes.extend_from_slice(format!("{n}: {v}\r\n").as_bytes());
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(&req.body);
    bytes
}
async fn exchange(addr: std::net::SocketAddr, req: &Request) -> Vec<u8> {
    let mut socket = TcpStream::connect(addr).await.unwrap();
    socket.write_all(&encode(req)).await.unwrap();
    let mut bytes = Vec::new();
    socket.read_to_end(&mut bytes).await.unwrap();
    bytes
}
fn json_body(bytes: &[u8]) -> Value {
    let end = bytes.windows(4).position(|b| b == b"\r\n\r\n").unwrap();
    serde_json::from_slice(&bytes[end + 4..]).unwrap()
}

#[tokio::test]
async fn real_tcp_audit_before_origin_and_negative_counter_zero() {
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin.local_addr().unwrap();
    let audit = Audit::default();
    let observed_audit = audit.clone();
    let count = Arc::new(AtomicUsize::new(0));
    let effects = count.clone();
    let origin_task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = origin.accept().await.unwrap();
            let req = crate::http::read(&mut socket).await.unwrap();
            assert!(req.header("authorization").is_none());
            assert!(req.header("x-humanymous-token").is_none());
            assert_eq!(req.body, b"original bytes");
            assert!(
                observed_audit
                    .records
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|r| r.event_type == "runtime.decision" && r.verdict == "ALLOW")
            );
            effects.fetch_add(1, Ordering::SeqCst);
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\n\r\nok").await.unwrap();
        }
    });
    let cfg = configuration(origin_addr);
    let server = TcpListener::bind(cfg.listen).await.unwrap();
    let address = server.local_addr().unwrap();
    let app = Application::new(
        configuration(origin_addr),
        Default::default(),
        Box::new(audit.clone()),
    )
    .unwrap();
    let server_task = tokio::spawn(app.serve(server));
    let issued =
        json_body(&exchange(address, &request("POST", "/_humanymous/session", vec![])).await);
    let now = humanymous_authority::now_ms().unwrap();
    let mut req = protected(&cfg, &issued, 1, now);
    req.body[0] = b'X';
    assert!(exchange(address, &req).await.starts_with(b"HTTP/1.1 403"));
    assert_eq!(count.load(Ordering::SeqCst), 0);
    let req = protected(&cfg, &issued, 1, now);
    let result = exchange(address, &req).await;
    assert_eq!(json_body(&result)["verdict"], "CHALLENGE");
    assert_eq!(count.load(Ordering::SeqCst), 0);
    let mut req = request(
        "POST",
        "/_humanymous/collect",
        client_report(issued["session"].as_str().unwrap()),
    );
    sign(
        &cfg,
        &mut req,
        issued["session"].as_str().unwrap(),
        issued["key"].as_str().unwrap(),
        2,
        now,
    );
    assert!(exchange(address, &req).await.starts_with(b"HTTP/1.1 200"));
    let req = protected(&cfg, &issued, 3, now);
    let result = exchange(address, &req).await;
    assert!(
        result.starts_with(b"HTTP/1.1 200"),
        "{} audit={}",
        String::from_utf8_lossy(&result),
        serde_json::to_string(&*audit.records.lock().unwrap()).unwrap()
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert!(
        result
            .windows(b"Set-Cookie: b=2\r".len())
            .any(|b| b == b"Set-Cookie: b=2\r")
    );
    assert!(exchange(address, &req).await.starts_with(b"HTTP/1.1 403"));
    assert_eq!(count.load(Ordering::SeqCst), 1);
    audit.fail.store(true, Ordering::SeqCst);
    let req = protected(&cfg, &issued, 4, now);
    assert!(exchange(address, &req).await.starts_with(b"HTTP/1.1 503"));
    assert_eq!(count.load(Ordering::SeqCst), 1);
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket
        .write_all(b"POST /effect HTTP/1.1\r\nHost: owned.test\r\nContent-Length: 20\r\n\r\nshort")
        .await
        .unwrap();
    socket.shutdown().await.unwrap();
    let mut ignored = Vec::new();
    let _ = socket.read_to_end(&mut ignored).await;
    assert_eq!(count.load(Ordering::SeqCst), 1);
    server_task.abort();
    origin_task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn client_abort_during_audit_never_dispatches_origin() {
    struct DelayedAudit {
        audit: Audit,
        armed: Arc<AtomicBool>,
        entered: Arc<AtomicBool>,
    }
    impl DurableAudit for DelayedAudit {
        fn append(&mut self, record: Record) -> std::io::Result<()> {
            self.audit.append(record)?;
            if self.armed.load(Ordering::SeqCst) {
                self.entered.store(true, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(80));
            }
            Ok(())
        }
    }
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let cfg = configuration(origin.local_addr().unwrap());
    let listener = TcpListener::bind(cfg.listen).await.unwrap();
    let address = listener.local_addr().unwrap();
    let armed = Arc::new(AtomicBool::new(false));
    let entered = Arc::new(AtomicBool::new(false));
    let audit = Audit::default();
    let app = Application::new(
        configuration(origin.local_addr().unwrap()),
        Default::default(),
        Box::new(DelayedAudit {
            audit: audit.clone(),
            armed: armed.clone(),
            entered: entered.clone(),
        }),
    )
    .unwrap();
    let server = tokio::spawn(app.serve(listener));
    let issued =
        json_body(&exchange(address, &request("POST", "/_humanymous/session", vec![])).await);
    let now = humanymous_authority::now_ms().unwrap();
    let mut collect = request(
        "POST",
        "/_humanymous/collect",
        client_report(issued["session"].as_str().unwrap()),
    );
    sign(
        &cfg,
        &mut collect,
        issued["session"].as_str().unwrap(),
        issued["key"].as_str().unwrap(),
        1,
        now,
    );
    assert!(
        exchange(address, &collect)
            .await
            .starts_with(b"HTTP/1.1 200")
    );
    armed.store(true, Ordering::SeqCst);
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket
        .write_all(&encode(&protected(&cfg, &issued, 2, now)))
        .await
        .unwrap();
    let socket = socket.into_std().unwrap();
    // Independent native thread ensures cancellation is delivered while audit
    // actually blocks, rather than waiting for that Tokio worker's timers.
    let abort = std::thread::spawn(move || {
        let start = std::time::Instant::now();
        while !entered.load(Ordering::SeqCst) {
            assert!(start.elapsed() < std::time::Duration::from_secs(1));
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        socket.shutdown(std::net::Shutdown::Both).unwrap();
    });
    tokio::task::spawn_blocking(move || abort.join().unwrap())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    assert!(
        audit
            .records
            .lock()
            .unwrap()
            .iter()
            .any(|r| r.verdict == "ALLOW")
    );
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(30), origin.accept())
            .await
            .is_err()
    );
    server.abort();
}
