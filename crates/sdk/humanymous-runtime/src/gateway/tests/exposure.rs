//! Owner decision B01 #1: the Gate configuration's public opt-in and the
//! accept loop's listener refusal, independent of custody.
use super::*;
use crate::gateway::service::ShutdownRequest;

fn tls_object() -> crate::gateway::tls::listener::ListenerTls {
    serde_json::from_value(json!({"certificate":"/x/chain.pem","key":"/x/key.pem"})).unwrap()
}

#[test]
fn public_edge_configuration_needs_the_key_and_tls() {
    let mut cfg = config();
    assert!(cfg.validate().is_ok());
    assert_eq!(cfg.readiness()["public_listen"], false);
    // A non-loopback listen without the key is refused, with or without TLS.
    cfg.listen = "0.0.0.0:8444".parse().unwrap();
    assert!(cfg.validate().is_err());
    cfg.tls = Some(tls_object());
    assert!(cfg.validate().is_err());
    // With the key and a provisioned TLS source it is accepted.
    cfg.public_listen = true;
    assert!(cfg.validate().is_ok());
    assert_eq!(cfg.readiness()["public_listen"], true);
    // No public plaintext listener.
    cfg.tls = None;
    assert!(cfg.validate().is_err());
    // No public development certificate.
    cfg.tls = Some(serde_json::from_value(json!({"dev_self_signed":true})).unwrap());
    assert!(cfg.validate().is_err());
    cfg.tls = Some(tls_object());
    // The origin stays loopback.
    cfg.origin = "192.0.2.10:9000".parse().unwrap();
    assert!(cfg.validate().is_err());
    cfg.origin = "127.0.0.1:32123".parse().unwrap();
    // PROXY v2 client addresses are trusted only from a loopback listener.
    cfg.ip_reputation = Some(
        serde_json::from_value(json!({"snapshot":"/x/snapshot","client_address":"proxy_v2"}))
            .unwrap(),
    );
    assert!(cfg.validate().is_err());
    cfg.ip_reputation = None;
    assert!(cfg.validate().is_ok());
    // The key on a loopback address is refused.
    cfg.listen = "127.0.0.1:8444".parse().unwrap();
    assert!(cfg.validate().is_err());
}

#[test]
fn admin_listener_configuration_is_loopback_and_distinct() {
    let admin = |listen: &str| -> crate::gateway::configuration::AdminListener {
        serde_json::from_value(json!({
            "listen": listen,
            "operators": [{"role":"auditor","id":"aud-1","token":"operator-fixture-token-a"}],
            "bans": {"capacity":64,"max_keys":64,"max_hits_per_key":256,"max_key_bytes":128,
                "max_payload_bytes":1_048_576,"gc_scan_budget":64},
        }))
        .unwrap()
    };
    let mut cfg = config();
    cfg.admin = Some(admin("127.0.0.1:8445"));
    assert!(cfg.validate().is_ok());
    assert_eq!(cfg.readiness()["admin_plane"], true);
    cfg.admin = Some(admin("0.0.0.0:8445"));
    assert!(cfg.validate().is_err());
    cfg.admin = Some(admin("192.0.2.10:8445"));
    assert!(cfg.validate().is_err());
    // A public edge does not widen the admin listener.
    cfg.listen = "0.0.0.0:8444".parse().unwrap();
    cfg.public_listen = true;
    cfg.tls = Some(tls_object());
    assert!(cfg.validate().is_err());
    cfg.admin = Some(admin("127.0.0.1:8445"));
    assert!(cfg.validate().is_ok());
    // The admin address cannot be the origin.
    cfg.admin = Some(admin("127.0.0.1:32123"));
    assert!(cfg.validate().is_err());
}

/// A loopback-only configuration never serves a listener bound beyond
/// loopback, so no non-loopback peer reaches it; plaintext never serves one.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loopback_only_application_refuses_a_wide_listener() {
    let (server_tls, _) = tls::configurations();
    let mut app =
        Application::new(config(), Default::default(), Box::new(Audit::default())).unwrap();
    let (_stop, stopping) = tokio::sync::watch::channel::<Option<ShutdownRequest>>(None);
    let wide = TcpListener::bind("0.0.0.0:0").await.unwrap();
    assert!(
        app.serve_tls_until(wide, server_tls, stopping.clone())
            .await
            .is_err()
    );
    let wide = TcpListener::bind("0.0.0.0:0").await.unwrap();
    assert!(app.serve_until(wide, stopping).await.is_err());
}

