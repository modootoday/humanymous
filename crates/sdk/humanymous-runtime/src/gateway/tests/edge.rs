//! Served-path ports of legacy internal/gate ban_integration_test.go,
//! forward_fidelity_test.go, prodhardening_test.go (security headers and
//! probes) and wargame_escalation_r161_test.go R162 at baseline 981d25c.
//! Every exchange goes through `Application::serve` on a real loopback
//! listener with a counting origin; expected bytes come from the oracle
//! capture `tests/fixtures/gateway/edge-gates-go-golden.json`.
use super::*;
use crate::gateway::admin_approval::Role;
use crate::gateway::admin_plane::{AdminPlane, PREFIX};
use crate::gateway::ban::{Store, gate::Mode};
use crate::gateway::guard::OriginKey;
use crate::gateway::service::ShutdownRequest;
use crate::network::history::rate::{Config as RateConfig, Limits};
use std::time::{Duration, Instant};

const ORIGIN_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn golden() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/gateway/edge-gates-go-golden.json"
    ))
    .unwrap()
}

/// A parsed downstream response: status, fields in wire order and body.
struct Reply {
    status: u16,
    fields: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Reply {
    fn parse(bytes: &[u8]) -> Self {
        let end = bytes.windows(4).position(|b| b == b"\r\n\r\n").unwrap();
        let head = std::str::from_utf8(&bytes[..end]).unwrap();
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .unwrap()
            .split(' ')
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        let fields = lines
            .map(|line| {
                let (name, value) = line.split_once(':').unwrap();
                (name.to_owned(), value.trim().to_owned())
            })
            .collect();
        Self {
            status,
            fields,
            body: bytes[end + 4..].to_vec(),
        }
    }

    fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(field, _)| field.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Status, body and every field the oracle recorded, compared exactly;
    /// `Retry-After` is compared separately (see the callers).
    fn assert_matches(&self, go: &Value) {
        assert_eq!(u64::from(self.status), go["status"].as_u64().unwrap());
        assert_eq!(
            std::str::from_utf8(&self.body).unwrap(),
            go["body"].as_str().unwrap()
        );
        for (name, value) in go["headers"].as_object().unwrap() {
            if name != "Retry-After" {
                assert_eq!(self.field(name), value.as_str(), "{name}");
            }
        }
    }
}

/// Header fields of one forwarded request, in arrival order.
type Fields = Vec<(String, String)>;

/// Counting origin that records each forwarded request's fields and answers
/// `response`.
struct Origin {
    address: std::net::SocketAddr,
    hits: Arc<AtomicUsize>,
    seen: Arc<Mutex<Vec<Fields>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Origin {
    async fn start(response: &'static [u8]) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let hits = Arc::new(AtomicUsize::new(0));
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (counter, record) = (hits.clone(), seen.clone());
        let task = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                counter.fetch_add(1, Ordering::SeqCst);
                let request = crate::gateway::http::read(&mut socket).await.unwrap();
                record.lock().unwrap().push(request.headers);
                socket.write_all(response).await.unwrap();
            }
        });
        Self {
            address,
            hits,
            seen,
            task,
        }
    }
}

