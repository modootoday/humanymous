//! Every captured prefix, malformed extension and opaque protocol against Go.
use humanymous_platform::testing::assert_json_equivalent;
use humanymous_runtime::network::tls::{
    engine_from_client_hello, extension_order_hash, ja3, ja4, parse_client_hello, parse_record,
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct Case {
    input: String,
    record: bool,
    hello: Option<Value>,
    ja4: String,
    raw: String,
}

#[test]
fn raw_observation_and_ja4_match_frozen_go() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../fixtures/network/tls.json")).unwrap();
    assert!(cases.len() > 2000);
    for case in cases {
        let input = hex::decode(case.input).unwrap();
        let result = if case.record {
            parse_record(&input)
        } else {
            parse_client_hello(&input)
        };
        let Some(expected) = case.hello else {
            assert!(result.is_err());
            continue;
        };
        let hello = result.unwrap();
        let actual = json!({
            "ja3":ja3(&hello),
            "engine":engine_from_client_hello(Some(&hello)),"extension_order_hash":extension_order_hash(Some(&hello)),
            "legacy_version":hello.legacy_version,"supported_versions":hello.supported_versions,
            "cipher_suites":hello.cipher_suites,"extensions":hello.extensions,
            "curves":hello.curves,"point_formats":hello.point_formats,
            "signature_algorithms":hello.signature_algorithms,
            "alpn_hex":hello.alpn.iter().map(hex::encode).collect::<Vec<_>>(),
            "sni_present":hello.sni_present,
        });
        assert_json_equivalent(&actual, &expected);
        assert_eq!(ja4(&hello), (case.ja4, case.raw));
    }
}
