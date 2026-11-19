use super::*;

#[test]
fn reads_one_family_per_entry() {
    let body = r#"{"syncToken":"1","creationTime":"t","prefixes":[
        {"ipv4Prefix":"192.0.2.0/24","service":"Google Cloud","scope":"x"},
        {"ipv6Prefix":"2001:db8::/32","service":"Google Cloud","scope":"x"}]}"#;
    assert_eq!(parse(body).unwrap().len(), 2);
}

#[test]
fn refuses_entries_with_zero_or_two_prefixes() {
    for body in [
        r#"{"prefixes":[{"service":"x"}]}"#,
        r#"{"prefixes":[{"ipv4Prefix":"192.0.2.0/24","ipv6Prefix":"2001:db8::/32"}]}"#,
    ] {
        assert_eq!(parse(body).err(), Some(SourceError::Malformed { line: 0 }));
    }
}
