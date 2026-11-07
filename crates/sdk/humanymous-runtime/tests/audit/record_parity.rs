//! Historical record wire shape and canonical hash against actual frozen source.
use humanymous_platform::testing::assert_json_equivalent;
use humanymous_runtime::audit::record::{Record, canonical_bytes, record_hash};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Case {
    input: Record,
    json: Value,
    previous: String,
    canonical: String,
    hash: String,
}
#[derive(Deserialize)]
struct Corpus {
    unicode: String,
    records: Vec<Case>,
}

#[test]
fn canonical_bytes_and_record_hashes_match_go_without_mutating_records() {
    let corpus: Corpus =
        serde_json::from_str(include_str!("../fixtures/audit/records.json")).unwrap();
    assert_eq!(corpus.unicode, "15.0.0");
    assert_eq!(corpus.records.len(), 92);
    for case in corpus.records {
        let before = case.input.clone();
        assert_json_equivalent(&serde_json::to_value(&case.input).unwrap(), &case.json);
        assert_eq!(hex::encode(canonical_bytes(&case.input)), case.canonical);
        assert_eq!(record_hash(&case.input, &case.previous), case.hash);
        assert_eq!(case.input, before);
    }
}
