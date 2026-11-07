//! Hosted-only incremental socket proof with synthetic report/audit fixtures.
use super::*;
use crate::configuration::ResponseProfile;

fn configured(origin: std::net::SocketAddr) -> Configuration {
    let mut config = configuration(origin);
    config.timeout_ms = 5000;
    config.routes[0].response_profile = ResponseProfile::NativeStream;
    config
}
#[tokio::test]
async fn native_stream_delivers_first_event_before_origin_is_allowed_to_finish() {
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_address = origin.local_addr().unwrap();
    let audit = Audit::default();
    let origin_audit = audit.clone();
    let (release, finish) = tokio::sync::oneshot::channel();
    let origin_task = tokio::spawn(async move {
        let (mut socket, _) = origin.accept().await.unwrap();
        assert_eq!(origin_audit.records.lock().unwrap().len(), 2);
        let request = crate::http::read(&mut socket).await.unwrap();
        assert_eq!(request.body, b"original bytes");
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\nb\r\ndata: one\n\n\r\n").await.unwrap();
        finish.await.unwrap();
        socket
            .write_all(b"b\r\ndata: two\n\n\r\n0\r\n\r\n")
            .await
            .unwrap();
        socket.shutdown().await.unwrap();
    });
    let binding = configured(origin_address);
    let mut app = Application::new(
        configured(origin_address),
        Default::default(),
        Box::new(audit.clone()),
    )
    .unwrap();
    let now = humanymous_authority::now_ms().unwrap();
    let issued = issue(&mut app, now);
    collected(&mut app, &binding, &issued, now);
    let request = protected(&binding, &issued, 2, now);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(app.serve(listener));
    let mut client = TcpStream::connect(address).await.unwrap();
    client.write_all(&encode(&request)).await.unwrap();
    let mut observed = Vec::new();
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let mut buffer = [0; 1024];
        loop {
            let count = client.read(&mut buffer).await.unwrap();
            assert_ne!(count, 0);
            observed.extend_from_slice(&buffer[..count]);
            if observed.windows(11).any(|bytes| bytes == b"data: one\n\n") {
                break;
            }
        }
    })
    .await
    .unwrap();
    assert!(!observed.windows(11).any(|bytes| bytes == b"data: two\n\n"));
    release.send(()).unwrap();
    client.read_to_end(&mut observed).await.unwrap();
    assert!(observed.windows(11).any(|bytes| bytes == b"data: two\n\n"));
    assert!(observed.ends_with(b"0\r\n\r\n"));
    origin_task.await.unwrap();
    server.abort();
    let _ = server.await;
}
