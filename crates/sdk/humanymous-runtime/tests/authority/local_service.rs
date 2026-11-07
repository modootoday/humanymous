//! Actual native generic-transport conformance, not a qualified admin product.
mod support;
use humanymous_runtime::authority::{
    Rejection,
    credentials::Credential,
    decision::NativePeer,
    protocol,
    transport::{Limits, Listener, LocalContext, LocalHandler, LocalRequest, LocalResponse},
};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use support::*;
use tokio::io::AsyncWriteExt;

fn frame(token: &str, extra: &str) -> Vec<u8> {
    format!("GET /management/status HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: 0\r\nConnection: close\r\n{extra}\r\n").into_bytes()
}
async fn roundtrip(endpoint: &str, wire: &[u8]) -> protocol::Response {
    let mut stream = connect(endpoint).await;
    stream.write_all(wire).await.unwrap();
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        protocol::read_response(&mut stream),
    )
    .await
    .unwrap()
    .unwrap();
    #[cfg(windows)]
    stream.assume_flushed();
    drop(stream);
    // Policy assertions are separate from admission tests. Give the actual
    // peer-close event a reactor turn; a response does not mean its server
    // connection task has already released the capacity permit.
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    response
}
struct ManagementFixture {
    token: Credential,
    expected: NativePeer,
}
impl LocalHandler for ManagementFixture {
    async fn handle(
        &self,
        request: LocalRequest,
        context: LocalContext,
    ) -> Result<LocalResponse, Rejection> {
        context.check()?;
        if context.peer() != self.expected {
            return Err(Rejection::new(403, "fixture_peer"));
        }
        if !self.token.authenticates(&request.authorization) {
            return Err(Rejection::new(401, "fixture_credential"));
        }
        if request.method != "GET" || request.target != "/management/status" {
            return Err(Rejection::new(404, "fixture_route"));
        }
        if request
            .headers
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case("origin"))
        {
            return Err(Rejection::new(403, "fixture_origin"));
        }
        LocalResponse::json(
            200,
            &serde_json::json!({"transport":"native","admin_qualified":false}),
        )
    }
}
#[tokio::test]
async fn native_handler_separates_peer_credentials_routes_and_origin_policy() {
    let d = Directory::new();
    let c = config(&d);
    let token = Credential::generate().unwrap();
    let secret = token.expose_for_provisioning().to_owned();
    let decision = Credential::generate().unwrap();
    let listener = Listener::bind_ipc(&c.endpoint, &c.peer).unwrap();
    let server = tokio::spawn(listener.serve_handler(
        ManagementFixture {
            token,
            expected: peer(),
        },
        Limits {
            capacity: 2,
            rate_per_second: 100,
            timeout_ms: 1000,
        },
    ));
    assert_eq!(
        roundtrip(&c.endpoint, &frame(&secret, "")).await.status,
        200
    );
    assert_eq!(
        roundtrip(&c.endpoint, &frame(decision.expose_for_provisioning(), ""))
            .await
            .status,
        401
    );
    assert_eq!(
        roundtrip(
            &c.endpoint,
            &frame(
                &secret,
                "Origin: https://foreign.invalid\r\nOrigin: null\r\n"
            )
        )
        .await
        .status,
        403
    );
    let wrong = String::from_utf8(frame(&secret, ""))
        .unwrap()
        .replace("/management/status", "/adapter/v0.1/decision");
    assert_eq!(roundtrip(&c.endpoint, wrong.as_bytes()).await.status, 404);
    server.abort();
    let _ = server.await;
}

struct BlockingFixture {
    gate: Arc<(Mutex<bool>, Condvar)>,
    started: Arc<tokio::sync::Notify>,
    finished: Arc<tokio::sync::Notify>,
    effects: Arc<AtomicUsize>,
}
impl LocalHandler for BlockingFixture {
    async fn handle(
        &self,
        _request: LocalRequest,
        context: LocalContext,
    ) -> Result<LocalResponse, Rejection> {
        let gate = self.gate.clone();
        let started = self.started.clone();
        let finished = self.finished.clone();
        let effects = self.effects.clone();
        tokio::task::spawn_blocking(move || {
            started.notify_one();
            let (lock, wake) = &*gate;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = wake.wait(released).unwrap();
            }
            drop(released);
            let result = context.check();
            if result.is_ok() {
                effects.fetch_add(1, Ordering::SeqCst);
            }
            finished.notify_one();
            result?;
            LocalResponse::json(200, &serde_json::json!({"fixture":true}))
        })
        .await
        .unwrap()
    }
}
#[tokio::test]
async fn cancelled_blocking_work_retains_admission_and_cannot_act() {
    let d = Directory::new();
    let c = config(&d);
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let started = Arc::new(tokio::sync::Notify::new());
    let finished = Arc::new(tokio::sync::Notify::new());
    let effects = Arc::new(AtomicUsize::new(0));
    let listener = Listener::bind_ipc(&c.endpoint, &c.peer).unwrap();
    let server = tokio::spawn(listener.serve_handler(
        BlockingFixture {
            gate: gate.clone(),
            started: started.clone(),
            finished: finished.clone(),
            effects: effects.clone(),
        },
        Limits {
            capacity: 1,
            rate_per_second: 100,
            timeout_ms: 1000,
        },
    ));
    let wire = frame(&"A".repeat(64), "");
    let mut abandoned = connect(&c.endpoint).await;
    abandoned.write_all(&wire).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), started.notified())
        .await
        .unwrap();
    drop(abandoned);
    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    assert_eq!(roundtrip(&c.endpoint, &wire).await.status, 429);
    {
        let (lock, wake) = &*gate;
        *lock.lock().unwrap() = true;
        wake.notify_all();
    }
    tokio::time::timeout(std::time::Duration::from_secs(2), finished.notified())
        .await
        .unwrap();
    assert_eq!(effects.load(Ordering::SeqCst), 0);
    // Worker/context really ended, so a new request can acquire the sole slot.
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    assert_eq!(roundtrip(&c.endpoint, &wire).await.status, 200);
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn explicit_stop_waits_for_blocking_context_and_never_reports_early_drain() {
    let d = Directory::new();
    let c = config(&d);
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let started = Arc::new(tokio::sync::Notify::new());
    let finished = Arc::new(tokio::sync::Notify::new());
    let effects = Arc::new(AtomicUsize::new(0));
    let listener = Listener::bind_ipc(&c.endpoint, &c.peer).unwrap();
    let (stop, receive) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(listener.serve_handler_until(
        BlockingFixture {
            gate: gate.clone(),
            started: started.clone(),
            finished,
            effects: effects.clone(),
        },
        Limits {
            capacity: 1,
            rate_per_second: 100,
            timeout_ms: 1000,
        },
        async {
            let _ = receive.await;
        },
        1000,
    ));
    let mut stream = connect(&c.endpoint).await;
    stream.write_all(&frame(&"A".repeat(64), "")).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), started.notified())
        .await
        .unwrap();
    stop.send(()).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    assert!(
        !server.is_finished(),
        "live detached context must prevent graceful acknowledgement"
    );
    {
        let (lock, wake) = &*gate;
        *lock.lock().unwrap() = true;
        wake.notify_all();
    }
    tokio::time::timeout(std::time::Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(effects.load(Ordering::SeqCst), 0);
    #[cfg(windows)]
    stream.assume_flushed();
}
