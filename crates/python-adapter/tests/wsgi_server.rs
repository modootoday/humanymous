//! Actual hosted Waitress/Flask/Django HTTP pilot; no authored Python source.
#![cfg(all(feature = "host-pilot", unix))]
#[path = "../../framework-adapter/tests/support/authority.rs"]
mod authority_fixture;
#[path = "../../node-adapter/tests/support/http.rs"]
mod http;
#[path = "support/wsgi_authority.rs"]
mod wsgi_authority;
#[path = "support/wsgi_host.rs"]
mod wsgi_host;
use authority_fixture::audit_log;
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    process::{Command, Stdio},
    time::Duration,
};
use wsgi_authority::{Gate, Running};
use wsgi_host::{Directory, Host, configuration, executable, host, request, stop};

#[test]
#[ignore = "mandatory actual hosted Waitress Flask/Django plus production authority process"]
fn production_authority_nonallow_is_audited_before_origin() {
    use std::io::{BufRead, BufReader};
    for framework in ["flask", "django"] {
        let directory = Directory::new();
        let config = configuration(&directory, "python-production-pilot");
        let mut authority = Host(
            Command::new(executable("HUMANYMOUS_AUTHORITY_BIN"))
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(fs::File::create(directory.0.join("authority-stderr.private.log")).unwrap())
                .spawn()
                .unwrap(),
        );
        authority
            .0
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&config).unwrap())
            .unwrap();
        let stdout = authority.0.stdout.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let reader = std::thread::spawn(move || {
            let mut line = String::new();
            BufReader::new(stdout)
                .take(65537)
                .read_line(&mut line)
                .unwrap();
            let _ = tx.send(line);
        });
        let ready: Value = serde_json::from_str(
            &rx.recv_timeout(Duration::from_secs(15))
                .expect("production authority readiness deadline"),
        )
        .unwrap();
        reader.join().unwrap();
        let (server, port) = host(
            &directory,
            framework,
            config["endpoint"].as_str().unwrap(),
            ready["credential"].as_str().unwrap(),
        );
        let wire = request(port, "/api", &[0, 255, 42], "");
        assert!(wire.starts_with(b"HTTP/1.1 409") || wire.starts_with(b"HTTP/1.1 403"));
        assert!(!directory.0.join("effects.jsonl").exists());
        stop(server, port);
        drop(authority);
        assert_eq!(
            audit_log(&directory.0, "python-production-pilot", false)
                .records()
                .unwrap()
                .len(),
            1
        );
        println!(
            "{}",
            json!({"framework":framework,"group":"production-authority-nonallow","cases":1,"auditRecords":1,"originEffects":0,"wholeProductQualified":false})
        );
    }
}

#[test]
#[ignore = "mandatory actual hosted Waitress Flask/Django with real engine synthetic evidence and durable audit"]
fn actual_allow_preserves_binary_and_two_cookies() {
    for framework in ["flask", "django"] {
        let directory = Directory::new();
        let engine = Running::start(&directory, "python-synthetic-pilot", None);
        let (server, port) = host(&directory, framework, &engine.endpoint, &engine.credential);
        let body = [0, 255, 13, 10, 42];
        let wire = request(port, "/api?q=1", &body, "");
        assert!(wire.starts_with(b"HTTP/1.1 201"));
        assert_eq!(http::body(&wire), body);
        let split = wire.windows(4).position(|p| p == b"\r\n\r\n").unwrap();
        let head = std::str::from_utf8(&wire[..split])
            .unwrap()
            .to_ascii_lowercase();
        assert_eq!(
            head.lines()
                .filter(|l| l.starts_with("set-cookie:"))
                .count(),
            2
        );
        let effects = fs::read_to_string(directory.0.join("effects.jsonl")).unwrap();
        let rows: Vec<Value> = effects
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["bodyLength"], body.len());
        use sha2::{Digest, Sha256};
        assert_eq!(rows[0]["bodySha256"], hex::encode(Sha256::digest(body)));
        for target in ["/unknown", "/_humanymous/admin"] {
            assert!(request(port, target, &body, "").starts_with(b"HTTP/1.1 503"));
        }
        let oversized = request(port, "/api", &vec![b'x'; 1025], "");
        assert!(oversized.starts_with(b"HTTP/1.1 500") || oversized.starts_with(b"HTTP/1.1 503"));
        assert!(
            request(
                port,
                "/api",
                &body,
                "Upgrade: websocket\r\nConnection: Upgrade\r\n"
            )
            .starts_with(b"HTTP/1.1 503")
        );
        assert_eq!(
            fs::read_to_string(directory.0.join("effects.jsonl")).unwrap(),
            effects
        );
        let observations = engine.observed.lock().unwrap();
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].mode, "wsgi-normalized");
        assert!(
            observations[0]
                .headers
                .iter()
                .any(|[name, value]| name == "content-length" && value == "5")
        );
        drop(observations);
        stop(server, port);
        let records = engine.finish(&directory, "python-synthetic-pilot");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].verdict, "ALLOW");
        println!(
            "{}",
            json!({"framework":framework,"group":"synthetic-engine-allow-and-invalid","cases":5,"auditRecords":1,"originEffects":1,"negativeCases":4,"wholeProductQualified":false,"productionBrowserEvidence":false})
        );
    }
}

