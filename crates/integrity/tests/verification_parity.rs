//! RIT failure precedence and JSON signal parity against the Go oracle.
use humanymous_integrity::request::{VerifyParams, verify};
use humanymous_test_support::assert_json_equivalent;
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    key: String,
    last_n: u64,
    presented_n: u64,
    presented_tb: u64,
    current_tb: u64,
    token: String,
    canonical: String,
    first_req: bool,
    expected: serde_json::Value,
}

#[test]
fn verifier_signals_and_precedence_match_go() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("fixtures/verification.json")).unwrap();
    assert_eq!(cases.len(), 27);
    for case in cases {
        let key = hex::decode(case.key).unwrap();
        let actual = verify(VerifyParams {
            session_key: &key,
            last_n: case.last_n,
            presented_n: case.presented_n,
            presented_tb: case.presented_tb,
            current_tb: case.current_tb,
            token: &case.token,
            canonical: &case.canonical,
            first_req: case.first_req,
        });
        assert_json_equivalent(&serde_json::to_value(actual).unwrap(), &case.expected);
    }
}
