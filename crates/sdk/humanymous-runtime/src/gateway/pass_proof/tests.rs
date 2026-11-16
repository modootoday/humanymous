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
