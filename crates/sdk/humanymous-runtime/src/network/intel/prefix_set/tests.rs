use super::*;

fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}
fn set(prefixes: &[&str]) -> PrefixSet {
    PrefixSet::new(prefixes.iter().map(|p| Prefix::parse_canonical(p).unwrap()))
}

#[test]
fn v4_boundaries_are_inclusive_and_exact() {
    let s = set(&["192.0.2.0/24", "198.51.100.128/25"]);
    assert!(!s.contains(ip("192.0.1.255")));
    assert!(s.contains(ip("192.0.2.0")));
    assert!(s.contains(ip("192.0.2.255")));
    assert!(!s.contains(ip("192.0.3.0")));
    assert!(!s.contains(ip("198.51.100.127")));
    assert!(s.contains(ip("198.51.100.128")));
    assert!(s.contains(ip("198.51.100.255")));
    assert!(!s.contains(ip("198.51.101.0")));
}

#[test]
fn v6_boundaries_are_inclusive_and_exact() {
    let s = set(&["2001:db8:1::/48", "2001:db8:ffff:ffff::/64"]);
    assert!(!s.contains(ip("2001:db8:0:ffff:ffff:ffff:ffff:ffff")));
    assert!(s.contains(ip("2001:db8:1::")));
    assert!(s.contains(ip("2001:db8:1:ffff:ffff:ffff:ffff:ffff")));
    assert!(!s.contains(ip("2001:db8:2::")));
    assert!(s.contains(ip("2001:db8:ffff:ffff:ffff:ffff:ffff:ffff")));
    assert!(!s.contains(ip("2001:db9::")));
}

#[test]
fn families_never_cross_except_ipv4_mapped_lookup() {
    let s = set(&["192.0.2.0/24"]);
    assert!(s.contains(ip("::ffff:192.0.2.7")));
    assert!(!s.contains(ip("::192.0.2.7")));
    // An IPv6 default route never swallows IPv4 or IPv4-mapped addresses.
    assert!(!set(&["::/0"]).contains(ip("192.0.2.1")));
    assert!(!set(&["::/0"]).contains(ip("::ffff:192.0.2.1")));
    let only_v6 = set(&["2001:db8::/32"]);
    assert!(!only_v6.contains(ip("32.1.13.184")));
}

#[test]
fn extreme_lengths_cover_whole_family_or_one_host() {
    let all = set(&["0.0.0.0/0"]);
    assert!(all.contains(ip("0.0.0.0")) && all.contains(ip("255.255.255.255")));
    let host = set(&["203.0.113.9", "2001:db8::9/128"]);
    assert!(host.contains(ip("203.0.113.9")) && !host.contains(ip("203.0.113.8")));
    assert!(host.contains(ip("2001:db8::9")) && !host.contains(ip("2001:db8::a")));
    let top = set(&["::/0"]);
    assert!(top.contains(ip("ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff")));
}

#[test]
fn overlapping_and_adjacent_prefixes_merge() {
    let s = set(&[
        "192.0.2.0/25",
        "192.0.2.128/25",
        "192.0.2.64/26",
        "255.255.255.255/32",
        "255.255.255.254/32",
    ]);
    assert_eq!(s.ranges(), (2, 0));
    assert!(s.contains(ip("192.0.2.200")));
    assert!(s.contains(ip("255.255.255.255")));
    assert!(PrefixSet::new([]).is_empty());
}

#[test]
fn canonical_parse_refuses_host_bits_and_bad_syntax() {
    assert_eq!(
        Prefix::parse_canonical("192.0.2.1/24"),
        Err(PrefixError::HostBits)
    );
    assert_eq!(
        Prefix::parse_masked("192.0.2.1/24").unwrap().to_string(),
        "192.0.2.0/24"
    );
    for bad in [
        "",
        "192.0.2.0/",
        "192.0.2.0/33",
        "192.0.2.0/024",
        "192.0.2.0/+4",
        "2001:db8::/129",
        "2001:db8::/1000",
        "192.0.2",
        "example.test",
        " 192.0.2.0/24",
    ] {
        assert_eq!(Prefix::parse_masked(bad), Err(PrefixError::Syntax), "{bad}");
    }
    let host = Prefix::parse_canonical("2001:db8::1").unwrap();
    assert_eq!((host.len(), host.is_empty()), (128, false));
    assert!(Prefix::parse_canonical("0.0.0.0/0").unwrap().is_empty());
}
