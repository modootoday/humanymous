//! Probe-derived predicates and signal payloads compared with the frozen Go engine.
use humanymous_core::scoring::derivation::{advanced_signals, environment_signals, guard_signals};
use humanymous_core::signals::{observation::Signal, report::ClientReport};
use humanymous_platform::testing::assert_json_equivalent;
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    client: ClientReport,
    has_client: bool,
    environment: Option<Vec<Signal>>,
    privacy: bool,
    guard: Option<Vec<Signal>>,
    advanced: Option<Vec<Signal>>,
}

fn compare(actual: Vec<Signal>, expected: Option<Vec<Signal>>) {
    assert_json_equivalent(
        &serde_json::to_value(actual).unwrap(),
        &serde_json::to_value(expected.unwrap_or_default()).unwrap(),
    );
}

#[test]
fn all_probe_derivation_paths_match_go() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/derivation.json")).unwrap();
    assert_eq!(cases.len(), 260);
    for case in cases {
        let environment = environment_signals(&case.client.environment, case.has_client);
        assert_eq!(environment.privacy, case.privacy);
        compare(environment.signals, case.environment);
        compare(guard_signals(&case.client.guard), case.guard);
        compare(
            advanced_signals(&case.client.advanced, &case.client.user_agent),
            case.advanced,
        );
    }
}
