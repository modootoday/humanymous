//! Plan B26 over real sockets: the public TLS listener takes the client from
//! the socket peer, or from the PROXY v2 header of a listed balancer, and the
//! edge ban gate (sharing the admin plane's store) keys on that client.
use super::*;
use crate::gateway::ban::gate::{BLOCK_BODY, Mode};
use crate::gateway::configuration::AdminListener;
use crate::gateway::tls::listener;
use crate::network::proxy::SIGNATURE;
use std::io::{Read as _, Write as _};
use std::net::{IpAddr, SocketAddr, TcpStream as NativeStream};
use std::time::Duration;

const BUDGET: Duration = Duration::from_secs(5);
const TOKEN: &str = "operator-fixture-token-a";
const DECLARED: &str = "203.0.113.7";

fn admin(settings: &std::path::Path) -> AdminListener {
    serde_json::from_value(json!({
        "listen": "127.0.0.1:0",
        "operators": [{"role":"operator","id":"op-1","token":TOKEN}],
        "bans": {"capacity":64,"max_keys":64,"max_hits_per_key":256,"max_key_bytes":128,
            "max_payload_bytes":1_048_576,"gc_scan_budget":64},
        "settings_dir": settings,
    }))
    .unwrap()
}

/// PROXY v2 PROXY command, TCP over IPv4, declaring `source`.
fn proxy_v4(source: [u8; 4]) -> Vec<u8> {
    let mut header = SIGNATURE.to_vec();
    header.extend([0x21, 0x11, 0x00, 0x0c]);
    header.extend(source);
    header.extend([192, 0, 2, 1]);
    header.extend(40_000u16.to_be_bytes());
    header.extend(443u16.to_be_bytes());
    header
}

/// PROXY v2 LOCAL command (a balancer health check).
fn proxy_local() -> Vec<u8> {
    let mut header = SIGNATURE.to_vec();
    header.extend([0x20, 0x00, 0x00, 0x00]);
    header
}

/// One TLS/H1 GET after an optional raw prefix; the response text, empty
/// when the connection was closed without one.
fn get(
    address: SocketAddr,
    root: &rustls::pki_types::CertificateDer<'static>,
    prefix: &[u8],
) -> String {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(root.clone()).unwrap();
    let mut client = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    client.alpn_protocols = vec![b"http/1.1".to_vec()];
    let mut socket = NativeStream::connect_timeout(&address, BUDGET).unwrap();
    socket.set_read_timeout(Some(BUDGET)).unwrap();
    socket.write_all(prefix).unwrap();
    let mut stream = rustls::StreamOwned::new(
        rustls::ClientConnection::new(
            Arc::new(client),
            rustls::pki_types::ServerName::try_from("owned.test").unwrap(),
        )
        .unwrap(),
        socket,
    );
    let request = "GET /effect HTTP/1.1\r\nHost: owned.test\r\nContent-Length: 0\r\n\r\n";
    if stream.write_all(request.as_bytes()).is_err() {
        return String::new();
    }
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);
    String::from_utf8_lossy(&response).into_owned()
}

