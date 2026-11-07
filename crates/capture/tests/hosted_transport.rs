//! Actual socket/TLS pilots for free standard hosted runners only. These are
//! component results; no test here asserts runtime authorization/product e2e.
use humanymous_capture::tls::{Connection, PendingHandshake};
use rustls::{
    ClientConfig, ClientConnection, RootCertStore, ServerConfig,
    pki_types::{PrivatePkcs8KeyDer, ServerName},
};
use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

const BUDGET: Duration = Duration::from_secs(5);

fn hosted() {
    assert_eq!(
        std::env::var("GITHUB_ACTIONS").as_deref(),
        Ok("true"),
        "actual pilots are hosted-only"
    );
    assert_eq!(
        std::env::var("RUNNER_ENVIRONMENT").as_deref(),
        Ok("github-hosted"),
        "free standard runner required"
    );
}
fn configs(alpn: &[u8]) -> (Arc<ServerConfig>, Arc<ClientConfig>) {
    let certified = rcgen::generate_simple_self_signed(vec!["capture.invalid".into()]).unwrap();
    let cert = certified.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    let mut server = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert.clone()], key.into())
        .unwrap();
    server.alpn_protocols = vec![alpn.to_vec()];
    let mut roots = RootCertStore::empty();
    roots.add(cert).unwrap();
    let mut client = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    client.alpn_protocols = vec![alpn.to_vec()];
    (Arc::new(server), Arc::new(client))
}
fn listener() -> TcpListener {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    listener
}
fn accept(listener: &TcpListener) -> TcpStream {
    let deadline = Instant::now() + BUDGET;
    loop {
        match listener.accept() {
            Ok((stream, _)) => return stream,
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(2))
            }
            Err(error) => panic!("bounded accept failed: {error}"),
        }
    }
}
fn client(
    addr: std::net::SocketAddr,
    config: Arc<ClientConfig>,
) -> rustls::StreamOwned<ClientConnection, TcpStream> {
    let socket = TcpStream::connect_timeout(&addr, BUDGET).unwrap();
    socket.set_read_timeout(Some(BUDGET)).unwrap();
    socket.set_write_timeout(Some(BUDGET)).unwrap();
    rustls::StreamOwned::new(
        ClientConnection::new(config, ServerName::try_from("capture.invalid").unwrap()).unwrap(),
        socket,
    )
}
fn frame(kind: u8, flags: u8, stream: u32, payload: &[u8]) -> Vec<u8> {
    let len = payload.len();
    let mut bytes = vec![(len >> 16) as u8, (len >> 8) as u8, len as u8, kind, flags];
    bytes.extend(stream.to_be_bytes());
    bytes.extend(payload);
    bytes
}
fn wire(stream: u32) -> Vec<u8> {
    let mut fields = vec![0x82, 1, 15];
    fields.extend(b"capture.invalid");
    fields.extend([0x87, 0x84]);
    let split = fields.len() / 2;
    [
        b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n".to_vec(),
        frame(4, 0, 0, &[0, 1, 0, 0, 0x10, 0]),
        frame(1, 1, stream, &fields[..split]),
        frame(9, 4, stream, &fields[split..]),
    ]
    .concat()
}

#[test]
#[ignore = "hosted-only actual TCP/TLS component pilot"]
fn actual_tls_h1_never_claims_h2() {
    hosted();
    let (server, config) = configs(b"http/1.1");
    let listener = listener();
    let addr = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let mut conn = Connection::accept(accept(&listener), server, BUDGET).unwrap();
        assert_eq!(conn.negotiated_alpn(), Some(b"http/1.1".as_slice()));
        assert!(
            conn.client_hello()
                .unwrap()
                .alpn
                .iter()
                .any(|name| name == b"http/1.1")
        );
        assert!(conn.h2_request(1).is_none());
        let mut bytes = [0; 4];
        let mut count = 0;
        while count < bytes.len() {
            count += conn.read_plaintext(&mut bytes[count..]).unwrap();
        }
        assert_eq!(&bytes, b"ping");
        conn.write_plaintext(b"pong").unwrap();
    });
    let mut client = client(addr, config);
    client.write_all(b"ping").unwrap();
    client.flush().unwrap();
    let mut reply = [0; 4];
    client.read_exact(&mut reply).unwrap();
    assert_eq!(&reply, b"pong");
    server.join().unwrap();
}

