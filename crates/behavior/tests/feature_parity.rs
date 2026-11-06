//! Exact float32 output and feature-schema parity with independent Go vectors.
use humanymous_behavior::features::{FEATURE_NAMES, extract, schema_hash};
use humanymous_signals::report::BehaviorSummary;
use serde::Deserialize;

#[derive(Deserialize)]
struct Corpus {
    names: Vec<String>,
    hash: String,
    vectors: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    input: BehaviorSummary,
    expected: Vec<f32>,
}

#[test]
fn schema_and_all_extraction_values_match_go() {
    let corpus: Corpus = serde_json::from_str(include_str!("fixtures/features.json")).unwrap();
    assert_eq!(corpus.names, FEATURE_NAMES);
    assert_eq!(corpus.hash, schema_hash());
    assert_eq!(corpus.vectors.len(), 68);
    for case in corpus.vectors {
        assert_eq!(extract(&case.input).as_slice(), case.expected.as_slice());
    }
}