impl Drop for Origin {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn plain(target: &str, extra: &[(&str, &str)]) -> Request {
    let mut headers = vec![
        ("Host".to_owned(), "owned.test".to_owned()),
        ("Content-Length".to_owned(), "0".to_owned()),
    ];
    headers.extend(
        extra
            .iter()
            .map(|(n, v)| ((*n).to_owned(), (*v).to_owned())),
    );
    Request {
        method: "GET".into(),
        target: target.into(),
        headers,
        body: Vec::new(),
    }
}

async fn serve(
    app: Application,
) -> (
    std::net::SocketAddr,
    tokio::task::JoinHandle<std::io::Result<()>>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    (address, tokio::spawn(app.serve(listener)))
}

fn records(audit: &Audit, event: &str) -> Vec<Record> {
    let all = audit.records.lock().unwrap();
    all.iter()
        .filter(|r| r.event_type == event)
        .cloned()
        .collect()
}

/// Go TestProxyBlocksSpoofedTrustHeaders: each forged trust header is denied
/// with the Go response and HR-27b record; the origin is never contacted.
#[tokio::test]
async fn spoofed_trust_headers_are_denied_before_origin() {
    let go = golden();
    let origin = Origin::start(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok").await;
    let audit = Audit::default();
    let app = Application::new(
        configuration(origin.address),
        Default::default(),
        Box::new(audit.clone()),
    )
    .unwrap();
    let (address, server) = serve(app).await;
    for name in [
        "X-Forwarded-For",
        "X-Real-Ip",
        "X-Hmny-Origin-Auth",
        "Forwarded",
        "Cf-Connecting-Ip",
        "X-Hmn-Verdict",
    ] {
        let reply =
            Reply::parse(&exchange(address, &plain("/api/thing", &[(name, "9.9.9.9")])).await);
        reply.assert_matches(&go["spoof"][name]);
        let record = records(&audit, "hdr.internal_header_ingress")
            .pop()
            .unwrap();
        assert_eq!(
            record.fail_reason,
            format!("inbound internal headers: {name}")
        );
        assert_eq!(record.triggered_rules, Some(vec!["HR-27b".to_owned()]));
        assert_eq!(
            (
                record.enforcement_action.as_str(),
                record.enforcement_mode.as_str()
            ),
            ("block", "enforce")
        );
    }
    // An empty value is no forged claim (Go spoofScan): no record, no deny.
    let reply = Reply::parse(&exchange(address, &plain("/api/thing", &[("X-Real-Ip", "")])).await);
    assert_eq!(
        (
            reply.status,
            records(&audit, "hdr.internal_header_ingress").len()
        ),
        (409, 6)
    );
    assert_eq!(origin.hits.load(Ordering::SeqCst), 0);
    server.abort();
}

/// Go spoofHeaderGate demotion: a non-enforcing mode records mode `monitor`
/// and the request continues (here to the route inventory refusal).
#[test]
fn spoof_gate_monitor_records_and_continues() {
    for mode in [Mode::Monitor, Mode::Shadow, Mode::Off] {
        let audit = Audit::default();
        let mut cfg = config();
        cfg.edge.spoof_header = mode;
        let mut app = Application::new(cfg, Default::default(), Box::new(audit.clone())).unwrap();
        let req = plain("/api/thing", &[("Forwarded", "for=9.9.9.9")]);
        assert_eq!(
            app.decide(&req, || Ok(10_000)).err(),
            Some(crate::gateway::Rejection::new(409, "route_inventory"))
        );
        let record = records(&audit, "hdr.internal_header_ingress")
            .pop()
            .unwrap();
        assert_eq!(record.enforcement_mode, "monitor", "{mode:?}");
    }
}

/// A gate record that cannot be appended fails closed and poisons the epoch.
#[test]
fn unaudited_gate_effect_is_refused_and_poisons_the_epoch() {
    let audit = Audit::default();
    audit.fail.store(true, Ordering::SeqCst);
    let mut app = Application::new(config(), Default::default(), Box::new(audit.clone())).unwrap();
    let req = plain("/api/thing", &[("X-Forwarded-For", "9.9.9.9")]);
    assert_eq!(
        app.decide(&req, || Ok(10_000)).err(),
        Some(crate::gateway::Rejection::new(503, "durable_audit"))
    );
    assert!(!app.epoch_health().serving());
}

/// Full ALLOW exchange through `serve`; returns the downstream reply and the
/// fields the origin received.
async fn allowed_exchange(
    cfg_edit: impl Fn(&mut Configuration),
    origin_response: &'static [u8],
) -> (Reply, Vec<(String, String)>) {
    let origin = Origin::start(origin_response).await;
    let mut cfg = configuration(origin.address);
    cfg_edit(&mut cfg);
    let mut app_cfg = configuration(origin.address);
    cfg_edit(&mut app_cfg);
    let app = Application::new(app_cfg, Default::default(), Box::new(Audit::default())).unwrap();
    let (address, server) = serve(app).await;
    let issued =
        json_body(&exchange(address, &request("POST", "/_humanymous/session", vec![])).await);
    let now = crate::authority::now_ms().unwrap();
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
    let reply = Reply::parse(&exchange(address, &protected(&cfg, &issued, 2, now)).await);
    server.abort();
    assert_eq!(origin.hits.load(Ordering::SeqCst), 1, "one origin effect");
    let seen = origin.seen.lock().unwrap().pop().unwrap();
    (reply, seen)
}

/// Go TestProxyForwardsClientRequestFaithfully (origin-cloak half) and
/// TestSecurityHeadersAddOnly on the plaintext listener: the forwarded
/// request carries one valid `X-Hmny-Origin-Auth`; the proxied response gains
/// the missing security fields and keeps the origin's own values.
#[tokio::test]
async fn forwarded_request_is_cloaked_and_response_gains_add_only_security_headers() {
    let go = golden();
    let (reply, seen) = allowed_exchange(
        |cfg| cfg.edge.origin_key_hex = Some(ORIGIN_KEY.to_owned().into()),
        b"HTTP/1.1 201 Created\r\nContent-Length: 2\r\nX-Frame-Options: DENY\r\nReferrer-Policy: no-referrer\r\nSet-Cookie: up=1\r\n\r\nok",
    )
    .await;
    let tokens: Vec<_> = seen
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("x-hmny-origin-auth"))
        .collect();
    assert_eq!(tokens.len(), 1);
    let key = OriginKey::new(hex::decode(ORIGIN_KEY).unwrap()).unwrap();
    let now = i64::try_from(crate::authority::now_ms().unwrap() / 1000).unwrap();
    assert!(key.valid(&tokens[0].1, now));
    assert_eq!((reply.status, reply.body.as_slice()), (201, &b"ok"[..]));
    assert_eq!(reply.field("Set-Cookie"), Some("up=1"));
    let origin_set = &go["security_origin_set"]["headers"];
    for name in [
        "X-Content-Type-Options",
        "Referrer-Policy",
        "X-Frame-Options",
    ] {
        assert_eq!(reply.field(name), origin_set[name].as_str(), "{name}");
    }
    assert!(reply.field("Strict-Transport-Security").is_none());

    // Without a configured key no cloak field is attached; Go defaults apply.
    let (reply, seen) =
        allowed_exchange(|_| {}, b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok").await;
    assert!(
        !seen
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("x-hmny-origin-auth"))
    );
    let defaults = &go["security_default"]["headers"];
    for name in [
        "X-Content-Type-Options",
        "Referrer-Policy",
        "X-Frame-Options",
    ] {
        assert_eq!(reply.field(name), defaults[name].as_str(), "{name}");
    }
    assert!(go["security_hsts"]["headers"]["Strict-Transport-Security"].is_string());
    assert!(reply.field("Strict-Transport-Security").is_none());
}

/// Owner decision C10: enabled cloaking without a valid key and HSTS on the
/// plaintext listener are configuration refusals.
#[test]
fn invalid_origin_key_and_plaintext_hsts_are_refused() {
    let mut cfg = config();
    cfg.edge.origin_key_hex = Some("ab".repeat(31).into());
    assert!(cfg.validate().is_err());
    assert!(Application::new(cfg, Default::default(), Box::new(Audit::default())).is_err());
    let mut cfg = config();
    cfg.edge.hsts = true;
    assert!(cfg.validate().is_err());
    let cfg = config();
    assert_eq!(
        cfg.readiness()["edge"],
        json!({"spoof_header":"enforce","ban":"enforce","hsts":false,"origin_cloak":false})
    );
}

/// Go TestReadinessProbe: ready, then draining (503) after the switch, while
/// liveness stays ok; the bodies are the oracle's.
#[tokio::test]
async fn readiness_probe_drains_and_liveness_stays_ok() {
    let go = golden();
    let app = Application::new(config(), Default::default(), Box::new(Audit::default())).unwrap();
    let switch = app.readiness_switch();
    let (address, server) = serve(app).await;
    let probe = move |target: &'static str| async move {
        Reply::parse(&exchange(address, &plain(target, &[])).await)
    };
    probe("/_humanymous/healthz")
        .await
        .assert_matches(&go["healthz"]);
    probe("/_humanymous/readyz")
        .await
        .assert_matches(&go["readyz"]);
    switch.set_ready(false);
    probe("/_humanymous/readyz")
        .await
        .assert_matches(&go["readyz_draining"]);
    probe("/_humanymous/healthz")
        .await
        .assert_matches(&go["healthz_draining"]);
    switch.set_ready(true);
    probe("/_humanymous/readyz")
        .await
        .assert_matches(&go["readyz"]);
    // Only GET with the configured authority; anything else stays reserved.
    let mut wrong_host = plain("/_humanymous/readyz", &[]);
    wrong_host.headers[0].1 = "other.test".into();
    assert!(
        exchange(address, &wrong_host)
            .await
            .starts_with(b"HTTP/1.1 403")
    );
    let mut post = plain("/_humanymous/readyz", &[]);
    post.method = "POST".into();
    assert_eq!(
        json_body(&exchange(address, &post).await)["error"],
        "reserved_route"
    );
    server.abort();
}

