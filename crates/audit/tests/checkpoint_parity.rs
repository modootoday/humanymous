//! Writer/witness signatures and secondary record MACs against frozen Go logs.
use ed25519_dalek::{Signer, SigningKey};
use humanymous_audit::{
    checkpoint::Checkpoint,
    record::{Record, record_hash, record_mac, verify_record_mac},
};
use humanymous_audit::{checkpoint::Witness, merkle::Tree};
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    seed: String,
    witness_seed: String,
    key: String,
    public: String,
    witness_public: String,
    checkpoint: Checkpoint,
    records: Vec<Record>,
}
fn key32(text: &str) -> [u8; 32] {
    hex::decode(text).unwrap().try_into().unwrap()
}

#[test]
fn checkpoint_keys_signature_bytes_and_hmacs_match_go() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/checkpoints.json")).unwrap();
    assert_eq!(cases.len(), 180);
    for case in cases {
        let signer = SigningKey::from_bytes(&key32(&case.seed));
        let witness = SigningKey::from_bytes(&key32(&case.witness_seed));
        let public = key32(&case.public);
        let witness_public = key32(&case.witness_public);
        assert_eq!(signer.verifying_key().to_bytes(), public);
        assert_eq!(witness.verifying_key().to_bytes(), witness_public);
        let mut checkpoint = case.checkpoint.clone();
        checkpoint.sign(&signer);
        assert_eq!(checkpoint.sig, case.checkpoint.sig);
        if case.checkpoint.witness_sig.is_empty() {
            assert_eq!(case.checkpoint.tree_size, 0);
            assert!(!case.checkpoint.verify_witness(&witness_public));
        } else {
            assert_eq!(
                hex::encode(witness.sign(&checkpoint.signing_bytes()).to_bytes()),
                case.checkpoint.witness_sig
            );
            assert!(case.checkpoint.verify_witness(&witness_public));
        }
        assert!(case.checkpoint.verify_writer(&public));
        checkpoint.root.push('x');
        assert!(!checkpoint.verify_writer(&public));
        let key = hex::decode(case.key).unwrap();
        for record in case.records {
            assert_eq!(record_hash(&record, &record.prev_hash), record.record_hash);
            assert_eq!(record_mac(&record.record_hash, &key).unwrap(), record.hmac);
            assert!(verify_record_mac(&record.record_hash, &record.hmac, &key));
            assert!(!verify_record_mac(
                &record.record_hash,
                &record.hmac.to_uppercase(),
                &key
            ));
            assert!(!verify_record_mac(&record.record_hash, &record.hmac, &[]));
        }
    }
}

#[test]
fn stateful_witness_keeps_go_signatures_for_valid_append_only_heads() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/checkpoints.json")).unwrap();
    let (groups, remainder) = cases.as_chunks::<12>();
    assert!(remainder.is_empty());
    for group in groups {
        let initial = &group[3];
        let seed = key32(&initial.witness_seed);
        let public = key32(&initial.public);
        let witness = Witness::from_seed(&seed, public, initial.checkpoint.node_id.clone());
        for case in group.iter().filter(|case| case.checkpoint.tree_size > 0) {
            let old = witness.last_size().unwrap() as usize;
            let leaves = case
                .records
                .iter()
                .map(|record| record.record_hash.as_bytes().to_vec())
                .collect::<Vec<_>>();
            let proof = if old > 0 && old < leaves.len() {
                Tree::new(&leaves).consistency_proof(old).unwrap()
            } else {
                vec![]
            };
            assert_eq!(
                witness.counter_sign(&case.checkpoint, &proof).unwrap(),
                case.checkpoint.witness_sig
            );
        }
        let restored = Witness::from_seed(&seed, public, initial.checkpoint.node_id.clone());
        restored.restore(&group.last().unwrap().checkpoint).unwrap();
        assert_eq!(restored.last_size().unwrap(), 8);
    }
}
