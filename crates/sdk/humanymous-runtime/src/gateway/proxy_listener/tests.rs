//! Real loopback TCP parity with the Go `proxyListener.Accept` cases captured
//! at 981d25c (fixture `accept` section; never regenerated to hide drift).
use super::*;
use serde_json::Value;
use std::sync::OnceLock;
use tokio::io::AsyncWriteExt;

fn golden() -> &'static Value {
    static GOLDEN: OnceLock<Value> = OnceLock::new();
    GOLDEN.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/network/proxyproto-go-golden.json"
        ))
        .expect("golden fixture")
    })
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The capture's header timeout.
const CAPTURE_TIMEOUT: Duration = Duration::from_millis(300);

async fn connect_and_send(address: SocketAddr, bytes: &str) -> TcpStream {
    let mut client = TcpStream::connect(address).await.unwrap();
    if !bytes.is_empty() {
        client.write_all(&unhex(bytes)).await.unwrap();
    }
    client
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accept_matches_go_golden_over_real_tcp() {
    let rows = golden()["accept"].as_array().unwrap();
    assert_eq!(rows.len(), 10);
    for row in rows {
        let name = row["name"].as_str().unwrap();
        let list = row["allowlist"].as_str().unwrap();
        let first = row["first_hex"].as_str().unwrap();
        let second = row["second_hex"].as_str().unwrap();
        let silent = row["first_silent"].as_bool().unwrap();
        let trusted = list == "127.0.0.0/8";

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let listener = ProxyListener::new(listener, Allowlist::parse(list).unwrap())
            .with_timeout(CAPTURE_TIMEOUT);
        let address = listener.local_addr().unwrap();
        let accepting = tokio::spawn(async move {
            let accepted = listener.accept().await;
            (listener, accepted)
        });
        let mut client1 = connect_and_send(address, first).await;
        let mut client2 = None;
        if !second.is_empty() {
            let wait = if silent { 450 } else { 50 };
            tokio::time::sleep(Duration::from_millis(wait)).await;
            client2 = Some(connect_and_send(address, second).await);
        }
        let (_listener, accepted) = tokio::time::timeout(Duration::from_secs(3), accepting)
            .await
            .unwrap_or_else(|_| panic!("{name}: accept hung"))
            .unwrap();
        let (mut stream, peer) = accepted.unwrap();

        match row["accepted"].as_str().unwrap() {
            "first" => {
                let socket = client1.local_addr().unwrap();
                let expected = if trusted {
                    Peer::Balancer(socket)
                } else {
                    Peer::Socket(socket)
                };
                assert_eq!(peer, expected, "{name}");
            }
            "declared" => match peer {
                Peer::Declared(source) => {
                    assert_eq!(
                        source.to_string(),
                        row["remote"].as_str().unwrap(),
                        "{name}"
                    );
                }
                other => panic!("{name}: {other:?}"),
            },
            other => panic!("{name}: unexpected golden outcome {other}"),
        }

        let mut buffer = [0u8; 256];
        let read = tokio::time::timeout(Duration::from_millis(200), stream.read(&mut buffer))
            .await
            .map(|read| read.unwrap())
            .unwrap_or(0);
        assert_eq!(
            hex(&buffer[..read]),
            row["remaining_hex"].as_str().unwrap(),
            "{name} remaining"
        );

        if let Some(closed) = row["first_closed_by_server"].as_bool() {
            let observed =
                tokio::time::timeout(Duration::from_millis(500), client1.read(&mut [0u8; 1]))
                    .await
                    .map(|read| matches!(read, Ok(0) | Err(_)))
                    .unwrap_or(false);
            assert_eq!(observed, closed, "{name} first connection closed");
        }
        drop(client2);
    }
}

#[test]
fn default_timeout_is_the_go_five_seconds() {
    let go = golden()["default_timeout_ns"].as_u64().unwrap();
    assert_eq!(u128::from(go), READ_TIMEOUT.as_nanos());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn untrusted_peer_bytes_are_never_consumed() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let listener = ProxyListener::new(listener, Allowlist::parse("10.0.0.0/8").unwrap());
    assert_eq!(listener.timeout(), READ_TIMEOUT);
    let address = listener.local_addr().unwrap();
    // A silent untrusted peer is returned at once, without waiting for bytes.
    let client = TcpStream::connect(address).await.unwrap();
    let (_stream, peer) = tokio::time::timeout(Duration::from_secs(1), listener.accept())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(peer, Peer::Socket(client.local_addr().unwrap()));
}