/// An explicitly public application serves a non-loopback peer over TLS and
/// still refuses to serve its listener in plaintext.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn public_application_serves_a_non_loopback_peer_over_tls_only() {
    use std::io::{Read as _, Write as _};
    let interface = interface_ip();
    let mut cfg = config();
    cfg.listen = "0.0.0.0:0".parse().unwrap();
    cfg.public_listen = true;
    cfg.tls = Some(tls_object());
    let (server_tls, client_tls) = tls::configurations();
    let mut plain = Application::new(
        {
            let mut cfg = config();
            cfg.listen = "0.0.0.0:0".parse().unwrap();
            cfg.public_listen = true;
            cfg.tls = Some(tls_object());
            cfg
        },
        Default::default(),
        Box::new(Audit::default()),
    )
    .unwrap();
    let (_stop_plain, stopping_plain) =
        tokio::sync::watch::channel::<Option<ShutdownRequest>>(None);
    let wide = TcpListener::bind("0.0.0.0:0").await.unwrap();
    assert!(plain.serve_until(wide, stopping_plain).await.is_err());

    let mut app = Application::new(cfg, Default::default(), Box::new(Audit::default())).unwrap();
    let listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (stop, stopping) = tokio::sync::watch::channel(None);
    let server =
        tokio::spawn(async move { app.serve_tls_until(listener, server_tls, stopping).await });
    let address = std::net::SocketAddr::new(interface, port);
    let response = tokio::task::spawn_blocking(move || {
        let socket =
            std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_secs(3))
                .unwrap();
        // The application sees a non-loopback peer.
        assert!(!socket.peer_addr().unwrap().ip().is_loopback());
        assert!(!socket.local_addr().unwrap().ip().is_loopback());
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        let mut stream = rustls::StreamOwned::new(
            rustls::ClientConnection::new(
                client_tls,
                rustls::pki_types::ServerName::try_from("owned.test").unwrap(),
            )
            .unwrap(),
            socket,
        );
        stream
            .write_all(b"GET /_humanymous/healthz HTTP/1.1\r\nHost: owned.test\r\nContent-Length: 0\r\n\r\n")
            .unwrap();
        let mut response = Vec::new();
        let _ = stream.read_to_end(&mut response);
        String::from_utf8_lossy(&response).into_owned()
    })
    .await
    .unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(
        response.ends_with("{\"plane\":\"gate\",\"status\":\"ok\"}\n"),
        "{response}"
    );
    stop.send(Some(ShutdownRequest::new(100).unwrap())).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

/// A public application serving on a wide listener with the given receive
/// bounds and connection lifetime; returns its interface address.
async fn public_app(
    max_receiving: usize,
    receive_ms: u64,
    timeout_ms: u64,
) -> (
    std::net::SocketAddr,
    tokio::sync::watch::Sender<Option<ShutdownRequest>>,
    tokio::task::JoinHandle<std::io::Result<()>>,
    Arc<rustls::ClientConfig>,
) {
    let mut cfg = config();
    cfg.listen = "0.0.0.0:0".parse().unwrap();
    cfg.public_listen = true;
    cfg.tls = Some(tls_object());
    cfg.timeout_ms = timeout_ms;
    cfg.public_receive = crate::gateway::configuration::exposure::PublicReceive {
        max_receiving,
        receive_ms,
    };
    cfg.validate().unwrap();
    let (server_tls, client_tls) = tls::configurations();
    let mut app = Application::new(cfg, Default::default(), Box::new(Audit::default())).unwrap();
    let listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
    let address = std::net::SocketAddr::new(interface_ip(), listener.local_addr().unwrap().port());
    let (stop, stopping) = tokio::sync::watch::channel(None);
    let server =
        tokio::spawn(async move { app.serve_tls_until(listener, server_tls, stopping).await });
    (address, stop, server, client_tls)
}

/// One TLS healthz request from the interface address; the response text.
fn public_healthz(address: std::net::SocketAddr, client_tls: Arc<rustls::ClientConfig>) -> String {
    use std::io::{Read as _, Write as _};
    let socket =
        std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_secs(3)).unwrap();
    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let mut stream = rustls::StreamOwned::new(
        rustls::ClientConnection::new(
            client_tls,
            rustls::pki_types::ServerName::try_from("owned.test").unwrap(),
        )
        .unwrap(),
        socket,
    );
    stream
        .write_all(
            b"GET /_humanymous/healthz HTTP/1.1\r\nHost: owned.test\r\nContent-Length: 0\r\n\r\n",
        )
        .unwrap();
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);
    String::from_utf8_lossy(&response).into_owned()
}

