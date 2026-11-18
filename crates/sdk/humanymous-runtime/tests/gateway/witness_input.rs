//! Real exec-inherited witness pipes into the shipped runtime binary (C3).
//! A separate audit-witness process co-signs; this is native component
//! evidence, not the Docker runtime-binary-witness-transport case.
#![cfg(unix)]
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use humanymous_core::integrity::request::{compute_token, seed};
use humanymous_runtime::{
    audit::{
        checkpoint::service::{WitnessServiceBootstrap, WitnessServiceConfig, write_frame},
        keys::{
            KeyMaterial,
            custody::{Custody, Owner},
        },
    },
    gateway::{configuration::Configuration, http::Request, session::canonical},
};
use rustix::io::{FdFlags, fcntl_setfd};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, MetadataExt},
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const WRITER_SEED: [u8; 32] = [0xbb; 32];
const WITNESS_SEED: [u8; 32] = [0xcc; 32];
const NODE: &str = "runtime-witnessed";

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
fn private(path: &Path) {
    std::fs::DirBuilder::new().mode(0o700).create(path).unwrap();
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn public(seed: &[u8; 32]) -> [u8; 32] {
    ed25519_dalek::SigningKey::from_bytes(seed)
        .verifying_key()
        .to_bytes()
}
fn provision(directory: &Path, passphrase: &[u8], seed: [u8; 32]) {
    private(directory);
    Custody::acquire(directory, Owner::CurrentUser)
        .unwrap()
        .load_or_create(passphrase, || {
            KeyMaterial::new(seed, vec![0xaa; 32], None, None)
        })
        .unwrap();
}
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Trusted supervisor role: start the witness, send its private first frame,
/// then let the runtime inherit the runtime-side pipe ends at their numbers.
fn witness(
    root: &Path,
    head: &str,
) -> (Process, std::process::ChildStdin, std::process::ChildStdout) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_audit-witness"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    write_frame(
        &mut input,
        &WitnessServiceConfig {
            key_directory: root.join("witness-keys"),
            state_directory: root.join(head),
            owner_sid: None,
            passphrase: zeroize::Zeroizing::new(b"witness-fixture".to_vec()),
            writer_public: public(&WRITER_SEED),
            node_id: NODE.into(),
            bootstrap: WitnessServiceBootstrap::CreateNew,
        },
    )
    .unwrap();
    (Process(child), input, output)
}

/// Spawn the shipped binary; return its readiness, or None when it refused.
fn runtime(config: &Value) -> (Process, Option<Value>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_humanymous-runtime"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(config).unwrap())
        .unwrap();
    let stdout = child.stdout.take().unwrap();
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
    (
        process,
        (!line.is_empty()).then(|| serde_json::from_str(&line).unwrap()),
    )
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

/// One sequential test: the inherited pipe numbers must not leak into a
/// concurrently spawned sibling child of this test binary.
#[test]
fn shipped_binary_inherited_witness_pins_cosigns_and_poisons_on_loss() {
    let root = std::env::temp_dir().join(format!(
        "h-runtime-witness-{}-{}",
        std::process::id(),
        now()
    ));
    private(&root);
    let root = PrivateDirectory(root.canonicalize().unwrap());
    for name in ["audit", "refused-head", "witness-head"] {
        private(&root.0.join(name));
    }
    provision(&root.0.join("keys"), b"native-fixture", WRITER_SEED);
    provision(
        &root.0.join("witness-keys"),
        b"witness-fixture",
        WITNESS_SEED,
    );
    let origin = TcpListener::bind("127.0.0.1:0").unwrap();
    origin.set_nonblocking(true).unwrap();
    let uid = std::fs::metadata(&root.0).unwrap().uid();
    let base = |read_fd: i32, write_fd: i32, witness_public: [u8; 32]| {
        json!({"listen":"127.0.0.1:0","origin":origin.local_addr().unwrap().to_string(),
        "scope":{"installation":"install","tenant":"tenant","origin":"origin","revision":"r1","authority":"owned.test"},
        "routes":[{"method":"POST","path":"/effect","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}}],
        "audit":{"directory":root.0.join("audit"),"node_id":NODE,"keys":{"directory":root.0.join("keys"),"passphrase":"native-fixture"},
            "allow_new":true,"trusted_head":null,"require_external_witness":true,
            "witness_input":{"read_fd":read_fd,"write_fd":write_fd,"witness_public":witness_public,
                "writer_public":public(&WRITER_SEED),"durability":"file_and_directory"}},
        "owner":{"kind":"unix","uid":uid},"session_capacity":16,"session_ttl_ms":60_000,"timeout_ms":5000,"rate_per_second":100})
    };

    // Numbers that were never inherited refuse startup without readiness.
    let (refused, ready) = runtime(&base(900, 901, public(&WITNESS_SEED)));
    assert!(ready.is_none());
    drop(refused);
    // A real witness whose key differs from the pin also refuses.
    {
        let (_witness, input, output) = witness(&root.0, "refused-head");
        fcntl_setfd(&input, FdFlags::empty()).unwrap();
        fcntl_setfd(&output, FdFlags::empty()).unwrap();
        let (refused, ready) = runtime(&base(
            output.as_raw_fd(),
            input.as_raw_fd(),
            public(&[0xdd; 32]),
        ));
        assert!(ready.is_none());
        drop(refused);
    }
    assert!(
        std::fs::read_dir(root.0.join("audit"))
            .unwrap()
            .next()
            .is_none()
    );

    let (mut witness_process, input, output) = witness(&root.0, "witness-head");
    // Clear close-on-exec only on the two runtime-side ends, then close this
    // supervisor's copies once the runtime inherited them.
    fcntl_setfd(&input, FdFlags::empty()).unwrap();
    fcntl_setfd(&output, FdFlags::empty()).unwrap();
    let value = base(output.as_raw_fd(), input.as_raw_fd(), public(&WITNESS_SEED));
    let (process, ready) = runtime(&value);
    drop(input);
    drop(output);
    let ready = ready.expect("witnessed runtime readiness");
    assert_eq!(ready["profile"]["audit_external_witness"], true);
    assert_eq!(
        ready["profile"]["audit_witness_required_durability"],
        "file_and_directory"
    );
    assert_eq!(
        ready["profile"]["audit_witness_independence_qualified"],
        false
    );
    assert_eq!(ready["profile"]["product_qualified"], false);
    let address: SocketAddr = ready["listen"].as_str().unwrap().parse().unwrap();
    let cfg: Configuration = serde_json::from_value(value).unwrap();

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
    // The separate witness process persisted the co-signed head.
    assert!(
        std::fs::read_dir(root.0.join("witness-head"))
            .unwrap()
            .next()
            .is_some()
    );

    // Witness loss: the next audited decision fails closed and the epoch stays
    // poisoned; no origin connection is ever attempted.
    witness_process.0.kill().unwrap();
    witness_process.0.wait().unwrap();
    let mut req = request(b"effect".to_vec(), "/effect");
    sign(&cfg, &mut req, &issued, 2);
    assert_eq!(exchange(address, &req).0, 503);
    assert_eq!(
        exchange(address, &request(vec![], "/_humanymous/session")).0,
        503
    );
    assert!(matches!(origin.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
    drop(process);
}