#[test]
#[ignore = "mandatory actual hosted Waitress Flask/Django dependency-down and rejected-route requests"]
fn dependency_down_and_invalid_routes_never_reach_origin() {
    for framework in ["flask", "django"] {
        let directory = Directory::new();
        let endpoint = directory.0.join("missing.sock");
        let (server, port) = host(
            &directory,
            framework,
            endpoint.to_str().unwrap(),
            &"0".repeat(32),
        );
        for target in ["/api", "/unknown", "/_humanymous/admin"] {
            assert!(request(port, target, &[0, 255], "").starts_with(b"HTTP/1.1 503"));
        }
        assert!(!directory.0.join("effects.jsonl").exists());
        stop(server, port);
        println!(
            "{}",
            json!({"framework":framework,"group":"dependency-down-and-routes","cases":3,"originEffects":0,"wholeProductQualified":false})
        );
    }
}

#[test]
#[ignore = "mandatory actual hosted Waitress Flask/Django body abort and server input deadline with available authority"]
fn incomplete_bodies_never_reach_available_authority_or_origin() {
    use std::net::{Shutdown, TcpStream};
    use std::time::Instant;
    for framework in ["flask", "django"] {
        let directory = Directory::new();
        let engine = Running::start(&directory, "python-incomplete-pilot", None);
        let (server, port) = host(&directory, framework, &engine.endpoint, &engine.credential);
        let partial = b"POST /api HTTP/1.1\r\nHost: example.test\r\nContent-Length: 5\r\n\r\nx";
        let mut aborted = TcpStream::connect(("127.0.0.1", port)).unwrap();
        aborted.write_all(partial).unwrap();
        aborted.shutdown(Shutdown::Both).unwrap();
        drop(aborted);
        let mut stalled = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stalled
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stalled.write_all(partial).unwrap();
        let began = Instant::now();
        let mut byte = [0];
        match stalled.read(&mut byte) {
            Ok(0) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::UnexpectedEof
                ) => {}
            _ => panic!("server must close an incomplete request rather than leave input pending"),
        }
        assert!(began.elapsed() < Duration::from_secs(5));
        drop(stalled);
        assert!(engine.observed.lock().unwrap().is_empty());
        stop(server, port);
        assert!(!directory.0.join("effects.jsonl").exists());
        assert!(
            engine
                .finish(&directory, "python-incomplete-pilot")
                .is_empty()
        );
        println!(
            "{}",
            json!({"framework":framework,"group":"incomplete-body","cases":2,"auditRecords":0,"originEffects":0,"wholeProductQualified":false})
        );
    }
}

