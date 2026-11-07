//! Full-constructor JSON parity with frozen independent Go observations.
use humanymous_core::signals::observation::Signal;
use humanymous_platform::testing::assert_json_equivalent;

#[test]
fn all_definitions_and_constructor_boundaries_match_go() {
    let expected: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("fixtures/observations.json")).unwrap();
    assert_eq!(expected.len(), 193);
    for value in expected {
        let observation: Signal = serde_json::from_value(value.clone()).unwrap();
        let actual = Signal::new(
            &observation.id,
            observation.value,
            observation.verdict,
            observation.confidence,
            observation.collected,
            &observation.notes,
        );
        assert_json_equivalent(&serde_json::to_value(actual).unwrap(), &value);
    }
}
