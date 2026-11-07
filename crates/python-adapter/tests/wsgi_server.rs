//! Actual hosted Waitress/Flask/Django HTTP pilot; no authored Python source.
#![cfg(all(feature = "host-pilot", unix))]
#[path = "../../framework-adapter/tests/support/authority.rs"]
mod authority_fixture;
#[path = "../../node-adapter/tests/support/http.rs"]
mod http;
use authority_fixture::{SyntheticEvidence, audit_log};
use humanymous_authority::{
    configuration::Configuration, decision::Authority, transport::Listener,
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        assert_eq!(
            std::env::var("HUMANYMOUS_HOSTED_NATIVE_PILOT").as_deref(),
            Ok("1")
        );
        let path = std::env::temp_dir().join(format!(
            "hpy-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
use std::os::unix::fs::DirBuilderExt;
struct Host(Child);
impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn executable(variable: &str) -> PathBuf {
    let path =
        PathBuf::from(std::env::var_os(variable).expect("provisioned actual executable required"));
    assert!(
        path.is_absolute() && path.is_file(),
        "absolute provisioned executable required"
    );
    path
}
fn configuration(directory: &Directory, node: &str) -> Value {
    json!({"adapter":"python-wsgi","endpoint":directory.0.join("authority.sock"),
        "peer":{"kind":"unix","uid":humanymous_native_security::process::current_uid()},
        "scope":{"installation":"i","tenant":"t","origin":"o","revision":"r","authority":"example.test"},
        "routes":[{"method":"POST","path":"/api","required":{"raw_tls":false,"original_h2":false,"raw_header_order":false,"collector":false,"rit":false}}],
        "audit":{"directory":directory.0,"node_id":node,"hmac_key_hex":"aa".repeat(32),"signing_seed_hex":"bb".repeat(32),"allow_new":true,"trusted_head":null},
        "capacity":8,"replay_capacity":32,"rate_per_second":100,"timeout_ms":2000,"clock_skew_ms":1000})
}
fn host(directory: &Directory, framework: &str, endpoint: &str, credential: &str) -> (Host, u16) {
    let port = TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let module = PathBuf::from(
        std::env::var_os("HUMANYMOUS_PYTHON_MODULE_DIR")
            .expect("actual host-pilot extension directory required"),
    );
    assert!(module.is_absolute() && module.is_dir());
    let mut child = Host(
        Command::new(executable("HUMANYMOUS_WAITRESS_SERVE"))
            .args([
                "--host=127.0.0.1",
                &format!("--port={port}"),
                "--channel-request-lookahead=1",
                "--channel-timeout=2",
                "--cleanup-interval=1",
                "--max-request-body-size=2048",
                "--call",
                "humanymous_python:pilot_wsgi",
            ])
            .env("PYTHONPATH", module)
            .env("PYTHONNOUSERSITE", "1")
            .env("HUMANYMOUS_HOSTED_NATIVE_PILOT", "1")
            .env_remove("PYTHONSTARTUP")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(fs::File::create(directory.0.join("server-stderr.private.log")).unwrap())
            .spawn()
            .expect("actual Waitress process required"),
    );
    let options = json!({"installation":"i","tenant":"t","origin":"o","revision":"r", "endpoint":endpoint,"credential":credential,
        "server":{"kind":"unix","uid":humanymous_native_security::process::current_uid()},"routes":["/api"],"timeoutMs":1000,"maxBody":1024,"syncInputDeadlineEnforced":true});
    child.0.stdin.take().unwrap().write_all(&serde_json::to_vec(&json!({"framework":framework,"configuration":options.to_string(),"effect_file":directory.0.join("effects.jsonl")})).unwrap()).unwrap();
    let began = Instant::now();
    loop {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "actual WSGI server exited before readiness; inspect private log"
        );
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        assert!(
            began.elapsed() < Duration::from_secs(15),
            "actual WSGI server readiness deadline"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    (child, port)
}
fn request(port: u16, target: &str, body: &[u8], extra: &str) -> Vec<u8> {
    let mut socket = TcpStream::connect(("127.0.0.1", port)).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(4)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(4)))
        .unwrap();
    write!(socket, "POST {target} HTTP/1.1\r\nHost: example.test\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\n{extra}Connection: close\r\n\r\n", body.len()).unwrap();
    socket.write_all(body).unwrap();
    http::receive(&mut socket).unwrap()
}
fn stop(mut host: Host, port: u16) {
    host.0.kill().unwrap();
    host.0.wait().unwrap();
    // Do not confuse accepted sockets in TIME_WAIT with a live listener.
    assert!(
        TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_millis(200))
            .is_err(),
        "owned server listener remained"
    );
}

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
        let config_value = configuration(&directory, "python-synthetic-pilot");
        let config: Configuration = serde_json::from_value(config_value.clone()).unwrap();
        let observed = Default::default();
        let authority = Authority::new(
            config,
            Default::default(),
            Box::new(SyntheticEvidence(std::sync::Arc::clone(&observed))),
            Box::new(audit_log(&directory.0, "python-synthetic-pilot", true)),
        )
        .unwrap();
        let credential = authority.credential().to_owned();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let config: Configuration = serde_json::from_value(config_value.clone()).unwrap();
        let listener = {
            let _entered = runtime.enter();
            Listener::bind(&config).unwrap()
        };
        let service = runtime.spawn(listener.serve(authority, 8, 100, 2000));
        let (server, port) = host(
            &directory,
            framework,
            config_value["endpoint"].as_str().unwrap(),
            &credential,
        );
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
        let observations = observed.lock().unwrap();
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
        service.abort();
        runtime.shutdown_timeout(Duration::from_secs(2));
        assert_eq!(
            audit_log(&directory.0, "python-synthetic-pilot", false)
                .records()
                .unwrap()
                .len(),
            1
        );
        println!(
            "{}",
            json!({"framework":framework,"group":"synthetic-engine-allow","cases":1,"auditRecords":1,"originEffects":1,"wholeProductQualified":false,"productionBrowserEvidence":false})
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
