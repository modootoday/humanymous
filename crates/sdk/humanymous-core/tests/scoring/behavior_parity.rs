//! Frozen heuristic thresholds and optional ML annotations against the Go scorer.
use humanymous_core::behavior::features::schema_hash;
use humanymous_core::model::inference::{Prediction, Predictor, Runtime};
use humanymous_core::scoring::derivation::behavior_signals;
use humanymous_core::signals::{observation::Signal, report::BehaviorSummary};
use humanymous_test_support::assert_json_equivalent;
use serde::Deserialize;
use std::sync::Arc;

struct Fixed {
    probability: f32,
    schema: String,
}
impl Predictor for Fixed {
    fn predict(&self, _: &[f32]) -> Prediction {
        Prediction {
            p_bot: self.probability,
            abstain: false,
        }
    }
    fn schema_hash(&self) -> &str {
        &self.schema
    }
    fn bundle_version(&self) -> &str {
        "oracle"
    }
}
#[derive(Deserialize)]
struct Case {
    input: BehaviorSummary,
    probability: Option<f32>,
    expected: Option<Vec<Signal>>,
}

#[test]
fn heuristic_thresholds_and_model_annotations_match_go() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/behavior.json")).unwrap();
    assert_eq!(cases.len(), 320);
    for case in cases {
        let runtime = Runtime::default();
        if let Some(probability) = case.probability {
            runtime.set(Some(Arc::new(Fixed {
                probability,
                schema: schema_hash(),
            })));
        }
        assert_json_equivalent(
            &serde_json::to_value(behavior_signals(&case.input, &runtime)).unwrap(),
            &serde_json::to_value(case.expected.unwrap_or_default()).unwrap(),
        );
    }
}
