use super::*;

#[test]
fn reads_addresses_and_cidrs_with_comments() {
    let body = "#\n# firehol-style header\n#\n192.0.2.0/24\n198.51.100.7\n203.0.113.0/24 ; SBL0\n2001:db8::/32 # note\n\n   \n";
    let parsed: Vec<String> = parse(body)
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        parsed,
        [
            "192.0.2.0/24",
            "198.51.100.7/32",
            "203.0.113.0/24",
            "2001:db8::/32"
        ]
    );
}

#[test]
fn refuses_ranges_and_garbage_with_line_number() {
    assert_eq!(
        parse("192.0.2.0/24\n192.0.2.1-192.0.2.9\n").err(),
        Some(SourceError::Prefix { line: 2 })
    );
    assert_eq!(
        parse("<html>\n").err(),
        Some(SourceError::Prefix { line: 1 })
    );
}
