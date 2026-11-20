//! Owner decision B01 #1 over real sockets and real custody: a public TLS
//! edge serving non-loopback peers beside the separate loopback admin TCP
//! listener, which alone serves the admin plane.
use super::*;
use crate::gateway::ban::gate::Mode;
use crate::gateway::configuration::AdminListener;
use crate::gateway::tls::listener;
use std::io::{Read as _, Write as _};
use std::net::{SocketAddr, TcpStream as NativeStream};
use std::time::Duration;

const BUDGET: Duration = Duration::from_secs(5);
const TOKEN: &str = "operator-fixture-token-a";

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

/// One TLS/H1 request from a chosen source address; the full response text.
fn tls_get(
    address: SocketAddr,
    root: rustls::pki_types::CertificateDer<'static>,
    target: &str,
    authorization: Option<&str>,
) -> String {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(root).unwrap();
    let mut client = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    client.alpn_protocols = vec![b"http/1.1".to_vec()];
    let socket = NativeStream::connect_timeout(&address, BUDGET).unwrap();
    assert!(!socket.local_addr().unwrap().ip().is_loopback());
    socket.set_read_timeout(Some(BUDGET)).unwrap();
    let mut stream = rustls::StreamOwned::new(
        rustls::ClientConnection::new(
            Arc::new(client),
            rustls::pki_types::ServerName::try_from("owned.test").unwrap(),
        )
        .unwrap(),
        socket,
    );
    let mut request = format!("GET {target} HTTP/1.1\r\nHost: owned.test\r\nContent-Length: 0\r\n");
    if let Some(value) = authorization {
        request.push_str(&format!("Authorization: {value}\r\n"));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);
    String::from_utf8_lossy(&response).into_owned()
}

