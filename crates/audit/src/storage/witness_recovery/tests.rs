use super::*;
use crate::{
    checkpoint::Witness,
    merkle::Tree,
    record::{record_hash, record_mac},
};
use ed25519_dalek::SigningKey;

fn fixture() -> (Vec<Record>, Vec<Checkpoint>, [u8; 32], [u8; 32]) {
    let signer = SigningKey::from_bytes(&[7; 32]);
    let writer = signer.verifying_key().to_bytes();
    let witness = Witness::from_seed(&[8; 32], writer, "node".into());
    let public = witness.public_key();
    let mut records: Vec<Record> = Vec::new();
    let mut checkpoints: Vec<Checkpoint> = Vec::new();
    for sequence in 1..=3 {
        let previous = records
            .last()
            .map_or("", |r| r.record_hash.as_str())
            .to_owned();
        let mut record = Record {
            seq: sequence,
            node_id: "node".into(),
            event_type: "synthetic".into(),
            prev_hash: previous.clone(),
            ..Default::default()
        };
        record.record_hash = record_hash(&record, &previous);
        record.hmac = record_mac(&record.record_hash, b"synthetic-hmac").unwrap();
        records.push(record);
        if sequence <= 2 {
            let leaves: Vec<_> = records
                .iter()
                .map(|r| r.record_hash.as_bytes().to_vec())
                .collect();
            let mut tree = Tree::new(&leaves);
            let mut cp = Checkpoint {
                node_id: "node".into(),
                tree_size: sequence,
                root: records.last().unwrap().record_hash.clone(),
                merkle_root: hex::encode(tree.root()),
                prev_checkpoint_hash: checkpoints
                    .last()
                    .map_or_else(String::new, |c| c.sig.clone()),
                ..Default::default()
            };
            cp.sign(&signer);
            let proof = if sequence == 1 {
                Vec::new()
            } else {
                tree.consistency_proof(1).unwrap()
            };
            cp.witness_sig = witness.counter_sign(&cp, &proof).unwrap();
            checkpoints.push(cp);
        }
    }
    (records, checkpoints, writer, public)
}
fn policy<'a>(
    writer: &'a [u8; 32],
    witness: &'a [u8; 32],
    head: &'a Checkpoint,
) -> RecoveryPolicy<'a> {
    RecoveryPolicy {
        node_id: "node",
        hmac_key: b"synthetic-hmac",
        writer_public: writer,
        witness_public: Some(witness),
        trusted_head: Some(head),
        allow_new: false,
    }
}
#[test]
fn covered_history_and_missing_local_head_have_distinct_plans() {
    let (records, checkpoints, writer, witness) = fixture();
    let head = &checkpoints[1];
    let covered = plan_witness_recovery(
        records[..2].to_vec(),
        checkpoints.clone(),
        policy(&writer, &witness, head),
        TailRecovery::RequireCovered,
    )
    .unwrap();
    assert!(!covered.needs_checkpoint_append());
    assert!(covered.coverage().fully_witnessed);
    assert_eq!(covered.coverage().durable_sequence, 2);
    assert_eq!(covered.coverage().ambiguous_tail_len, 0);
    let repair = plan_witness_recovery(
        records[..2].to_vec(),
        checkpoints[..1].to_vec(),
        policy(&writer, &witness, head),
        TailRecovery::RequireCovered,
    )
    .unwrap();
    assert!(repair.needs_checkpoint_append());
    assert_eq!(
        repair.coverage().local_latest_checkpoint.as_ref(),
        Some(&checkpoints[0])
    );
    assert_eq!(repair.history().checkpoints(), checkpoints);
    assert_eq!(repair.coverage().authenticated_external_head, *head);
}
#[test]
fn complete_unwitnessed_tail_requires_explicit_preservation() {
    let (records, checkpoints, writer, witness) = fixture();
    let head = &checkpoints[1];
    assert!(
        plan_witness_recovery(
            records.clone(),
            checkpoints.clone(),
            policy(&writer, &witness, head),
            TailRecovery::RequireCovered
        )
        .is_err()
    );
    let plan = plan_witness_recovery(
        records.clone(),
        checkpoints.clone(),
        policy(&writer, &witness, head),
        TailRecovery::PreserveAmbiguousTail,
    )
    .unwrap();
    assert_eq!(plan.history().records(), records);
    assert_eq!(plan.coverage().durable_sequence, 3);
    assert_eq!(plan.coverage().witnessed_through, 2);
    assert_eq!(plan.coverage().ambiguous_tail_len, 1);
    assert!(!plan.coverage().fully_witnessed);
    assert!(!plan.needs_checkpoint_append());
    assert_eq!(
        plan.coverage().authentication,
        RecoveryAuthentication {
            hmac_checked: true,
            writer_checked: true,
            witness_checked: true
        }
    );
}
#[test]
fn rollback_fork_bad_hmac_wrong_witness_and_missing_chain_fail_closed() {
    let (records, checkpoints, writer, witness) = fixture();
    let head = &checkpoints[1];
    assert!(
        plan_witness_recovery(
            records.clone(),
            checkpoints.clone(),
            policy(&writer, &witness, &checkpoints[0]),
            TailRecovery::PreserveAmbiguousTail
        )
        .is_err()
    );
    assert!(
        plan_witness_recovery(
            records.clone(),
            Vec::new(),
            policy(&writer, &witness, head),
            TailRecovery::PreserveAmbiguousTail
        )
        .is_err()
    );
    let mut wrong_hmac = policy(&writer, &witness, head);
    wrong_hmac.hmac_key = b"wrong";
    assert!(
        plan_witness_recovery(
            records.clone(),
            checkpoints.clone(),
            wrong_hmac,
            TailRecovery::PreserveAmbiguousTail
        )
        .is_err()
    );
    let wrong = SigningKey::from_bytes(&[9; 32]).verifying_key().to_bytes();
    assert!(
        plan_witness_recovery(
            records.clone(),
            checkpoints.clone(),
            policy(&writer, &wrong, head),
            TailRecovery::PreserveAmbiguousTail
        )
        .is_err()
    );
    let mut forged = head.clone();
    forged.root = "ab".repeat(32);
    assert!(
        plan_witness_recovery(
            records.clone(),
            checkpoints[..1].to_vec(),
            policy(&writer, &witness, &forged),
            TailRecovery::PreserveAmbiguousTail
        )
        .is_err()
    );
    let mut corrupt = records;
    corrupt[2].event_type = "changed".into();
    assert!(
        plan_witness_recovery(
            corrupt,
            checkpoints.clone(),
            policy(&writer, &witness, head),
            TailRecovery::PreserveAmbiguousTail
        )
        .is_err()
    );
}
#[test]
fn recovery_requires_independent_pin_and_cannot_provision() {
    let (records, checkpoints, writer, witness) = fixture();
    let head = &checkpoints[1];
    let mut no_pin = policy(&writer, &witness, head);
    no_pin.trusted_head = None;
    assert!(
        plan_witness_recovery(
            records.clone(),
            checkpoints.clone(),
            no_pin,
            TailRecovery::PreserveAmbiguousTail
        )
        .is_err()
    );
    let mut no_witness = policy(&writer, &witness, head);
    no_witness.witness_public = None;
    assert!(
        plan_witness_recovery(
            records.clone(),
            checkpoints.clone(),
            no_witness,
            TailRecovery::PreserveAmbiguousTail
        )
        .is_err()
    );
    assert!(
        plan_witness_recovery(
            records.clone(),
            checkpoints.clone(),
            policy(&writer, &writer, head),
            TailRecovery::PreserveAmbiguousTail
        )
        .is_err()
    );
    let mut provision = policy(&writer, &witness, head);
    provision.allow_new = true;
    assert!(
        plan_witness_recovery(
            Vec::new(),
            Vec::new(),
            provision,
            TailRecovery::RequireCovered
        )
        .is_err()
    );
}
