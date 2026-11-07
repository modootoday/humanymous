//! Actual owned TLS sockets; synthetic client/audit fixtures are not browser proof.
mod dispatch;
mod observation;
use super::*;
use crate::gateway::service::ShutdownRequest;
use rustls::{ClientConfig, ClientConnection, RootCertStore, ServerConfig};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream as NativeStream},
    time::Duration,
};

const BUDGET: Duration = Duration::from_secs(3);

fn configurations() -> (Arc<ServerConfig>, Arc<ClientConfig>) {
    let certified = rcgen::generate_simple_self_signed(vec!["owned.test".into()]).unwrap();
    let certificate = certified.cert.der().clone();
    let key = rustls::pki_types::PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    let mut server = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![certificate.clone()], key.into())
        .unwrap();
    server.alpn_protocols = vec![b"http/1.1".to_vec()];
    let mut roots = RootCertStore::empty();
    roots.add(certificate).unwrap();
    let mut client = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    client.alpn_protocols = vec![b"http/1.1".to_vec()];
    (Arc::new(server), Arc::new(client))
}

fn client(
    address: SocketAddr,
    config: Arc<ClientConfig>,
) -> rustls::StreamOwned<ClientConnection, NativeStream> {
    let socket = NativeStream::connect_timeout(&address, BUDGET).unwrap();
    socket.set_read_timeout(Some(BUDGET)).unwrap();
    socket.set_write_timeout(Some(BUDGET)).unwrap();
    rustls::StreamOwned::new(
        ClientConnection::new(
            config,
            rustls::pki_types::ServerName::try_from("owned.test").unwrap(),
        )
        .unwrap(),
        socket,
    )
}

async fn exchange_tls(address: SocketAddr, config: Arc<ClientConfig>, bytes: Vec<u8>) -> Vec<u8> {
    tokio::task::spawn_blocking(move || {
        let mut socket = client(address, config);
        socket.write_all(&bytes).unwrap();
        socket.flush().unwrap();
        assert_eq!(socket.conn.alpn_protocol(), Some(b"http/1.1".as_slice()));
        let mut response = Vec::new();
        match socket.read_to_end(&mut response) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {}
            Err(error) => panic!("TLS response failed: {error}"),
        }
        response
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actual_tls_coalesced_request_is_refused_before_audit_or_origin() {
    refused_frame(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actual_tls_partial_body_is_refused_before_audit_or_origin() {
    refused_frame(true).await;
}

async fn refused_frame(partial: bool) {
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = configuration(origin.local_addr().unwrap());
    let audit = Audit::default();
    let mut app = Application::new(
        configuration(config.origin),
        Default::default(),
        Box::new(audit.clone()),
    )
    .unwrap();
    let now = crate::authority::now_ms().unwrap();
    let issued = issue(&mut app, now);
    collected(&mut app, &config, &issued, now);
    let protected = protected(&config, &issued, 2, now);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (server_tls, client_tls) = configurations();
    let (stop, stopping) = tokio::sync::watch::channel(None);
    let server =
        tokio::spawn(async move { app.serve_tls_until(listener, server_tls, stopping).await });
    let mut bytes = encode(&protected);
    if partial {
        bytes.pop();
    } else {
        bytes.extend_from_slice(b"GET /effect HTTP/1.1\r\nHost: owned.test\r\n\r\n");
    }
    let response = exchange_tls(address, client_tls, bytes).await;
    stop.send(Some(ShutdownRequest::new(100).unwrap())).unwrap();
    tokio::time::timeout(BUDGET, server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(response.is_empty());
    assert_eq!(audit.records.lock().unwrap().len(), 1);
    assert!(
        tokio::time::timeout(Duration::from_millis(30), origin.accept())
            .await
            .is_err()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_actual_tls_handshake_joins_worker_and_closes_owned_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut peer = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (socket, _) = listener.accept().await.unwrap();
    let (server_tls, _) = configurations();
    let pending = crate::gateway::tls::CapturedRequest::receive(
        socket.into_std().unwrap(),
        server_tls,
        BUDGET,
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(30), pending)
            .await
            .is_err()
    );
    // timeout dropped the future; its synchronous Drop cancels and joins before
    // returning. The peer observes the same socket closed, not a detached worker.
    let mut byte = [0; 1];
    assert_eq!(
        tokio::time::timeout(BUDGET, peer.read(&mut byte))
            .await
            .unwrap()
            .unwrap(),
        0
    );
}
