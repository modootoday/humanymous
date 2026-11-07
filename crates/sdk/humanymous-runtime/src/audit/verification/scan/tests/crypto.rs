use super::*;
use crate::audit::{
    merkle::Tree,
    record::{record_hash, record_mac},
    verification::{Mismatch, verify, verify_with_witness},
};
use ed25519_dalek::{Signer, SigningKey};
use serde::Serialize;

fn lines<T: Serialize>(values: &[T]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in values {
        serde_json::to_writer(&mut bytes, value).unwrap();
        bytes.push(b'\n');
    }
    bytes
}
fn history() -> (Vec<Record>, Vec<Checkpoint>, SigningKey, SigningKey) {
    let writer = SigningKey::from_bytes(&[7; 32]);
    let witness = SigningKey::from_bytes(&[8; 32]);
    let mut records = Vec::<Record>::new();
    let mut checkpoints = Vec::<Checkpoint>::new();
    for seq in 1..=3 {
        let mut record = Record {
            seq,
            node_id: "node".into(),
            prev_hash: records
                .last()
                .map_or_else(String::new, |record| record.record_hash.clone()),
            ..Default::default()
        };
        record.record_hash = record_hash(&record, &record.prev_hash);
        record.hmac = record_mac(&record.record_hash, b"key").unwrap();
        records.push(record);
        let leaves = records
            .iter()
            .map(|record| record.record_hash.as_bytes().to_vec())
            .collect::<Vec<_>>();
        let mut checkpoint = Checkpoint {
            node_id: "node".into(),
            tree_size: seq,
            root: records.last().unwrap().record_hash.clone(),
            merkle_root: hex::encode(Tree::new(&leaves).root()),
            prev_checkpoint_hash: checkpoints
                .last()
                .map_or_else(String::new, |checkpoint| checkpoint.sig.clone()),
            ..Default::default()
        };
        checkpoint.sign(&writer);
        checkpoint.witness_sig = hex::encode(witness.sign(&checkpoint.signing_bytes()).to_bytes());
        checkpoints.push(checkpoint);
    }
    (records, checkpoints, writer, witness)
}

fn run(source: &Fixture, policy: &RecoveryPolicy<'_>) -> io::Result<super::super::Diagnostic> {
    let cancel = AtomicBool::new(false);
    let control = Control::new(&cancel, Duration::from_secs(30))?;
    super::super::inspect(source, policy, &limits(), &control)
}
fn compare(
    records: &[Record],
    checkpoints: &[Checkpoint],
    writer: &SigningKey,
    witness: Option<&SigningKey>,
    head: Option<&Checkpoint>,
) -> super::super::Diagnostic {
    let public = writer.verifying_key().to_bytes();
    let witness = witness.map(|key| key.verifying_key().to_bytes());
    let policy = RecoveryPolicy {
        node_id: "node",
        hmac_key: b"key",
        writer_public: &public,
        witness_public: witness.as_ref(),
        trusted_head: head,
        allow_new: false,
    };
    let expected = match witness.as_ref() {
        Some(witness) => verify_with_witness(records, checkpoints, b"key", Some(&public), witness),
        None => verify(records, checkpoints, b"key", Some(&public)),
    };
    let source = fixture(
        records
            .iter()
            .map(|record| lines(std::slice::from_ref(record)))
            .collect(),
        Some(lines(checkpoints)),
    );
    let diagnostic = run(&source, &policy).unwrap();
    assert_eq!(diagnostic.report.as_ref(), Some(&expected));
    assert_eq!(diagnostic.work.record_attempts, records.len());
    assert_eq!(source.checks.get(), 4);
    diagnostic
}

#[test]
fn complete_two_pass_crypto_and_frontier_join_match_sequential_scope() {
    let (records, checkpoints, writer, witness) = history();
    let result = compare(
        &records,
        &checkpoints,
        &writer,
        Some(&witness),
        Some(&checkpoints[0]),
    );
    assert!(result.report.unwrap().ok);
    assert_eq!(result.work.base_successes, 3);
    assert_eq!(result.work.witness_successes, 3);
}

