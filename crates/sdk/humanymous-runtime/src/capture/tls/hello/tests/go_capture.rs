//! Differential parity against legacy cmd/server/tlsconn.go `captureConn`
//! outputs captured from the frozen Go oracle at 981d25c (fixture
//! `provenance`): real Go `crypto/tls` ClientHellos and their variants, fed
//! through `captureConn.Read` in the recorded chunks. Expectations are never
//! regenerated.
//!
//! The Rust assembler is deliberately stricter than Go's tolerant first-record
//! parse. Agreement is exact everywhere else; the named deviations are:
//! - a ClientHello fragmented over several records: Go parsed only the first
//!   record and captured nothing, Rust reassembles the same hello Go captured
//!   from the unfragmented record;
//! - a record-layer version outside 3.1..3.3, or bytes after the ClientHello
//!   inside its record: Go captured the hello, Rust refuses the epoch.
use super::*;
use crate::network::tls::{engine_from_client_hello, ja3, ja4};
use serde_json::{Value, json};

fn golden() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/network/tlsconn-go-golden.json"
    ))
    .expect("golden fixture")
}

fn describe(hello: &ClientHello) -> Value {
    let (ja3_hash, ja3_text) = ja3(hello);
    let (ja4, ja4_raw) = ja4(hello);
    json!({
        "legacy_version": hello.legacy_version,
        "supported_versions": hello.supported_versions,
        "cipher_suites": hello.cipher_suites,
        "extensions": hello.extensions,
        "curves": hello.curves,
        "point_formats": hello.point_formats,
        "signature_algorithms": hello.signature_algorithms,
        "alpn_hex": hello.alpn.iter().map(hex::encode).collect::<Vec<_>>(),
        "sni_present": hello.sni_present,
        "ja3_hash": ja3_hash,
        "ja3_text": ja3_text,
        "ja4": ja4,
        "ja4_raw": ja4_raw,
        "engine": engine_from_client_hello(Some(hello)),
    })
}

/// Feed the recorded chunks; returns whether capture settled (complete or
/// refused, Go's `done`) and the finished hello.
fn observe(case: &Value) -> (bool, Option<Value>) {
    let stream = hex::decode(case["stream"].as_str().unwrap()).unwrap();
    let mut capture = ClientHelloCapture::default();
    let mut refused = false;
    let mut at = 0;
    for size in case["chunks"].as_array().unwrap() {
        let size = usize::try_from(size.as_u64().unwrap()).unwrap();
        refused |= capture.feed(&stream[at..at + size]).is_err();
        at += size;
    }
    assert_eq!(at, stream.len());
    let settled = refused || capture.is_complete();
    (settled, capture.finish().ok().map(|hello| describe(&hello)))
}

#[test]
fn first_record_capture_matches_go_capture_conn() {
    let golden = golden();
    let cases = golden["outputs"].as_array().unwrap();
    assert_eq!(cases.len(), 50);
    let whole = |base: &str| {
        cases
            .iter()
            .find(|case| case["base"] == base && case["name"] == "whole")
            .map(|case| case["hello"].clone())
            .unwrap()
    };
    let mut exact = 0;
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let base = case["base"].as_str().unwrap();
        let label = format!("{base}/{name}");
        let (settled, hello) = observe(case);
        assert_eq!(settled, case["done"].as_bool().unwrap(), "{label}: settled");
        if name.starts_with("two_record_fragments") {
            assert!(case["hello"].is_null(), "{label}: Go captured nothing");
            assert_eq!(hello, Some(whole(base)), "{label}: Rust reassembles");
        } else if matches!(
            name,
            "record_version_3_0" | "record_version_2_0" | "record_trailing_bytes"
        ) {
            assert_eq!(case["hello"], whole(base), "{label}: Go was tolerant");
            assert_eq!(hello, None, "{label}: Rust refuses");
        } else {
            assert_eq!(
                hello.unwrap_or(Value::Null),
                case["hello"],
                "{label}: hello"
            );
            exact += 1;
        }
    }
    assert_eq!(exact, 32);
}
