//! Coarse claims, execution evidence and every platform family against Go L6 output.
use humanymous_scoring::consistency::cross_checks;
use humanymous_signals::report::{CrossCheck, SessionReport};
use humanymous_test_support::assert_json_equivalent;
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    report: SessionReport,
    expected: Option<Vec<CrossCheck>>,
}

#[test]
fn crosscheck_order_predicates_weights_and_notes_match_go() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/crosschecks.json")).unwrap();
    assert_eq!(cases.len(), 504);
    for case in cases {
        assert_json_equivalent(
            &serde_json::to_value(cross_checks(&case.report)).unwrap(),
            &serde_json::to_value(case.expected.unwrap_or_default()).unwrap(),
        );
    }
}
