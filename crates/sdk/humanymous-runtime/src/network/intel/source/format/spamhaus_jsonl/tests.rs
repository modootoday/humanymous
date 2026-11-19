use super::*;

#[test]
fn reads_records_and_skips_metadata() {
    let body = concat!(
        "{\"cidr\":\"192.0.2.0/24\",\"sblid\":\"SBL0\",\"rir\":\"test\"}\n",
        "{\"cidr\":\"2001:db8::/32\",\"sblid\":\"SBL1\",\"rir\":\"test\"}\n",
        "{\"type\":\"metadata\",\"timestamp\":1,\"records\":2}\n"
    );
    assert_eq!(parse(body).unwrap().len(), 2);
}

#[test]
fn refuses_unknown_records_and_html() {
    assert_eq!(
        parse("{\"cidr\":\"192.0.2.0/24\"}\n{\"type\":\"other\"}\n").err(),
        Some(SourceError::Malformed { line: 2 })
    );
    assert_eq!(
        parse("<html>\n").err(),
        Some(SourceError::Malformed { line: 1 })
    );
}
