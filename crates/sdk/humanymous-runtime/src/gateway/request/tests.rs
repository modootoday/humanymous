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
}