#[test]
#[ignore = "hosted-only actual TCP/TLS/H2 component pilot"]
fn actual_h2_continuation_and_stream_binding() {
    hosted();
    let (server, config) = configs(b"h2");
    let listener = listener();
    let addr = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let mut conn = Connection::accept(accept(&listener), server, BUDGET).unwrap();
        assert_eq!(conn.negotiated_alpn(), Some(b"h2".as_slice()));
        let mut byte = [0];
        let mut count = 0;
        while conn.h2_request(1).is_none() {
            assert_eq!(conn.read_plaintext(&mut byte).unwrap(), 1);
            count += 1;
            assert!(count <= wire(1).len());
            assert!(conn.h2_request(3).is_none());
        }
        let req = conn.h2_request(1).unwrap();
        assert_eq!(req.fingerprint().pseudo_string(), "m,a,s,p");
        assert_eq!(req.fields()[1].1, b"capture.invalid");
        // Minimal actual h2 response: SETTINGS, ACK client settings, HEADERS200.
        conn.write_plaintext(
            &[
                frame(4, 0, 0, &[]),
                frame(4, 1, 0, &[]),
                frame(1, 5, 1, &[0x88]),
            ]
            .concat(),
        )
        .unwrap();
    });
    let mut client = client(addr, config);
    for part in wire(1).chunks(3) {
        client.write_all(part).unwrap();
        client.flush().unwrap();
    }
    let mut response = [0; 28];
    client.read_exact(&mut response).unwrap();
    assert_eq!(response[3], 4);
    assert_eq!(&response[18..], &frame(1, 5, 1, &[0x88]));
    server.join().unwrap();
}

#[test]
#[ignore = "hosted-only actual malformed H2 refusal pilot"]
fn actual_h2_wrong_stream_invalidates_all_capture() {
    hosted();
    let (server, config) = configs(b"h2");
    let listener = listener();
    let addr = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let mut conn = Connection::accept(accept(&listener), server, BUDGET).unwrap();
        let mut bytes = [0; 32];
        let error = loop {
            if let Err(error) = conn.read_plaintext(&mut bytes) {
                break error;
            }
        };
        assert!(
            error.to_string().contains("H2 capture Malformed"),
            "unexpected refusal: {error}"
        );
        assert!(conn.client_hello().is_none());
        assert!(conn.h2_request(1).is_none());
    });
    let mut bytes = wire(1);
    // Change the final CONTINUATION frame stream identifier from1to3.
    let continuation = 24 + 15 + 9 + ((15 + 5) / 2);
    bytes[continuation + 8] = 3;
    let mut client = client(addr, config);
    client.write_all(&bytes).unwrap();
    client.flush().unwrap();
    server.join().unwrap();
}

#[test]
#[ignore = "hosted-only actual TLS handshake failure pilot"]
fn actual_invalid_tls_never_creates_an_owner() {
    hosted();
    let (server, _) = configs(b"h2");
    let listener = listener();
    let addr = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        assert!(Connection::accept(accept(&listener), server, BUDGET).is_err())
    });
    let mut socket = TcpStream::connect_timeout(&addr, BUDGET).unwrap();
    socket.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();
    socket.shutdown(std::net::Shutdown::Write).unwrap();
    server.join().unwrap();
}

