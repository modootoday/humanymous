use super::*;

#[test]
fn reads_both_families_and_ignores_metadata() {
    let body = r#"{"syncToken":"1","createDate":"2026-01-01-00-00-00",
        "prefixes":[{"ip_prefix":"192.0.2.0/24","region":"x","service":"AMAZON"}],
        "ipv6_prefixes":[{"ipv6_prefix":"2001:db8::/32","region":"x"}]}"#;
    let parsed: Vec<String> = parse(body)
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(parsed, ["192.0.2.0/24", "2001:db8::/32"]);
}

#[test]
fn refuses_wrong_shape_and_bad_prefix() {
    assert_eq!(
        parse(r#"{"prefixes":{}}"#).err(),
        Some(SourceError::Malformed { line: 0 })
    );
    assert_eq!(
        parse(r#"{"prefixes":[{"ip_prefix":"192.0.2.0/40"}]}"#).err(),
        Some(SourceError::Prefix { line: 0 })
    );
}
