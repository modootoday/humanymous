//! Served-path port of legacy internal/gate/integration_test.go
//! `TestProxyEnforcementAndAudit` at baseline 981d25c: a scored bot beacon,
//! the bot blocked at the edge without origin contact, and the sealed audit
//! chain verified at the end. Every exchange goes through
//! `Application::serve` on a real loopback listener with a counting origin,
//! and the application appends to a real sealed log over a write-ahead log.
//!
//! Deliberate differences from Go, each asserted here:
//! - The beacon is answered `COLLECTED`, not `DENY` with a sticky verdict
//!   cookie: the verdict is scored on the next protected request of the same
//!   request-integrity session (the initial profile refuses every cookie).
//! - The block is a `runtime.decision` record with verdict `DENY` and a 403
//!   `{"verdict":"DENY"}` body, not Go's `enforcement.deny` record and
//!   `http.Error` text.
//! - HTML injection is not composed on the served path: a human's allowed
//!   HTML response is forwarded byte for byte, without the loader snippet
//!   (the ledger row records this gap).
use super::*;
use crate::audit::log::{Config as LogConfig, Log};
use crate::audit::storage::{Wal, WalOptions};
use crate::audit::verification::Mismatch;
use std::path::PathBuf;

/// The legacy origin page.
const PAGE: &[u8] = b"<html><head><title>x</title></head><body>ORIGIN</body></html>";

/// Scratch directory removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "humanymous-gate-integration-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Ok(entries) = std::fs::read_dir(&self.0) {
            for entry in entries.flatten() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        let _ = std::fs::remove_dir(&self.0);
    }
}

/// Durable sink over a shared sealed log, like Go `audit.NewSink(alog)`.
struct Sealed(Arc<Mutex<Log>>);

impl DurableAudit for Sealed {
    fn append(&mut self, record: Record) -> std::io::Result<()> {
        self.0.lock().unwrap().append(record).map(|_| ())
    }
}

/// Go `buildStack`'s log: node `test`, HMAC key `k`, a head every four.
fn sealed_log(dir: &std::path::Path) -> Arc<Mutex<Log>> {
    let log = Log::open(
        Wal::open(dir, WalOptions::default()).unwrap(),
        LogConfig {
            node_id: "test".into(),
            hmac_key: b"k".to_vec(),
            signing_seed: [3; 32],
            checkpoint_every: 4,
            witness: None,
        },
        None,
        true,
    )
    .unwrap();
    Arc::new(Mutex::new(log))
}

/// Go's bot beacon: webdriver and a headless User-Agent token, no motor data.
fn bot_report(id: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"sessionId":id,"userAgent":"HeadlessChrome/126",
        "environment":{"probed":true},"behavior":{"mouse":{"samples":0},"events":{"totalEvents":0}},
        "signals":[
            {"id":"l1.navigator.webdriver","value":true,"verdict":"BOT","confidence":0.95,"collected":"wasm"},
            {"id":"l1.ua.headless_token","value":true,"verdict":"BOT","confidence":0.95,"collected":"wasm"}]}))
    .unwrap()
}

async fn served(
    origin: std::net::SocketAddr,
    log: Arc<Mutex<Log>>,
) -> (
    std::net::SocketAddr,
    tokio::task::JoinHandle<std::io::Result<()>>,
) {
    let app = Application::new(
        configuration(origin),
        Default::default(),
        Box::new(Sealed(log)),
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    (address, tokio::spawn(app.serve(listener)))
}

fn beacon(cfg: &Configuration, issued: &Value, body: Vec<u8>, n: u64, now: u64) -> Request {
    let mut req = request("POST", "/_humanymous/collect", body);
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

/// Go TestProxyEnforcementAndAudit: the bot's beacon is scored, its next
/// request is blocked at the edge with the origin untouched, a human's HTML
/// passes through, and the sealed chain verifies with the DENY decision in it.
#[tokio::test]
async fn bot_beacon_is_blocked_at_the_edge_and_the_chain_verifies() {
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    let origin_task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = origin.accept().await.unwrap();
            let _ = crate::gateway::http::read(&mut socket).await.unwrap();
            counter.fetch_add(1, Ordering::SeqCst);
            let mut response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                PAGE.len()
            )
            .into_bytes();
            response.extend_from_slice(PAGE);
            socket.write_all(&response).await.unwrap();
        }
    });
    let scratch = Scratch::new();
    let log = sealed_log(&scratch.0);
    let cfg = configuration(origin_addr);
    let (address, server) = served(origin_addr, log.clone()).await;
    let now = crate::authority::now_ms().unwrap();

    // The bot's beacon is collected; its next request is scored DENY and
    // blocked at the edge without contacting the origin.
    let bot = json_body(&exchange(address, &request("POST", "/_humanymous/session", vec![])).await);
    let collected = exchange(
        address,
        &beacon(
            &cfg,
            &bot,
            bot_report(bot["session"].as_str().unwrap()),
            1,
            now,
        ),
    )
    .await;
    assert!(collected.starts_with(b"HTTP/1.1 200"));
    let blocked = exchange(address, &protected(&cfg, &bot, 2, now)).await;
    assert!(
        blocked.starts_with(b"HTTP/1.1 403"),
        "{}",
        String::from_utf8_lossy(&blocked)
    );
    assert_eq!(json_body(&blocked)["verdict"], "DENY");
    assert_eq!(hits.load(Ordering::SeqCst), 0);

    // A human session's HTML is forwarded byte for byte (no injection).
    let human =
        json_body(&exchange(address, &request("POST", "/_humanymous/session", vec![])).await);
    let report = client_report(human["session"].as_str().unwrap());
    assert!(
        exchange(address, &beacon(&cfg, &human, report, 1, now))
            .await
            .starts_with(b"HTTP/1.1 200")
    );
    let allowed = exchange(address, &protected(&cfg, &human, 2, now)).await;
    assert!(allowed.starts_with(b"HTTP/1.1 200"));
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    let end = allowed.windows(4).position(|b| b == b"\r\n\r\n").unwrap();
    assert_eq!(&allowed[end + 4..], PAGE);
    assert!(
        !allowed
            .windows(crate::gateway::inject::MARKER.len())
            .any(|b| b == crate::gateway::inject::MARKER.as_bytes())
    );
    server.abort();
    origin_task.abort();

    // The headline invariant: the sealed chain verifies and holds the DENY.
    let log = log.lock().unwrap();
    let report = log.self_verify();
    assert!(report.ok, "{report:?}");
    assert_eq!(report.class, Mismatch::Ok);
    let records = log.records().unwrap();
    assert_eq!(records.len(), 4);
    assert!(
        records
            .iter()
            .any(|r| r.event_type == "runtime.decision" && r.verdict == "DENY")
    );
    assert!(
        records
            .iter()
            .any(|r| r.event_type == "runtime.decision" && r.verdict == "ALLOW")
    );
    assert!(!log.checkpoints().unwrap().is_empty());
}
