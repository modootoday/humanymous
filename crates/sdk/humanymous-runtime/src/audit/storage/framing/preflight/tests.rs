use super::*;
use crate::audit::record::Record;

fn limits() -> Limits {
    Limits {
        strings: 4 * 1024 * 1024,
        array_items: 4096,
        values: 65536,
        depth: 64,
    }
}

#[test]
fn exact_decoded_string_bytes_include_keys_escapes_and_surrogate_pairs() {
    for (json, length) in [
        (r#""é""#, 2),
        (r#""\u00e9""#, 2),
        (r#""\uD83D\uDE00""#, 4),
        (r#""\n\t\"\\""#, 4),
        (r#"{"a":"<"}"#, 2),
        (r#""\\u003c""#, 6),
    ] {
        let mut budget = limits();
        budget.strings = length;
        assert_eq!(check(json.as_bytes(), budget).unwrap().strings, length);
        if length > 1 {
            budget.strings -= 1;
            assert_eq!(check(json.as_bytes(), budget), Err(Exhausted::Strings));
        }
    }
}

#[test]
fn independent_array_limits_values_and_fixed_depth_refuse_before_owned_decode() {
    let mut budget = limits();
    budget.array_items = 2;
    let counts = check(br#"{"a":[null,{}],"b":[true,false]}"#, budget).unwrap();
    assert_eq!(counts.values, 7);
    assert_eq!(counts.depth, 3);
    assert_eq!(check(b"[null,{},[]]", budget), Err(Exhausted::ArrayItems));
    budget.values = 6;
    assert_eq!(
        check(br#"{"a":[null,{}],"b":[true,false]}"#, budget),
        Err(Exhausted::Values)
    );
    let nested = format!("{}0{}", "[".repeat(64), "]".repeat(64));
    assert_eq!(check(nested.as_bytes(), limits()).unwrap().depth, 64);
    let nested = format!("[{nested}]");
    assert_eq!(check(nested.as_bytes(), limits()), Err(Exhausted::Depth));
    budget = limits();
    budget.depth = 65;
    assert_eq!(check(b"{}", budget), Err(Exhausted::InvalidLimits));
    budget = limits();
    budget.values = 0;
    assert_eq!(check(b"{}", budget), Err(Exhausted::InvalidLimits));
}

#[test]
fn unknown_numbers_are_lexical_and_semantics_stay_with_existing_decoder() {
    for literal in ["1e400", "-1e400", "184467440737095516160000000000"] {
        let bytes = format!("{{\"unknown\":{literal}}}");
        assert_eq!(check(bytes.as_bytes(), limits()).unwrap().values, 2);
        assert_eq!(
            serde_json::from_str::<Record>(&bytes).unwrap(),
            Record::default()
        );
    }
    for bytes in [b"[1,]".as_slice(), b"{\"actor\":7}", b"{} {}"] {
        assert!(check(bytes, limits()).is_ok());
        assert!(serde_json::from_slice::<Record>(bytes).is_err());
    }
}

#[test]
fn checked_budget_arithmetic_never_wraps_or_mutates_on_refusal() {
    let mut count = usize::MAX;
    assert_eq!(
        add(&mut count, 1, usize::MAX, Exhausted::Values),
        Err(Exhausted::Values)
    );
    assert_eq!(count, usize::MAX);
}

fn oracle(value: &serde_json::Value, depth: usize, counts: &mut Counts) {
    counts.values += 1;
    match value {
        serde_json::Value::String(text) => counts.strings += text.len(),
        serde_json::Value::Array(items) => {
            counts.depth = counts.depth.max(depth + 1);
            for item in items {
                oracle(item, depth + 1, counts);
            }
        }
        serde_json::Value::Object(fields) => {
            counts.depth = counts.depth.max(depth + 1);
            for (key, value) in fields {
                counts.strings += key.len();
                oracle(value, depth + 1, counts);
            }
        }
        _ => {}
    }
}

#[test]
fn frozen_record_and_checkpoint_inputs_match_independent_owned_count_oracle() {
    let records: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/audit/records.json"
    ))
    .unwrap();
    let checkpoints: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/audit/checkpoints.json"
    ))
    .unwrap();
    let records = records["records"].as_array().unwrap();
    let checkpoints = checkpoints.as_array().unwrap();
    assert_eq!(records.len(), 92);
    assert_eq!(checkpoints.len(), 180);
    for value in records
        .iter()
        .map(|case| &case["json"])
        .chain(checkpoints.iter().map(|case| &case["checkpoint"]))
    {
        let bytes = serde_json::to_vec(value).unwrap();
        let mut expected = Counts::default();
        oracle(value, 0, &mut expected);
        assert_eq!(check(&bytes, limits()).unwrap(), expected);
    }
}