/// Read from a raw socket until EOF, an error or `wait`; true when the
/// server closed the connection within `wait`.
fn closed_within(socket: &mut std::net::TcpStream, wait: std::time::Duration) -> bool {
    use std::io::Read as _;
    socket.set_read_timeout(Some(wait)).unwrap();
    let mut byte = [0u8; 1];
    match socket.read(&mut byte) {
        Ok(0) => true,
        Ok(_) => false,
        Err(error) => !matches!(
            error.kind(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
        ),
    }
}

async fn stop_public(
    stop: tokio::sync::watch::Sender<Option<ShutdownRequest>>,
    server: tokio::task::JoinHandle<std::io::Result<()>>,
) {
    stop.send(Some(ShutdownRequest::new(100).unwrap())).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[test]
fn public_receive_bounds_are_validated() {
    use crate::gateway::configuration::exposure::PublicReceive;
    let mut cfg = config();
    assert_eq!(cfg.public_receive, PublicReceive::default());
    for (max_receiving, receive_ms) in [(0, 5_000), (1025, 5_000), (8, 99), (8, 60_001)] {
        cfg.public_receive = PublicReceive {
            max_receiving,
            receive_ms,
        };
        assert!(cfg.validate().is_err(), "{max_receiving} {receive_ms}");
    }
    cfg.public_receive = PublicReceive {
        max_receiving: 1,
        receive_ms: 100,
    };
    assert!(cfg.validate().is_ok());
    // The receive deadline never exceeds the connection lifetime.
    assert_eq!(
        PublicReceive::default().deadline(1_000),
        std::time::Duration::from_millis(1_000)
    );
}

/// Review finding on PR #156: a peer that connects and sends nothing must
/// not hold the public listener. With a 30 s connection lifetime and a 10 s
/// receive deadline, a second client is still served at once.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn silent_public_peer_does_not_block_another_client() {
    let (address, stop, server, client_tls) = public_app(8, 10_000, 30_000).await;
    let silent = tokio::task::spawn_blocking(move || {
        std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_secs(3)).unwrap()
    })
    .await
    .unwrap();
    // Let the listener accept the silent peer before the real client.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let started = std::time::Instant::now();
    let response = tokio::task::spawn_blocking(move || public_healthz(address, client_tls))
        .await
        .unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "served after {:?}",
        started.elapsed()
    );
    // The silent peer was still waiting, not closed.
    let mut silent = silent;
    assert!(
        !tokio::task::spawn_blocking(move || closed_within(
            &mut silent,
            std::time::Duration::from_millis(200)
        ))
        .await
        .unwrap()
    );
    stop_public(stop, server).await;
}

/// Beyond `max_receiving` a peer is closed at once instead of queued, and a
/// silent peer is closed at its receive deadline, freeing its permit.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn public_receive_overflow_closes_and_silent_peers_expire() {
    let (address, stop, server, client_tls) = public_app(1, 1_500, 30_000).await;
    let (holder_closed, overflow_closed, served) = tokio::task::spawn_blocking(move || {
        let connect = || {
            std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_secs(3))
                .unwrap()
        };
        let mut holder = connect();
        std::thread::sleep(std::time::Duration::from_millis(100));
        // The only permit is held by the silent peer: the next one is closed
        // long before the 1.5 s receive deadline.
        let started = std::time::Instant::now();
        let mut overflow = connect();
        let overflow_closed = closed_within(&mut overflow, std::time::Duration::from_secs(1))
            && started.elapsed() < std::time::Duration::from_secs(1);
        // The silent holder is closed at its receive deadline.
        let holder_closed = closed_within(&mut holder, std::time::Duration::from_secs(5));
        // Its permit is free again.
        let served = public_healthz(address, client_tls);
        (holder_closed, overflow_closed, served)
    })
    .await
    .unwrap();
    assert!(overflow_closed, "overflow peer was queued");
    assert!(holder_closed, "silent peer outlived its receive deadline");
    assert!(served.starts_with("HTTP/1.1 200"), "{served}");
    stop_public(stop, server).await;
}
