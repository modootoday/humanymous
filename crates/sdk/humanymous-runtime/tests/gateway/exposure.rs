//! Owner decision B01 #1 through the real runtime binary: the Core role
//! (`humanymous-runtime core`), the explicit public TLS opt-in of the Gate
//! and Core, and the separate loopback admin TCP listener. Non-loopback peers
//! connect from a local interface address; a host without one fails loudly.
use serde_json::{Value, json};
use std::{
    io::{BufRead, Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

const BUDGET: Duration = Duration::from_secs(5);

fn interface_ip() -> IpAddr {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
    socket
        .connect("198.51.100.1:9")
        .expect("exposure tests need a non-loopback IPv4 interface with a route");
    let ip = socket.local_addr().unwrap().ip();
    assert!(
        !ip.is_loopback() && !ip.is_unspecified(),
        "exposure tests need a non-loopback IPv4 interface, found {ip}"
    );
    ip
}

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "h-exposure-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Child(std::process::Child);
impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn pem(label: &str, der: &[u8]) -> String {
    use base64::Engine;
    let body = base64::engine::general_purpose::STANDARD.encode(der);
    let lines: Vec<&str> = body
        .as_bytes()
        .chunks(64)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect();
    format!(
        "-----BEGIN {label}-----\n{}\n-----END {label}-----\n",
        lines.join("\n")
    )
}

/// A provisioned pair for `name`; returns the `tls` object and the root.
fn pair(dir: &std::path::Path, name: &str) -> (Value, rustls::pki_types::CertificateDer<'static>) {
    let issued = rcgen::generate_simple_self_signed(vec![name.to_owned()]).unwrap();
    std::fs::write(dir.join("chain.pem"), pem("CERTIFICATE", issued.cert.der())).unwrap();
    std::fs::write(
        dir.join("key.pem"),
        pem("PRIVATE KEY", &issued.signing_key.serialize_der()),
    )
    .unwrap();
    (
        json!({"certificate": dir.join("chain.pem"), "key": dir.join("key.pem")}),
        issued.cert.der().clone(),
    )
}

/// Start the binary with `arguments` and one private document; return the
/// child and its readiness line, or its failure and stderr.
fn start(arguments: &[&str], document: &Value) -> Result<(Child, Value), String> {
    let mut child = Child(
        Command::new(env!("CARGO_BIN_EXE_humanymous-runtime"))
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    child
        .0
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(document).unwrap())
        .unwrap();
    let stdout = child.0.stdout.take().unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = std::io::BufReader::new(stdout).read_line(&mut line);
        let _ = send.send((result, line));
    });
    let (result, line) = receive
        .recv_timeout(Duration::from_secs(20))
        .expect("readiness timeout");
    result.unwrap();
    if line.is_empty() {
        let deadline = Instant::now() + Duration::from_secs(20);
        while child.0.try_wait().unwrap().is_none() {
            assert!(
                Instant::now() < deadline,
                "runtime neither ready nor exited"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let mut stderr = String::new();
        child
            .0
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        return Err(stderr.trim_end().to_owned());
    }
    Ok((child, serde_json::from_str(&line).expect("readiness line")))
}

fn plain(address: SocketAddr, request: &str) -> String {
    let mut socket = TcpStream::connect_timeout(&address, BUDGET).unwrap();
    socket.set_read_timeout(Some(BUDGET)).unwrap();
    socket.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    let _ = socket.read_to_string(&mut response);
    response
}

fn tls(
    address: SocketAddr,
    root: rustls::pki_types::CertificateDer<'static>,
    name: &'static str,
    request: &str,
) -> String {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(root).unwrap();
    let mut client = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    client.alpn_protocols = vec![b"http/1.1".to_vec()];
    let socket = TcpStream::connect_timeout(&address, BUDGET).unwrap();
    socket.set_read_timeout(Some(BUDGET)).unwrap();
    let mut stream = rustls::StreamOwned::new(
        rustls::ClientConnection::new(
            Arc::new(client),
            rustls::pki_types::ServerName::try_from(name).unwrap(),
        )
        .unwrap(),
        socket,
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);
    String::from_utf8_lossy(&response).into_owned()
}

fn core_document(listen: &str, web: &Scratch) -> Value {
    json!({
        "listen": listen,
        "web_dir": web.0,
        "budget_ms": 5000,
        "max_conns": 8,
        "limits": {
            "traffic": [4096, 4096, 4096, 67_108_864],
            "correlation": [4096, 4096, 256, 4096, 67_108_864],
            "correlation_population": [16, 256, 256, 1_000_000],
            "rate": [4096, 4096, 4096, 67_108_864, 4096],
            "watermark_capacity": 4096,
            "watermark_codec": [1_048_576, 2_097_152, 8],
            "fingerprint_sessions": 4096
        }
    })
}

#[test]
fn runtime_binary_core_role_starts_on_loopback_and_reports_readiness() {
    let web = Scratch::new("core-web");
    let (child, ready) = start(&["core"], &core_document("127.0.0.1:0", &web)).unwrap();
    assert_eq!(ready["role"], "core");
    assert_eq!(
        ready["profile"]["profile"],
        "experimental-core-loopback-http1"
    );
    assert_eq!(ready["profile"]["public_listen"], false);
    assert_eq!(ready["profile"]["pass"], true);
    assert_eq!(ready["profile"]["product_qualified"], false);
    assert!(ready.get("tls_mode").is_none());
    let address: SocketAddr = ready["listen"].as_str().unwrap().parse().unwrap();
    assert!(address.ip().is_loopback());
    let health = plain(address, "GET /healthz HTTP/1.1\r\nHost: core.test\r\n\r\n");
    assert!(health.starts_with("HTTP/1.1 200 OK\r\n"), "{health}");
    assert!(health.contains("\"status\":\"ok\""), "{health}");
    drop(child);
}

#[test]
fn runtime_binary_core_role_refuses_exposure_without_the_opt_in_and_tls() {
    let web = Scratch::new("core-refused");
    let material = Scratch::new("core-refused-tls");
    let (pair, _) = pair(&material.0, "core.test");
    let mut no_key = core_document("0.0.0.0:0", &web);
    no_key["tls"] = pair.clone();
    let mut plaintext = core_document("0.0.0.0:0", &web);
    plaintext["public_listen"] = json!(true);
    for document in [core_document("0.0.0.0:0", &web), no_key, plaintext] {
        let refused = start(&["core"], &document).err().expect("core started");
        assert_eq!(refused, "humanymous-runtime: core configuration");
    }
}

#[test]
fn runtime_binary_public_core_serves_a_non_loopback_peer_over_tls() {
    let interface = interface_ip();
    let web = Scratch::new("core-public");
    let material = Scratch::new("core-public-tls");
    let (pair, root) = pair(&material.0, "core.test");
    let mut document = core_document("0.0.0.0:0", &web);
    document["public_listen"] = json!(true);
    document["tls"] = pair;
    let (child, ready) = start(&["core"], &document).unwrap();
    assert_eq!(
        ready["profile"]["profile"],
        "experimental-core-public-tls-h1"
    );
    assert_eq!(ready["profile"]["original_h2"], false);
    assert_eq!(ready["profile"]["alpn"], json!(["http/1.1"]));
    assert_eq!(ready["tls_mode"], "operator_certificate");
    let bound: SocketAddr = ready["listen"].as_str().unwrap().parse().unwrap();
    assert!(bound.ip().is_unspecified());
    let health = tls(
        SocketAddr::new(interface, bound.port()),
        root,
        "core.test",
        "GET /healthz HTTP/1.1\r\nHost: core.test\r\n\r\n",
    );
    assert!(health.starts_with("HTTP/1.1 200 OK\r\n"), "{health}");
    drop(child);
}

fn gate_document(owner: Value) -> Value {
    json!({"listen":"127.0.0.1:0","origin":"127.0.0.1:9",
        "scope":{"installation":"install","tenant":"tenant","origin":"origin","revision":"r1","authority":"owned.test"},
        "routes":[{"method":"POST","path":"/effect","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}}],
        "audit":{"directory":"never-opened","node_id":"runtime-test","keys":{"directory":"never-opened-keys","passphrase":"unit-fixture"},"allow_new":false,"trusted_head":null},
        "owner":owner,
        "session_capacity":16,"session_ttl_ms":60_000,"timeout_ms":5000,"rate_per_second":1000})
}

fn admin_object() -> Value {
    json!({
        "listen": "127.0.0.1:0",
        "operators": [{"role":"auditor","id":"aud-1","token":"operator-fixture-token-a"}],
        "bans": {"capacity":64,"max_keys":64,"max_hits_per_key":256,"max_key_bytes":128,
            "max_payload_bytes":1_048_576,"gc_scan_budget":64}
    })
}

#[test]
fn runtime_binary_gate_refuses_exposure_before_custody() {
    let owner = if cfg!(windows) {
        json!({"kind":"windows","sid":"S-1-5-21-1","pid":1})
    } else {
        json!({"kind":"unix","uid":0})
    };
    let material = Scratch::new("gate-refused-tls");
    let (pair, _) = pair(&material.0, "owned.test");
    let mut no_key = gate_document(owner.clone());
    no_key["listen"] = json!("0.0.0.0:0");
    no_key["tls"] = pair;
    let mut plaintext = gate_document(owner.clone());
    plaintext["listen"] = json!("0.0.0.0:0");
    plaintext["public_listen"] = json!(true);
    let mut admin = gate_document(owner);
    admin["admin"] = admin_object();
    admin["admin"]["listen"] = json!("0.0.0.0:0");
    for document in [no_key, plaintext, admin] {
        let refused = start(&[], &document).err().expect("gate started");
        assert_eq!(refused, "humanymous-runtime: configuration pipe");
    }
}

/// The Gate binary with real custody: a public TLS edge on every interface
/// serving a non-loopback peer, and the admin plane on its own loopback
/// listener only.
#[cfg(unix)]
#[test]
fn runtime_binary_serves_a_public_tls_edge_beside_a_loopback_admin_listener() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let interface = interface_ip();
    let root = Scratch::new("gate-public");
    let private = |name: &str| {
        let path = root.0.join(name);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        path
    };
    let audit = private("audit");
    let keys = private("keys");
    let material = private("tls");
    let settings = private("settings");
    let uid = std::fs::metadata(&audit).unwrap().uid();
    let mut custody = humanymous_runtime::audit::keys::custody::Custody::acquire(
        &keys,
        humanymous_runtime::audit::keys::custody::Owner::CurrentUser,
    )
    .unwrap();
    custody
        .load_or_create(b"exposure-fixture", || {
            humanymous_runtime::audit::keys::KeyMaterial::new(
                [0xbb; 32],
                vec![0xaa; 32],
                None,
                None,
            )
        })
        .unwrap();
    drop(custody);
    let (pair, certificate) = pair(&material, "owned.test");
    let mut document = gate_document(json!({"kind":"unix","uid":uid}));
    document["listen"] = json!("0.0.0.0:0");
    document["public_listen"] = json!(true);
    document["tls"] = pair;
    document["audit"] = json!({"directory":audit,"node_id":"runtime-exposure","keys":{"directory":keys,"passphrase":"exposure-fixture"},"allow_new":true,"trusted_head":null});
    document["edge"] = json!({"ban":"off"});
    document["admin"] = admin_object();
    document["admin"]["settings_dir"] = json!(settings);
    let (child, ready) = start(&[], &document).unwrap();
    assert!(!ready.to_string().contains("operator-fixture-token-a"));
    assert_eq!(ready["tls_mode"], "operator_certificate");
    assert_eq!(ready["profile"]["profile"], "experimental-public-tls");
    assert_eq!(ready["profile"]["public_listen"], true);
    assert_eq!(ready["profile"]["original_h2"], true);
    assert_eq!(ready["profile"]["product_qualified"], false);
    let edge: SocketAddr = ready["listen"].as_str().unwrap().parse().unwrap();
    assert!(edge.ip().is_unspecified());
    let admin: SocketAddr = ready["admin_listen"].as_str().unwrap().parse().unwrap();
    assert!(admin.ip().is_loopback());
    let public = SocketAddr::new(interface, edge.port());
    let probe = tls(
        public,
        certificate.clone(),
        "owned.test",
        "GET /_humanymous/healthz HTTP/1.1\r\nHost: owned.test\r\nContent-Length: 0\r\n\r\n",
    );
    assert!(probe.starts_with("HTTP/1.1 200"), "{probe}");
    let refused = tls(
        public,
        certificate,
        "owned.test",
        "GET /_humanymous/admin/whoami HTTP/1.1\r\nHost: owned.test\r\nContent-Length: 0\r\n\
         Authorization: Bearer operator-fixture-token-a\r\n\r\n",
    );
    assert!(!refused.starts_with("HTTP/1.1 200"), "{refused}");
    let whoami = plain(
        admin,
        "GET /_humanymous/admin/whoami HTTP/1.1\r\nHost: admin\r\nContent-Length: 0\r\n\
         Authorization: Bearer operator-fixture-token-a\r\n\r\n",
    );
    assert!(whoami.starts_with("HTTP/1.1 200"), "{whoami}");
    assert!(whoami.contains("\"auditor\""), "{whoami}");
    assert!(TcpStream::connect_timeout(&SocketAddr::new(interface, admin.port()), BUDGET).is_err());
    drop(child);
}