/// Readiness is withdrawn once the serving epoch has failed closed.
#[test]
fn poisoned_epoch_is_not_ready() {
    let audit = Audit::default();
    let mut app = Application::new(config(), Default::default(), Box::new(audit.clone())).unwrap();
    audit.fail.store(true, Ordering::SeqCst);
    let _ = app.decide(&plain("/api/thing", &[("X-Real-Ip", "1.2.3.4")]), || {
        Ok(10_000)
    });
    let Decision::Local(status, body) = app
        .decide(&plain("/_humanymous/readyz", &[]), || Ok(10_000))
        .unwrap()
    else {
        panic!("probe")
    };
    assert_eq!(
        (status, body.as_slice()),
        (503, &b"{\"plane\":\"gate\",\"status\":\"draining\"}\n"[..])
    );
}

fn ban_store(soft: usize, hard: usize) -> Arc<Mutex<Store>> {
    let limits = Limits::new(64, 64, 128, Duration::from_secs(60), 1 << 20, 64).unwrap();
    let rate = RateConfig::new(Duration::from_secs(60), soft, hard, limits).unwrap();
    Arc::new(Mutex::new(Store::new(rate, 64).unwrap()))
}

fn edge_config(origin: std::net::SocketAddr) -> Configuration {
    let mut cfg = configuration(origin);
    cfg.ip_reputation = Some(
        serde_json::from_value(json!({"snapshot":"/nonexistent/ip-intel.json",
            "client_address":"proxy_v2","proxy_header_timeout_ms":1000}))
        .unwrap(),
    );
    cfg
}

