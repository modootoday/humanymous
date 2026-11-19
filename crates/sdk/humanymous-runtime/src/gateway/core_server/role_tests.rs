//! Core role startup, readiness and exposure over real sockets.
use super::*;
use crate::gateway::tests::{interface_ip, provisioned_pair};
use crate::gateway::tls::listener;
use serde_json::json;
use std::io::Write as _;
use std::net::{IpAddr, TcpStream};
use std::time::SystemTime;

const BUDGET: Duration = Duration::from_secs(5);

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let mut nonce = [0u8; 8];
        getrandom::fill(&mut nonce).unwrap();
        let path = std::env::temp_dir().join(format!("hmn-core-role-{tag}-{}", hex::encode(nonce)));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn document(listen: &str, web: &Scratch) -> serde_json::Value {
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

fn role(value: serde_json::Value) -> CoreRole {
    serde_json::from_value(value).unwrap()
}

fn clock() -> Clock {
    Arc::new(SystemTime::now)
}

fn plain_get(address: SocketAddr, source: Option<IpAddr>) -> String {
    let socket = match source {
        Some(ip) => {
            let socket = socket2::Socket::new(
                socket2::Domain::for_address(address),
                socket2::Type::STREAM,
                None,
            )
            .unwrap();
            socket.bind(&SocketAddr::new(ip, 0).into()).unwrap();
            socket.connect_timeout(&address.into(), BUDGET).unwrap();
            TcpStream::from(socket)
        }
        None => TcpStream::connect_timeout(&address, BUDGET).unwrap(),
    };
    socket.set_read_timeout(Some(BUDGET)).unwrap();
    let mut socket = socket;
    socket
        .write_all(b"GET /healthz HTTP/1.1\r\nHost: core.test\r\n\r\n")
        .unwrap();
    let mut response = String::new();
    let _ = socket.read_to_string(&mut response);
    response
}

#[test]
fn core_listen_is_loopback_unless_public_with_a_real_tls_source() {
    let web = Scratch::new("refusals");
    assert!(role(document("127.0.0.1:0", &web)).validate().is_ok());
    // A non-loopback listen without the key is refused, TLS or not.
    assert!(role(document("0.0.0.0:0", &web)).validate().is_err());
    let mut tls = document("0.0.0.0:0", &web);
    tls["tls"] = json!({"certificate": "/x/chain.pem", "key": "/x/key.pem"});
    assert!(role(tls.clone()).validate().is_err());
    // The key without TLS (public plaintext) is refused.
    let mut plain = document("0.0.0.0:0", &web);
    plain["public_listen"] = json!(true);
    assert!(role(plain).validate().is_err());
    // The key with the development certificate is refused.
    let mut dev = document("0.0.0.0:0", &web);
    dev["public_listen"] = json!(true);
    dev["tls"] = json!({"dev_self_signed": true});
    assert!(role(dev).validate().is_err());
    // The key on a loopback address is refused.
    let mut loopback = document("127.0.0.1:0", &web);
    loopback["public_listen"] = json!(true);
    loopback["tls"] = json!({"certificate": "/x/chain.pem", "key": "/x/key.pem"});
    assert!(role(loopback).validate().is_err());
    // The key with a provisioned pair is accepted.
    tls["public_listen"] = json!(true);
    assert!(role(tls).validate().is_ok());
    // Go -max-conns 0 (disabled) is refused; bounds and keys fail closed.
    let mut zero = document("127.0.0.1:0", &web);
    zero["max_conns"] = json!(0);
    assert!(role(zero).validate().is_err());
    let mut short = document("127.0.0.1:0", &web);
    short["token_key_hex"] = json!("00ff");
    assert!(role(short).validate().is_err());
    let mut proxies = document("127.0.0.1:0", &web);
    proxies["trusted_proxies"] = json!(["10.0.0.0/33"]);
    assert!(role(proxies).validate().is_err());
    let mut limits = document("127.0.0.1:0", &web);
    limits["limits"]["rate"][0] = json!(0);
    assert!(role(limits).validate().is_err());
    let mut unknown = document("127.0.0.1:0", &web);
    unknown["ops_token"] = json!("x");
    assert!(serde_json::from_value::<CoreRole>(unknown).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn core_role_starts_on_loopback_and_reports_readiness() {
    let web = Scratch::new("loopback");
    let mut document = document("127.0.0.1:0", &web);
    document["token_key_hex"] = json!(hex::encode([7u8; 32]));
    let role = read_role(serde_json::to_vec(&document).unwrap().as_slice()).unwrap();
    let service = CoreService::bind(&role, None, clock()).await.unwrap();
    let ready = service.readiness().clone();
    assert_eq!(ready["role"], "core");
    assert_eq!(
        ready["profile"]["profile"],
        "experimental-core-loopback-http1"
    );
    assert_eq!(ready["profile"]["public_listen"], false);
    assert_eq!(ready["profile"]["raw_tls"], false);
    assert_eq!(ready["profile"]["original_h2"], false);
    assert_eq!(ready["profile"]["pass"], true);
    assert_eq!(ready["profile"]["step_up_receipts"], true);
    assert_eq!(ready["profile"]["product_qualified"], false);
    let text = ready.to_string();
    assert!(!text.contains(&hex::encode([7u8; 32])), "key in readiness");
    let address: SocketAddr = ready["listen"].as_str().unwrap().parse().unwrap();
    assert!(address.ip().is_loopback() && address.port() != 0);
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(service.run(async {
        let _ = stopped.await;
    }));
    let health = tokio::task::spawn_blocking(move || plain_get(address, None))
        .await
        .unwrap();
    assert!(health.starts_with("HTTP/1.1 200 OK\r\n"), "{health}");
    assert!(health.contains("\"status\":\"ok\""), "{health}");
    let pass = {
        let socket = TcpStream::connect_timeout(&address, BUDGET).unwrap();
        tokio::task::spawn_blocking(move || {
            let mut socket = socket;
            socket.set_read_timeout(Some(BUDGET)).unwrap();
            socket
                .write_all(
                    b"POST /api/pass/new HTTP/1.1\r\nHost: core.test\r\nContent-Length: 0\r\n\r\n",
                )
                .unwrap();
            let mut response = String::new();
            let _ = socket.read_to_string(&mut response);
            response
        })
        .await
        .unwrap()
    };
    // The Pass flow is mounted: the route is not the unmounted 404.
    assert!(!pass.starts_with("HTTP/1.1 404"), "{pass}");
    stop.send(()).unwrap();
    tokio::time::timeout(BUDGET, task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    // A TLS request without its prepared source is refused before binding.
    let mut tls = document.clone();
    tls["tls"] = json!({"certificate": "/x/chain.pem", "key": "/x/key.pem"});
    assert!(
        CoreService::bind(&role_of(tls), None, clock())
            .await
            .is_err()
    );
}

fn role_of(value: serde_json::Value) -> CoreRole {
    serde_json::from_value(value).unwrap()
}

/// A non-loopback peer reaches an explicitly public TLS Core; a listener
/// bound beyond loopback without the opt-in is refused before serving.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn public_core_serves_non_loopback_peers_over_tls_only() {
    let interface = interface_ip();
    let web = Scratch::new("public");
    let material = Scratch::new("public-tls");
    let (tls, certificate) = provisioned_pair(&material.0, "core.test");
    let mut document = document("0.0.0.0:0", &web);
    document["public_listen"] = json!(true);
    document["tls"] = json!({"certificate": tls.certificate, "key": tls.key});
    let role = role_of(document);
    let prepared = listener::prepare(role.tls.as_ref().unwrap()).unwrap();
    let offer = core_tls(&prepared);
    assert!(!offer.alpn_protocols.iter().any(|p| p == b"h2"));
    let service = CoreService::bind(&role, Some(offer), clock())
        .await
        .unwrap();
    let ready = service.readiness().clone();
    assert_eq!(
        ready["profile"]["profile"],
        "experimental-core-public-tls-h1"
    );
    assert_eq!(ready["profile"]["public_listen"], true);
    assert_eq!(ready["profile"]["original_h2"], false);
    assert_eq!(ready["profile"]["alpn"], json!(["http/1.1"]));
    let bound: SocketAddr = ready["listen"].as_str().unwrap().parse().unwrap();
    assert!(bound.ip().is_unspecified());
    let address = SocketAddr::new(interface, bound.port());
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(service.run(async {
        let _ = stopped.await;
    }));
    let response = tokio::task::spawn_blocking(move || {
        let mut roots = rustls::RootCertStore::empty();
        roots.add(certificate).unwrap();
        let mut client = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        let socket = TcpStream::connect_timeout(&address, BUDGET).unwrap();
        // The peer address the Core sees is the interface, not loopback.
        assert!(!socket.local_addr().unwrap().ip().is_loopback());
        socket.set_read_timeout(Some(BUDGET)).unwrap();
        let mut stream = rustls::StreamOwned::new(
            rustls::ClientConnection::new(
                Arc::new(client),
                rustls::pki_types::ServerName::try_from("core.test").unwrap(),
            )
            .unwrap(),
            socket,
        );
        stream
            .write_all(b"GET /healthz HTTP/1.1\r\nHost: core.test\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response);
        assert_eq!(stream.conn.alpn_protocol(), Some(b"http/1.1".as_slice()));
        response
    })
    .await
    .unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    // Plaintext to the public TLS listener gets no HTTP answer.
    let plain = tokio::task::spawn_blocking(move || plain_get(address, Some(interface)))
        .await
        .unwrap();
    assert!(!plain.contains("200 OK"), "{plain}");
    stop.send(()).unwrap();
    tokio::time::timeout(BUDGET, task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    // Without the opt-in a non-loopback listener is refused, TLS or not, so
    // no non-loopback peer is ever accepted by a loopback-only Core.
    let server = Arc::new(
        CoreServer::new(
            Config {
                web_dir: web.0.to_str().unwrap().to_owned(),
                master_key: Zeroizing::new(vec![9; 32]),
                rit_enabled: true,
                ops_token: String::new(),
                trusted_proxies: Vec::new(),
                version: "dev".into(),
                watermark_fallback: Fallback::OpaqueTrailer,
                limits: super::super::tests::limits(),
            },
            clock(),
        )
        .unwrap(),
    );
    for (public, tls) in [
        (false, None),
        (false, Some(core_tls(&prepared))),
        (true, None),
    ] {
        let wide = TcpListener::bind("0.0.0.0:0").await.unwrap();
        let options = super::super::serve::Options {
            tls,
            budget: BUDGET,
            cap: ConnectionCap::new(1).unwrap(),
            public,
        };
        let refused = super::super::serve::serve(server.clone(), wide, options, async {}).await;
        assert!(refused.is_err(), "public={public}");
    }
}
