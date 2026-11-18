//! Live B02 ACME issuance against Let's Encrypt's Pebble test CA in Docker.
//!
//! Pebble validates TLS-ALPN-01 by connecting to 127.0.0.1:5001, where this
//! test serves the issuer's ingress configuration; pebble-challtestsrv answers
//! every DNS name with 127.0.0.1. Both images are pinned by digest. Run with
//! `cargo test -p humanymous-runtime --features acme --test gateway-acme-pebble
//! -- --ignored --test-threads=1` on a host with a Docker engine.
use humanymous_runtime::gateway::tls::{
    acme::{AcmeIssuer, AcmeSettings, CHALLENGE_ALPN},
    certificate::issuer_config,
};
use rustls::ServerConnection;
use std::{
    io::ErrorKind,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const PEBBLE: &str = "ghcr.io/letsencrypt/pebble@sha256:0c81f05db371f436c97fb894bd6d2ab8fc9c8e2ce40f8bef8a8e15941bcd75ef";
const CHALLTESTSRV: &str = "ghcr.io/letsencrypt/pebble-challtestsrv@sha256:0152c8cb506e2ecba2900ed47d23932e27e3e6575a7d2c8ef93c7719ee3dea4c";
const DOMAIN: &str = "gate.humanymous.test";

/// Removes the named containers when the test ends, pass or fail.
struct Containers(Vec<String>);

impl Drop for Containers {
    fn drop(&mut self) {
        for name in &self.0 {
            let _ = Command::new("docker")
                .args(["rm", "--force", name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

fn docker(args: &[&str]) {
    let output = Command::new("docker").args(args).output().unwrap();
    assert!(
        output.status.success(),
        "docker {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hmn-acme-pebble-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Pebble resolves challenge names over TCP DNS; it must not start validating
/// before pebble-challtestsrv accepts on that port.
fn wait_for_dns() {
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        if std::net::TcpStream::connect("127.0.0.1:8053").is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(250));
    }
    panic!("pebble-challtestsrv DNS did not start");
}

fn wait_for_directory(root: &Path) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        if std::net::TcpStream::connect("127.0.0.1:14000").is_ok() && root.exists() {
            return;
        }
        thread::sleep(Duration::from_millis(250));
    }
    panic!("Pebble directory did not start");
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .try_into()
        .unwrap()
}

#[test]
#[ignore = "needs a Docker engine; run by the CI ACME job"]
fn pebble_issues_over_tls_alpn_01_and_the_cache_survives_restart() {
    let suffix = std::process::id().to_string();
    let pebble = format!("hmn-pebble-{suffix}");
    let dns = format!("hmn-challtestsrv-{suffix}");
    let _containers = Containers(vec![pebble.clone(), dns.clone()]);
    // Disable every challtestsrv challenge listener so 5001 stays ours.
    docker(&[
        "run",
        "--detach",
        "--name",
        &dns,
        "--network",
        "host",
        CHALLTESTSRV,
        "-defaultIPv4",
        "127.0.0.1",
        "-defaultIPv6",
        "",
        "-dnsserver",
        "127.0.0.1:8053",
        "-http01",
        "",
        "-https01",
        "",
        "-tlsalpn01",
        "",
        "-doh",
        "",
        "-management",
        "127.0.0.1:8055",
    ]);
    wait_for_dns();
    // PEBBLE_VA_NOSLEEP removes artificial validation delay only; validation
    // itself still dials this host. PEBBLE_WFE_NONCEREJECT=0 turns off random
    // bad-nonce fault injection so the run is deterministic.
    docker(&[
        "run",
        "--detach",
        "--name",
        &pebble,
        "--network",
        "host",
        "--env",
        "PEBBLE_VA_NOSLEEP=1",
        "--env",
        "PEBBLE_WFE_NONCEREJECT=0",
        PEBBLE,
        "-config",
        "/test/config/pebble-config.json",
        "-dnsserver",
        "127.0.0.1:8053",
    ]);
    let dir = scratch();
    let root = dir.join("pebble.minica.pem");
    docker(&[
        "cp",
        &format!("{pebble}:/test/certs/pebble.minica.pem"),
        root.to_str().unwrap(),
    ]);
    wait_for_directory(&root);

    let settings = AcmeSettings {
        directory: "https://localhost:14000/dir".into(),
        cache: dir.join("cache"),
        email: None,
        directory_root: Some(root),
    };
    let domains = vec![DOMAIN.to_owned()];
    let issuer = AcmeIssuer::new(settings.clone(), domains.clone()).unwrap();
    assert!(issuer.due(now()));
    let config = Arc::new(issuer_config(&domains, &issuer).unwrap());

    let listener = TcpListener::bind("127.0.0.1:5001").unwrap();
    listener.set_nonblocking(true).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let negotiated = Arc::new(Mutex::new(Vec::<Option<Vec<u8>>>::new()));
    let server = {
        let (stop, negotiated, config) = (stop.clone(), negotiated.clone(), config.clone());
        thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(10)))
                            .unwrap();
                        let mut connection = ServerConnection::new(config.clone()).unwrap();
                        let _ = connection.complete_io(&mut stream);
                        negotiated
                            .lock()
                            .unwrap()
                            .push(connection.alpn_protocol().map(<[u8]>::to_vec));
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(error) => panic!("accept: {error}"),
                }
            }
        })
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let issued = runtime.block_on(issuer.issue());
    stop.store(true, Ordering::SeqCst);
    server.join().unwrap();
    issued.unwrap();

    assert!(
        negotiated
            .lock()
            .unwrap()
            .contains(&Some(CHALLENGE_ALPN.to_vec())),
        "Pebble never completed an acme-tls/1 validation handshake"
    );
    let not_after = issuer.not_after().expect("issued certificate installed");
    assert!(!issuer.due(now()));
    runtime.block_on(issuer.ensure()).unwrap();
    assert_eq!(
        issuer.not_after(),
        Some(not_after),
        "no renewal while valid"
    );

    let restarted = AcmeIssuer::new(settings, domains).unwrap();
    assert_eq!(restarted.not_after(), Some(not_after));
    std::fs::remove_dir_all(dir).unwrap();
}
