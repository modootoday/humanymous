use super::*;
use serde_json::{Value, json};
fn request() -> Value {
    json!({"protocol":"humanymous.adapter/0.1","id":"01234567-89ab-4cde-8fab-0123456789ab","tenant":"t","origin":"o","revision":"r","deadline":2000,"method":"POST","target":"/api?q=1","authority":"example.test","peer":"127.0.0.1","headers":[["Host","example.test"],["Content-Length","3"],["X-Tag","first"],["x-tag","second"]],"headerMode":"node-raw","body":{"layer":"transfer-decoded-content-encoded","complete":true,"length":3,"sha256":hex::encode(Sha256::digest(b"abc")),"base64":"YWJj"},"capabilities":{"rawTls":false,"originalH2":false,"rawHeaderOrder":true,"completeBody":true},"stream":null,"capture":null,"connectionId":"01234567-89ab-4cde-8fab-0123456789ab","adapter":{"id":"humanymous.node","version":"0.1.0-experimental.1","installation":"i"}})
}
fn accepts(v: &Value) -> bool {
    parse(&serde_json::to_vec(v).unwrap(), 1000).is_ok()
}

#[test]
fn language_profiles_reject_cross_mode_identity_and_invented_capture() {
    let profiles = [
        (
            AdapterProfile::PythonAsgi,
            "humanymous.python",
            "asgi-normalized",
        ),
        (
            AdapterProfile::PythonWsgi,
            "humanymous.python",
            "wsgi-normalized",
        ),
        (
            AdapterProfile::RubyRack,
            "humanymous.ruby",
            "rack-normalized",
        ),
    ];
    for (profile, id, mode) in profiles {
        let mut value = request();
        value["adapter"]["id"] = json!(id);
        value["headerMode"] = json!(mode);
        value["peer"] = Value::Null;
        value["connectionId"] = Value::Null;
        value["capabilities"]["rawHeaderOrder"] = json!(false);
        let check = |value: &Value, selected| {
            parse_with_profile(&serde_json::to_vec(value).unwrap(), 1000, selected)
        };
        let parsed = check(&value, profile).unwrap();
        assert_eq!(parsed.body(), b"abc");
        assert_eq!(parsed.header_mode(), mode);
        assert_eq!(parsed.adapter_identity(), (id, "0.1.0-experimental.1"));
        for selected in [
            AdapterProfile::Node,
            AdapterProfile::Nginx,
            AdapterProfile::PythonAsgi,
            AdapterProfile::PythonWsgi,
            AdapterProfile::RubyRack,
        ] {
            assert_eq!(check(&value, selected).is_ok(), selected == profile);
        }
        for capability in ["rawTls", "originalH2", "rawHeaderOrder"] {
            let mut invalid = value.clone();
            invalid["capabilities"][capability] = json!(true);
            assert!(check(&invalid, profile).is_err(), "{mode}: {capability}");
        }
        for (field, replacement) in [
            ("peer", json!("127.0.0.1")),
            (
                "connectionId",
                json!("01234567-89ab-4cde-8fab-0123456789ab"),
            ),
            ("capture", json!({})),
            ("stream", json!({})),
        ] {
            let mut invalid = value.clone();
            invalid[field] = replacement;
            assert!(check(&invalid, profile).is_err(), "{mode}: {field}");
        }
    }
}

#[test]
fn nginx_profile_binds_identity_modes_and_forbids_invented_capture() {
    let mut v = request();
    v["adapter"]["id"] = json!("humanymous.nginx");
    v["connectionId"] = json!(null);
    v["headerMode"] = json!("nginx-raw");
    let check = |value: &Value, profile| {
        parse_with_profile(&serde_json::to_vec(value).unwrap(), 1000, profile)
    };
    let parsed = check(&v, AdapterProfile::Nginx).unwrap();
    assert_eq!(parsed.headers()[2], ["X-Tag", "first"]);
    assert_eq!(parsed.headers()[3], ["x-tag", "second"]);
    assert_eq!(parsed.connection_id(), None);
    assert!(check(&v, AdapterProfile::Node).is_err());
    for (field, replacement) in [
        (
            "connectionId",
            json!("01234567-89ab-4cde-8fab-0123456789ab"),
        ),
        ("headerMode", json!("node-raw")),
        ("capture", json!({})),
        ("stream", json!({})),
        ("protocol", json!("humanymous.adapter/0.2")),
    ] {
        let mut invalid = v.clone();
        invalid[field] = replacement;
        assert!(check(&invalid, AdapterProfile::Nginx).is_err(), "{field}");
    }
    for field in ["rawTls", "originalH2"] {
        let mut invalid = v.clone();
        invalid["capabilities"][field] = json!(true);
        assert!(check(&invalid, AdapterProfile::Nginx).is_err());
    }
    v["headerMode"] = json!("nginx-normalized");
    v["capabilities"]["rawHeaderOrder"] = json!(false);
    assert!(check(&v, AdapterProfile::Nginx).is_err());
    v["peer"] = json!(null);
    assert!(check(&v, AdapterProfile::Nginx).is_ok());
    v["capabilities"]["rawHeaderOrder"] = json!(true);
    assert!(check(&v, AdapterProfile::Nginx).is_err());
}

#[test]
fn provisioned_identity_is_exact_and_legacy_parser_stays_node_only() {
    let mut value = request();
    value["adapter"]["id"] = json!("humanymous.nginx");
    let bytes = serde_json::to_vec(&value).unwrap();
    assert!(parse(&bytes, 1000).is_err());
    let expected = ExpectedAdapter {
        id: "humanymous.nginx",
        version: "0.1.0-experimental.1",
    };
    let parsed = parse_for(&bytes, 1000, expected).unwrap();
    assert_eq!(parsed.adapter_identity(), (expected.id, expected.version));
    assert_eq!(parsed.header_mode(), "node-raw");
    assert_eq!(
        parsed.connection_id(),
        Some("01234567-89ab-4cde-8fab-0123456789ab")
    );
    for invalid in [
        ExpectedAdapter {
            id: expected.id,
            version: "0.2",
        },
        ExpectedAdapter {
            id: "",
            version: expected.version,
        },
        ExpectedAdapter {
            id: "untrusted/adapter",
            version: expected.version,
        },
        ExpectedAdapter {
            id: expected.id,
            version: "",
        },
    ] {
        assert!(parse_for(&bytes, 1000, invalid).is_err());
    }
    for (field, replacement) in [
        ("capture", json!({})),
        ("stream", json!({})),
        ("deadline", json!(1000)),
        ("headerMode", json!("nginx-raw")),
    ] {
        let mut invalid = value.clone();
        invalid[field] = replacement;
        assert!(parse_for(&serde_json::to_vec(&invalid).unwrap(), 1000, expected).is_err());
    }
    value["body"]["sha256"] = json!("0".repeat(64));
    assert!(parse_for(&serde_json::to_vec(&value).unwrap(), 1000, expected).is_err());
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