#[test]
#[ignore = "hosted-only actual supervisor cancellation pilot"]
fn actual_supervisor_cancellation_revokes_the_connection_epoch() {
    hosted();
    // Cancel before dispatch: the pending epoch must never enter TLS I/O.
    let (server, _) = configs(b"http/1.1");
    let listener = listener();
    let addr = listener.local_addr().unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let (resume, wait) = std::sync::mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        let pending = PendingHandshake::new(accept(&listener), server, BUDGET).unwrap();
        send.send(pending.cancellation().unwrap()).unwrap();
        wait.recv_timeout(BUDGET).unwrap();
        assert!(pending.remaining().is_err());
        let result = pending.accept();
        assert_eq!(
            result.err().unwrap().kind(),
            io::ErrorKind::ConnectionAborted
        );
    });
    let _peer = TcpStream::connect_timeout(&addr, BUDGET).unwrap();
    let cancellation = receive.recv_timeout(BUDGET).unwrap();
    cancellation.cancel().unwrap();
    cancellation.cancel().unwrap();
    resume.send(()).unwrap();
    worker.join().unwrap();

    // Observe an actual server TLS flight before revoking a pending handshake.
    // The client deliberately never sends its Finished, so no owner can qualify.
    let (server, config) = configs(b"http/1.1");
    let listener = self::listener();
    let addr = listener.local_addr().unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        let pending = PendingHandshake::new(accept(&listener), server, BUDGET).unwrap();
        send.send(pending.cancellation().unwrap()).unwrap();
        assert!(pending.accept().is_err());
    });
    let mut peer = TcpStream::connect_timeout(&addr, BUDGET).unwrap();
    peer.set_read_timeout(Some(BUDGET)).unwrap();
    peer.set_write_timeout(Some(BUDGET)).unwrap();
    let mut tls =
        ClientConnection::new(config, ServerName::try_from("capture.invalid").unwrap()).unwrap();
    while tls.wants_write() {
        tls.write_tls(&mut peer).unwrap();
    }
    let mut first_flight = [0];
    assert_eq!(peer.read(&mut first_flight).unwrap(), 1);
    let cancellation = receive.recv_timeout(BUDGET).unwrap();
    cancellation.cancel().unwrap();
    worker.join().unwrap();

    // A surviving supervisor handle must not keep a dropped pending socket live.
    let (server, _) = configs(b"http/1.1");
    let listener = self::listener();
    let addr = listener.local_addr().unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        let pending = PendingHandshake::new(accept(&listener), server, BUDGET).unwrap();
        send.send(pending.cancellation().unwrap()).unwrap();
        drop(pending);
    });
    let mut peer = TcpStream::connect_timeout(&addr, BUDGET).unwrap();
    peer.set_read_timeout(Some(BUDGET)).unwrap();
    let surviving = receive.recv_timeout(BUDGET).unwrap();
    assert_eq!(peer.read(&mut [0]).unwrap(), 0);
    worker.join().unwrap();
    surviving.cancel().unwrap();

    // A pre-handshake handle also revokes the successfully handshaken owner.
    let (server, config) = configs(b"http/1.1");
    let listener = self::listener();
    let addr = listener.local_addr().unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let server = thread::spawn(move || {
        let pending = PendingHandshake::new(accept(&listener), server, BUDGET).unwrap();
        let cancellation = pending.cancellation().unwrap();
        let mut conn = pending.accept().unwrap();
        let mut first = [0];
        assert_eq!(conn.read_plaintext(&mut first).unwrap(), 1);
        send.send(cancellation).unwrap();
        assert!(conn.read_plaintext(&mut first).is_err());
        assert!(conn.client_hello().is_none());
        assert!(conn.negotiated_alpn().is_none());
        assert!(conn.remaining().is_err());
        assert!(conn.write_plaintext(b"late").is_err());
    });
    let mut client = client(addr, config);
    client.write_all(b"x").unwrap();
    client.flush().unwrap();
    let cancellation = receive.recv_timeout(BUDGET).unwrap();
    cancellation.cancel().unwrap();
    server.join().unwrap();
}
