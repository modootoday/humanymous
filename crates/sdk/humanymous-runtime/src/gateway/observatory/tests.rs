//! Ports of legacy internal/livefeed/hub_test.go and cmd/server
//! playground_test.go / launch_parity_test.go at baseline 981d25c.
use super::*;
use std::{
    collections::BTreeSet,
    path::Path,
    sync::{Mutex as StdMutex, mpsc},
};

// --- hub --------------------------------------------------------------------

#[test]
fn publish_delivers_in_order() {
    let hub = Hub::new(16, 16);
    let (_, subscription) = hub.subscribe(0);
    hub.publish("session.scored", &json!({"n": 1}));
    hub.publish("session.scored", &json!({"n": 2}));
    let ids: Vec<u64> = (0..2).map(|_| subscription.recv().unwrap().id).collect();
    assert_eq!(ids, [1, 2]);
}

#[test]
fn publish_never_blocks_and_counts_drops() {
    let hub = Hub::new(1024, 2);
    let (_, subscription) = hub.subscribe(0);
    for i in 0..100 {
        hub.publish("x", &json!({"i": i}));
    }
    assert_eq!(subscription.lagged() + 2, 100);
}

#[test]
fn backlog_replays_after_last_id_and_the_ring_is_bounded() {
    let hub = Hub::new(4, 16);
    for i in 0..10 {
        hub.publish("x", &json!({"i": i}));
    }
    let (backlog, _first) = hub.subscribe(0);
    assert_eq!(
        backlog.iter().map(|event| event.id).collect::<Vec<_>>(),
        [7, 8, 9, 10]
    );
    let (backlog, _second) = hub.subscribe(8);
    assert_eq!(
        backlog.iter().map(|event| event.id).collect::<Vec<_>>(),
        [9, 10]
    );
    assert_eq!(hub.last_id(), 10);
}

#[test]
fn no_event_is_both_replayed_and_streamed() {
    let hub = Hub::new(256, 256);
    let publishers: Vec<_> = (0..4)
        .map(|_| {
            let hub = hub.clone();
            thread::spawn(move || {
                for i in 0..50 {
                    hub.publish("x", &json!({"i": i}));
                }
            })
        })
        .collect();
    let (backlog, subscription) = hub.subscribe(0);
    for publisher in publishers {
        publisher.join().unwrap();
    }
    subscription.cancel();
    let mut seen = BTreeSet::new();
    for event in backlog {
        assert!(seen.insert(event.id), "duplicate {}", event.id);
    }
    while let Some(event) = subscription.try_recv() {
        assert!(
            seen.insert(event.id),
            "id {} in backlog and stream",
            event.id
        );
    }
    assert_eq!(seen.len(), 200, "no event lost between backlog and stream");
}

#[test]
fn cancel_stops_delivery_and_is_idempotent() {
    let hub = Hub::new(16, 16);
    let (_, subscription) = hub.subscribe(0);
    assert_eq!(hub.subscribers(), 1);
    subscription.cancel();
    assert_eq!(hub.subscribers(), 0);
    hub.publish("x", &json!({"i": 1}));
    assert!(subscription.recv().is_none());
    subscription.cancel();
    let (_, dropped) = hub.subscribe(0);
    drop(dropped);
    assert_eq!(hub.subscribers(), 0);
}

#[test]
fn event_payload_shape() {
    let hub = Hub::new(8, 8);
    let event = hub.publish("session.scored", &json!({"verdict": "DENY", "risk": 67}));
    assert_eq!((event.kind.as_str(), event.id), ("session.scored", 1));
    let payload: Value = serde_json::from_str(&event.data).unwrap();
    assert_eq!(payload["verdict"], "DENY");
    assert_eq!(
        serde_json::to_value(&event).unwrap(),
        json!({"id": 1, "type": "session.scored", "data": {"verdict": "DENY", "risk": 67}})
    );
    assert_eq!(
        sse_frame(&event),
        "id: 1\nevent: session.scored\ndata: {\"risk\":67,\"verdict\":\"DENY\"}\n\n"
    );
}