#[test]
#[ignore = "mandatory actual hosted Waitress Flask/Django disconnect before released real-engine ALLOW"]
fn disconnected_client_cannot_reach_origin_after_allow() {
    use std::{
        net::{Shutdown, TcpStream},
        sync::mpsc,
    };
    for framework in ["flask", "django"] {
        for disconnect in [false, true] {
            let directory = Directory::new();
            let (entered_tx, entered_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let engine = Running::start(
                &directory,
                "python-disconnected-pilot",
                Some(Gate {
                    entered: entered_tx,
                    release: release_rx,
                }),
            );
            let (server, port) = host(&directory, framework, &engine.endpoint, &engine.credential);
            let mut socket = TcpStream::connect(("127.0.0.1", port)).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let began = std::time::Instant::now();
            socket
                .write_all(
                    b"POST /api HTTP/1.1\r\nHost: example.test\r\nContent-Length: 1\r\n\r\nx",
                )
                .unwrap();
            entered_rx
                .recv_timeout(Duration::from_secs(3))
                .expect("actual HTTP request must reach gated authority");
            if disconnect {
                socket.shutdown(Shutdown::Both).unwrap();
            }
            // Matched connected control uses the same gate and release delay.
            std::thread::sleep(Duration::from_millis(150));
            release_tx.send(()).unwrap();
            while engine.observed.lock().unwrap().is_empty() {
                assert!(
                    began.elapsed() < Duration::from_millis(700),
                    "released evidence must precede the request deadline"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            if disconnect {
                // Keep the process alive through the whole handoff budget.
                std::thread::sleep(Duration::from_millis(1100));
            } else {
                assert!(
                    http::receive(&mut socket)
                        .unwrap()
                        .starts_with(b"HTTP/1.1 201")
                );
            }
            drop(socket);
            stop(server, port);
            let effect_file = directory.0.join("effects.jsonl");
            if disconnect {
                assert!(!effect_file.exists());
            } else {
                assert_eq!(fs::read_to_string(effect_file).unwrap().lines().count(), 1);
            }
            let records = engine.finish(&directory, "python-disconnected-pilot");
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].verdict, "ALLOW");
            println!(
                "{}",
                json!({"framework":framework,"group":"disconnect-before-handoff","disconnected":disconnect,"cases":1,"auditRecords":1,"originEffects":if disconnect{0}else{1},"wholeProductQualified":false,"productionBrowserEvidence":false})
            );
        }
    }
}
#[test]
#[ignore = "mandatory actual hosted Waitress Flask/Django accepted IPC response timeout"]
fn authority_response_deadline_closes_without_origin() {
    use std::{
        os::unix::net::UnixListener,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
    };
    struct Stalled {
        stop: Arc<AtomicBool>,
        worker: Option<std::thread::JoinHandle<()>>,
    }
    impl Stalled {
        fn finish(mut self) {
            self.stop.store(true, Ordering::Release);
            self.worker
                .take()
                .unwrap()
                .join()
                .expect("owned stalled IPC worker must join");
        }
    }
    impl Drop for Stalled {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }
    for framework in ["flask", "django"] {
        let directory = Directory::new();
        let endpoint = directory.0.join("stalled.sock");
        let listener = UnixListener::bind(&endpoint).unwrap();
        listener.set_nonblocking(true).unwrap();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_worker = Arc::clone(&stop_flag);
        let (accepted_tx, accepted_rx) = mpsc::channel();
        let stalled = Stalled {
            stop: stop_flag,
            worker: Some(std::thread::spawn(move || {
                let began = std::time::Instant::now();
                while !stop_worker.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((mut socket, _)) => {
                            socket
                                .set_read_timeout(Some(Duration::from_secs(2)))
                                .unwrap();
                            let mut byte = [0];
                            assert_eq!(
                                socket.read(&mut byte).unwrap(),
                                1,
                                "actual IPC request must be sent before timeout"
                            );
                            accepted_tx.send(()).unwrap();
                            while !stop_worker.load(Ordering::Acquire) {
                                std::thread::sleep(Duration::from_millis(10));
                            }
                            return;
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(error) => panic!("owned stalled IPC listener: {error}"),
                    }
                    assert!(began.elapsed() < Duration::from_secs(20));
                    std::thread::sleep(Duration::from_millis(10));
                }
            })),
        };
        let (server, port) = host(
            &directory,
            framework,
            endpoint.to_str().unwrap(),
            &"0".repeat(32),
        );
        let began = std::time::Instant::now();
        assert!(request(port, "/api", &[0, 255], "").starts_with(b"HTTP/1.1 503"));
        assert!(began.elapsed() < Duration::from_secs(3));
        accepted_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("timeout must follow an accepted actual IPC request");
        stop(server, port);
        stalled.finish();
        assert!(!directory.0.join("effects.jsonl").exists());
        println!(
            "{}",
            json!({"framework":framework,"group":"authority-response-timeout","cases":1,"originEffects":0,"wholeProductQualified":false})
        );
    }
}