fn proxy_v2(source: [u8; 4]) -> Vec<u8> {
    let mut header = crate::network::proxy::SIGNATURE.to_vec();
    header.extend_from_slice(&[0x21, 0x11, 0x00, 0x0c]);
    header.extend_from_slice(&source);
    header.extend_from_slice(&[127, 0, 0, 1, 0x30, 0x39, 0x01, 0xbb]);
    header
}

async fn from(address: std::net::SocketAddr, source: [u8; 4], request: &Request) -> Reply {
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket.write_all(&proxy_v2(source)).await.unwrap();
    socket.write_all(&encode(request)).await.unwrap();
    let mut bytes = Vec::new();
    socket.read_to_end(&mut bytes).await.unwrap();
    Reply::parse(&bytes)
}

/// Served application with an installed ban store behind a PROXY v2 edge.
async fn banned_edge(
    store: Arc<Mutex<Store>>,
    origin: &Origin,
) -> (
    Audit,
    std::net::SocketAddr,
    tokio::task::JoinHandle<std::io::Result<()>>,
) {
    let audit = Audit::default();
    let mut app = Application::new(
        edge_config(origin.address),
        Default::default(),
        Box::new(audit.clone()),
    )
    .unwrap();
    app.install_bans(store, Instant::now()).unwrap();
    let (address, server) = serve(app).await;
    (audit, address, server)
}

fn retry_after(reply: &Reply) -> u64 {
    reply.field("Retry-After").unwrap().parse().unwrap()
}

