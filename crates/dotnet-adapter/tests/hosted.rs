//! Explicit allocated real Kestrel/IPC/GC pilot; not in the default unit suite.
#[path = "hosted/generic.rs"]
mod generic;
#[path = "hosted/http.rs"]
mod http;
#[path = "hosted/launch.rs"]
mod launch;
#[cfg(test)]
#[path = "hosted/protocol.rs"]
mod protocol;
#[path = "hosted/retained.rs"]
mod retained;
mod support;
use http::Socket;
use humanymous_runtime::authority::{
    configuration::Configuration,
    decision::{ApplicationHttp, Authority, DurableAudit, Evidence},
    transport::Listener,
};
use retained::Host;
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::Shutdown,
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

struct Synthetic {
    gc_entered: mpsc::SyncSender<()>,
    gc_release: mpsc::Receiver<()>,
    run_until: Instant,
}
impl Evidence for Synthetic {
    fn report(
        &mut self,
        request: &humanymous_core::integrity::adapter::ValidatedRequest,
        _: bool,
    ) -> Result<
        humanymous_core::signals::report::SessionReport,
        humanymous_runtime::authority::Rejection,
    > {
        let path = request.request_target().1;
        if matches!(path, "/gc-await" | "/abort-await" | "/hold-await") {
            self.gc_entered
                .try_send(())
                .expect("pilot GC gate observer");
            // `/hold-await` must outlive the 2000 ms host request deadline.
            let ceiling = if path == "/hold-await" { 4000 } else { 1500 };
            self.gc_release
                .recv_timeout(stage_remaining(
                    self.run_until,
                    Duration::from_millis(ceiling),
                ))
                .expect("bounded pilot GC gate");
        }
        let text = match request.request_target().1.split('?').next().unwrap_or("") {
            "/deny" => {
                include_str!(
                    "../../sdk/humanymous-core/tests/scoring/fixtures/frozen/bot-webdriver-cdp-hr9.json"
                )
            }
            "/challenge" => include_str!(
                "../../sdk/humanymous-core/tests/scoring/fixtures/frozen/go-http-client.json"
            ),
            _ => include_str!(
                "../../sdk/humanymous-core/tests/scoring/fixtures/frozen/human-chrome-allow.json"
            ),
        };
        let golden: Value = serde_json::from_str(text).unwrap();
        Ok(serde_json::from_value(golden["report"].clone()).unwrap())
    }
}
struct CountingAudit {
    inner: Box<dyn DurableAudit>,
    counts: Arc<Mutex<std::collections::BTreeMap<String, u64>>>,
}
impl DurableAudit for CountingAudit {
    fn append(&mut self, record: humanymous_runtime::audit::record::Record) -> std::io::Result<()> {
        let verdict = record.verdict.clone();
        self.inner.append(record)?;
        *self.counts.lock().unwrap().entry(verdict).or_default() += 1;
        Ok(())
    }
}
fn stage_remaining(run_until: Instant, ceiling: Duration) -> Duration {
    run_until
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .expect("original host stage budget exhausted")
        .min(ceiling)
}
/// Install a new host configuration through the bounded control line.
fn replace_host(host: &mut Host, configuration: Value) -> bool {
    let control = json!({"operation":"replace-host","configuration":configuration});
    assert!(
        serde_json::to_vec(&control).unwrap().len() < 1024,
        "replace-host control exceeds the pilot line bound"
    );
    host.send(control);
    host.frame("replaced")["ok"].as_bool().unwrap()
}
/// A request held at the authority gate; a transport error reads as no bytes.
fn hold_request(host: &mut Host, port: u16, run_until: Instant) {
    host.spawn_http(move || {
        try_request(port, "POST", "/hold-await", b"x", false, run_until).unwrap_or_default()
    })
    .unwrap();
}
/// The configured server identity with one OS fact changed: a real peer whose
/// kernel-reported uid (Linux) or pipe server PID (Windows) differs.
fn wrong_peer(server: &Value) -> Value {
    let mut wrong = server.clone();
    if server["kind"] == "unix" {
        wrong["uid"] = json!(server["uid"].as_u64().unwrap() + 1);
    } else {
        wrong["pid"] = json!(server["pid"].as_u64().unwrap() + 1);
    }
    wrong
}
fn socket(port: u16, run_until: Instant) -> Socket {
    Socket::connect(port, run_until).unwrap()
}
fn request(
    port: u16,
    method: &str,
    path: &str,
    body: &[u8],
    chunked: bool,
    run_until: Instant,
) -> Vec<u8> {
    try_request(port, method, path, body, chunked, run_until).unwrap()
}
fn try_request(
    port: u16,
    method: &str,
    path: &str,
    body: &[u8],
    chunked: bool,
    run_until: Instant,
) -> std::io::Result<Vec<u8>> {
    let mut stream = Socket::connect(port, run_until)?;
    let framing = if chunked {
        "Transfer-Encoding: chunked\r\n".into()
    } else {
        format!("Content-Length: {}\r\n", body.len())
    };
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: example.test\r\nConnection: close\r\n{framing}Authorization: Bearer application-fixture\r\nX-Forwarded-For: forged\r\n\r\n"
    )?;
    if chunked {
        write!(stream, "{:x}\r\n", body.len())?;
        stream.write_all(body)?;
        stream.write_all(b"\r\n0\r\n\r\n")?;
    } else {
        stream.write_all(body)?;
    }
    let mut output = Vec::new();
    stream.take(2_097_153).read_to_end(&mut output)?;
    assert!(output.len() <= 2_097_152);
    Ok(output)
}
fn body(wire: &[u8]) -> &[u8] {
    let i = wire.windows(4).position(|b| b == b"\r\n\r\n").unwrap();
    &wire[i + 4..]
}
fn negative_receive(stream: &mut Socket, case: &str) {
    let started = std::time::Instant::now();
    let mut bytes = Vec::new();
    let error = stream.take(65_537).read_to_end(&mut bytes).err();
    assert!(bytes.len() <= 65_536);
    if let Some(error) = &error {
        assert!(
            !matches!(
                error.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ),
            "negative case stalled"
        );
    }
    assert!(started.elapsed() < Duration::from_secs(5));
    println!(
        "{}",
        json!({"case":case,"received_bytes":bytes.len(),"elapsed_ms":started.elapsed().as_millis(),"transport_error":error.map(|e|format!("{:?}",e.kind()))})
    );
}
fn chunked_entity(wire: &[u8]) -> Vec<u8> {
    let mut input = body(wire);
    let mut output = Vec::new();
    loop {
        let line = input.windows(2).position(|b| b == b"\r\n").unwrap();
        assert!(line <= 8);
        let n = usize::from_str_radix(std::str::from_utf8(&input[..line]).unwrap(), 16).unwrap();
        input = &input[line + 2..];
        if n == 0 {
            assert_eq!(input, b"\r\n");
            return output;
        }
        assert!(n <= 1024 && input.len() >= n + 2);
        output.extend_from_slice(&input[..n]);
        assert_eq!(&input[n..n + 2], b"\r\n");
        input = &input[n + 2..];
        assert!(output.len() <= 1024);
    }
}