/// One plaintext admin request.
fn admin_get(address: SocketAddr, target: &str, token: Option<&str>) -> String {
    let mut socket = NativeStream::connect_timeout(&address, BUDGET).unwrap();
    socket.set_read_timeout(Some(BUDGET)).unwrap();
    let mut request = format!("GET {target} HTTP/1.1\r\nHost: admin\r\nContent-Length: 0\r\n");
    if let Some(token) = token {
        request.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    request.push_str("\r\n");
    socket.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    let _ = socket.read_to_string(&mut response);
    response
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unix_public_tls_edge_and_loopback_admin_listener() {
    let interface = interface_ip();
    let fixture = PrivateFixture::new();
    let (tls, root) = provisioned_pair(&fixture.0.join("tls"), "owned.test");
    let mut config = fixture.configuration();
    config.listen = "0.0.0.0:0".parse().unwrap();
    config.public_listen = true;
    config.tls = Some(tls);
    // An enforcing ban gate needs a trusted client address, which a public
    // listener cannot take from PROXY v2; the operator turns the gate off.
    config.edge.ban = Mode::Off;
    config.admin = Some(admin(&fixture.0.join("settings")));
    config.validate().unwrap();
    let prepared = listener::prepare(config.tls.as_ref().unwrap()).unwrap();
    let host = ServiceHost::bind_tls(config, None, None, None, None, prepared.server_config())
        .await
        .unwrap();
    let ready = serde_json::to_value(host.readiness()).unwrap();
    let text = ready.to_string();
    assert!(!text.contains(TOKEN), "token in readiness");
    assert_eq!(ready["profile"]["profile"], "experimental-public-tls");
    assert_eq!(ready["profile"]["public_listen"], true);
    assert_eq!(ready["profile"]["admin_plane"], true);
    assert_eq!(ready["profile"]["original_h2"], true);
    assert_eq!(ready["profile"]["product_qualified"], false);
    assert_eq!(ready["profile"]["public_receive"]["max_receiving"], 64);
    assert_eq!(ready["profile"]["admin_listener"]["operators"], 1);
    assert_eq!(ready["profile"]["admin_listener"]["settings"], true);
    let edge = host.readiness().listen;
    assert!(edge.ip().is_unspecified());
    let admin_address = host.readiness().admin_listen.unwrap();
    assert!(admin_address.ip().is_loopback());
    assert_eq!(
        ready["profile"]["admin_listener"]["listen"],
        admin_address.to_string()
    );

    let (sender, receiver) = tokio::sync::watch::channel(None);
    let running = tokio::spawn(host.run(receiver));
    let public = SocketAddr::new(interface, edge.port());
    let blocking = |work: Box<dyn FnOnce() -> String + Send>| tokio::task::spawn_blocking(work);

    // A non-loopback peer is served on the public listener.
    let probe = {
        let root = root.clone();
        blocking(Box::new(move || {
            tls_get(public, root, "/_humanymous/healthz", None)
        }))
        .await
        .unwrap()
    };
    assert!(probe.starts_with("HTTP/1.1 200"), "{probe}");
    assert!(probe.contains("\"plane\":\"gate\""), "{probe}");
    // The admin plane is not served on the public listener, token or not.
    let refused = {
        let root = root.clone();
        let bearer = format!("Bearer {TOKEN}");
        blocking(Box::new(move || {
            tls_get(public, root, "/_humanymous/admin/whoami", Some(&bearer))
        }))
        .await
        .unwrap()
    };
    assert!(!refused.starts_with("HTTP/1.1 200"), "{refused}");
    assert!(
        !refused.contains("op-1") && !refused.contains("operator"),
        "{refused}"
    );

    // The admin plane is served on the admin listener only.
    let whoami = blocking(Box::new(move || {
        admin_get(admin_address, "/_humanymous/admin/whoami", Some(TOKEN))
    }))
    .await
    .unwrap();
    assert!(whoami.starts_with("HTTP/1.1 200"), "{whoami}");
    assert!(whoami.contains("\"operator\""), "{whoami}");
    let anonymous = blocking(Box::new(move || {
        admin_get(admin_address, "/_humanymous/admin/whoami", None)
    }))
    .await
    .unwrap();
    assert!(anonymous.starts_with("HTTP/1.1 404"), "{anonymous}");
    let settings = blocking(Box::new(move || {
        admin_get(
            admin_address,
            "/_humanymous/admin/settings/effective",
            Some(TOKEN),
        )
    }))
    .await
    .unwrap();
    assert!(settings.starts_with("HTTP/1.1 200"), "{settings}");
    // The admin listener is loopback-bound: the interface address has no listener.
    assert!(
        NativeStream::connect_timeout(&SocketAddr::new(interface, admin_address.port()), BUDGET)
            .is_err()
    );

    sender
        .send(Some(StopBudget::new(1000, 1000, 5000).unwrap().request()))
        .unwrap();
    let report = tokio::time::timeout(BUDGET, running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(report.stopped && report.drained && report.audit_flushed);
    assert!(NativeStream::connect_timeout(&admin_address, BUDGET).is_err());
    assert!(Custody::acquire(&fixture.0.join("keys"), Owner::CurrentUser).is_ok());
}

/// An admin plane whose bans an enforcing edge gate could never apply (no
/// trusted client address) is refused at startup, before readiness.
#[tokio::test]
async fn unix_admin_bans_need_an_enforceable_gate() {
    let fixture = PrivateFixture::new();
    let mut config = fixture.configuration();
    config.admin = Some(admin(&fixture.0.join("settings")));
    assert_eq!(config.edge.ban, Mode::Enforce);
    assert!(ServiceHost::bind(config).await.is_err());
    assert!(Custody::acquire(&fixture.0.join("keys"), Owner::CurrentUser).is_ok());
    // With the gate off the same loopback deployment starts and serves admin.
    let mut config = fixture.configuration();
    config.admin = Some(admin(&fixture.0.join("settings")));
    config.edge.ban = Mode::Off;
    let host = ServiceHost::bind(config).await.unwrap();
    let admin_address = host.readiness().admin_listen.unwrap();
    let (sender, receiver) = tokio::sync::watch::channel(None);
    let running = tokio::spawn(host.run(receiver));
    let whoami = tokio::task::spawn_blocking(move || {
        admin_get(admin_address, "/_humanymous/admin/whoami", Some(TOKEN))
    })
    .await
    .unwrap();
    assert!(whoami.starts_with("HTTP/1.1 200"), "{whoami}");
    sender
        .send(Some(StopBudget::new(1000, 1000, 5000).unwrap().request()))
        .unwrap();
    let report = tokio::time::timeout(BUDGET, running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(report.stopped && report.drained && report.audit_flushed);
}
