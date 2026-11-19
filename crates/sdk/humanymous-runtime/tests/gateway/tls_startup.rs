//! Runtime binary B02 certificate-source refusals as a real child process.
//! Every case here is refused while preparing the TLS source, before witness
//! pinning or custody: the configuration's audit paths are never opened, and
//! no readiness line is written. Source selection and ACME maintenance
//! themselves are covered by the library `gateway::tls::listener` tests.
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn configuration(tls: Value) -> Value {
    let owner = if cfg!(windows) {
        json!({"kind":"windows","sid":"S-1-5-21-1","pid":1})
    } else {
        json!({"kind":"unix","uid":0})
    };
    json!({"listen":"127.0.0.1:0","origin":"127.0.0.1:9",
        "scope":{"installation":"install","tenant":"tenant","origin":"origin","revision":"r1","authority":"owned.test"},
        "routes":[{"method":"POST","path":"/effect","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}}],
        "audit":{"directory":"never-opened","node_id":"runtime-test","keys":{"directory":"never-opened-keys","passphrase":"unit-fixture"},"allow_new":false,"trusted_head":null},
        "owner":owner,
        "session_capacity":16,"session_ttl_ms":60_000,"timeout_ms":1000,"rate_per_second":1000,
        "tls":tls})
}

/// Run the binary on one configuration; return its exit success, stdout and stderr.
fn run(config: &Value) -> (bool, String, String) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let scratch = std::env::temp_dir().join(format!(
        "h-tls-startup-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_humanymous-runtime"))
        // A relative audit path could only resolve inside this empty directory.
        .current_dir(&scratch)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(&serde_json::to_vec(config).unwrap())
            .unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("runtime did not refuse in time");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    let opened: Vec<_> = std::fs::read_dir(&scratch).unwrap().collect();
    assert!(opened.is_empty(), "custody was touched: {opened:?}");
    std::fs::remove_dir_all(&scratch).unwrap();
    (status.success(), stdout, stderr)
}

fn refused(tls: Value, code: &str) {
    let (success, stdout, stderr) = run(&configuration(tls));
    assert!(!success, "runtime accepted the TLS request");
    assert!(stdout.is_empty(), "readiness written: {stdout}");
    assert_eq!(stderr.trim_end(), format!("humanymous-runtime: {code}"));
}

#[test]
fn runtime_binary_refuses_tls_requests_without_exactly_one_usable_source() {
    // Go served a self-signed certificate here; the runtime fails closed.
    refused(
        json!({}),
        "tls certificate source absent; self-signed is not a fallback",
    );
    refused(
        json!({"certificate": "/no/such/chain.pem"}),
        "tls certificate and key must be provided together",
    );
    refused(
        json!({"certificate": "/no/such/chain.pem", "key": "/no/such/key.pem"}),
        "tls certificate or key unusable",
    );
    refused(
        json!({"dev_self_signed": true, "certificate": "/no/such/chain.pem", "key": "/no/such/key.pem"}),
        "tls certificate source ambiguous",
    );
    refused(
        json!({"acme": {"domains": [], "cache": "/no/such/cache"}}),
        "tls acme domains absent",
    );
    refused(
        json!({"acme": {"domains": ["*.example.com"], "cache": "/no/such/cache"}}),
        "tls acme domains invalid",
    );
    // An unknown TLS field fails the whole configuration document.
    refused(json!({"self_signed": true}), "configuration pipe");
}

#[test]
fn runtime_binary_refuses_the_dev_flag_without_the_build_feature() {
    if cfg!(feature = "dev-self-signed") {
        // A dev build accepts the flag; mixing it with ACME is still refused.
        refused(
            json!({"dev_self_signed": true, "acme": {"domains": ["a.example.com"], "cache": "/no/such/cache"}}),
            "tls certificate source ambiguous",
        );
    } else {
        refused(
            json!({"dev_self_signed": true}),
            "tls dev self-signed certificate unavailable in this build",
        );
    }
}

#[test]
fn runtime_binary_refuses_acme_it_cannot_run() {
    if cfg!(feature = "acme") {
        refused(
            json!({"acme": {"domains": ["a.example.com"], "cache": "/no/such/cache", "check_interval_seconds": 0}}),
            "tls acme check interval invalid",
        );
        refused(
            json!({"acme": {"domains": ["a.example.com"], "cache": "/no/such/cache", "directory": "http://acme.example/directory"}}),
            "tls acme issuer refused",
        );
    } else {
        refused(
            json!({"acme": {"domains": ["a.example.com"], "cache": "/no/such/cache"}}),
            "tls acme unavailable in this build",
        );
    }
}

/// Unix positive path: a provisioned pair from the `tls` object is loaded by
/// the binary, which binds its loopback TLS listener after custody recovery,
/// reports the TLS readiness profile and answers a real TLS/H1 request.
#[cfg(unix)]
#[test]
fn runtime_binary_serves_tls_from_a_provisioned_pair() {
    use std::{io::BufRead, os::unix::fs::PermissionsExt, sync::Arc};
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "h-tls-serve-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let private = |name: &str| {
        let path = root.join(name);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        path
    };
    let audit = private("audit");
    let keys = private("keys");
    let material = private("tls");
    let uid = {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(&audit).unwrap().uid()
    };
    let mut custody = humanymous_runtime::audit::keys::custody::Custody::acquire(
        &keys,
        humanymous_runtime::audit::keys::custody::Owner::CurrentUser,
    )
    .unwrap();
    custody
        .load_or_create(b"tls-fixture", || {
            humanymous_runtime::audit::keys::KeyMaterial::new(
                [0xbb; 32],
                vec![0xaa; 32],
                None,
                None,
            )
        })
        .unwrap();
    drop(custody);
    let issued = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    let pem = |label: &str, der: &[u8]| {
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
    };
    std::fs::write(
        material.join("chain.pem"),
        pem("CERTIFICATE", issued.cert.der()),
    )
    .unwrap();
    std::fs::write(
        material.join("key.pem"),
        pem("PRIVATE KEY", &issued.signing_key.serialize_der()),
    )
    .unwrap();
    let origin = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mut config = configuration(json!({
        "certificate": material.join("chain.pem"),
        "key": material.join("key.pem")
    }));
    config["origin"] = json!(origin.local_addr().unwrap().to_string());
    config["audit"] = json!({"directory":audit,"node_id":"runtime-tls","keys":{"directory":keys,"passphrase":"tls-fixture"},"allow_new":true,"trusted_head":null});
    config["owner"] = json!({"kind":"unix","uid":uid});
    config["timeout_ms"] = json!(5000);

    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = Child(
        Command::new(env!("CARGO_BIN_EXE_humanymous-runtime"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    child
        .0
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&config).unwrap())
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
        .expect("TLS readiness timeout");
    result.unwrap();
    let ready: Value = serde_json::from_str(&line).expect("readiness line");
    assert_eq!(ready["profile"]["profile"], "experimental-loopback-tls-h1");
    assert_eq!(ready["profile"]["raw_tls"], true);
    assert_eq!(ready["profile"]["product_qualified"], false);
    // The selected source name, as Go logged tls_mode; no path or key material.
    assert_eq!(ready["tls_mode"], "operator_certificate");
    assert!(!line.contains("chain.pem") && !line.contains("key.pem"));
    let address: std::net::SocketAddr = ready["listen"].as_str().unwrap().parse().unwrap();
    assert!(address.ip().is_loopback());

    let mut roots = rustls::RootCertStore::empty();
    roots.add(issued.cert.der().clone()).unwrap();
    let mut client = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    client.alpn_protocols = vec![b"http/1.1".to_vec()];
    let socket = std::net::TcpStream::connect_timeout(&address, Duration::from_secs(5)).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut tls = rustls::StreamOwned::new(
        rustls::ClientConnection::new(
            Arc::new(client),
            rustls::pki_types::ServerName::try_from("localhost").unwrap(),
        )
        .unwrap(),
        socket,
    );
    tls.write_all(
        b"POST /_humanymous/session HTTP/1.1\r\nHost: owned.test\r\nContent-Length: 0\r\n\
          Content-Type: application/json\r\nsec-ch-ua: Chromium\r\n\
          User-Agent: Mozilla/5.0 Chrome/126.0 Safari/537.36\r\n\
          Sec-Fetch-Site: same-origin\r\nAccept-Encoding: gzip, br, zstd\r\n\r\n",
    )
    .unwrap();
    tls.flush().unwrap();
    assert_eq!(tls.conn.alpn_protocol(), Some(b"http/1.1".as_slice()));
    let mut response = Vec::new();
    match tls.read_to_end(&mut response) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {}
        Err(error) => panic!("TLS response failed: {error}"),
    }
    assert!(
        response.starts_with(b"HTTP/1.1 200"),
        "{}",
        String::from_utf8_lossy(&response)
    );
    drop(child);
    std::fs::remove_dir_all(&root).unwrap();
}
