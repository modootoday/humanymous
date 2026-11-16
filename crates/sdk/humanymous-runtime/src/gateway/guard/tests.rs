//! Ports of legacy pkg/origincloak/origincloak_test.go and the internal/gate
//! guard cases of forward_fidelity_test.go (baseline 981d25c).
use super::*;

const KEY: &[u8; 32] = b"0123456789abcdef0123456789abcdef";

fn key(seed: &[u8]) -> OriginKey {
    let mut bytes = seed.to_vec();
    bytes.resize(MIN_ORIGIN_KEY, b'.');
    OriginKey::new(bytes).unwrap()
}

fn headers(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn token_matches_go_golden_bytes() {
    // Regenerated with Go 1.25.3 at baseline 981d25c: origincloak.Token and
    // origincloak.Epoch(time.Unix(1_700_003_600, 0)).
    let k = OriginKey::new(KEY.to_vec()).unwrap();
    assert_eq!(
        k.token("e472222"),
        "d160a699cbc9745f1aefb43739d9f448b730d6fdf69e60e46f0387281b12830a"
    );
    assert_eq!(origin_epoch(1_700_003_600), "e472223");
}

#[test]
fn token_is_hmac_hex() {
    let t = key(b"secret").token("e42");
    assert_eq!(t.len(), 64);
    assert_eq!(key(b"secret").token("e42"), t, "deterministic");
    assert_ne!(key(b"other").token("e42"), t, "key-bound");
    assert_ne!(key(b"secret").token("e43"), t, "epoch-bound");
}

#[test]
fn epoch_buckets_by_hour() {
    let base = 1_700_000_000 / 3600 * 3600;
    assert_eq!(origin_epoch(base), origin_epoch(base + 59 * 60));
    assert_ne!(origin_epoch(base), origin_epoch(base + 3600));
    assert_eq!(
        origin_epoch(-1),
        "e0",
        "Go integer division truncates toward zero"
    );
}

#[test]
fn valid_with_symmetric_grace() {
    let k = key(b"shared-origin-key");
    let now = 1_700_003_600;
    let current = k.token(&origin_epoch(now));
    assert!(k.valid(&current, now));
    assert!(k.valid(&current, now + 3600), "origin one bucket behind");
    assert!(k.valid(&current, now - 3600), "origin one bucket ahead");
    assert!(!k.valid(&current, now + 2 * 3600 + 60), "two buckets stale");
    assert!(!key(b"wrong").valid(&current, now));
    assert!(!k.valid("", now));
    assert!(!k.valid("deadbeef", now));
}

#[test]
fn origin_handler_example_direct_hit_fails() {
    let k = key(b"k");
    let now = 1_700_007_200;
    assert!(k.valid(&k.token(&origin_epoch(now)), now));
    assert!(!k.valid("", now), "no header is a direct hit (421)");
}

#[test]
fn short_origin_key_is_refused() {
    assert!(OriginKey::new(vec![1; MIN_ORIGIN_KEY - 1]).is_err());
    assert!(OriginKey::new(Vec::new()).is_err());
}

#[test]
fn spoofed_trust_headers_are_reported_and_stripped() {
    let mut h = headers(&[
        ("x-forwarded-for", "1.2.3.4"),
        ("Accept", "text/html"),
        ("X-HMNY-ORIGIN-AUTH", "forged"),
        ("cf-connecting-ip", "5.6.7.8"),
        ("X-Real-IP", ""),
        ("X-Forwarded-For", "9.9.9.9"),
        ("Forwarded", "for=1.1.1.1"),
        ("x-hmn-verdict", "ALLOW"),
    ]);
    assert_eq!(
        spoof_scan(&h),
        vec![
            "X-Hmny-Origin-Auth",
            "X-Hmn-Verdict",
            "X-Forwarded-For",
            "Forwarded",
            "Cf-Connecting-Ip"
        ],
        "legacy order, deduplicated, empty values are not a claim"
    );
    strip_inbound(&mut h);
    assert_eq!(h, headers(&[("Accept", "text/html")]));
    assert!(spoof_scan(&h).is_empty());
}

#[test]
fn clean_request_has_no_spoof() {
    let mut h = headers(&[("Host", "x"), ("User-Agent", "Chrome/126")]);
    assert!(spoof_scan(&h).is_empty());
    let before = h.clone();
    strip_inbound(&mut h);
    assert_eq!(h, before);
}
