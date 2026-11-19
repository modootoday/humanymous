//! Boundaries of the legacy internal/gate/request.go helpers (baseline 981d25c
//! has no direct Go tests for them; these pin the Go branch order).
use super::*;

fn h(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn route_class_follows_legacy_priority() {
    assert_eq!(
        route_class(&h(&[("upgrade", "WebSocket")])),
        RouteClass::Upgrade
    );
    assert_eq!(
        route_class(&h(&[("Sec-WebSocket-Key", "x"), ("Accept", "text/html")])),
        RouteClass::Upgrade
    );
    assert_eq!(
        route_class(&h(&[("Sec-Fetch-Dest", "document")])),
        RouteClass::Html
    );
    assert_eq!(
        route_class(&h(&[("Accept", "text/html,application/json")])),
        RouteClass::Html,
        "html wins over json"
    );
    assert_eq!(
        route_class(&h(&[("Accept", "application/json")])),
        RouteClass::Api
    );
    assert_eq!(
        route_class(&h(&[("Sec-Fetch-Dest", "empty")])),
        RouteClass::Api
    );
    assert_eq!(
        route_class(&h(&[("Accept", "image/png")])),
        RouteClass::Static
    );
    assert_eq!(route_class(&[]), RouteClass::Static);
    assert_eq!(
        route_class(&h(&[("Accept", "image/png"), ("Accept", "text/html")])),
        RouteClass::Static,
        "only the first value counts, like Header.Get"
    );
    assert_eq!(RouteClass::Html.as_str(), "html");
}

#[test]
fn latency_saturates_and_never_goes_negative() {
    let start = Duration::from_secs(100);
    assert_eq!(latency_us(None, start), 0);
    assert_eq!(
        latency_us(Some(start), start + Duration::from_micros(1500)),
        1500
    );
    assert_eq!(latency_us(Some(start), Duration::from_secs(99)), 0);
    assert_eq!(
        latency_us(Some(Duration::ZERO), Duration::from_secs(5 * 3600)),
        u32::MAX
    );
}

#[test]
fn correlation_ids_are_sixteen_hex_and_fresh() {
    let a = correlation_id().unwrap();
    let b = correlation_id().unwrap();
    assert_eq!(a.len(), 16);
    assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
    assert_ne!(a, b);
}

#[test]
fn upstream_errors_map_to_stable_classes_without_raw_text() {
    use io::{Error, ErrorKind};
    assert_eq!(
        upstream_error_class(&Error::from(ErrorKind::TimedOut)),
        "timeout"
    );
    assert_eq!(
        upstream_error_class(&Error::from(ErrorKind::Interrupted)),
        "canceled"
    );
    assert_eq!(
        upstream_error_class(&Error::from(ErrorKind::ConnectionRefused)),
        "connection_refused"
    );
    assert_eq!(
        upstream_error_class(&Error::from(ErrorKind::UnexpectedEof)),
        "eof"
    );
    assert_eq!(
        upstream_error_class(&Error::other("failed to lookup address information")),
        "dns"
    );
    assert_eq!(
        upstream_error_class(&Error::other("secret /path/token leaked")),
        "unreachable"
    );
    // Refusals without a transport error keep the same closed class set.
    assert_eq!(origin_error_class("origin_timeout"), "timeout");
    assert_eq!(origin_error_class("origin_truncated_head"), "eof");
    assert_eq!(origin_error_class("origin_chunk_truncated"), "eof");
    assert_eq!(origin_error_class("origin_framing"), "unreachable");
    assert_eq!(origin_error_class("origin_stream_budget"), "unreachable");
}

#[test]
fn traffic_ip_hash_matches_go_golden_and_needs_a_key() {
    // Regenerated with Go 1.25.3 from the legacy trafficIPHash construction.
    let key = b"0123456789abcdef0123456789abcdef";
    let ip = "203.0.113.9".parse().unwrap();
    assert_eq!(
        traffic_ip_hash(key, ip).as_deref(),
        Some("7bcf9fc29a76291b")
    );
    assert_ne!(traffic_ip_hash(b"other", ip), traffic_ip_hash(key, ip));
    assert_eq!(traffic_ip_hash(b"", ip), None);
}

#[test]
fn day_bucket_matches_go_and_clamps_pre_epoch() {
    assert_eq!(day_bucket(1_700_000_000), 19675);
    assert_eq!(day_bucket(86_399), 0);
    assert_eq!(day_bucket(-1), 0);
}

#[test]
fn ja4_stable_matches_go_golden() {
    // Go 1.25.3 capture of legacy ja4Stable (cmd/server/helpers.go, 981d25c).
    let golden: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/gateway/pow-asset-go-golden.json"
    ))
    .unwrap();
    let list = |value: &serde_json::Value| -> Vec<u16> {
        value
            .as_array()
            .into_iter()
            .flatten()
            .map(|item| u16::try_from(item.as_u64().unwrap()).unwrap())
            .collect()
    };
    let cases = golden["outputs"]["ja4_stable"].as_array().unwrap();
    assert_eq!(cases.len(), 4);
    for case in cases {
        let go = &case["hello"];
        let hello = (!go.is_null()).then(|| ClientHello {
            legacy_version: u16::try_from(go["LegacyVersion"].as_u64().unwrap()).unwrap(),
            supported_versions: list(&go["SupportedVers"]),
            cipher_suites: list(&go["CipherSuites"]),
            extensions: list(&go["Extensions"]),
            curves: list(&go["Curves"]),
            point_formats: list(&go["PointFormats"]),
            signature_algorithms: list(&go["SigAlgs"]),
            alpn: go["ALPN"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|name| name.as_str().unwrap().as_bytes().to_vec())
                .collect(),
            sni_present: go["SNIPresent"].as_bool().unwrap(),
        });
        let name = case["name"].as_str().unwrap();
        if let Some(hello) = &hello {
            assert_eq!(ja4(hello).0, case["ja4"].as_str().unwrap(), "{name}: ja4");
        }
        assert_eq!(
            ja4_stable(hello.as_ref()),
            case["stable"].as_str().unwrap(),
            "{name}: stable part"
        );
    }
}
