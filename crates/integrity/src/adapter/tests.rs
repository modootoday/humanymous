use super::*;
use serde_json::{Value, json};
fn request() -> Value {
    json!({"protocol":"humanymous.adapter/0.1","id":"01234567-89ab-4cde-8fab-0123456789ab","tenant":"t","origin":"o","revision":"r","deadline":2000,"method":"POST","target":"/api?q=1","authority":"example.test","peer":"127.0.0.1","headers":[["Host","example.test"],["Content-Length","3"],["X-Tag","first"],["x-tag","second"]],"headerMode":"node-raw","body":{"layer":"transfer-decoded-content-encoded","complete":true,"length":3,"sha256":hex::encode(Sha256::digest(b"abc")),"base64":"YWJj"},"capabilities":{"rawTls":false,"originalH2":false,"rawHeaderOrder":true,"completeBody":true},"stream":null,"capture":null,"connectionId":"01234567-89ab-4cde-8fab-0123456789ab","adapter":{"id":"humanymous.node","version":"0.1.0-experimental.1","installation":"i"}})
}
fn accepts(v: &Value) -> bool {
    parse(&serde_json::to_vec(v).unwrap(), 1000).is_ok()
}
#[test]
fn preserves_order_and_verifies_exact_entity_bytes() {
    let parsed = parse(&serde_json::to_vec(&request()).unwrap(), 1000).unwrap();
    assert_eq!(parsed.body(), b"abc");
    assert_eq!(parsed.headers()[2], ["X-Tag", "first"]);
    assert_eq!(parsed.headers()[3], ["x-tag", "second"]);
    for (field, value) in [
        ("length", json!(2)),
        ("base64", json!("YWJj=")),
        ("sha256", json!("0".repeat(64))),
        ("complete", json!(false)),
    ] {
        let mut v = request();
        v["body"][field] = value;
        assert!(!accepts(&v));
    }
}
#[test]
fn every_field_is_required_even_nullable_and_null_only_fields() {
    let original = request();
    for field in original.as_object().unwrap().keys() {
        let mut v = original.clone();
        v.as_object_mut().unwrap().remove(field);
        assert!(!accepts(&v), "missing {field} accepted");
    }
    for object in ["adapter", "body", "capabilities"] {
        for field in original[object].as_object().unwrap().keys() {
            let mut v = original.clone();
            v[object].as_object_mut().unwrap().remove(field);
            assert!(!accepts(&v), "missing {object}.{field} accepted");
        }
    }
}
#[test]
fn rejects_duplicate_escaped_unknown_and_non_utf8_json() {
    let valid = serde_json::to_string(&request()).unwrap();
    for injected in [
        "\"tenant\":\"t\",",
        "\"ten\\u0061nt\":\"t\",",
        "\"unknown\":null,",
    ] {
        let duplicate = format!("{{{injected}{}", &valid[1..]);
        assert!(parse(duplicate.as_bytes(), 1000).is_err());
    }
    for object in ["adapter", "body", "capabilities"] {
        let mut v = request();
        v[object]["extra"] = json!(null);
        assert!(!accepts(&v));
    }
    assert!(parse(&[0xff], 1000).is_err());
    assert!(parse(format!("\u{feff}{valid}").as_bytes(), 1000).is_err());
    assert!(parse(&vec![b' '; 1_600_001], 1000).is_err());
}
#[test]
fn rejects_ambiguous_framing_targets_and_fabricated_capabilities() {
    for target in [
        "//evil.test/a",
        "/a/../b",
        "/%2e%2e/x",
        "/x%2fy",
        "/x%5Cy",
        "/x%00",
        "/bad%",
        "/bad%ff",
        "/a#fragment",
        "/a b",
    ] {
        let mut v = request();
        v["target"] = json!(target);
        assert!(!accepts(&v), "{target}");
    }
    for headers in [
        json!([["Content-Length", "3"], ["content-length", "3"]]),
        json!([["Host", "other.test"]]),
        json!([["Content-Length", "3"], ["Transfer-Encoding", "chunked"]]),
        json!([["x-forwarded-for", "127.0.0.1"]]),
        json!([["Upgrade", "websocket"]]),
        json!([["Trailer", "digest"]]),
        json!([["X-Test", "bad\r\nvalue"]]),
    ] {
        let mut v = request();
        v["headers"] = headers;
        assert!(!accepts(&v));
    }
    let mut v = request();
    v["capabilities"]["rawTls"] = json!(true);
    assert!(!accepts(&v));
    v = request();
    v["headerMode"] = json!("fetch-normalized");
    assert!(!accepts(&v));
    v["capabilities"]["rawHeaderOrder"] = json!(false);
    v["connectionId"] = json!(null);
    v["peer"] = json!(null);
    assert!(accepts(&v));
    for deadline in [1000_u64, 31001, 9_007_199_254_740_992] {
        let mut v = request();
        v["deadline"] = json!(deadline);
        assert!(!accepts(&v));
    }
}
