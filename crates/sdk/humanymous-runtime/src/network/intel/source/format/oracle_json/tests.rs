use super::*;

#[test]
fn reads_regions_and_optional_ipv6() {
    let body = r#"{"last_updated_timestamp":"t","regions":[
        {"region":"a","cidrs":[{"cidr":"192.0.2.0/24","tags":["OCI"]}],"ipv6_cidrs":[{"cidr":"2001:db8::/32","tags":["OCI"]}]},
        {"region":"b","cidrs":[{"cidr":"198.51.100.0/24","tags":["OSN"]}]}]}"#;
    assert_eq!(parse(body).unwrap().len(), 3);
}

#[test]
fn refuses_bad_cidr() {
    assert_eq!(
        parse(r#"{"regions":[{"cidrs":[{"cidr":"x"}]}]}"#).err(),
        Some(SourceError::Prefix { line: 0 })
    );
}
