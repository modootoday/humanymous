use super::*;

#[test]
fn unions_all_tags() {
    let body = r#"{"changeNumber":1,"cloud":"Public","values":[
        {"name":"A","id":"A","properties":{"changeNumber":1,"addressPrefixes":["192.0.2.0/25","2001:db8::/48"]}},
        {"name":"B","id":"B","properties":{"addressPrefixes":["192.0.2.128/25"]}}]}"#;
    assert_eq!(parse(body).unwrap().len(), 3);
}

#[test]
fn refuses_missing_prefix_array() {
    assert_eq!(
        parse(r#"{"values":[{"properties":{}}]}"#).err(),
        Some(SourceError::Malformed { line: 0 })
    );
}
