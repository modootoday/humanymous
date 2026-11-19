use super::*;

#[test]
fn reads_first_column_and_skips_comments() {
    let body = "# self-published geofeed\n# ip_prefix, alpha2code\n192.0.2.0/24,ZZ,ZZ-1,City,\n\n2001:db8::/32,ZZ,,,\r\n";
    let parsed: Vec<String> = parse(body)
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(parsed, ["192.0.2.0/24", "2001:db8::/32"]);
}

#[test]
fn reports_the_failing_line() {
    assert_eq!(
        parse("# c\n192.0.2.0/24,ZZ\nnot-a-prefix,ZZ\n").err(),
        Some(SourceError::Prefix { line: 3 })
    );
}
