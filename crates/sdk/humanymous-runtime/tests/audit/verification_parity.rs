//! Full mismatch precedence and signed commitment checks against frozen Go.
use humanymous_platform::testing::assert_json_equivalent;
use humanymous_runtime::audit::{
    checkpoint::Checkpoint,
    record::Record,
    verification::{verify, verify_with_witness},
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Case {
    records: Vec<Record>,
    checkpoints: Vec<Checkpoint>,
    key: String,
    public: String,
    witness: String,
    expected: Value,
}
fn key32(text: &str) -> [u8; 32] {
    hex::decode(text).unwrap().try_into().unwrap()
}

#[test]
fn every_verifier_result_and_priority_matches_go() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../fixtures/audit/verification.json")).unwrap();
    assert_eq!(cases.len(), 840);
    for case in cases {
        let key = hex::decode(case.key).unwrap();
        let public = key32(&case.public);
        let report = if case.witness.is_empty() {
            verify(&case.records, &case.checkpoints, &key, Some(&public))
        } else {
            verify_with_witness(
                &case.records,
                &case.checkpoints,
                &key,
                Some(&public),
                &key32(&case.witness),
            )
        };
        assert_json_equivalent(&serde_json::to_value(report).unwrap(), &case.expected);
    }
}
