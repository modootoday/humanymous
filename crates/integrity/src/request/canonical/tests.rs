use super::*;
use crate::request::{compute_token, verify_token};

// Frozen pre-optimization oracle; never used by production code.
fn baseline(input: CanonicalInput<'_>) -> String {
    let mut result = String::new();
    for name in SIGNED_HEADERS {
        result.push_str(name);
        result.push(':');
        let value = input
            .headers
            .get(name)
            .map(String::as_str)
            .unwrap_or_default();
        result.push_str(&value.split_whitespace().collect::<Vec<_>>().join(" "));
        result.push('\n');
    }
    result.push_str("cookie:");
    result.push_str(input.cookie_id);
    result.push_str("\nbody:");
    result.push_str(&hex::encode(Sha256::digest(input.body)));
    result
}

#[test]
fn sorts_case_variants_before_collapsing_and_ignores_unsigned_headers() {
    let headers = BTreeMap::from([
        ("User-Agent".into(), "discarded".into()),
        ("user-agent".into(), "  browser\t\u{85}\u{a0}  1 ".into()),
        ("x-other".into(), "unsigned".into()),
    ]);
    let headers = lower_header_map(&headers);
    let value = canonical(CanonicalInput {
        headers: &headers,
        cookie_id: "sid",
        body: b"",
    });
    assert!(value.starts_with("user-agent:browser 1\nsec-ch-ua:\n"));
    assert!(!value.contains("unsigned"));
    assert!(value.ends_with("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));
}

#[test]
fn streaming_emission_preserves_unicode_whitespace_and_non_whitespace_bytes() {
    let whitespace: String = (0..=0x10ffff)
        .filter_map(char::from_u32)
        .filter(|character| character.is_whitespace())
        .collect();
    let mut values = vec![
        String::new(),
        whitespace.clone(),
        format!("{whitespace}a{whitespace}b{whitespace}"),
        "\u{85}a\u{a0}b\u{2028}".into(),
        "a\u{200b}b\u{feff}c\u{180e}d".into(),
        "e\u{301}\t한글\n🦀".into(),
        "x".repeat(8192),
    ];
    values.extend(
        whitespace
            .chars()
            .map(|separator| format!("{separator}a{separator}{separator}b{separator}")),
    );
    for value in values {
        for name in SIGNED_HEADERS {
            let headers = BTreeMap::from([(name.to_owned(), value.clone())]);
            let input = || CanonicalInput {
                headers: &headers,
                cookie_id: "raw:\n\u{a0}",
                body: &[0, 255, 13, 10],
            };
            let expected = baseline(input());
            let actual = canonical(input());
            assert_eq!(actual.as_bytes(), expected.as_bytes());
            assert!(!actual.ends_with('\n'));
            let token = compute_token(b"fixed-fixture-seed", &expected, 7, 11);
            assert!(verify_token(b"fixed-fixture-seed", &actual, 7, 11, &token));
            assert!(!verify_token(b"fixed-fixture-seed", &actual, 8, 11, &token));
        }
    }
}

#[test]
fn empty_headers_and_unicode_cookie_keep_exact_framing_and_body_binding() {
    let headers = BTreeMap::new();
    let input = || CanonicalInput {
        headers: &headers,
        cookie_id: "e\u{301}\n\u{200b}",
        body: b"\0\xff",
    };
    let text = canonical(input());
    assert_eq!(text, baseline(input()));
    assert!(text.starts_with("user-agent:\nsec-ch-ua:\nsec-ch-ua-platform:\nsec-ch-ua-mobile:\naccept-language:\naccept:\ncookie:e\u{301}\n\u{200b}\nbody:"));
    let changed = canonical(CanonicalInput {
        headers: &headers,
        cookie_id: "e\u{301}\n\u{200b}",
        body: b"\0\xfe",
    });
    let token = compute_token(b"fixed-fixture-seed", &text, 7, 11);
    assert!(!verify_token(
        b"fixed-fixture-seed",
        &changed,
        7,
        11,
        &token
    ));
}