#[test]
#[ignore = "actual explicitly allocated Kestrel/JIT or NativeAOT, IPC and GC"]
fn actual_kestrel_engine_audit_body_gc_and_shutdown() -> Result<(), Box<Host>> {
    let execution_environment = support::execution_environment();
    for synthetic in [false, true] {
        let mut host = Host::start().expect("qualified native host admission required");
        let root = host
            .audit_root()
            .expect("admitted private audit root")
            .to_path_buf();
        let run_until = host.run_deadline();
        // Keep the actual owner OUTSIDE the assertion boundary. A failed case
        // still reaches explicit cancellation and retained settlement below.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let first = host.frame("starting");
            assert_eq!(first["pid"], host.pid());
            let profile = if synthetic { "synthetic" } else { "production" };
            #[cfg(windows)]
            let directory =
                humanymous_runtime::native_security::storage::create_user_private_directory(
                    &root, profile,
                )
                .expect("ordinary actor and verified private parent required");
            #[cfg(unix)]
            let directory = {
                use std::os::unix::fs::DirBuilderExt;
                let path = root.join(profile);
                std::fs::DirBuilder::new()
                    .mode(0o700)
                    .create(&path)
                    .expect("new private audit child required");
                path
            };
            let directory = directory.canonicalize().unwrap();
            assert!(
                std::fs::read_dir(&directory).unwrap().next().is_none(),
                "new audit fixture must be empty"
            );
            #[cfg(windows)]
            let (endpoint, peer, server) = {
                let sid = humanymous_runtime::native_security::process::current_identity()
                    .unwrap()
                    .sid()
                    .to_owned();
                (
                    format!(
                        r"\\.\pipe\humanymous-dotnet-{}-{}",
                        std::process::id(),
                        host.pid()
                    ),
                    json!({"kind":"windows","sid":sid,"pid":host.pid()}),
                    json!({"kind":"windows","sid":sid,"pid":std::process::id()}),
                )
            };
            #[cfg(unix)]
            let (endpoint, peer, server) = {
                let uid = humanymous_runtime::native_security::process::current_uid();
                (
                    directory
                        .join("decision.sock")
                        .to_string_lossy()
                        .into_owned(),
                    json!({"kind":"unix","uid":uid}),
                    json!({"kind":"unix","uid":uid}),
                )
            };
            let paths = [
                "/api",
                "/mutate",
                "/head",
                "/no-content",
                "/not-modified",
                "/stream",
                "/deny",
                "/challenge",
                "/app-error",
                "/gc-await",
                "/abort-await",
                "/hold-await",
            ];
            let mut routes = Vec::new();
            for path in paths {
                for method in ["POST", "HEAD"] {
                    routes.push(json!({"method":method,"path":path,"required":{"raw_tls":false,"original_h2":false,"raw_header_order":false,"collector":false,"rit":false}}));
                }
            }
            let value = json!({"adapter":"aspnetcore","endpoint":endpoint,"peer":peer,"scope":{"installation":"i","tenant":"t","origin":"o","revision":"r","authority":"example.test"},"routes":routes,
            "audit":{"directory":directory,"node_id":"dotnet-host-pilot","hmac_key_hex":"aa".repeat(32),"signing_seed_hex":"bb".repeat(32),"allow_new":true,"trusted_head":null},
            "capacity":8,"replay_capacity":256,"rate_per_second":1000,"timeout_ms":2000,"clock_skew_ms":1000});
            let mut config: Configuration = serde_json::from_value(value.clone()).unwrap();
            config.validate().unwrap();
            let log = humanymous_runtime::authority::startup::open_audit(
                &mut config.audit,
                &config.peer,
                None,
            )
            .unwrap();
            let counts = Arc::new(Mutex::new(std::collections::BTreeMap::new()));
            let (gc_entered_tx, gc_entered_rx) = mpsc::sync_channel(1);
            let (gc_release_tx, gc_release_rx) = mpsc::sync_channel(1);
            let evidence: Box<dyn Evidence> = if synthetic {
                Box::new(Synthetic {
                    gc_entered: gc_entered_tx,
                    gc_release: gc_release_rx,
                    run_until,
                })
            } else {
                Box::new(ApplicationHttp)
            };
            let authority = Authority::new(
                config,
                Default::default(),
                evidence,
                Box::new(CountingAudit {
                    inner: log,
                    counts: counts.clone(),
                }),
            )
            .unwrap();
            let credential = authority.credential().to_owned();
            let expiry = authority.test_expiry();
            let mut runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();
            let listener = {
                let _entered = runtime.enter();
                Listener::bind(&serde_json::from_value(value.clone()).unwrap()).unwrap()
            };
            let mut service = Some(runtime.spawn(listener.serve(authority, 8, 1000, 2000)));
            let configuration = |credential: &str, server: &Value| json!({"installation":"i","tenant":"t","origin":"o","revision":"r","endpoint":endpoint,"credential":credential,"server":server,"routes":paths,"timeoutMs":2000,"maxBody":1024});
            host.send(json!({"configuration":configuration(&credential, &server)}));
            let ready = host.frame("ready");
            if let Ok(expected) = std::env::var("HUMANYMOUS_DOTNET_EXPECT_AOT") {
                assert!(expected == "true" || expected == "false");
                assert_eq!(
                    ready["dynamic_code"],
                    expected == "false",
                    "actual runtime must match selected JIT/AOT profile"
                );
            }
            let address = ready["address"].as_str().unwrap();
            let port = address.rsplit(':').next().unwrap().parse().unwrap();
            let binary = [0, 255, 128, 13, 10, 42];
            let first = request(port, "POST", "/api?q=1", &binary, false, run_until);
            let mut passed = 1;
            let mut covered = std::collections::BTreeSet::new();
            let mut shutdown_frame: Option<Value> = None;
            let mut epoch2_verdicts = Value::Null;
            let mut timings = serde_json::Map::new();
            let mut last_live_resources = host.counters()["native_resources"].clone();
            assert_eq!(last_live_resources, 0);
            if !synthetic {
                assert!(first.starts_with(b"HTTP/1.1 403") || first.starts_with(b"HTTP/1.1 503"));
                assert_eq!(host.counters()["contacts"], 0);
            } else {
                covered.insert("binary-replay");
                covered.insert("pooled-buffer-reuse");
                assert!(first.starts_with(b"HTTP/1.1 200"));
                assert_eq!(body(&first), binary);
                assert!(
                    String::from_utf8_lossy(&first)
                        .to_ascii_lowercase()
                        .contains("x-pilot-forwarded: absent")
                );
                assert_eq!(
                    String::from_utf8_lossy(&first)
                        .to_ascii_lowercase()
                        .matches("set-cookie:")
                        .count(),
                    2
                );
                passed += 1;
                host.send(json!({"operation":"gc-start"}));
                for _ in 0..8 {
                    let reply = request(port, "POST", "/api", &binary, true, run_until);
                    assert!(reply.starts_with(b"HTTP/1.1 200"));
                    assert_eq!(body(&reply), binary);
                    passed += 1;
                }
                host.send(json!({"operation":"gc-stop"}));
                assert!(host.counters()["collections"].as_u64().unwrap() > 0);
                covered.insert("unknown-length-eof");
                assert_eq!(host.counters()["native_resources"], 0);
                host.spawn_http(move || {
                    request(port, "POST", "/gc-await", &binary, false, run_until)
                })
                .unwrap();
                gc_entered_rx
                    .recv_timeout(stage_remaining(run_until, Duration::from_secs(1)))
                    .expect("actual native IPC entered authority");
                let baseline = host.counters()["collections"].as_u64().unwrap();
                host.send(json!({"operation":"gc-start"}));
                let until = run_until.min(Instant::now() + Duration::from_secs(1));
                loop {
                    let observed = host.counters();
                    assert_eq!(
                        (observed["native_resources"].as_u64().unwrap() >> 32) & 0xffff,
                        1,
                        "native decision must still hold its lease during GC"
                    );
                    if observed["collections"].as_u64().unwrap() > baseline {
                        break;
                    }
                    assert!(
                        std::time::Instant::now() < until,
                        "managed GC did not overlap the held native call"
                    );
                    std::thread::sleep(stage_remaining(run_until, Duration::from_millis(5)));
                }
                gc_release_tx.send(()).unwrap();
                let reply = host.join_http().unwrap();
                assert!(reply.starts_with(b"HTTP/1.1 200"));
                assert_eq!(body(&reply), binary);
                host.send(json!({"operation":"gc-stop"}));
                assert_eq!(host.counters()["native_resources"], 0);
                covered.insert("gc-pressure");
                passed += 1;
                let before_abort = host.counters();
                let mut pending_abort = socket(port, run_until);
                pending_abort
                .write_all(b"POST /abort-await HTTP/1.1\r\nHost: example.test\r\nContent-Length: 1\r\nConnection: close\r\n\r\nx")
                .unwrap();
                gc_entered_rx
                    .recv_timeout(stage_remaining(run_until, Duration::from_secs(1)))
                    .expect("abort fixture must enter actual authority IPC first");
                assert_eq!(
                    (host.counters()["native_resources"].as_u64().unwrap() >> 32) & 0xffff,
                    1,
                    "abort must overlap the retained native decision lease"
                );
                pending_abort.shutdown(Shutdown::Both).unwrap();
                drop(pending_abort);
                // Keep authority evidence blocked until cancellation releases the
                // native flight, rather than racing an ALLOW against socket teardown.
                let abort_until = run_until.min(Instant::now() + Duration::from_secs(1));
                loop {
                    let observed = host.counters();
                    assert_eq!(observed["contacts"], before_abort["contacts"]);
                    assert_eq!(observed["mutations"], before_abort["mutations"]);
                    if observed["native_resources"] == 0 {
                        break;
                    }
                    assert!(
                        std::time::Instant::now() < abort_until,
                        "aborted IPC retained native resources"
                    );
                    std::thread::sleep(stage_remaining(run_until, Duration::from_millis(5)));
                }
                gc_release_tx.send(()).unwrap();
                // A following request serializes with the released authority work;
                // its sole contact proves no cancelled completion invoked next.
                let after_abort = request(port, "POST", "/api", &binary, false, run_until);
                assert!(after_abort.starts_with(b"HTTP/1.1 200"));
                assert_eq!(body(&after_abort), binary);
                assert_eq!(
                    host.counters()["contacts"].as_u64().unwrap(),
                    before_abort["contacts"].as_u64().unwrap() + 1
                );
                covered.insert("abort-during-authority");
                passed += 1;
                for (path, status) in [("/deny", 403), ("/challenge", 503)] {
                    let before = host.counters()["contacts"].as_u64().unwrap();
                    let reply = request(port, "POST", path, b"", false, run_until);
                    assert!(reply.starts_with(format!("HTTP/1.1 {status}").as_bytes()));
                    assert_eq!(host.counters()["contacts"], before);
                    passed += 1;
                }
                covered.insert("deny-challenge");
                // Each binding field the contract names is mutated on its own:
                // route, method, raw target, host and sanitized headers.
                for (mode, case) in [
                    ("rewrite", "request-rewrite"),
                    ("rewrite-method", "request-rewrite"),
                    ("rewrite-target", "request-rewrite"),
                    ("rewrite-host", "request-rewrite"),
                    ("rewrite-header", "request-rewrite"),
                    ("expire", "request-deadline"),
                    ("response-started", "response-started"),
                ] {
                    host.send(json!({"operation":"handoff-mode","mode":mode}));
                    let before = host.counters()["contacts"].as_u64().unwrap();
                    let reply = try_request(port, "POST", "/api", b"x", false, run_until);
                    if mode != "response-started" {
                        assert!(reply.unwrap().starts_with(b"HTTP/1.1 503"));
                    } else if let Ok(wire) = reply {
                        // The started response keeps its one status line: no
                        // later 503, no application header and no completed body.
                        let text = String::from_utf8_lossy(&wire).to_ascii_lowercase();
                        assert!(wire.starts_with(b"HTTP/1.1 200"));
                        assert_eq!(text.matches("http/1.1 ").count(), 1);
                        assert!(!text.contains("set-cookie:"));
                        assert!(!text.contains("x-pilot-forwarded:"));
                        assert!(!wire.ends_with(b"0\r\n\r\n"));
                    } else if let Err(error) = reply {
                        assert!(!matches!(
                            error.kind(),
                            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                        ));
                        println!(
                            "{}",
                            json!({"case":case,"expected_host_abort_read_error":format!("{:?}",error.kind())})
                        );
                    }
                    let counters = host.counters();
                    assert_eq!(counters["contacts"], before);
                    assert_eq!(counters["native_resources"], 0);
                    // This delay exercises the original request deadline, not an
                    // independently expired authenticated authority response.
                    if mode != "expire" {
                        covered.insert(case);
                    }
                    passed += 1;
                }
                host.send(json!({"operation":"handoff-mode","mode":"normal"}));
                host.counters();
                // expired-result (owner decision D4). A test authority issues an
                // authenticated ALLOW whose expiry precedes the request deadline.
                // The control run proves the same 600 ms hold passes under the
                // production expiry rule, so only authority expiry refuses it.
                host.send(json!({"operation":"handoff-mode","mode":"hold"}));
                for (ttl, allowed) in [(0, true), (300, false)] {
                    expiry.set(ttl);
                    let before = host.counters()["contacts"].as_u64().unwrap();
                    let reply = request(port, "POST", "/api", b"x", false, run_until);
                    let counters = host.counters();
                    if allowed {
                        assert!(
                            reply.starts_with(b"HTTP/1.1 200"),
                            "production expiry must admit the hold"
                        );
                        assert_eq!(counters["contacts"].as_u64().unwrap(), before + 1);
                    } else {
                        assert!(
                            reply.starts_with(b"HTTP/1.1 503"),
                            "an expired authority result must refuse"
                        );
                        assert_eq!(counters["contacts"], before);
                    }
                    assert_eq!(counters["native_resources"], 0);
                    passed += 1;
                }
                expiry.set(0);
                host.send(json!({"operation":"handoff-mode","mode":"normal"}));
                covered.insert("expired-result");
                // drop-pending-decision. The authority holds its decision past
                // the 2000 ms host deadline; the host must drop the request
                // while the decision is still pending.
                let before = host.counters();
                hold_request(&mut host, port, run_until);
                gc_entered_rx
                    .recv_timeout(stage_remaining(run_until, Duration::from_secs(1)))
                    .expect("held decision must enter actual authority IPC");
                let held = Instant::now();
                let dropped = host
                    .http_finished_by(held + Duration::from_millis(3000))
                    .unwrap();
                if !dropped {
                    gc_release_tx.send(()).unwrap();
                }
                assert!(
                    dropped,
                    "host deadline must drop a decision the authority still holds"
                );
                let reply = host.join_http().unwrap();
                timings.insert("drop_pending_ms".into(), json!(held.elapsed().as_millis()));
                assert!(reply.starts_with(b"HTTP/1.1 503"));
                let after = host.counters();
                assert_eq!(after["contacts"], before["contacts"]);
                assert_eq!(after["mutations"], before["mutations"]);
                assert_eq!(after["native_resources"], 0);
                gc_release_tx.send(()).unwrap();
                // The next decision serializes behind the released one, so a
                // single contact proves the late completion reactivated nothing.
                let next = request(port, "POST", "/api", b"x", false, run_until);
                assert!(next.starts_with(b"HTTP/1.1 200"));
                assert_eq!(
                    host.counters()["contacts"].as_u64().unwrap(),
                    before["contacts"].as_u64().unwrap() + 1
                );
                covered.insert("drop-pending-decision");
                passed += 1;
                // wrong-peer. The host is reconfigured to expect a server
                // identity the real authority socket does not have.
                assert!(replace_host(
                    &mut host,
                    configuration(&credential, &wrong_peer(&server))
                ));
                let before = host.counters();
                let verdicts_before: u64 = counts.lock().unwrap().values().sum();
                let reply = request(port, "POST", "/api", b"x", false, run_until);
                assert!(reply.starts_with(b"HTTP/1.1 503"));
                let after = host.counters();
                assert_eq!(after["contacts"], before["contacts"]);
                assert_eq!(after["native_resources"], 0);
                assert_eq!(
                    counts.lock().unwrap().values().sum::<u64>(),
                    verdicts_before,
                    "a mismatched server peer must receive no request"
                );
                assert!(replace_host(&mut host, configuration(&credential, &server)));
                let restored = request(port, "POST", "/api", b"x", false, run_until);
                assert!(restored.starts_with(b"HTTP/1.1 200"));
                assert_eq!(
                    host.counters()["contacts"].as_u64().unwrap(),
                    before["contacts"].as_u64().unwrap() + 1
                );
                covered.insert("wrong-peer");
                passed += 1;
                // dispose-race. The installation is replaced and disposed while
                // its request holds a decision; release must not reach next, and
                // the replacement serves exactly one new request.
                let before = host.counters();
                hold_request(&mut host, port, run_until);
                gc_entered_rx
                    .recv_timeout(stage_remaining(run_until, Duration::from_secs(1)))
                    .expect("dispose-race decision must enter actual authority IPC");
                assert!(replace_host(&mut host, configuration(&credential, &server)));
                gc_release_tx.send(()).unwrap();
                let reply = host.join_http().unwrap();
                assert!(
                    reply.starts_with(b"HTTP/1.1 503"),
                    "a decision held across host disposal must not reach the application"
                );
                assert_eq!(host.counters()["contacts"], before["contacts"]);
                let fresh = request(port, "POST", "/api", b"x", false, run_until);
                assert!(fresh.starts_with(b"HTTP/1.1 200"));
                let after = host.counters();
                assert_eq!(
                    after["contacts"].as_u64().unwrap(),
                    before["contacts"].as_u64().unwrap() + 1
                );
                assert_eq!(after["native_resources"], 0);
                covered.insert("dispose-race");
                passed += 1;
                let oversized = request(port, "POST", "/api", &[7; 1025], false, run_until);
                assert!(oversized.starts_with(b"HTTP/1.1 503"));
                passed += 1;
                covered.insert("body-limit");
                let before = host.counters()["contacts"].as_u64().unwrap();
                let mut aborted = socket(port, run_until);
                aborted
                .write_all(
                    b"POST /api HTTP/1.1\r\nHost: example.test\r\nContent-Length: 1000\r\n\r\nx",
                )
                .unwrap();
                aborted.shutdown(Shutdown::Both).unwrap();
                drop(aborted);
                std::thread::sleep(stage_remaining(run_until, Duration::from_millis(100)));
                assert_eq!(host.counters()["contacts"], before);
                passed += 1;
                covered.insert("abort-during-input");
                let before = host.counters()["contacts"].as_u64().unwrap();
                let mut truncated = socket(port, run_until);
                truncated
                    .write_all(
                        b"POST /api HTTP/1.1\r\nHost: example.test\r\nContent-Length: 2\r\n\r\nx",
                    )
                    .unwrap();
                truncated.shutdown(Shutdown::Write).unwrap();
                negative_receive(&mut truncated, "truncated-body");
                assert_eq!(host.counters()["contacts"], before);
                covered.insert("truncated-body");
                passed += 1;
                for wire in [b"GET /api HTTP/1.1\r\nHost: example.test\r\nConnection: close, Upgrade\r\nUpgrade: websocket\r\n\r\n".as_slice(),b"CONNECT example.test:443 HTTP/1.1\r\nHost: example.test\r\nConnection: close\r\n\r\n".as_slice()] {
                let mut connection=socket(port,run_until);connection.write_all(wire).unwrap();negative_receive(&mut connection,"unsupported-upgrade");
                assert_eq!(host.counters()["contacts"],before);passed+=1;
            }
                covered.insert("unsupported-upgrade");
                for (path, status) in [("/no-content", 204), ("/not-modified", 304)] {
                    let reply = request(port, "POST", path, b"", false, run_until);
                    assert!(reply.starts_with(format!("HTTP/1.1 {status}").as_bytes()));
                    assert!(body(&reply).is_empty());
                    passed += 1;
                }
                let head = request(port, "HEAD", "/head", b"", false, run_until);
                assert!(head.starts_with(b"HTTP/1.1 200"));
                assert!(body(&head).is_empty());
                passed += 1;
                let stream = request(port, "POST", "/stream", b"", false, run_until);
                assert!(stream.starts_with(b"HTTP/1.1 200"));
                assert_eq!(chunked_entity(&stream), vec![1u8, 2]);
                passed += 1;
                covered.insert("response-semantics");
                let application_error = request(port, "POST", "/app-error", b"", false, run_until);
                assert!(application_error.starts_with(b"HTTP/1.1 500"));
                passed += 1;
                last_live_resources = host.counters()["native_resources"].clone();
                assert_eq!(last_live_resources, 0);
                // authority-restart. Epoch 1 dies with a decision in flight: the
                // serve task is cancelled and its runtime shut down, so no
                // connection task can still deliver that decision.
                let before_restart = host.counters();
                hold_request(&mut host, port, run_until);
                gc_entered_rx
                    .recv_timeout(stage_remaining(run_until, Duration::from_secs(1)))
                    .expect("restart decision must enter epoch 1 IPC");
                let stopped_authority = service.take().unwrap();
                stopped_authority.abort();
                runtime.block_on(async {
                    let result = stopped_authority.await;
                    assert!(result.unwrap_err().is_cancelled());
                });
                let epoch2_runtime = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .unwrap();
                let restarted = Instant::now();
                std::mem::replace(&mut runtime, epoch2_runtime)
                    .shutdown_timeout(Duration::from_millis(200));
                let held_reply = host.join_http().unwrap();
                timings.insert(
                    "restart_held_ms".into(),
                    json!(restarted.elapsed().as_millis()),
                );
                assert!(
                    held_reply.starts_with(b"HTTP/1.1 503"),
                    "an epoch 1 decision in flight must not survive its authority"
                );
                gc_release_tx.send(()).unwrap();
                assert_eq!(host.counters()["contacts"], before_restart["contacts"]);
                passed += 1;
                // authority-down, while no epoch serves the endpoint.
                let before_down = host.counters();
                let down_started = std::time::Instant::now();
                let unavailable = request(port, "POST", "/api", b"x", false, run_until);
                assert!(unavailable.starts_with(b"HTTP/1.1 503"));
                assert!(down_started.elapsed() < Duration::from_secs(4));
                let after_down = host.counters();
                assert_eq!(after_down["contacts"], before_down["contacts"]);
                assert_eq!(after_down["mutations"], before_down["mutations"]);
                assert_eq!(after_down["native_resources"], 0);
                covered.insert("authority-down");
                passed += 1;
                // Epoch 2 binds the same endpoint with a new credential and a
                // new audit chain. The epoch 1 credential must not authenticate.
                #[cfg(windows)]
                let epoch2_directory =
                    humanymous_runtime::native_security::storage::create_user_private_directory(
                        &root,
                        "synthetic-epoch2",
                    )
                    .expect("ordinary actor and verified private parent required");
                #[cfg(unix)]
                let epoch2_directory = {
                    use std::os::unix::fs::DirBuilderExt;
                    let path = root.join("synthetic-epoch2");
                    std::fs::DirBuilder::new()
                        .mode(0o700)
                        .create(&path)
                        .expect("new private epoch 2 audit child required");
                    path
                };
                let mut epoch2_value = value.clone();
                epoch2_value["audit"]["directory"] =
                    json!(epoch2_directory.canonicalize().unwrap());
                let mut epoch2_config: Configuration =
                    serde_json::from_value(epoch2_value.clone()).unwrap();
                epoch2_config.validate().unwrap();
                let epoch2_log = humanymous_runtime::authority::startup::open_audit(
                    &mut epoch2_config.audit,
                    &epoch2_config.peer,
                    None,
                )
                .unwrap();
                let epoch2_counts = Arc::new(Mutex::new(std::collections::BTreeMap::new()));
                let (hold_entered_tx, hold_entered_rx) = mpsc::sync_channel(1);
                let (hold_release_tx, hold_release_rx) = mpsc::sync_channel(1);
                let epoch2 = Authority::new(
                    epoch2_config,
                    Default::default(),
                    Box::new(Synthetic {
                        gc_entered: hold_entered_tx,
                        gc_release: hold_release_rx,
                        run_until,
                    }),
                    Box::new(CountingAudit {
                        inner: epoch2_log,
                        counts: epoch2_counts.clone(),
                    }),
                )
                .unwrap();
                let epoch2_credential = epoch2.credential().to_owned();
                assert_ne!(epoch2_credential, credential);
                let epoch2_listener = {
                    let _entered = runtime.enter();
                    Listener::bind(&serde_json::from_value(epoch2_value).unwrap())
                        .expect("epoch 2 must bind the endpoint epoch 1 released")
                };
                service = Some(runtime.spawn(epoch2_listener.serve(epoch2, 8, 1000, 2000)));
                let before = host.counters();
                let stale = request(port, "POST", "/api", b"x", false, run_until);
                assert!(
                    stale.starts_with(b"HTTP/1.1 503"),
                    "an epoch 1 credential must not authenticate to epoch 2"
                );
                assert_eq!(host.counters()["contacts"], before["contacts"]);
                assert!(epoch2_counts.lock().unwrap().is_empty());
                assert!(replace_host(
                    &mut host,
                    configuration(&epoch2_credential, &server)
                ));
                let renewed = request(port, "POST", "/api", b"x", false, run_until);
                assert!(renewed.starts_with(b"HTTP/1.1 200"));
                assert_eq!(
                    host.counters()["contacts"].as_u64().unwrap(),
                    before["contacts"].as_u64().unwrap() + 1
                );
                assert_eq!(epoch2_counts.lock().unwrap().values().sum::<u64>(), 1);
                covered.insert("authority-restart");
                passed += 1;
                host.send(json!({"operation":"dispose-host"}));
                let before = host.counters()["contacts"].as_u64().unwrap();
                let after = request(port, "POST", "/api", b"x", false, run_until);
                assert!(after.starts_with(b"HTTP/1.1 503"));
                assert_eq!(host.counters()["contacts"], before);
                passed += 1;
                assert!(replace_host(
                    &mut host,
                    configuration(&epoch2_credential, &server)
                ));
                // The process-lived profile refuses a collectible load twice and
                // claims no native runtime unload.
                host.send(json!({"operation":"collectible-probe"}));
                let probe = host.frame("collectible");
                assert_eq!(probe["rejected"], true);
                assert_eq!(
                    probe["reason"],
                    if ready["dynamic_code"] == true {
                        "collectible-context"
                    } else {
                        "no-dynamic-loading"
                    }
                );
                passed += 1;
                // shutdown-active. Stop arrives with one decision held at the
                // epoch 2 authority and one request still awaiting body input.
                let before = host.counters();
                assert_eq!(before["native_resources"], 0);
                last_live_resources = before["native_resources"].clone();
                hold_request(&mut host, port, run_until);
                hold_entered_rx
                    .recv_timeout(stage_remaining(run_until, Duration::from_secs(1)))
                    .expect("shutdown decision must enter epoch 2 IPC");
                let mut pending_input = socket(port, run_until);
                pending_input
                    .write_all(b"POST /api HTTP/1.1\r\nHost: example.test\r\nContent-Length: 10\r\nConnection: close\r\n\r\nx")
                    .unwrap();
                std::thread::sleep(stage_remaining(run_until, Duration::from_millis(50)));
                let stop_started = Instant::now();
                host.send(json!({"operation":"stop"}));
                let stopped = host.frame("stopped");
                timings.insert(
                    "shutdown_ms".into(),
                    json!(stop_started.elapsed().as_millis()),
                );
                hold_release_tx.send(()).unwrap();
                assert_eq!(stopped["contacts"], before["contacts"]);
                assert_eq!(stopped["mutations"], before["mutations"]);
                let held = host.join_http().unwrap();
                assert!(
                    !held.starts_with(b"HTTP/1.1 2"),
                    "a decision pending at shutdown must not complete"
                );
                let mut tail = Vec::new();
                let _ = pending_input.take(65_537).read_to_end(&mut tail);
                assert!(!tail.starts_with(b"HTTP/1.1 2"));
                covered.insert("shutdown-active");
                passed += 1;
                epoch2_verdicts = json!(*epoch2_counts.lock().unwrap());
                shutdown_frame = Some(stopped);
            }
            let final_counts = match shutdown_frame {
                Some(stopped) => stopped,
                None => {
                    let counters = host.counters();
                    host.send(json!({"operation":"stop"}));
                    host.frame("stopped");
                    counters
                }
            };
            if let Some(service) = service {
                runtime.block_on(async {
                    service.abort();
                    let _ = service.await;
                });
            }
            drop(runtime);
            let contract: Value =
                serde_json::from_str(include_str!("../../../adapters/dotnet/pilot-contract.json"))
                    .unwrap();
            let not_run: Vec<_> = contract["hostCases"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c["id"].as_str().unwrap())
                .filter(|id| !covered.contains(id))
                .collect();
            json!({"schema":"humanymous.dotnet-host-observation/1","execution_environment":execution_environment,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"profile":if synthetic{"real-engine-audit-synthetic-evidence"}else{"production-application-http"},"runtime":ready["runtime"],"dynamic_code":ready["dynamic_code"],"passed":passed,"failed":0,"ignored":0,"covered_contract_cases":covered,"not_run_contract_cases":not_run,"origin_contacts":final_counts["contacts"],"origin_mutations":final_counts["mutations"],"gc_collections":final_counts["collections"],"remaining_native_resources_before_dispose":last_live_resources,"verdict_counts":*counts.lock().unwrap(),"epoch2_verdict_counts":epoch2_verdicts,"timings_ms":timings,"native_runtime_unload_claimed":false,"full_product":false,"package_install":false})
        }));
        if outcome.is_err() {
            host.cancel();
            let _ = host.request_stop();
        }
        // Unknown cleanup returns the actual owner, never a success observation.
        let settled = host.finish()?;
        if let Err(payload) = outcome {
            std::panic::resume_unwind(payload);
        }
        let progress = settled.progress();
        assert!(settled.status().success());
        assert!(progress.issues.is_empty());
        assert_eq!(progress.workers_started, 3);
        assert_eq!(progress.workers_completed, 3);
        assert_eq!(progress.workers_joined, 3);
        assert!(progress.stdout_eof && progress.stderr_eof && progress.input_drained);
        assert_eq!(
            progress.input_messages_accepted,
            progress.input_messages_written
        );
        if let Ok(observation) = outcome {
            println!("{observation}");
        }
    }
    Ok(())
}