#[test]
fn sparse_prefix_checkpoints_cross_power_boundaries_and_keep_unsigned_tail() {
    let writer = SigningKey::from_bytes(&[7; 32]);
    let witness = SigningKey::from_bytes(&[8; 32]);
    let mut records = Vec::<Record>::new();
    for seq in 1..=65 {
        let mut record = Record {
            seq,
            node_id: "node".into(),
            prev_hash: records
                .last()
                .map_or_else(String::new, |record| record.record_hash.clone()),
            ..Default::default()
        };
        record.record_hash = record_hash(&record, &record.prev_hash);
        record.hmac = record_mac(&record.record_hash, b"key").unwrap();
        records.push(record);
    }
    let leaves = records
        .iter()
        .map(|record| record.record_hash.as_bytes().to_vec())
        .collect::<Vec<_>>();
    let mut reference = Tree::new(&leaves);
    let mut checkpoints = Vec::<Checkpoint>::new();
    for size in [1, 2, 3, 4, 7, 8, 9, 16, 17, 32, 33, 63, 64] {
        let mut checkpoint = Checkpoint {
            node_id: "node".into(),
            tree_size: size as u64,
            root: records[size - 1].record_hash.clone(),
            merkle_root: hex::encode(reference.root_at(size).unwrap()),
            prev_checkpoint_hash: checkpoints
                .last()
                .map_or_else(String::new, |checkpoint| checkpoint.sig.clone()),
            ..Default::default()
        };
        checkpoint.sign(&writer);
        checkpoint.witness_sig = hex::encode(witness.sign(&checkpoint.signing_bytes()).to_bytes());
        checkpoints.push(checkpoint);
    }
    let public = writer.verifying_key().to_bytes();
    let witness_public = witness.verifying_key().to_bytes();
    let expected = verify_with_witness(
        &records,
        &checkpoints,
        b"key",
        Some(&public),
        &witness_public,
    );
    let policy = RecoveryPolicy {
        writer_public: &public,
        witness_public: Some(&witness_public),
        trusted_head: Some(&checkpoints[4]),
        ..policy(None)
    };
    let result = run(
        &fixture(vec![lines(&records)], Some(lines(&checkpoints))),
        &policy,
    )
    .unwrap();
    assert_eq!(result.report, Some(expected));
    assert_eq!(result.work.record_successes, 65);
    assert_eq!(result.work.base_successes, 13);
    assert_eq!(checkpoints.last().unwrap().tree_size, 64);
}

#[test]
fn later_record_after_trusted_pin_overrides_early_witness_failure() {
    let (mut records, mut checkpoints, writer, witness) = history();
    checkpoints[0].witness_sig.clear();
    records[2].hmac = "bad".into();
    let result = compare(
        &records,
        &checkpoints,
        &writer,
        Some(&witness),
        Some(&checkpoints[0]),
    );
    let report = result.report.unwrap();
    assert_eq!(report.class, Mismatch::HmacInvalid);
    assert_eq!(report.scope.records, 2);
    assert_eq!(report.scope.checkpoints, 0);
    assert_eq!(result.work.base_successes, 3);
    assert_eq!(result.work.witness_attempts, 3);
    let (mut records, checkpoints, _, _) = history();
    records[1].seq = 99;
    records[1].prev_hash = "wrong".into();
    let result = compare(
        &records,
        &checkpoints,
        &writer,
        Some(&witness),
        Some(&checkpoints[0]),
    );
    let report = result.report.unwrap();
    assert_eq!(report.class, Mismatch::SequenceGap);
    assert_eq!(report.at_seq, 99);
    assert_eq!(report.scope.records, 1);
    assert_eq!(result.work.record_successes, 2);
    assert_eq!(result.work.base_successes, 3);
}

#[test]
fn later_base_failure_overrides_witness_and_witness_scope_waits_for_all_base() {
    let (records, mut checkpoints, writer, witness) = history();
    checkpoints[0].witness_sig.clear();
    checkpoints[2].root = "different".into();
    checkpoints[2].sign(&writer);
    checkpoints[2].witness_sig =
        hex::encode(witness.sign(&checkpoints[2].signing_bytes()).to_bytes());
    let result = compare(&records, &checkpoints, &writer, Some(&witness), None);
    let report = result.report.unwrap();
    assert_eq!(report.class, Mismatch::CheckpointMismatch);
    assert_eq!(report.scope.checkpoints, 2);
    assert_eq!(report.scope.witnessed_checkpoints, 0);
    let (_, mut checkpoints, _, _) = history();
    checkpoints[1].witness_sig.clear();
    let result = compare(&records, &checkpoints, &writer, Some(&witness), None);
    let report = result.report.unwrap();
    assert_eq!(report.scope.checkpoints, 3);
    assert_eq!(report.scope.witnessed_checkpoints, 1);
    assert_eq!(result.work.witness_successes, 2);
}

