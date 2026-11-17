//! Real listener dispatch of the legacy surfaces; not a browser e2e run.
use super::*;
use crate::audit::record::Record;
use crate::authority::decision::DurableAudit;
use crate::gateway::{application::Application, configuration::Configuration};
use serde_json::Value;
use std::net::SocketAddr;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[derive(Clone, Default)]
struct Audit(Arc<Mutex<Vec<Record>>>);
impl DurableAudit for Audit {
    fn append(&mut self, record: Record) -> std::io::Result<()> {
        self.0.lock().unwrap().push(record);
        Ok(())
    }
}

fn web_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../web")
        .canonicalize()
        .unwrap()
}

fn policy(pass: bool, demo: bool, static_assets: bool) -> LegacyPolicy {
    LegacyPolicy {
        web_dir: web_dir(),
        pass,
        demo,
        static_assets,
    }
}

fn configuration(legacy: Option<LegacyPolicy>) -> Configuration {
    let mut config = crate::gateway::tests::config();
    config.legacy = legacy;
    config
}

/// Bind the real application listener; returns its address and audit.
async fn serve(legacy: Option<LegacyPolicy>) -> (SocketAddr, Audit, tokio::task::JoinHandle<()>) {
    let audit = Audit::default();
    let app = Application::new(
        configuration(legacy),
        Default::default(),
        Box::new(audit.clone()),
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let _ = app.serve(listener).await;
    });
    (address, audit, task)
}

struct Reply {
    status: u16,
    head: String,
    body: Vec<u8>,
}
impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.head.split("\r\n").skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name).then(|| value.trim())
        })
    }
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }
}

/// One raw HTTP/1.1 exchange shaped like a browser navigation: no
/// Content-Length on bodyless requests unless `fields` adds one.
async fn exchange(address: SocketAddr, method: &str, target: &str, fields: &str) -> Reply {
    exchange_body(address, method, target, fields, b"").await
}

async fn exchange_body(
    address: SocketAddr,
    method: &str,
    target: &str,
    fields: &str,
    body: &[u8],
) -> Reply {
    let mut socket = TcpStream::connect(address).await.unwrap();
    let mut frame = format!(
        "{method} {target} HTTP/1.1\r\nHost: owned.test\r\nUser-Agent: Mozilla/5.0 Chrome/126.0\r\nAccept: */*\r\n{fields}"
    )
    .into_bytes();
    if !body.is_empty() {
        frame.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    }
    frame.extend_from_slice(b"\r\n");
    frame.extend_from_slice(body);
    socket.write_all(&frame).await.unwrap();
    let mut bytes = Vec::new();
    socket.read_to_end(&mut bytes).await.unwrap();
    let end = bytes.windows(4).position(|b| b == b"\r\n\r\n").unwrap();
    let head = String::from_utf8(bytes[..end].to_vec()).unwrap();
    let status = head[9..12].parse().unwrap();
    Reply {
        status,
        head,
        body: bytes[end + 4..].to_vec(),
    }
}