/// Go TestAutoBanEndToEnd: a flood past the hard threshold is banned at the
/// edge with the Go response and record sequence; the origin is never
/// contacted for the banned source and an unrelated address is unaffected.
#[tokio::test]
async fn flood_is_auto_banned_at_the_edge_and_spares_other_address() {
    let go = golden();
    let origin = Origin::start(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok").await;
    let (audit, address, server) = banned_edge(ban_store(8, 10), &origin).await;
    let mut statuses = Vec::new();
    let mut replies = Vec::new();
    for _ in 0..12 {
        let reply = from(address, [203, 0, 113, 9], &plain("/data", &[])).await;
        statuses.push(reply.status == 403);
        replies.push(reply);
    }
    let expected: Vec<bool> = go["ban_statuses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|status| status == 403)
        .collect();
    assert_eq!(statuses, expected);
    // Go measured 3599 for the first block (its clock had moved on); the
    // kernel evaluates creation and block at one instant, so 3600 or 3599.
    for reply in &replies[9..] {
        reply.assert_matches(&go["ban_blocked_response"]);
        assert!((3599..=3600).contains(&retry_after(reply)));
    }
    let events: Vec<_> = audit
        .records
        .lock()
        .unwrap()
        .iter()
        .map(|r| {
            json!({"event":r.event_type,"actor":r.actor.kind,"action":r.enforcement_action,
                "mode":r.enforcement_mode,"verdict":r.verdict.to_ascii_lowercase(),
                "rules":r.triggered_rules.clone().unwrap_or_default(),"fail_reason":r.fail_reason,
                "route_class":r.route_class})
        })
        .collect();
    assert_eq!(Value::Array(events), go["ban_events"]);
    let other = from(address, [198, 51, 100, 2], &plain("/data", &[])).await;
    assert_ne!(other.status, 403);
    assert_eq!(origin.hits.load(Ordering::SeqCst), 0);
    server.abort();
}

/// Go TestFingerprintAutoBan: a flood that rotates its address but keeps one
/// header fingerprint is banned by `fp:`, and a fresh address with that
/// fingerprint is blocked.
#[tokio::test]
async fn ip_rotating_flood_is_banned_by_fingerprint_on_the_served_path() {
    let origin = Origin::start(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok").await;
    let store = ban_store(8, 10);
    let (_, address, server) = banned_edge(store.clone(), &origin).await;
    let browser = [
        ("User-Agent", "Mozilla/5.0 Chrome/126.0 Safari/537.36"),
        ("Accept-Language", "en-US,en;q=0.9"),
        ("sec-ch-ua", "\"Chromium\";v=\"126\""),
    ];
    for i in 0..20u8 {
        from(address, [10, 0, 0, i], &plain("/data", &browser)).await;
    }
    let fresh = from(address, [172, 16, 9, 9], &plain("/data", &browser)).await;
    assert_eq!(fresh.status, 403);
    let bans = store.lock().unwrap().list(Duration::ZERO);
    assert!(
        bans.iter()
            .any(|entry| entry.key.as_str().starts_with("fp:"))
    );
    server.abort();
}

/// An unattributed edge connection cannot be metered: refused when the ban
/// gate enforces. Installing a store without a client-address source is
/// refused.
#[tokio::test]
async fn ban_gate_requires_a_trusted_client_address() {
    let mut app =
        Application::new(config(), Default::default(), Box::new(Audit::default())).unwrap();
    assert!(app.install_bans(ban_store(8, 10), Instant::now()).is_err());
    let mut cfg = config();
    cfg.edge.ban = Mode::Off;
    let mut app = Application::new(cfg, Default::default(), Box::new(Audit::default())).unwrap();
    assert!(app.install_bans(ban_store(8, 10), Instant::now()).is_ok());

    let origin = Origin::start(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok").await;
    let (_, address, server) = banned_edge(ban_store(8, 10), &origin).await;
    let mut socket = TcpStream::connect(address).await.unwrap();
    let mut local = crate::network::proxy::SIGNATURE.to_vec();
    local.extend_from_slice(&[0x20, 0x00, 0x00, 0x00]);
    socket.write_all(&local).await.unwrap();
    socket
        .write_all(&encode(&plain("/data", &[])))
        .await
        .unwrap();
    let mut bytes = Vec::new();
    socket.read_to_end(&mut bytes).await.unwrap();
    assert_eq!(json_body(&bytes)["error"], "ban_client_address");
    assert!(bytes.starts_with(b"HTTP/1.1 503"));
    server.abort();
}

/// Admin plane on its own loopback listener sharing the ban store.
struct Admin {
    address: std::net::SocketAddr,
    plane: Arc<AdminPlane>,
    _stop: tokio::sync::watch::Sender<Option<ShutdownRequest>>,
    task: tokio::task::JoinHandle<std::io::Result<()>>,
}

impl Drop for Admin {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Admin {
    async fn start(store: Arc<Mutex<Store>>) -> Self {
        let plane = Arc::new(AdminPlane::new(
            "test",
            dual_control::scope(),
            b"k",
            store,
            Box::new(Audit::default()),
        ));
        for role in [Role::Operator, Role::Approver] {
            plane
                .auth()
                .add(
                    &format!("{}-token-0123456789", role.as_str()),
                    &format!("{}-1", role.as_str()),
                    role,
                )
                .unwrap();
        }
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, shutdown) = tokio::sync::watch::channel(None);
        let task = tokio::spawn(plane.clone().serve_until(listener, shutdown));
        Self {
            address,
            plane,
            _stop: stop,
            task,
        }
    }

    async fn call(&self, role: &str, method: &str, sub: &str, body: &str) -> (u16, Value) {
        let mut bytes = format!(
            "{method} {PREFIX}{sub} HTTP/1.1\r\nHost: admin\r\nContent-Length: {}\r\nAuthorization: Bearer {role}-token-0123456789\r\n",
            body.len()
        );
        if !body.is_empty() {
            bytes.push_str("Content-Type: application/json\r\n");
        }
        bytes.push_str("\r\n");
        bytes.push_str(body);
        let mut socket = TcpStream::connect(self.address).await.unwrap();
        socket.write_all(bytes.as_bytes()).await.unwrap();
        let mut reply = Vec::new();
        socket.read_to_end(&mut reply).await.unwrap();
        let reply = Reply::parse(&reply);
        (
            reply.status,
            serde_json::from_slice(&reply.body).unwrap_or(Value::Null),
        )
    }
}

/// Go TestConsoleBanManagement over the served path: an operator's
/// temporary ban on the admin plane blocks the source at the public edge
/// with Retry-After, is listed, and passes again once lifted.
#[tokio::test]
async fn admin_temporary_ban_blocks_at_the_edge_until_lifted() {
    let origin = Origin::start(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok").await;
    let store = ban_store(998, 1000);
    let (_, address, server) = banned_edge(store.clone(), &origin).await;
    let admin = Admin::start(store).await;
    let (status, _) = admin
        .call(
            "operator",
            "POST",
            "bans",
            r#"{"Key":"ip:192.0.2.44","Reason":"manual investigation","DurationSec":3600,"Incident":"INC-9"}"#,
        )
        .await;
    assert_eq!(status, 200);
    let blocked = from(address, [192, 0, 2, 44], &plain("/", &[])).await;
    assert_eq!(blocked.status, 403);
    assert!((3590..=3600).contains(&retry_after(&blocked)));
    let (_, listed) = admin.call("operator", "GET", "bans", "").await;
    assert!(listed.to_string().contains("192.0.2.44"));
    let (status, _) = admin
        .call("operator", "POST", "bans/lift?key=ip:192.0.2.44", "")
        .await;
    assert_eq!(status, 200);
    assert_ne!(
        from(address, [192, 0, 2, 44], &plain("/", &[]))
            .await
            .status,
        403
    );
    assert_eq!(origin.hits.load(Ordering::SeqCst), 0);
    server.abort();
}

/// Commits with a retained signed permit from a real control ledger.
mod dual_control {
    use super::*;
    use crate::audit::control::Scope;

    pub(super) fn scope() -> Scope {
        Scope {
            installation: "installation".into(),
            node: "node".into(),
        }
    }

    #[cfg(unix)]
    mod ledger {
        use super::*;
        use crate::audit::control::{
            Action, Administrators, Approval, ApprovalRequest, ControlAuthority, LedgerBootstrap,
            NativeAnchorRetainer, RetainedActionPermit, RetentionRequirement,
        };
        use crate::audit::keys::custody::Owner;
        use crate::gateway::ban::Key;
        use ed25519_dalek::{Signer, SigningKey};
        use std::collections::BTreeMap;
        use std::path::PathBuf;
        use std::sync::atomic::AtomicU64;

        static NEXT: AtomicU64 = AtomicU64::new(0);

        struct Dir(PathBuf);

        impl Dir {
            fn new() -> Self {
                use std::os::unix::fs::PermissionsExt;
                let path = std::env::temp_dir().join(format!(
                    "humanymous-edge-gates-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                std::fs::create_dir(&path).unwrap();
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
                Self(std::fs::canonicalize(path).unwrap())
            }
        }

        impl Drop for Dir {
            fn drop(&mut self) {
                std::fs::remove_dir_all(&self.0).unwrap();
            }
        }

        fn signers() -> [(&'static str, SigningKey); 2] {
            [
                ("one", SigningKey::from_bytes(&[11; 32])),
                ("two", SigningKey::from_bytes(&[12; 32])),
            ]
        }

        /// One permanent-ban permit for `key`, approved by both fixture
        /// administrators and retained by an independent anchor.
        fn ban_permit(key: &str) -> RetainedActionPermit {
            let (local, independent) = (Dir::new(), Dir::new());
            let administrators = Administrators::new(
                scope(),
                signers()
                    .iter()
                    .map(|(id, key)| ((*id).to_owned(), key.verifying_key().to_bytes()))
                    .collect::<BTreeMap<_, _>>(),
                100,
            )
            .unwrap();
            let mut authority = ControlAuthority::open(
                &local.0,
                Owner::CurrentUser,
                &[13; 32],
                administrators,
                20,
                LedgerBootstrap::New,
            )
            .unwrap();
            let mut retainer = NativeAnchorRetainer::open(
                &independent.0,
                Owner::CurrentUser,
                scope(),
                authority.public_key(),
                authority.anchor(),
                true,
            )
            .unwrap();
            let now = crate::authority::now_ms().unwrap() / 1000;
            let proposal = authority
                .propose(
                    Action::PermanentBan,
                    Key::parse(key).unwrap().commitment(),
                    now,
                    now + 90,
                )
                .unwrap();
            let approvals: Vec<Approval> = signers()
                .iter()
                .map(|(id, signer)| Approval {
                    administrator: (*id).into(),
                    signature: hex::encode(
                        signer.sign(&proposal.signing_bytes().unwrap()).to_bytes(),
                    ),
                })
                .collect();
            authority
                .authorize_retained_action(
                    ApprovalRequest {
                        proposal: &proposal,
                        approvals: &approvals,
                        now,
                    },
                    &mut retainer,
                    RetentionRequirement::FileSynced,
                )
                .unwrap()
        }

        /// Go TestWargameR162_CommitThenEdgeDenies: after the dual-control
        /// commit on the admin plane, the public listener answers 403 for
        /// that source from the shared ban store, without Retry-After.
        #[tokio::test]
        async fn r162_commit_then_edge_denies() {
            let origin = Origin::start(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok").await;
            let store = ban_store(998, 1000);
            let (_, address, server) = banned_edge(store.clone(), &origin).await;
            let admin = Admin::start(store).await;
            admin
                .plane
                .retain_ban_permit(ban_permit("ip:203.0.113.162"))
                .unwrap();
            assert_ne!(
                from(address, [203, 0, 113, 162], &plain("/", &[]))
                    .await
                    .status,
                403
            );
            let (_, pending) = admin
                .call(
                    "operator",
                    "POST",
                    "bans",
                    r#"{"Key":"ip:203.0.113.162","DurationSec":0,"Reason":"r162"}"#,
                )
                .await;
            let id = pending["approvalId"].as_str().unwrap().to_owned();
            let (status, _) = admin
                .call("approver", "POST", &format!("approvals/{id}"), "")
                .await;
            assert_eq!(status, 200);
            let denied = from(address, [203, 0, 113, 162], &plain("/", &[])).await;
            assert_eq!(denied.status, 403);
            assert_eq!(denied.field("X-Hmn-Incident"), Some("see-audit-log"));
            assert!(denied.field("Retry-After").is_none(), "permanent");
            assert_eq!(origin.hits.load(Ordering::SeqCst), 0);
            server.abort();
        }
    }
}