/// Ban `ip` through the admin listener (a temporary operator ban).
fn ban(admin: SocketAddr, ip: IpAddr) {
    let body = json!({"Key": format!("ip:{ip}"), "Reason": "b26 test", "Incident": "", "DurationSec": 600}).to_string();
    let mut socket = NativeStream::connect_timeout(&admin, BUDGET).unwrap();
    socket.set_read_timeout(Some(BUDGET)).unwrap();
    let request = format!(
        "POST /_humanymous/admin/bans HTTP/1.1\r\nHost: admin\r\nAuthorization: Bearer {TOKEN}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    socket.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    let _ = socket.read_to_string(&mut response);
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
}

fn blocked(response: &str) -> bool {
    response.starts_with("HTTP/1.1 403") && response.contains(BLOCK_BODY)
}

struct Running {
    public: SocketAddr,
    admin: SocketAddr,
    root: rustls::pki_types::CertificateDer<'static>,
    ready: serde_json::Value,
    sender: tokio::sync::watch::Sender<Option<crate::gateway::service::ShutdownRequest>>,
    running: tokio::task::JoinHandle<std::io::Result<crate::gateway::service::ExitReport>>,
    _fixture: PrivateFixture,
}

async fn start(trusted: &[String]) -> Running {
    let interface = interface_ip();
    let fixture = PrivateFixture::new();
    let (tls, root) = provisioned_pair(&fixture.0.join("tls"), "owned.test");
    let mut config = fixture.configuration();
    config.listen = "0.0.0.0:0".parse().unwrap();
    config.public_listen = true;
    config.tls = Some(tls);
    config.trusted_proxies = trusted.to_vec();
    // An enforcing ban gate on a public listener: the client is attributable.
    config.edge.ban = Mode::Enforce;
    config.admin = Some(admin(&fixture.0.join("settings")));
    config.validate().unwrap();
    let prepared = listener::prepare(config.tls.as_ref().unwrap()).unwrap();
    let host = ServiceHost::bind_tls(config, None, None, None, None, prepared.server_config())
        .await
        .unwrap();
    let ready = serde_json::to_value(host.readiness()).unwrap();
    let public = SocketAddr::new(interface, host.readiness().listen.port());
    let admin = host.readiness().admin_listen.unwrap();
    let (sender, receiver) = tokio::sync::watch::channel(None);
    let running = tokio::spawn(host.run(receiver));
    Running {
        public,
        admin,
        root,
        ready,
        sender,
        running,
        _fixture: fixture,
    }
}

async fn stop(running: Running) {
    running
        .sender
        .send(Some(StopBudget::new(1000, 1000, 5000).unwrap().request()))
        .unwrap();
    let report = tokio::time::timeout(BUDGET, running.running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(report.stopped && report.drained && report.audit_flushed);
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(work).await.unwrap()
}

/// Without `trusted_proxies` the socket peer is the client: banning it
/// blocks the next request, and a PROXY v2 header is never parsed (its bytes
/// break the TLS handshake instead of declaring a source).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unix_public_listener_keys_bans_on_the_socket_peer() {
    let run = start(&[]).await;
    assert_eq!(
        run.ready["profile"]["client_address"]["source"], "socket",
        "{}",
        run.ready
    );
    assert_eq!(run.ready["profile"]["client_address"]["trusted_proxies"], 0);
    let (public, admin, root) = (run.public, run.admin, run.root.clone());
    blocking(move || {
        assert!(!blocked(&get(public, &root, &[])));
        // A spoofed header from an unlisted peer declares nothing.
        assert!(!blocked(&get(public, &root, &proxy_v4([203, 0, 113, 7]))));
        ban(admin, DECLARED.parse().unwrap());
        assert!(
            !blocked(&get(public, &root, &[])),
            "declared source must not key an unlisted peer"
        );
        ban(admin, public.ip());
        assert!(blocked(&get(public, &root, &[])));
    })
    .await;
    stop(run).await;
}

/// A listed balancer's declared source is the client; LOCAL is served
/// unattributed (refused 503 `ban_client_address` by an enforcing gate); a
/// listed peer without a valid header is closed; a slow listed peer does not
/// hold the listener.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unix_trusted_balancer_declares_the_client() {
    let interface = interface_ip();
    let run = start(&[format!("{interface}/32")]).await;
    assert_eq!(
        run.ready["profile"]["client_address"]["source"], "socket_or_trusted_proxy_v2",
        "{}",
        run.ready
    );
    assert_eq!(run.ready["profile"]["client_address"]["trusted_proxies"], 1);
    assert!(
        !run.ready.to_string().contains(&format!("{interface}/32")),
        "network in readiness"
    );
    let (public, admin, root) = (run.public, run.admin, run.root.clone());
    blocking(move || {
        let declared = proxy_v4([203, 0, 113, 7]);
        assert!(!blocked(&get(public, &root, &declared)));
        // Banning the balancer itself does not block the client it declares.
        ban(admin, public.ip());
        assert!(!blocked(&get(public, &root, &declared)));
        ban(admin, DECLARED.parse().unwrap());
        assert!(blocked(&get(public, &root, &declared)));
        // A different declared client behind the same balancer still passes.
        assert!(!blocked(&get(public, &root, &proxy_v4([198, 51, 100, 9]))));
        // LOCAL: unattributed, so the enforcing gate fails closed.
        let local = get(public, &root, &proxy_local());
        assert!(local.starts_with("HTTP/1.1 503"), "{local}");
        assert!(!blocked(&local));
        // No header from a listed peer: the TLS bytes are not a PROXY header.
        assert_eq!(get(public, &root, &[]), "");
        // A silent listed peer holds one receive permit, not the listener.
        let silent = NativeStream::connect_timeout(&public, BUDGET).unwrap();
        assert!(!blocked(&get(public, &root, &proxy_v4([198, 51, 100, 10]))));
        drop(silent);
    })
    .await;
    stop(run).await;
}

#[test]
fn trusted_proxies_need_a_public_listener_and_a_narrow_allowlist() {
    let fixture = PrivateFixture::new();
    let mut config = fixture.configuration();
    config.trusted_proxies = vec!["10.0.0.0/8".into()];
    assert!(config.validate().is_err(), "loopback listener");
    let (tls, _) = provisioned_pair(&fixture.0.join("tls"), "owned.test");
    config.listen = "0.0.0.0:443".parse().unwrap();
    config.public_listen = true;
    config.tls = Some(tls);
    assert!(config.validate().is_ok());
    for refused in [
        vec!["0.0.0.0/0".to_owned()],
        vec!["10.0.0.0/7".into()],
        vec!["not-a-cidr".into()],
        vec![String::new()],
    ] {
        config.trusted_proxies = refused.clone();
        assert!(config.validate().is_err(), "{refused:?}");
    }
    config.trusted_proxies = vec!["192.0.2.10".into(), "2001:db8::/32".into()];
    assert!(config.validate().is_ok());
    assert_eq!(config.trusted_proxy_allowlist().unwrap().entries().len(), 2);
}
