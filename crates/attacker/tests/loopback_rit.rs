//! End-to-end exercise of the loopback client's session bootstrap and RIT
//! attack families against an in-process Rust server that implements the Go
//! `/api/session` + `/api/collect` RIT contract.
//!
//! Defense-only: the server binds 127.0.0.1:0 and the client refuses any
//! non-loopback target, so nothing here can reach a third party. This verifies
//! the ported client behaviour (cookie bootstrap, RIT signing, replay/tamper
//! detection); verdict parity with the real detector is a recorded FM11 gap,
//! because this test server re-implements only the RIT check, not scoring.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use humanymous_attacker::wire::client::LoopbackClient;
use humanymous_attacker::wire::rit::{now_tb, rit_token};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

const SERVER_SEED: &[u8] = b"owned-lab-server-seed-000000000000";
const SID: &str = "sess-e2e-0001";

/// A tiny HTTP/1.1 server implementing the owned-runtime RIT contract.
fn spawn_server() -> SocketAddr {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback");
    let address = listener.local_addr().expect("local addr");
    let last_n = Arc::new(AtomicU64::new(0));
    std::thread::spawn(move || {
        // Bounded: handle exactly the connections this test opens, then stop.
        for _ in 0..8 {
            let Ok((stream, _)) = listener.accept() else {
                break;
            };
            handle(stream, &last_n);
        }
    });
    address
}

fn handle(mut stream: TcpStream, last_n: &AtomicU64) {
    let mut raw = Vec::new();
    let mut buf = [0u8; 2048];
    // Read until headers complete, then the declared body.
    loop {
        let Ok(n) = stream.read(&mut buf) else { return };
        if n == 0 {
            break;
        }
        raw.extend_from_slice(&buf[..n]);
        if let Some(split) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&raw[..split]).to_string();
            let content_length = header(&head, "content-length")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0);
            if raw.len() >= split + 4 + content_length {
                let body = raw[split + 4..split + 4 + content_length].to_vec();
                respond(&mut stream, &head, &body, last_n);
                return;
            }
        }
    }
}

fn header(head: &str, name: &str) -> Option<String> {
    head.split("\r\n").skip(1).find_map(|line| {
        let (k, v) = line.split_once(':')?;
        k.eq_ignore_ascii_case(name).then(|| v.trim().to_owned())
    })
}

fn respond(stream: &mut TcpStream, head: &str, body: &[u8], last_n: &AtomicU64) {
    let path = head.split(' ').nth(1).unwrap_or("");
    let (status, extra, payload) = if path.starts_with("/api/session") {
        let seed_b64 = URL_SAFE_NO_PAD.encode(SERVER_SEED);
        (
            200,
            format!("Set-Cookie: hm={SID}; Path=/\r\n"),
            format!("{{\"ritSeed\":\"{seed_b64}\",\"ritN\":1}}"),
        )
    } else {
        let token = header(head, "x-hm-token").unwrap_or_default();
        let n = header(head, "x-hm-n")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let tb = header(head, "x-hm-tb")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        // This server reuses the client's own `rit_token`, so this check alone
        // is circular: it proves client/server agreement, not the canonical
        // form. The canonical form is pinned independently by the golden
        // vector in `wire::rit::tests::golden_vector_pins_the_canonical_form`.
        let expected = rit_token(SERVER_SEED, SID, n, tb, body);
        if token != expected {
            (
                403,
                String::new(),
                "{\"verdict\":\"DENY\",\"rule\":\"l5.rit.header_tampered\"}".into(),
            )
        } else if n <= last_n.load(Ordering::SeqCst) {
            (
                403,
                String::new(),
                "{\"verdict\":\"DENY\",\"rule\":\"l5.rit.stale_replay\"}".into(),
            )
        } else {
            last_n.store(n, Ordering::SeqCst);
            (200, String::new(), "{\"verdict\":\"COLLECTED\"}".into())
        }
    };
    let response = format!(
        "HTTP/1.1 {status} X\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

fn signed_headers(
    seed: &[u8],
    n: u64,
    tb: u64,
    cookie: &str,
    body: &[u8],
) -> Vec<(String, String)> {
    vec![
        ("Cookie".into(), cookie.into()),
        ("Content-Type".into(), "application/json".into()),
        ("X-HM-Token".into(), rit_token(seed, SID, n, tb, body)),
        ("X-HM-N".into(), n.to_string()),
        ("X-HM-TB".into(), tb.to_string()),
    ]
}

#[test]
fn session_bootstrap_and_rit_families_behave_against_the_owned_contract() {
    let address = spawn_server();
    let client = LoopbackClient::new(address).expect("loopback client");

    // 1. Cookie-session bootstrap against /api/session.
    let (cookie, seed, n) = client.session(&[]).expect("session bootstrap");
    assert_eq!(seed, SERVER_SEED);
    assert_eq!(n, 1);

    let tb = now_tb(1_700_000_000);
    let body = br#"{"userAgent":"Chrome","signals":[]}"#;

    // 2. A correctly signed collect is COLLECTED.
    let ok = client
        .send(
            "POST",
            "/api/collect",
            &signed_headers(&seed, n + 1, tb, &cookie, body),
            body,
        )
        .expect("signed collect");
    assert_eq!(ok.status, 200);
    assert_eq!(ok.verdict().as_deref(), Some("COLLECTED"));

    // 3. rit-replay: reuse the same counter -> stale replay DENY.
    let replay = client
        .send(
            "POST",
            "/api/collect",
            &signed_headers(&seed, n + 1, tb, &cookie, body),
            body,
        )
        .expect("replay collect");
    assert_eq!(replay.status, 403);
    assert_eq!(replay.verdict().as_deref(), Some("DENY"));

    // 4. rit-tamper: sign one body, send a different body -> tamper DENY.
    let mut tampered = signed_headers(&seed, n + 2, tb, &cookie, body); // token signs `body`
    let other = br#"{"userAgent":"Chrome","injected":"evil"}"#;
    // Content-Length is recomputed by the client from the actual (tampered) body.
    tampered.retain(|(k, _)| !k.eq_ignore_ascii_case("content-type"));
    let tamper = client
        .send("POST", "/api/collect", &tampered, other)
        .expect("tampered collect");
    assert_eq!(tamper.status, 403);
    assert_eq!(tamper.verdict().as_deref(), Some("DENY"));
}
