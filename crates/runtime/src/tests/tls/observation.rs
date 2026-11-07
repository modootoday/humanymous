//! Offered protocol order and negotiation remain facts of the same socket.
use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actual_client_hello_and_negotiated_alpn_share_the_request_owner() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (server_tls, client_tls) = configurations();
    let mut client_config = (*client_tls).clone();
    client_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    let request = request("POST", "/_humanymous/session", b"{}".to_vec());
    let client_task = tokio::spawn(exchange_tls(
        address,
        Arc::new(client_config),
        encode(&request),
    ));
    let (socket, _) = listener.accept().await.unwrap();
    let captured =
        crate::tls::CapturedRequest::receive(socket.into_std().unwrap(), server_tls, BUDGET)
            .await
            .unwrap();
    assert_eq!(captured.request().body, b"{}");
    assert_eq!(
        captured.connection().client_hello().unwrap().alpn,
        [b"h2".to_vec(), b"http/1.1".to_vec()]
    );
    assert_eq!(
        captured.connection().negotiated_alpn(),
        Some(b"http/1.1".as_slice())
    );
    captured
        .respond(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec())
        .await
        .unwrap();
    assert!(client_task.await.unwrap().starts_with(b"HTTP/1.1 200"));
}