// --- playground -------------------------------------------------------------

#[derive(Default)]
struct Store(StdMutex<BTreeMap<String, SessionReport>>);
impl Sessions for Store {
    fn report(&self, session: &str) -> Option<SessionReport> {
        self.0.lock().unwrap().get(session).cloned()
    }
}

fn observatory_with(runner: ProfileRunner, reset: Arc<dyn Fn() + Send + Sync>) -> Observatory {
    Observatory::new(
        Some("1"),
        "127.0.0.1:8443",
        PathBuf::from("/srv/web"),
        Arc::new(Engine::default()),
        Arc::new(Store::default()),
        runner,
        reset,
    )
    .unwrap()
    .unwrap()
}

fn observatory() -> Observatory {
    observatory_with(Arc::new(|_, _, _| Ok("{}".into())), Arc::new(|| {}))
}

fn get<'a>(path: &'a str, host: &'a str) -> Request<'a> {
    Request {
        method: "GET",
        path,
        host,
        ..Request::default()
    }
}

fn launch<'a>(body: &'a str) -> Request<'a> {
    Request {
        method: "POST",
        path: "/playground/launch",
        host: "127.0.0.1:8443",
        body: body.as_bytes(),
        ..Request::default()
    }
}

fn body_text(response: &Response) -> String {
    String::from_utf8(response.bytes().unwrap().to_vec()).unwrap()
}

#[test]
fn playground_gate_off_and_off_loopback() {
    let build = |enable: Option<&str>, listen: &str| {
        Observatory::new(
            enable,
            listen,
            PathBuf::new(),
            Arc::new(Engine::default()),
            Arc::new(Store::default()),
            Arc::new(|_, _, _| Ok(String::new())),
            Arc::new(|| {}),
        )
    };
    assert!(build(None, "127.0.0.1:8443").unwrap().is_none());
    assert!(build(Some("true"), "127.0.0.1:8443").unwrap().is_none());
    assert!(build(Some("1"), ":8443").is_err());
    assert!(build(Some("1"), "0.0.0.0:8443").is_err());
    assert!(
        observatory()
            .handle(&get("/api/collect", "127.0.0.1"))
            .is_none()
    );
}

#[test]
fn playground_meta_has_policy_bands_and_anti_embed_headers() {
    let response = observatory()
        .handle(&get("/playground/meta", "127.0.0.1:8443"))
        .unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.header("X-Frame-Options"), Some("DENY"));
    assert_eq!(
        response.header("Content-Security-Policy"),
        Some("frame-ancestors 'none'")
    );
    assert_eq!(response.header("Cache-Control"), Some("no-store"));
    let meta: Value = serde_json::from_slice(response.bytes().unwrap()).unwrap();
    assert_eq!(meta["policy"]["denyAt"], 70.0);
    assert_eq!(meta["policy"]["challengeAt"], 30.0);
    assert_eq!(meta["policy"]["layerCap"], 60.0);
    assert_eq!(meta["bands"].as_array().unwrap().len(), 3);
    assert_eq!(meta["ruleCount"], 24);
    assert_eq!(meta["stages"].as_array().unwrap().len(), 7);
}

#[test]
fn playground_rejects_non_loopback_host() {
    let response = observatory()
        .handle(&get("/playground/meta", "attacker.example.com"))
        .unwrap();
    assert_eq!(response.status, 403);
}

#[test]
fn playground_page_is_served_from_the_web_directory() {
    let response = observatory()
        .handle(&get("/playground", "localhost"))
        .unwrap();
    assert!(
        matches!(&response.body, Body::File(path) if path == Path::new("/srv/web/playground.html"))
    );
    assert_eq!(response.header("X-Frame-Options"), Some("DENY"));
}

