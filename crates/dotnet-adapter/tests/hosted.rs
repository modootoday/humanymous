//! Explicit allocated real Kestrel/IPC/GC pilot; not in the default unit suite.
#[path = "hosted/generic.rs"]
mod generic;
#[cfg(test)]
#[path = "hosted/launch.rs"]
mod launch;
#[cfg(test)]
#[path = "hosted/protocol.rs"]
mod protocol;
mod support;
use humanymous_authority::{
    configuration::Configuration,
    decision::{ApplicationHttp, Authority, DurableAudit, Evidence},
    transport::Listener,
};
use humanymous_test_support::process::ManagedChild;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpStream},
    path::PathBuf,
    process::{ChildStdin, Command},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

struct Host {
    process: ManagedChild,
    input: ChildStdin,
    frames: mpsc::Receiver<Value>,
}
impl Host {
    fn start() -> Self {
        let path = PathBuf::from(
            std::env::var_os("HUMANYMOUS_DOTNET_PILOT").expect("published actual host required"),
        );
        assert!(path.is_file());
        let (process, input, output) = ManagedChild::spawn_duplex(&mut Command::new(path)).unwrap();
        let (send, frames) = mpsc::sync_channel(64);
        std::thread::spawn(move || {
            let mut output = BufReader::new(output);
            loop {
                let mut bytes = Vec::new();
                if output
                    .by_ref()
                    .take(4097)
                    .read_until(b'\n', &mut bytes)
                    .is_err()
                    || bytes.is_empty()
                    || bytes.len() > 4096
                    || bytes.last() != Some(&b'\n')
                {
                    break;
                }
                let Ok(frame) = serde_json::from_slice(&bytes) else {
                    break;
                };
                if send.send(frame).is_err() {
                    break;
                }
            }
        });
        let host = Self {
            process,
            input,
            frames,
        };
        let first = host.frame("starting");
        assert_eq!(first["pid"], host.process.id());
        host
    }
    fn frame(&self, kind: &str) -> Value {
        for _ in 0..64 {
            let frame = self
                .frames
                .recv_timeout(Duration::from_secs(15))
                .expect("bounded actual host frame");
            if frame["kind"] == kind {
                return frame;
            }
            if frame["kind"] == "generic-native-abi" {
                assert_eq!(frame["passed"], 1);
                println!(
                    "{}",
                    json!({"profile":"generic-native-abi","passed":1,"failed":0,"case":frame["case"],"full_product":false})
                );
                continue;
            }
            assert_eq!(
                frame["kind"], "application-contact",
                "unexpected host event"
            );
        }
        panic!("host event budget exceeded")
    }
    fn send(&mut self, value: Value) {
        serde_json::to_writer(&mut self.input, &value).unwrap();
        self.input.write_all(b"\n").unwrap();
        self.input.flush().unwrap();
    }
    fn counters(&mut self) -> Value {
        self.send(json!({"operation":"counters"}));
        self.frame("counters")
    }
}
struct Synthetic {
    gc_entered: mpsc::SyncSender<()>,
    gc_release: mpsc::Receiver<()>,
}
impl Evidence for Synthetic {
    fn report(
        &mut self,
        request: &humanymous_integrity::adapter::ValidatedRequest,
        _: bool,
    ) -> Result<humanymous_signals::report::SessionReport, humanymous_authority::Rejection> {
        if matches!(request.request_target().1, "/gc-await" | "/abort-await") {
            self.gc_entered.send(()).expect("pilot GC gate observer");
            self.gc_release
                .recv_timeout(Duration::from_millis(1500))
                .expect("bounded pilot GC gate");
        }
        let text = match request.request_target().1.split('?').next().unwrap_or("") {
            "/deny" => {
                include_str!("../../scoring/tests/fixtures/frozen/bot-webdriver-cdp-hr9.json")
            }
            "/challenge" => include_str!("../../scoring/tests/fixtures/frozen/go-http-client.json"),
            _ => include_str!("../../scoring/tests/fixtures/frozen/human-chrome-allow.json"),
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
    fn append(&mut self, record: humanymous_audit::record::Record) -> std::io::Result<()> {
        let verdict = record.verdict.clone();
        self.inner.append(record)?;
        *self.counts.lock().unwrap().entry(verdict).or_default() += 1;
        Ok(())
    }
}
fn socket(port: u16) -> TcpStream {
    let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(8)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(8)))
        .unwrap();
    stream
}
fn request(port: u16, method: &str, path: &str, body: &[u8], chunked: bool) -> Vec<u8> {
    try_request(port, method, path, body, chunked).unwrap()
}
fn try_request(
    port: u16,
    method: &str,
    path: &str,
    body: &[u8],
    chunked: bool,
) -> std::io::Result<Vec<u8>> {
    let mut stream = socket(port);
    let framing = if chunked {
        "Transfer-Encoding: chunked\r\n".into()
    } else {
        format!("Content-Length: {}\r\n", body.len())
    };
    write!(stream,"{method} {path} HTTP/1.1\r\nHost: example.test\r\nConnection: close\r\n{framing}Authorization: Bearer application-fixture\r\nX-Forwarded-For: forged\r\n\r\n").unwrap();
    if chunked {
        write!(stream, "{:x}\r\n", body.len()).unwrap();
        stream.write_all(body).unwrap();
        stream.write_all(b"\r\n0\r\n\r\n").unwrap();
    } else {
        stream.write_all(body).unwrap();
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
fn negative_receive(stream: &mut TcpStream, case: &str) {
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
fn actual_kestrel_engine_audit_body_gc_and_shutdown() {
    let execution_environment = support::execution_environment();
    let root = PathBuf::from(
        std::env::var_os("HUMANYMOUS_DOTNET_AUDIT_DIR")
            .expect("supervisor-provisioned private fixture directory"),
    );
    for synthetic in [false, true] {
        let mut host = Host::start();
        let profile = if synthetic { "synthetic" } else { "production" };
        #[cfg(windows)]
        let directory =
            humanymous_native_security::storage::create_user_private_directory(&root, profile)
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
            let sid = humanymous_native_security::process::current_identity()
                .unwrap()
                .sid()
                .to_owned();
            (
                format!(
                    r"\\.\pipe\humanymous-dotnet-{}-{}",
                    std::process::id(),
                    host.process.id()
                ),
                json!({"kind":"windows","sid":sid,"pid":host.process.id()}),
                json!({"kind":"windows","sid":sid,"pid":std::process::id()}),
            )
        };
        #[cfg(unix)]
        let (endpoint, peer, server) = {
            let uid = humanymous_native_security::process::current_uid();
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
        let log = humanymous_authority::startup::open_audit(&mut config.audit, &config.peer, None)
            .unwrap();
        let counts = Arc::new(Mutex::new(std::collections::BTreeMap::new()));
        let (gc_entered_tx, gc_entered_rx) = mpsc::sync_channel(1);
        let (gc_release_tx, gc_release_rx) = mpsc::sync_channel(1);
        let evidence: Box<dyn Evidence> = if synthetic {
            Box::new(Synthetic {
                gc_entered: gc_entered_tx,
                gc_release: gc_release_rx,
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
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let listener = {
            let _entered = runtime.enter();
            Listener::bind(&serde_json::from_value(value).unwrap()).unwrap()
        };
        let mut service = Some(runtime.spawn(listener.serve(authority, 8, 1000, 2000)));
        host.send(json!({"generic_probe":std::env::var_os("HUMANYMOUS_DOTNET_GENERIC_PROBE").is_some(),"configuration":{"installation":"i","tenant":"t","origin":"o","revision":"r","endpoint":endpoint,"credential":credential,"server":server,"routes":paths,"timeoutMs":2000,"maxBody":1024}}));
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
        let first = request(port, "POST", "/api?q=1", &binary, false);
        let mut passed = 1;
        let mut covered = std::collections::BTreeSet::new();
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
                let reply = request(port, "POST", "/api", &binary, true);
                assert!(reply.starts_with(b"HTTP/1.1 200"));
                assert_eq!(body(&reply), binary);
                passed += 1;
            }
            host.send(json!({"operation":"gc-stop"}));
            assert!(host.counters()["collections"].as_u64().unwrap() > 0);
            covered.insert("unknown-length-eof");
            assert_eq!(host.counters()["native_resources"], 0);
            let pending =
                std::thread::spawn(move || request(port, "POST", "/gc-await", &binary, false));
            gc_entered_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("actual native IPC entered authority");
            let baseline = host.counters()["collections"].as_u64().unwrap();
            host.send(json!({"operation":"gc-start"}));
            let until = std::time::Instant::now() + Duration::from_secs(1);
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
                std::thread::sleep(Duration::from_millis(5));
            }
            gc_release_tx.send(()).unwrap();
            let reply = pending.join().unwrap();
            assert!(reply.starts_with(b"HTTP/1.1 200"));
            assert_eq!(body(&reply), binary);
            host.send(json!({"operation":"gc-stop"}));
            assert_eq!(host.counters()["native_resources"], 0);
            covered.insert("gc-pressure");
            passed += 1;
            let before_abort = host.counters();
            let mut pending_abort = socket(port);
            pending_abort
                .write_all(b"POST /abort-await HTTP/1.1\r\nHost: example.test\r\nContent-Length: 1\r\nConnection: close\r\n\r\nx")
                .unwrap();
            gc_entered_rx
                .recv_timeout(Duration::from_secs(1))
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
            let abort_until = std::time::Instant::now() + Duration::from_secs(1);
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
                std::thread::sleep(Duration::from_millis(5));
            }
            gc_release_tx.send(()).unwrap();
            // A following request serializes with the released authority work;
            // its sole contact proves no cancelled completion invoked next.
            let after_abort = request(port, "POST", "/api", &binary, false);
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
                let reply = request(port, "POST", path, b"", false);
                assert!(reply.starts_with(format!("HTTP/1.1 {status}").as_bytes()));
                assert_eq!(host.counters()["contacts"], before);
                passed += 1;
            }
            covered.insert("deny-challenge");
            for (mode, case) in [
                ("rewrite", "request-rewrite"),
                ("expire", "request-deadline"),
                ("response-started", "response-started"),
            ] {
                host.send(json!({"operation":"handoff-mode","mode":mode}));
                let before = host.counters()["contacts"].as_u64().unwrap();
                let reply = try_request(port, "POST", "/api", b"x", false);
                if mode != "response-started" {
                    assert!(reply.unwrap().starts_with(b"HTTP/1.1 503"));
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
            let oversized = request(port, "POST", "/api", &[7; 1025], false);
            assert!(oversized.starts_with(b"HTTP/1.1 503"));
            passed += 1;
            covered.insert("body-limit");
            let before = host.counters()["contacts"].as_u64().unwrap();
            let mut aborted = socket(port);
            aborted
                .write_all(
                    b"POST /api HTTP/1.1\r\nHost: example.test\r\nContent-Length: 1000\r\n\r\nx",
                )
                .unwrap();
            aborted.shutdown(Shutdown::Both).unwrap();
            drop(aborted);
            std::thread::sleep(Duration::from_millis(100));
            assert_eq!(host.counters()["contacts"], before);
            passed += 1;
            covered.insert("abort-during-input");
            let before = host.counters()["contacts"].as_u64().unwrap();
            let mut truncated = socket(port);
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
                let mut connection=socket(port);connection.write_all(wire).unwrap();negative_receive(&mut connection,"unsupported-upgrade");
                assert_eq!(host.counters()["contacts"],before);passed+=1;
            }
            covered.insert("unsupported-upgrade");
            for (path, status) in [("/no-content", 204), ("/not-modified", 304)] {
                let reply = request(port, "POST", path, b"", false);
                assert!(reply.starts_with(format!("HTTP/1.1 {status}").as_bytes()));
                assert!(body(&reply).is_empty());
                passed += 1;
            }
            let head = request(port, "HEAD", "/head", b"", false);
            assert!(head.starts_with(b"HTTP/1.1 200"));
            assert!(body(&head).is_empty());
            passed += 1;
            let stream = request(port, "POST", "/stream", b"", false);
            assert!(stream.starts_with(b"HTTP/1.1 200"));
            assert_eq!(chunked_entity(&stream), vec![1u8, 2]);
            passed += 1;
            covered.insert("response-semantics");
            let application_error = request(port, "POST", "/app-error", b"", false);
            assert!(application_error.starts_with(b"HTTP/1.1 500"));
            passed += 1;
            last_live_resources = host.counters()["native_resources"].clone();
            assert_eq!(last_live_resources, 0);
            let stopped_authority = service.take().unwrap();
            stopped_authority.abort();
            runtime.block_on(async {
                let result = stopped_authority.await;
                assert!(result.unwrap_err().is_cancelled());
            });
            let before_down = host.counters();
            let down_started = std::time::Instant::now();
            let unavailable = request(port, "POST", "/api", b"x", false);
            assert!(unavailable.starts_with(b"HTTP/1.1 503"));
            assert!(down_started.elapsed() < Duration::from_secs(4));
            let after_down = host.counters();
            assert_eq!(after_down["contacts"], before_down["contacts"]);
            assert_eq!(after_down["mutations"], before_down["mutations"]);
            assert_eq!(after_down["native_resources"], 0);
            covered.insert("authority-down");
            passed += 1;
            host.send(json!({"operation":"dispose-host"}));
            let before = host.counters()["contacts"].as_u64().unwrap();
            let after = request(port, "POST", "/api", b"x", false);
            assert!(after.starts_with(b"HTTP/1.1 503"));
            assert_eq!(host.counters()["contacts"], before);
            passed += 1;
        }
        let final_counts = host.counters();
        host.send(json!({"operation":"stop"}));
        host.frame("stopped");
        assert!(
            host.process
                .wait(Duration::from_secs(10))
                .unwrap()
                .success()
        );
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
        println!(
            "{}",
            json!({"schema":"humanymous.dotnet-host-observation/1","execution_environment":execution_environment,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"profile":if synthetic{"real-engine-audit-synthetic-evidence"}else{"production-application-http"},"runtime":ready["runtime"],"dynamic_code":ready["dynamic_code"],"passed":passed,"failed":0,"ignored":0,"covered_contract_cases":covered,"not_run_contract_cases":not_run,"origin_contacts":final_counts["contacts"],"origin_mutations":final_counts["mutations"],"gc_collections":final_counts["collections"],"remaining_native_resources_before_dispose":last_live_resources,"verdict_counts":*counts.lock().unwrap(),"full_product":false,"package_install":false})
        );
    }
}