#[test]
fn malformed_raw_hash_and_signature_links_are_preserved_after_first_failure() {
    let (mut records, mut checkpoints, writer, _) = history();
    records.truncate(2);
    records[0].record_hash = "x".repeat(200);
    records[1].prev_hash = records[0].record_hash.clone();
    records[1].record_hash = record_hash(&records[1], &records[1].prev_hash);
    records[1].hmac = record_mac(&records[1].record_hash, b"key").unwrap();
    let result = compare(&records, &[], &writer, None, None);
    assert_eq!(result.report.unwrap().class, Mismatch::HashBreak);
    assert_eq!(result.work.record_successes, 1);
    let (records, _, _, _) = history();
    checkpoints.truncate(2);
    checkpoints[0].sig = "x".repeat(300);
    checkpoints[1].prev_checkpoint_hash = checkpoints[0].sig.clone();
    checkpoints[1].sign(&writer);
    let result = compare(&records, &checkpoints, &writer, None, None);
    assert_eq!(result.report.unwrap().class, Mismatch::CheckpointMismatch);
    assert_eq!(result.work.base_successes, 1);
}

#[test]
fn raw_changed_between_passes_refuses_even_when_crypto_fields_still_match() {
    let (records, checkpoints, writer, _) = history();
    let public = writer.verifying_key().to_bytes();
    let policy = RecoveryPolicy {
        writer_public: &public,
        trusted_head: Some(&checkpoints[0]),
        ..policy(None)
    };
    let bytes = lines(&records);
    let mut source = fixture(vec![bytes.clone()], Some(lines(&checkpoints)));
    let mut changed = bytes.clone();
    changed.insert(changed.len() - 1, b' ');
    source.second_pass_segments = Some(vec![changed]);
    assert_eq!(
        run(&source, &policy).err().unwrap().to_string(),
        "audit source transcript changed between passes"
    );
    let mut source = fixture(vec![bytes], Some(lines(&checkpoints)));
    source.second_pass_sidecar = Some(None);
    assert!(run(&source, &policy).is_err());
}

#[test]
fn checkpoint_beyond_eof_and_missing_required_witness_never_imply_coverage() {
    let (records, mut checkpoints, writer, witness) = history();
    checkpoints.truncate(1);
    checkpoints[0].tree_size = 4;
    checkpoints[0].sign(&writer);
    let result = compare(&records, &checkpoints, &writer, None, None);
    assert_eq!(result.report.unwrap().class, Mismatch::CheckpointMismatch);
    assert_eq!(result.work.base_attempts, 1);
    let public = writer.verifying_key().to_bytes();
    let witness = witness.verifying_key().to_bytes();
    let policy = RecoveryPolicy {
        writer_public: &public,
        witness_public: Some(&witness),
        ..policy(None)
    };
    assert_eq!(
        run(&fixture(vec![lines(&records)], None), &policy)
            .err()
            .unwrap()
            .to_string(),
        "required independent audit anchor missing"
    );
    let first_install = RecoveryPolicy {
        allow_new: true,
        ..policy
    };
    let empty = run(&fixture(Vec::new(), None), &first_install).unwrap();
    assert!(empty.report.is_none());
    assert_eq!(empty.work.record_attempts, 0);
}

#[test]
fn final_b_source_cancel_and_resource_errors_cannot_publish_a_report() {
    let (records, checkpoints, writer, _) = history();
    let public = writer.verifying_key().to_bytes();
    let policy = RecoveryPolicy {
        writer_public: &public,
        ..policy(None)
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let mut source = fixture(vec![lines(&records)], Some(lines(&checkpoints)));
    source.cancel_on_final = Some(cancel.clone());
    source.cancel_at = 4;
    let control = Control::new(cancel.as_ref(), Duration::from_secs(30)).unwrap();
    assert_eq!(
        super::super::inspect(&source, &policy, &limits(), &control)
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::Interrupted
    );
    let source = fixture(vec![lines(&records)], Some(lines(&checkpoints)));
    let cancel = AtomicBool::new(false);
    let control = Control::new(&cancel, Duration::from_secs(30)).unwrap();
    let mut profile = limits();
    profile.canonical_bytes = 1;
    assert_eq!(
        super::super::inspect(&source, &policy, &profile, &control)
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::FileTooLarge
    );
}
