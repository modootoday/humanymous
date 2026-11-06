//! Independent Go vectors freeze canonical bytes and token compatibility.
use humanymous_integrity::request::{
    CanonicalInput, canonical, compute_token, lower_header_map, seed, session_key, verify_token,
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Case {
    key: String,
    session: String,
    counter: u64,
    bucket: u64,
    headers: BTreeMap<String, String>,
    lowered: BTreeMap<String, String>,
    cookie: String,
    body_hex: String,
    canonical: String,
    session_key: String,
    seed: String,
    token: String,
}

#[test]
fn matches_go_canonical_and_rotation_protocols() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/request.json")).unwrap();
    assert_eq!(cases.len(), 81, "oracle coverage must not silently shrink");
    for case in cases {
        let headers = lower_header_map(&case.headers);
        assert_eq!(headers, case.lowered);
        let body = hex::decode(case.body_hex).unwrap();
        let text = canonical(CanonicalInput {
            headers: &headers,
            cookie_id: &case.cookie,
            body: &body,
        });
        assert_eq!(text, case.canonical);
        let key = session_key(case.key.as_bytes(), &case.session);
        assert_eq!(hex::encode(key), case.session_key);
        let seed = seed(&key, case.counter);
        assert_eq!(hex::encode(seed), case.seed);
        assert_eq!(
            compute_token(&seed, &text, case.counter, case.bucket),
            case.token
        );
        assert!(verify_token(
            &seed,
            &text,
            case.counter,
            case.bucket,
            &case.token
        ));
    }
}
