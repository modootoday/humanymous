//! Real child process, authenticated WAL restart and loopback HTTP fixture pilot.
//! This is not a Docker/browser qualification suite.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use humanymous_core::integrity::request::{compute_token, seed};
use humanymous_runtime::gateway::{
    configuration::Configuration, http::Request, session::canonical,
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct PrivateDirectory(PathBuf);
impl Drop for PrivateDirectory {
    fn drop(&mut self) {
        assert!(
            self.0
                .starts_with(std::env::temp_dir().canonicalize().unwrap())
        );
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn directory() -> (PrivateDirectory, Value) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "h-runtime-{}-{}-{}",
        std::process::id(),
        now(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir(&path).unwrap();
    #[cfg(unix)]
    let owner = {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        json!({"kind":"unix","uid":std::fs::metadata(&path).unwrap().uid()})
    };
    #[cfg(windows)]
    let owner = {
        // Native OS utilities, with literal argument arrays and no script source.
        let output = Command::new("whoami.exe")
            .args(["/user", "/fo", "csv", "/nh"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        let sid = text
            .trim()
            .trim_matches('"')
            .rsplit(",\"")
            .next()
            .unwrap()
            .to_owned();
        assert!(sid.starts_with("S-1-5-"));
        assert!(
            Command::new("icacls.exe")
                .arg(&path)
                .args(["/inheritance:r", "/grant:r"])
                .arg(format!("*{sid}:(OI)(CI)F"))
                .stdout(Stdio::null())
                .status()
                .unwrap()
                .success()
        );
        json!({"kind":"windows","sid":sid,"pid":std::process::id()})
    };
    (PrivateDirectory(path.canonicalize().unwrap()), owner)
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn start(config: &Value) -> (Process, SocketAddr) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_humanymous-runtime"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(config).unwrap())
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    // A bounded receiver prevents a hung startup from hanging the test runner.
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line);
        let _ = send.send((result, line));
    });
    let process = Process(child);
    let (result, line) = receive
        .recv_timeout(Duration::from_secs(10))
        .expect("native readiness timeout");
    result.unwrap();
    assert!(!line.is_empty(), "child emitted no readiness");
    let ready: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(ready["profile"]["product_qualified"], false);
    (process, ready["listen"].as_str().unwrap().parse().unwrap())
}
fn request(body: Vec<u8>, target: &str) -> Request {
    Request {
        method: "POST".into(),
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
            ("Sec-Fetch-Site".into(), "same-origin".into()),
            ("Accept-Encoding".into(), "gzip, br, zstd".into()),
        ],
        body,
    }
}
fn sign(config: &Configuration, request: &mut Request, issued: &Value, n: u64) {
    let now = now();
    let deadline = now + 4000;
    let id = issued["session"].as_str().unwrap();
    request.headers.extend([
        ("Authorization".into(), format!("Bearer {id}")),
        ("x-humanymous-counter".into(), n.to_string()),
        ("x-humanymous-bucket".into(), (now / 10_000).to_string()),
        ("x-humanymous-deadline".into(), deadline.to_string()),
    ]);
    let key = URL_SAFE_NO_PAD
        .decode(issued["key"].as_str().unwrap())
        .unwrap();
    let token = compute_token(
        &seed(&key, n - 1),
        &canonical(config, request, id, deadline),
        n,
        now / 10_000,
    );
    request.headers.push(("x-humanymous-token".into(), token));
}
fn exchange(address: SocketAddr, request: &Request) -> (u16, Vec<u8>) {
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = format!("{} {} HTTP/1.1\r\n", request.method, request.target).into_bytes();
    for (name, value) in &request.headers {
        bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(&request.body);
    stream.write_all(&bytes).unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    let end = response.windows(4).position(|b| b == b"\r\n\r\n").unwrap();
    (
        std::str::from_utf8(&response[9..12])
            .unwrap()
            .parse()
            .unwrap(),
        response[end + 4..].to_vec(),
    )
}

#[test]
fn native_binary_session_collection_origin_restart_and_wal_authentication() {
    let (directory, owner) = directory();
    let (key_directory, _) = self::directory();
    #[cfg(unix)]
    let key_owner = humanymous_runtime::audit::keys::custody::Owner::CurrentUser;
    #[cfg(windows)]
    let key_owner = humanymous_runtime::audit::keys::custody::Owner::WindowsSid(
        owner["sid"].as_str().unwrap().into(),
    );
    let mut custody =
        humanymous_runtime::audit::keys::custody::Custody::acquire(&key_directory.0, key_owner)
            .unwrap();
    custody
        .load_or_create(b"native-fixture", || {
            humanymous_runtime::audit::keys::KeyMaterial::new(
                [0xbb; 32],
                vec![0xaa; 32],
                None,
                None,
            )
        })
        .unwrap();
    drop(custody);
    let origin = TcpListener::bind("127.0.0.1:0").unwrap();
    origin.set_nonblocking(true).unwrap();
    let value = json!({"listen":"127.0.0.1:0","origin":origin.local_addr().unwrap().to_string(),
        "scope":{"installation":"install","tenant":"tenant","origin":"origin","revision":"r1","authority":"owned.test"},
        "routes":[{"method":"POST","path":"/effect","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}}],
        "audit":{"directory":directory.0,"node_id":"runtime-native","keys":{"directory":key_directory.0,"passphrase":"native-fixture"},"allow_new":true,"trusted_head":null},
        "owner":owner,"session_capacity":16,"session_ttl_ms":60_000,"timeout_ms":5000,"rate_per_second":100});
    let cfg: Configuration = serde_json::from_value(value.clone()).unwrap();
    let (process, address) = start(&value);
    let (status, bytes) = exchange(address, &request(vec![], "/_humanymous/session"));
    assert_eq!(status, 200);
    let issued: Value = serde_json::from_slice(&bytes).unwrap();
    let mut req = request(b"effect".to_vec(), "/effect");
    sign(&cfg, &mut req, &issued, 1);
    let (status, bytes) = exchange(address, &req);
    assert_eq!(status, 403);
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap()["verdict"],
        "CHALLENGE"
    );
    assert!(matches!(origin.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
    let report = json!({"sessionId":issued["session"],"userAgent":"Mozilla/5.0 Chrome/126.0 Safari/537.36",
        "uaClientHints":{"platform":"Windows","brands":[{"brand":"Chromium","version":"126"}]},
        "signals":[{"id":"l1.navigator.webdriver","verdict":"OK","value":false,"confidence":1,"collected":"wasm"}],"environment":{"probed":true}});
    let mut collect = request(serde_json::to_vec(&report).unwrap(), "/_humanymous/collect");
    sign(&cfg, &mut collect, &issued, 2);
    assert_eq!(exchange(address, &collect).0, 200);
    // Origin executes one bounded request in a native thread, independently of app state.
    let effects = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = effects.clone();
    origin.set_nonblocking(false).unwrap();
    let worker = std::thread::spawn(move || {
        let (mut stream, _) = origin.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut scratch = [0; 1024];
        loop {
            let n = stream.read(&mut scratch).unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&scratch[..n]);
            if bytes.ends_with(b"\r\n\r\neffect") {
                break;
            }
            assert!(bytes.len() < 8192);
        }
        observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .unwrap();
    });
    let mut req = request(b"effect".to_vec(), "/effect");
    sign(&cfg, &mut req, &issued, 3);
    assert_eq!(exchange(address, &req), (200, b"ok".to_vec()));
    worker.join().unwrap();
    assert_eq!(effects.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(exchange(address, &req).0, 403);
    drop(process);
    // Authentic recovery succeeds but session credentials never survive restart.
    let (process, address) = start(&value);
    assert_eq!(exchange(address, &req).0, 401);
    drop(process);
    let mut wrong = value.clone();
    wrong["audit"]["keys"]["passphrase"] = json!("wrong-fixture");
    let mut child = Command::new(env!("CARGO_BIN_EXE_humanymous-runtime"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&wrong).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    // WAL has real authenticated decisions; no plaintext capability/report/body.
    let mut all = Vec::new();
    for entry in std::fs::read_dir(&directory.0).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            all.extend(std::fs::read(path).unwrap());
        }
    }
    let text = String::from_utf8_lossy(&all);
    assert!(text.contains("runtime.decision"));
    assert!(!text.contains(issued["session"].as_str().unwrap()));
    assert!(!text.contains("Mozilla"));
}
