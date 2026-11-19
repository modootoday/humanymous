//! Go golden vectors for the legacy pass_proof.go trace digest (Go 1.25.3).
use super::*;

fn hex(d: TraceDigest) -> String {
    hex::encode(d.0)
}

#[test]
fn digest_matches_go_golden() {
    let proof = PassProof {
        raw_t: vec![0.0, 16.4, 33.5, 50.49, -2.5],
        durations: vec![16.7, 17.2],
        pressures: vec![0.5, 0.525, 0.024],
        key_durs: vec![120.5, 98.25],
        ..PassProof::default()
    };
    assert_eq!(
        hex(trace_digest(&proof).unwrap()),
        "897c5bbac00e068f67777a3a10b1d7f19deaba9325c9e4b33fde13aebc22ab22"
    );
    assert_eq!(
        hex(trace_digest(&PassProof::default()).unwrap()),
        "45ca31c3315a5978f40438aab46040d75e99c9b125c2fd01db6e10ac80bef906"
    );
}

#[test]
fn digest_ignores_instance_fields_and_collides_on_sub_quantum_replay() {
    let base = PassProof {
        raw_t: vec![1.0, 17.3, 34.1],
        durations: vec![16.3, 16.8],
        ..PassProof::default()
    };
    let rewrapped = PassProof {
        bucket: 9,
        challenge_nonce: "fresh".into(),
        offsets: vec![3, -1, 2],
        attest_token: "t".into(),
        raw_t: vec![1.2, 17.1, 34.0],
        ..base.clone()
    };
    assert_eq!(trace_digest(&base), trace_digest(&rewrapped));
    let jittered = PassProof {
        raw_t: vec![2.0, 17.3, 34.1],
        ..base.clone()
    };
    assert_ne!(trace_digest(&base), trace_digest(&jittered));
    let moved = PassProof {
        durations: vec![],
        key_durs: vec![16.3, 16.8],
        ..base.clone()
    };
    assert_ne!(
        trace_digest(&base),
        trace_digest(&moved),
        "channel separators keep lanes distinct"
    );
}

/// Legacy TestTraceDigestDistinguishesDifferentMotion.
#[test]
fn distinct_key_durations_produce_distinct_digests() {
    let a = PassProof {
        key_durs: vec![137.0, 89.0, 211.0, 123.0],
        ..PassProof::default()
    };
    let b = PassProof {
        key_durs: vec![140.0, 92.0, 205.0, 119.0],
        ..PassProof::default()
    };
    assert_ne!(trace_digest(&a), trace_digest(&b));
}

#[test]
fn non_finite_samples_refuse() {
    for bad in [f64::NAN, f64::INFINITY, -f64::INFINITY, 1e300] {
        let proof = PassProof {
            pressures: vec![bad],
            ..PassProof::default()
        };
        assert_eq!(trace_digest(&proof), None, "{bad}");
    }
}

#[test]
fn wire_model_uses_legacy_field_names() {
    let proof: PassProof = serde_json::from_str(
        r#"{"bucket":7,"challengeNonce":"n","attestToken":"a","offsets":[1,-2],
            "trusted":true,"moves":6,"coalesced":12,"durations":[1.5],"pathLen":40.5,
            "rawT":[0.1],"pressures":[0.3],"pointerType":"touch","keys":0,
            "keyDurs":[],"motion":[0.01]}"#,
    )
    .unwrap();
    assert_eq!(proof.bucket, 7);
    assert_eq!(proof.offsets, vec![1, -2]);
    assert_eq!(proof.pointer_type, "touch");
    assert_eq!(proof.raw_t, vec![0.1]);
    assert_eq!(proof.path_len, 40.5);
    let missing: PassProof = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, PassProof::default());
}

fn golden() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/gateway/pass-flow-go-golden.json"
    ))
    .unwrap()
}

fn bits(value: &serde_json::Value) -> f64 {
    f64::from_bits(u64::from_str_radix(value.as_str().unwrap(), 16).unwrap())
}

/// B1 differential: `meanFloats`, `stddev` and `allIntegerMs` are bit-exact
/// against the Go oracle, including empty, overflowing and near-integer input.
#[test]
fn statistics_match_go_golden_bit_for_bit() {
    let golden = golden();
    let cases = golden["outputs"]["stats"].as_array().unwrap();
    assert!(cases.len() >= 18);
    for case in cases {
        let input: Vec<f64> = case["input"].as_array().unwrap().iter().map(bits).collect();
        assert_eq!(
            mean(&input).to_bits(),
            bits(&case["mean"]).to_bits(),
            "{input:?}"
        );
        assert_eq!(
            stddev(&input).to_bits(),
            bits(&case["stddev"]).to_bits(),
            "{input:?}"
        );
        assert_eq!(
            all_integer_ms(&input),
            case["allIntegerMs"].as_bool().unwrap(),
            "{input:?}"
        );
    }
}

/// B1 differential: `realEventOK` accepts and rejects with Go's reason for
/// every branch of the oracle, decoding the exact bytes Go decoded.
#[test]
fn real_event_filter_matches_go_golden() {
    let golden = golden();
    let cases = golden["outputs"]["realEvent"].as_array().unwrap();
    assert!(cases.len() >= 21);
    for case in cases {
        let text = case["proof"].as_str().unwrap();
        let proof: PassProof = serde_json::from_str(text).unwrap();
        let expected = if case["ok"].as_bool().unwrap() {
            Ok(())
        } else {
            Err(case["reason"].as_str().unwrap())
        };
        assert_eq!(real_event_ok(&proof), expected, "{text}");
    }
}

/// Legacy TestBehaviorTimingTells: fractional human timing is not integer,
/// is slower than 15 ms and varies; integer machine timing is the opposite.
#[test]
fn behavior_timing_statistics_separate_human_and_machine() {
    let human = [137.3, 89.7, 211.4, 123.8];
    let machine = [7.0, 5.0, 9.0, 6.0];
    assert!(!all_integer_ms(&human));
    assert!(all_integer_ms(&machine));
    assert!(mean(&human) >= 15.0);
    assert!(mean(&machine) < 15.0);
    assert!(stddev(&human) >= 6.0);
    assert!(stddev(&machine) < 6.0);
}

/// Go `int` fields accept negative counts; a negative move count is simply
/// not a pointer channel.
#[test]
fn negative_counts_decode_and_are_not_interaction() {
    let proof: PassProof =
        serde_json::from_str(r#"{"trusted":true,"moves":-5,"pathLen":20,"keys":-1}"#).unwrap();
    assert_eq!(proof.moves, -5);
    assert_eq!(real_event_ok(&proof), Err("insufficient interaction"));
}
