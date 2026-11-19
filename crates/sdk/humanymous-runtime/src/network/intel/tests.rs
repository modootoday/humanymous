use super::*;
#[test]
fn client_address_is_one_strict_literal() {
    assert_eq!(
        client_address("192.0.2.1"),
        Some("192.0.2.1".parse().unwrap())
    );
    assert_eq!(
        client_address("::ffff:192.0.2.1"),
        Some("192.0.2.1".parse().unwrap())
    );
    assert_eq!(
        client_address("2001:db8::1"),
        Some("2001:db8::1".parse().unwrap())
    );
    for bad in [
        "",
        " 192.0.2.1",
        "192.0.2.1:443",
        "192.0.2.1, 198.51.100.1",
        "[2001:db8::1]",
        "unknown",
        "192.0.2.0/24",
    ] {
        assert_eq!(client_address(bad), None, "{bad}");
    }
}
#[test]
fn private_and_local_addresses_are_never_classified() {
    for local in [
        "10.0.0.1",
        "172.16.0.1",
        "192.168.1.1",
        "127.0.0.1",
        "169.254.0.1",
        "0.0.0.0",
        "::1",
        "::",
        "fd00::1",
        "fe80::1",
        "::ffff:10.0.0.1",
    ] {
        assert!(!is_classifiable(local.parse().unwrap()), "{local}");
    }
    for public in ["192.0.2.1", "198.51.100.1", "2001:db8::1", "8.8.8.8"] {
        assert!(is_classifiable(public.parse().unwrap()), "{public}");
    }
}
#[test]
fn only_datacenter_feeds_scoring() {
    let scoring: Vec<_> = IntelCategory::ALL
        .into_iter()
        .filter(|c| c.feeds_scoring())
        .collect();
    assert_eq!(scoring, [IntelCategory::Datacenter]);
    let all = Classification {
        datacenter: true,
        tor_exit: true,
        blocklist: true,
    };
    assert!(IntelCategory::ALL.iter().all(|&c| all.contains(c)));
}
