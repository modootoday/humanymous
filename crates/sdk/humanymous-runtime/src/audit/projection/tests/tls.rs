//! HTTPS ClickHouse inserts (Go `net/http` spoke HTTPS too) against an
//! in-process rustls server with a throwaway self-signed certificate.
use super::super::{
    RecordSink,
    clickhouse::{ClickHouseOptions, ClickHouseSink},
};
use super::{parity::read_http_request, sample_record};
use rustls::{ClientConfig, RootCertStore, ServerConfig, ServerConnection, StreamOwned};
use std::{
    io::{BufReader, Write},
    net::TcpListener,
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

/// Request line and body of one insert the TLS stand-in received, or `None`
/// when no request arrived (a failed handshake).
type Received = Option<(String, Vec<u8>)>;

fn certified() -> (
    rustls::pki_types::CertificateDer<'static>,
    rustls::pki_types::PrivateKeyDer<'static>,
) {
    let certified = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
    let key = rustls::pki_types::PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    (certified.cert.der().clone(), key.into())
}

fn client_trusting(certificate: rustls::pki_types::CertificateDer<'static>) -> Arc<ClientConfig> {
    let mut roots = RootCertStore::empty();
    roots.add(certificate).unwrap();
    Arc::new(
        ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}

/// TLS stand-in: answers each insert with 200 and reports the request line
/// and body, or reports a failed handshake as `None`.
fn tls_stand_in(
    certificate: rustls::pki_types::CertificateDer<'static>,
    key: rustls::pki_types::PrivateKeyDer<'static>,
) -> (String, mpsc::Receiver<Received>) {
    let server = Arc::new(
        ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![certificate], key)
            .unwrap(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { return };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let connection = ServerConnection::new(Arc::clone(&server)).unwrap();
            let mut tls = StreamOwned::new(connection, stream);
            let request = read_http_request(&mut BufReader::new(&mut tls));
            let seen = request.map(|(line, _, body)| {
                let _ = tls.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
                let _ = tls.flush();
                tls.conn.send_close_notify();
                let _ = tls.flush();
                (line, body)
            });
            if sender.send(seen).is_err() {
                return;
            }
        }
    });
    (format!("https://{address}"), receiver)
}

fn options(endpoint: String) -> ClickHouseOptions {
    ClickHouseOptions {
        endpoint,
        table: "audit_log".into(),
        max_rows: 100,
        max_wait: Duration::from_secs(3600),
        backlog: 10,
    }
}

/// An `https://` endpoint inserts over TLS with the supplied trust roots,
/// sending the same request line and body as plain HTTP.
#[test]
fn https_insert_uses_the_supplied_trust_roots() {
    let (certificate, key) = certified();
    let client = client_trusting(certificate.clone());
    let (endpoint, requests) = tls_stand_in(certificate, key);
    let sink = ClickHouseSink::with_tls(options(endpoint), client).unwrap();
    sink.append_record(&sample_record(7)).unwrap();
    sink.flush().expect("https insert");
    let (line, body) = requests
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .expect("the TLS stand-in saw no request");
    assert!(
        line.starts_with("POST /?query=INSERT+INTO+audit_log+FORMAT+JSONEachRow&"),
        "{line}"
    );
    let shipped: crate::audit::record::Record =
        serde_json::from_slice(&body[..body.len() - 1]).unwrap();
    assert_eq!(shipped, sample_record(7));
    assert_eq!(sink.dropped(), 0);
    sink.close().unwrap();
}

/// A server whose certificate the supplied roots do not trust gets no
/// request: the handshake fails, the insert fails and its rows are counted.
/// `new` refuses `https://` outright rather than pick trust roots.
#[test]
fn https_refuses_an_untrusted_server_and_needs_explicit_roots() {
    let (certificate, key) = certified();
    let (other, _) = certified();
    let (endpoint, requests) = tls_stand_in(certificate, key);
    assert!(ClickHouseSink::new(options(endpoint.clone())).is_err());
    let sink = ClickHouseSink::with_tls(options(endpoint), client_trusting(other)).unwrap();
    sink.append_record(&sample_record(1)).unwrap();
    sink.append_record(&sample_record(2)).unwrap();
    assert!(sink.flush().is_err());
    assert_eq!(sink.dropped(), 2);
    assert_eq!(requests.recv_timeout(Duration::from_secs(5)).unwrap(), None);
    sink.close().unwrap();
}