fn error_code(reply: &Reply) -> String {
    reply.json()["error"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

#[tokio::test]
async fn legacy_absent_keeps_prior_gateway_behaviour() {
    let (address, audit, task) = serve(None).await;
    for path in ["/pass", "/api/pass/new", "/demo", "/static/js/demo.js"] {
        let bare = exchange(address, "GET", path, "").await;
        assert_eq!(
            (bare.status, error_code(&bare).as_str()),
            (400, "missing_integer"),
            "{path}"
        );
        let framed = exchange(address, "GET", path, "Content-Length: 0\r\n").await;
        assert_eq!(
            (framed.status, error_code(&framed).as_str()),
            (409, "route_inventory"),
            "{path}"
        );
    }
    let other = exchange(address, "GET", "/other", "").await;
    assert_eq!(
        other.status, 400,
        "non-legacy bodyless framing stays refused"
    );
    assert!(audit.0.lock().unwrap().is_empty());
    task.abort();
}

#[tokio::test]
async fn browser_get_pass_serves_the_page_with_session_and_security_headers() {
    let (address, audit, task) = serve(Some(policy(true, false, false))).await;
    let page = exchange(address, "GET", "/pass", "").await;
    assert_eq!(page.status, 200, "{}", page.head);
    assert_eq!(
        page.header("Content-Type"),
        Some("text/html; charset=utf-8")
    );
    let cookie = page.header("Set-Cookie").unwrap();
    assert!(
        cookie.starts_with("hsid=") && cookie.ends_with("; Path=/; HttpOnly; Secure; SameSite=Lax")
    );
    assert_eq!(page.header("X-Content-Type-Options"), Some("nosniff"));
    assert_eq!(
        page.header("Referrer-Policy"),
        Some("strict-origin-when-cross-origin")
    );
    assert!(
        page.header("Content-Security-Policy-Report-Only")
            .unwrap()
            .contains("'wasm-unsafe-eval'")
    );
    assert_eq!(
        page.body,
        std::fs::read(web_dir().join("pass.html")).unwrap()
    );
    assert_eq!(
        page.header("Content-Length"),
        Some(page.body.len().to_string().as_str())
    );
    let head = exchange(address, "HEAD", "/pass", "").await;
    assert_eq!(head.status, 200);
    assert!(head.body.is_empty());
    // Public sessionless surface: no gateway decision is written.
    assert!(audit.0.lock().unwrap().is_empty());
    task.abort();
}

#[tokio::test]
async fn pass_api_issues_a_challenge_and_enforces_post_only_routes() {
    let (address, _, task) = serve(Some(policy(true, false, false))).await;
    let fresh = exchange(address, "GET", "/api/pass/new", "").await;
    assert_eq!(fresh.status, 200, "{}", fresh.head);
    let session = fresh.header("Set-Cookie").unwrap();
    let session = session.split(';').next().unwrap().to_owned();
    let issued = fresh.json();
    assert_eq!(issued["ok"], true);
    assert_eq!(
        issued["triesLeft"],
        crate::gateway::pass_store::PASS_MAX_TRIES
    );
    assert!(issued["challenge"].is_object());
    assert!(!issued["challengeNonce"].as_str().unwrap().is_empty());
    assert!(issued["preflight"]["pow"].is_object());
    let cookie = format!("Cookie: {session}\r\n");
    let again = exchange(address, "GET", "/api/pass/new", &cookie).await;
    assert_eq!(again.header("Set-Cookie"), None, "the session is reused");
    assert_ne!(again.json()["challengeNonce"], issued["challengeNonce"]);
    for path in ["/api/pass/pow", "/api/pass/attest", "/api/pass/solve"] {
        let refused = exchange(address, "GET", path, &cookie).await;
        assert_eq!(refused.status, 405, "{path}");
        assert_eq!(refused.body, b"POST only\n");
    }
    let stale = exchange_body(
        address,
        "POST",
        "/api/pass/attest",
        &format!("{cookie}Content-Type: application/json\r\n"),
        br#"{"challengeNonce":"00"}"#,
    )
    .await;
    assert_eq!(stale.status, 200);
    assert_eq!(stale.json()["reason"], "stale challenge");
    let kpi = exchange(address, "GET", "/api/pass/kpi", "").await;
    assert_eq!(kpi.status, 200);
    assert!(kpi.json().is_object());
    task.abort();
}

#[tokio::test]
async fn demo_and_static_follow_the_go_paths() {
    let (address, _, task) = serve(Some(policy(false, true, true))).await;
    let demo = exchange(address, "GET", "/demo", "").await;
    assert_eq!(demo.status, 200);
    assert!(demo.header("Set-Cookie").unwrap().starts_with("hsid="));
    assert_eq!(
        demo.body,
        std::fs::read(web_dir().join("demo.html")).unwrap()
    );
    let script = exchange(address, "GET", "/static/js/demo.js?v=1", "").await;
    assert_eq!(script.status, 200);
    assert_eq!(
        script.header("Content-Type"),
        Some("text/javascript; charset=utf-8")
    );
    assert_eq!(
        script.body,
        std::fs::read(web_dir().join("js/demo.js")).unwrap()
    );
    let head = exchange(address, "HEAD", "/static/js/demo.js", "").await;
    assert_eq!((head.status, head.body.len()), (200, 0));
    for missing in [
        "/static/",
        "/static/js/",
        "/static/js/absent.js",
        "/static/js//demo.js",
    ] {
        let reply = exchange(address, "GET", missing, "").await;
        assert_eq!(reply.status, 404, "{missing}");
        assert_eq!(reply.body, b"404 page not found\n");
    }
    // Traversal never reaches the dispatcher: the strict parser refuses it.
    let traversal = exchange(address, "GET", "/static/../Cargo.toml", "").await;
    assert_eq!(traversal.status, 400);
    task.abort();
}

#[tokio::test]
async fn disabled_and_unmounted_surfaces_keep_gateway_behaviour() {
    let (address, _, task) = serve(Some(policy(true, false, false))).await;
    let demo = exchange(address, "GET", "/demo", "").await;
    assert_eq!(
        (demo.status, error_code(&demo).as_str()),
        (400, "missing_integer")
    );
    for path in [
        "/demo",
        "/static/js/demo.js",
        "/playground",
        "/__hmn/admin/console",
        "/other",
    ] {
        let reply = exchange(address, "GET", path, "Content-Length: 0\r\n").await;
        assert_eq!(
            (reply.status, error_code(&reply).as_str()),
            (409, "route_inventory"),
            "{path}"
        );
    }
    // The exact authority binding still applies to mounted surfaces.
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket
        .write_all(b"GET /pass HTTP/1.1\r\nHost: attacker.test\r\n\r\n")
        .await
        .unwrap();
    let mut bytes = Vec::new();
    socket.read_to_end(&mut bytes).await.unwrap();
    assert!(bytes.starts_with(b"HTTP/1.1 403"));
    task.abort();
}

#[test]
fn configuration_refuses_ambiguous_or_incomplete_legacy_policy() {
    assert!(
        configuration(Some(policy(true, true, true)))
            .validate()
            .is_ok()
    );
    let mut relative = policy(true, false, false);
    relative.web_dir = PathBuf::from("web");
    assert!(configuration(Some(relative)).validate().is_err());
    assert!(
        configuration(Some(policy(false, false, false)))
            .validate()
            .is_err()
    );
    let mut colliding = configuration(Some(policy(true, false, false)));
    colliding.routes[0].path = "/api/pass/solve".into();
    assert!(colliding.validate().is_err());
    let mut disabled = configuration(Some(policy(false, true, false)));
    disabled.routes[0].path = "/api/pass/solve".into();
    assert!(disabled.validate().is_ok(), "only mounted paths collide");
    let mut empty = policy(true, false, false);
    empty.web_dir = web_dir().join("js");
    assert!(
        Application::new(
            configuration(Some(empty)),
            Default::default(),
            Box::new(Audit::default())
        )
        .is_err(),
        "a web directory without pass.html refuses startup"
    );
    let parsed: Result<LegacyPolicy, _> =
        serde_json::from_str(r#"{"web_dir":"/w","pass":true,"console":true}"#);
    assert!(parsed.is_err(), "the console cannot be mounted here");
}

#[test]
fn pass_context_uses_the_peer_and_the_unavailable_ja4_form() {
    assert_eq!(ja4_stable(None), "");
    assert_eq!(
        scoring::fingerprint(&ja4_stable(None), "127.0.0.1"),
        "|127.0.0"
    );
    let hello = ClientHello::default();
    let stable = ja4_stable(Some(&hello));
    let (full, _) = ja4(&hello);
    assert_eq!(stable.matches('_').count(), 1);
    assert!(full.starts_with(&format!("{stable}_")));
}

#[test]
fn memory_reports_are_bounded() {
    let reports = MemoryReports::new(1);
    reports.ensure("a");
    reports.ensure("b");
    let mut scored = SessionReport::default();
    scored.scoring.risk_score = 42.0;
    reports.store_scored("b", scored.clone());
    assert_eq!(reports.get("b").scoring.risk_score, 0.0);
    reports.store_scored("a", scored);
    assert_eq!(reports.get("a").scoring.risk_score, 42.0);
}