/// An SSE client that disconnects after it has seen event 3 and a heartbeat.
#[derive(Clone, Default)]
struct Client(Arc<StdMutex<Vec<u8>>>);
impl Client {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}
impl io::Write for Client {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let text = self.text();
        if text.contains("id: 3\n") && text.contains(": ping\n\n") {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn playground_sse_streams_retry_backlog_live_events_and_heartbeats() {
    let observatory = observatory();
    observatory.hub().publish("a", &json!({"n": 1}));
    observatory.hub().publish("b", &json!({"n": 2}));
    let response = observatory
        .handle(&Request {
            last_event_id: Some("1"),
            ..get("/playground/events", "localhost:8443")
        })
        .unwrap();
    assert_eq!(response.header("Content-Type"), Some("text/event-stream"));
    let Body::Events(stream) = response.body else {
        panic!("expected an event stream");
    };
    assert_eq!(stream.backlog().len(), 1);
    let hub = observatory.hub().clone();
    let client = Client::default();
    let sink = client.clone();
    let writer = thread::spawn(move || {
        let mut out = sink;
        stream.serve(&mut out, Duration::from_millis(20))
    });
    hub.publish("c", &json!({"n": 3}));
    // The client hangs up once it has the live event and a heartbeat; the
    // failed write ends the stream and unregisters the subscriber.
    let error = writer.join().unwrap().unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(hub.subscribers(), 0);
    let text = client.text();
    assert!(text.starts_with("retry: 3000\n\n"), "{text}");
    assert!(text.contains("id: 2\nevent: b\n"), "{text}");
    assert!(!text.contains("id: 1\n"), "{text}");
    assert!(text.contains("id: 3\nevent: c\n"), "{text}");
    assert!(text.contains(": ping\n\n"), "{text}");
}

#[test]
fn sse_last_id_query_fallback() {
    let observatory = observatory();
    for _ in 0..3 {
        observatory.hub().publish("x", &json!({}));
    }
    let response = observatory
        .handle(&Request {
            query: Some("lastId=2"),
            ..get("/playground/events", "localhost")
        })
        .unwrap();
    let Body::Events(stream) = response.body else {
        panic!("expected an event stream");
    };
    assert_eq!(
        stream.backlog().iter().map(|e| e.id).collect::<Vec<_>>(),
        [3]
    );
}

#[test]
fn tap_a_publishes_a_scored_session() {
    let observatory = observatory();
    let (_, subscription) = observatory.hub().subscribe(0);
    let mut report = SessionReport::default();
    Engine::default().score(&mut report).unwrap();
    observatory.publish_scored("sid-1", None, &report);
    let event = subscription.recv().unwrap();
    assert_eq!(event.kind, "session.scored");
    let payload: Value = serde_json::from_str(&event.data).unwrap();
    assert_eq!(payload["sessionId"], "sid-1");
    assert_eq!(payload["source"], "organic");
    assert!(
        !payload["report"]["scoring"]["verdict"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    assert_eq!(live_source(Some("human.mjs")), "profile:human.mjs");
}

#[test]
fn launch_nonce_is_single_use_and_required() {
    let observatory = observatory();
    let nonce = observatory.nonces().issue().unwrap();
    assert_eq!(nonce.len(), 32);
    assert!(observatory.nonces().consume(&nonce));
    assert!(!observatory.nonces().consume(&nonce));
    assert!(!observatory.nonces().consume(""));
    let response = observatory
        .handle(&launch(r#"{"profileId":"human.mjs"}"#))
        .unwrap();
    assert_eq!(response.status, 403);
    let nonce = observatory
        .handle(&get("/playground/nonce", "127.0.0.1"))
        .unwrap();
    let nonce: Value = serde_json::from_slice(nonce.bytes().unwrap()).unwrap();
    assert!(
        observatory
            .nonces()
            .consume(nonce["nonce"].as_str().unwrap())
    );
}

#[test]
fn launch_rejects_target_override_unknown_profile_method_and_origin() {
    let observatory = observatory();
    let override_ = observatory
        .handle(&launch(
            r#"{"profileId":"human.mjs","host":"evil.com","nonce":"x"}"#,
        ))
        .unwrap();
    assert_eq!(override_.status, 400);
    for key in ["url", "upstream"] {
        let body = format!(r#"{{"profileId":"human.mjs","{key}":"x","nonce":"x"}}"#);
        assert_eq!(observatory.handle(&launch(&body)).unwrap().status, 400);
    }
    for profile in ["evil.mjs", "_driver.mjs"] {
        let nonce = observatory.nonces().issue().unwrap();
        let body = format!(r#"{{"profileId":"{profile}","nonce":"{nonce}"}}"#);
        assert_eq!(observatory.handle(&launch(&body)).unwrap().status, 400);
    }
    let mut get_launch = launch("{}");
    get_launch.method = "GET";
    assert_eq!(observatory.handle(&get_launch).unwrap().status, 405);
    let mut foreign = launch("{}");
    foreign.origin = Some("https://evil.example");
    assert_eq!(observatory.handle(&foreign).unwrap().status, 403);
    assert_eq!(observatory.handle(&launch("not json")).unwrap().status, 400);
    let huge = format!(r#"{{"profileId":"{}"}}"#, "a".repeat(MAX_LAUNCH_BODY));
    assert_eq!(observatory.handle(&launch(&huge)).unwrap().status, 400);
}

#[test]
fn launch_rejects_docker_only_profiles_without_running() {
    let called = Arc::new(AtomicBool::new(false));
    let flag = called.clone();
    let observatory = observatory_with(
        Arc::new(move |_, _, _| {
            flag.store(true, Ordering::SeqCst);
            Ok(String::new())
        }),
        Arc::new(|| {}),
    );
    for profile in DOCKER_ONLY_PROFILES {
        let nonce = observatory.nonces().issue().unwrap();
        let body = format!(r#"{{"profileId":"{profile}","nonce":"{nonce}"}}"#);
        let response = observatory.handle(&launch(&body)).unwrap();
        assert_eq!(response.status, 409, "{profile}");
        assert!(body_text(&response).contains("Docker-only"));
    }
    thread::sleep(Duration::from_millis(20));
    assert!(!called.load(Ordering::SeqCst));
}

#[test]
fn launches_are_serialized() {
    let (release, hold) = mpsc::channel::<()>();
    let hold = Arc::new(StdMutex::new(hold));
    let observatory = observatory_with(
        Arc::new(move |_, _, _| {
            let _ = hold.lock().unwrap().recv();
            Ok("{}".into())
        }),
        Arc::new(|| {}),
    );
    let (_, subscription) = observatory.hub().subscribe(0);
    let nonce = observatory.nonces().issue().unwrap();
    let first = observatory
        .handle(&launch(&format!(
            r#"{{"profileId":"human.mjs","nonce":"{nonce}"}}"#
        )))
        .unwrap();
    assert_eq!(first.status, 200);
    let nonce = observatory.nonces().issue().unwrap();
    let second = observatory
        .handle(&launch(&format!(
            r#"{{"profileId":"selenium.mjs","nonce":"{nonce}"}}"#
        )))
        .unwrap();
    assert_eq!(second.status, 429);
    release.send(()).unwrap();
    while subscription.recv().unwrap().kind != "attack.completed" {}
}

#[test]
fn accepted_launch_resets_state_targets_loopback_and_publishes_lifecycle() {
    let seen = Arc::new(StdMutex::new(Vec::new()));
    let log = seen.clone();
    let reset = Arc::new(AtomicBool::new(false));
    let was_reset = reset.clone();
    let observatory = observatory_with(
        Arc::new(move |profile, base, _| {
            log.lock()
                .unwrap()
                .push((profile.to_owned(), base.to_owned()));
            if profile == "human.mjs" {
                Ok(r#"{"verdict":"ALLOW","riskScore":10}"#.into())
            } else {
                Err(("Cannot find module".into(), "exit 1".into()))
            }
        }),
        Arc::new(move || was_reset.store(true, Ordering::SeqCst)),
    );
    let (_, subscription) = observatory.hub().subscribe(0);
    let nonce = observatory.nonces().issue().unwrap();
    let response = observatory
        .handle(&launch(&format!(
            r#"{{"profileId":"human.mjs","runs":9,"nonce":"{nonce}"}}"#
        )))
        .unwrap();
    assert_eq!(response.status, 200);
    let accepted: Value = serde_json::from_slice(response.bytes().unwrap()).unwrap();
    assert_eq!(
        accepted,
        json!({"accepted": true, "profileId": "human.mjs", "runs": 5})
    );
    assert!(reset.load(Ordering::SeqCst));
    let mut kinds = Vec::new();
    loop {
        let event = subscription.recv().unwrap();
        kinds.push(event.kind.clone());
        if event.kind == "attack.completed" {
            break;
        }
    }
    assert_eq!(kinds.first().map(String::as_str), Some("attack.launched"));
    assert_eq!(kinds.iter().filter(|kind| *kind == "attack.ran").count(), 5);
    assert!(
        seen.lock()
            .unwrap()
            .iter()
            .all(|(profile, base)| profile == "human.mjs" && base == LAUNCH_BASE)
    );
    // A failing run is reported, classified, and the launcher frees its slot.
    let nonce = observatory.nonces().issue().unwrap();
    let response = observatory
        .handle(&launch(&format!(
            r#"{{"profileId":"selenium.mjs","nonce":"{nonce}"}}"#
        )))
        .unwrap();
    assert_eq!(response.status, 200);
    loop {
        let event = subscription.recv().unwrap();
        if event.kind == "attack.error" {
            let payload: Value = serde_json::from_str(&event.data).unwrap();
            assert_eq!(payload["skipped"], true);
        }
        if event.kind == "attack.completed" {
            break;
        }
    }
}

#[test]
fn abort_cancels_remaining_runs() {
    let (started, wait) = mpsc::channel::<()>();
    let started = Arc::new(StdMutex::new(started));
    let observatory = observatory_with(
        Arc::new(move |_, _, control: &RunControl| {
            let _ = started.lock().unwrap().send(());
            while !control.cancelled() {
                thread::yield_now();
            }
            Err((String::new(), "aborted".into()))
        }),
        Arc::new(|| {}),
    );
    let (_, subscription) = observatory.hub().subscribe(0);
    let nonce = observatory.nonces().issue().unwrap();
    observatory
        .handle(&launch(&format!(
            r#"{{"profileId":"human.mjs","runs":3,"nonce":"{nonce}"}}"#
        )))
        .unwrap();
    wait.recv().unwrap();
    let abort = observatory
        .handle(&Request {
            method: "POST",
            ..get("/playground/abort", "127.0.0.1")
        })
        .unwrap();
    assert_eq!(body_text(&abort), r#"{"aborted":true}"#);
    let mut kinds = Vec::new();
    loop {
        let event = subscription.recv().unwrap();
        kinds.push(event.kind.clone());
        if event.kind == "attack.completed" {
            break;
        }
    }
    assert!(kinds.contains(&"attack.aborted".to_owned()), "{kinds:?}");
    assert_eq!(
        kinds.iter().filter(|kind| *kind == "attack.error").count(),
        1
    );
}

#[test]
fn explain_returns_the_trace_for_a_stored_session() {
    let store = Arc::new(Store::default());
    store
        .0
        .lock()
        .unwrap()
        .insert("sid".into(), SessionReport::default());
    let observatory = Observatory::new(
        Some("1"),
        "localhost:8443",
        PathBuf::new(),
        Arc::new(Engine::default()),
        store.clone(),
        Arc::new(|_, _, _| Ok(String::new())),
        Arc::new(|| {}),
    )
    .unwrap()
    .unwrap();
    let response = observatory
        .handle(&get("/playground/explain/sid", "127.0.0.1"))
        .unwrap();
    assert_eq!(response.status, 200);
    let trace: Value = serde_json::from_slice(response.bytes().unwrap()).unwrap();
    assert!(trace.is_object());
    // The stored report is not rescored in place.
    assert_eq!(store.report("sid"), Some(SessionReport::default()));
    let missing = observatory
        .handle(&get("/playground/explain/none", "127.0.0.1"))
        .unwrap();
    assert_eq!(missing.status, 404);
}

#[test]
fn listen_addr_is_loopback_cases() {
    for (address, want) in [
        (":8443", false),
        ("0.0.0.0:8443", false),
        ("127.0.0.1:8443", true),
        ("localhost:8443", true),
        ("[::1]:8443", true),
        ("192.168.1.10:844", false),
    ] {
        assert_eq!(listen_addr_is_loopback(address), want, "{address}");
    }
}

#[test]
fn host_is_loopback_cases() {
    for (host, want) in [
        ("127.0.0.1:8443", true),
        ("localhost:8443", true),
        ("localhost", true),
        ("[::1]:8443", true),
        ("::1", true),
        ("attacker.example.com", false),
        ("10.0.0.5:8443", false),
        ("localhost.evil.com", false),
    ] {
        assert_eq!(host_is_loopback(host), want, "{host}");
    }
    assert!(origin_is_loopback("https://127.0.0.1:8443"));
    assert!(origin_is_loopback("http://localhost"));
    assert!(!origin_is_loopback("https://evil.example"));
    assert!(!origin_is_loopback("null"));
    assert!(!origin_is_loopback("https://127.0.0.1@evil.example"));
}

// --- catalog parity (PLAN-07 R5) -------------------------------------------

fn repository_file(relative: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    std::fs::read_to_string(root.join(relative)).unwrap()
}

/// Quoted `<prefix>[a-z0-9_]+<suffix>` literals, in order.
fn literals(source: &str, prefix: &str, suffix: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find(prefix) {
        rest = &rest[start + prefix.len()..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
            .collect();
        if !name.is_empty() && rest[name.len()..].starts_with(suffix) {
            found.push(name);
        }
    }
    found
}

fn launch_set() -> BTreeSet<String> {
    LAUNCH_PROFILES
        .iter()
        .map(|name| (*name).to_owned())
        .collect()
}

#[test]
fn launch_profiles_match_catalog() {
    let catalog: BTreeSet<String> = literals(&repository_file("test/e2e/runner.mjs"), "'", ".mjs'")
        .into_iter()
        .map(|name| format!("{name}.mjs"))
        .collect();
    assert!(!catalog.is_empty(), "parsed zero catalog profiles");
    assert_eq!(catalog, launch_set());
    assert_eq!(
        LAUNCH_PROFILES.len(),
        launch_set().len(),
        "duplicate profile"
    );
}

#[test]
fn observatory_catalog_matches_launch_profiles() {
    let page: BTreeSet<String> = literals(&repository_file("web/playground.html"), "[\"", ".mjs\"")
        .into_iter()
        .map(|name| format!("{name}.mjs"))
        .collect();
    assert!(!page.is_empty(), "parsed zero observatory profiles");
    assert_eq!(page, launch_set());
}

#[test]
fn docker_only_profiles_match_observatory_and_never_overlap_the_launcher() {
    let page = literals(
        &repository_file("web/playground.html"),
        "{id:\"",
        "\",executionKind:\"docker-external-input\"",
    );
    assert_eq!(page, DOCKER_ONLY_PROFILES);
    for profile in DOCKER_ONLY_PROFILES {
        assert!(!LAUNCH_PROFILES.contains(profile));
    }
}

#[test]
fn missing_dependency_classification() {
    assert!(looks_like_missing_dependency(
        "Error: Cannot find module 'x'",
        ""
    ));
    assert!(looks_like_missing_dependency(
        "",
        "exec: \"node\": executable file not found"
    ));
    assert!(looks_like_missing_dependency("connect ECONNREFUSED", ""));
    assert!(!looks_like_missing_dependency(
        "verdict DENY",
        "exit status 1"
    ));
}
