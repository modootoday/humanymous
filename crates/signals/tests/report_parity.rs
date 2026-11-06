//! Every report field is covered by independent zero, populated and null Go fixtures.
use humanymous_signals::report::SessionReport;
use humanymous_test_support::assert_json_equivalent;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Case {
    kind: String,
    input: Value,
    expected: Value,
}

#[test]
fn complete_report_shape_and_defaults_match_go() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/reports.json")).unwrap();
    assert_eq!(cases.len(), 4);
    for case in cases {
        let report: SessionReport = serde_json::from_value(case.input)
            .unwrap_or_else(|error| panic!("{}: {error}", case.kind));
        assert_json_equivalent(&serde_json::to_value(report).unwrap(), &case.expected);
    }
}
