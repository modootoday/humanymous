use super::*;
use crate::{
    checkpoint::Checkpoint,
    merkle::Tree,
    record::{Record, record_hash, record_mac},
    verification::{checks, verify, verify_with_witness},
};
use ed25519_dalek::{Signer, SigningKey};
use serde::Deserialize;

fn drive(
    records: &[Record],
    checkpoints: &[Checkpoint],
    key: &[u8],
    public: &[u8; 32],
    witness: Option<&[u8; 32]>,
) -> io::Result<(Report, Work)> {
    // Test-only history oracle intentionally schedules checkpoint/witness work
    // early. This is an accumulator differential, not an operational scanner.
    let leaves = records
        .iter()
        .map(|record| record.record_hash.as_bytes().to_vec())
        .collect::<Vec<_>>();
    let mut tree = Tree::new(&leaves);
    let mut accumulator = Accumulator::new(!key.is_empty());
    let mut previous = "";
    let mut signature = String::new();
    let mut checkpoint_index = 0;
    let mut check = |checkpoint: &Checkpoint, accumulator: &mut Accumulator| -> io::Result<()> {
        let size = usize::try_from(checkpoint.tree_size).ok();
        let linear = size.and_then(|size| {
            if size == 0 {
                Some("")
            } else {
                records
                    .get(size - 1)
                    .map(|record| record.record_hash.as_str())
            }
        });
        accumulator.checkpoint(checks::checkpoint(
            checkpoint,
            &signature,
            linear,
            || size.and_then(|size| tree.root_at(size)),
            Some(public),
            16384,
        )?)?;
        if let Some(witness) = witness {
            accumulator.witness(
                checkpoint.tree_size,
                checkpoint.verify_signature_bounded(witness, &checkpoint.witness_sig, 16384)?,
            )?;
        }
        signature = checkpoint.sig.clone();
        Ok(())
    };
    for (index, record) in records.iter().enumerate() {
        accumulator.record(checks::record(
            record,
            index as u64 + 1,
            previous,
            key,
            16 << 20,
            4096,
        )?)?;
        previous = &record.record_hash;
        if let Some(checkpoint) = checkpoints.get(checkpoint_index) {
            check(checkpoint, &mut accumulator)?;
            checkpoint_index += 1;
        }
    }
    for checkpoint in &checkpoints[checkpoint_index..] {
        check(checkpoint, &mut accumulator)?;
    }
    assert_eq!(accumulator.actual_work().record_attempts, records.len());
    accumulator.finish()
}

fn fixture() -> (Vec<Record>, Vec<Checkpoint>, SigningKey, SigningKey) {
    let writer = SigningKey::from_bytes(&[7; 32]);
    let witness = SigningKey::from_bytes(&[8; 32]);
    let mut records = Vec::new();
    let mut checkpoints = Vec::<Checkpoint>::new();
    for seq in 1..=3 {
        let mut record = Record {
            seq,
            node_id: "node".into(),
            prev_hash: records
                .last()
                .map_or_else(String::new, |record: &Record| record.record_hash.clone()),
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
            tree_size: seq,
            node_id: "node".into(),
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

fn compare(
    records: &[Record],
    checkpoints: &[Checkpoint],
    writer: &SigningKey,
    witness: &SigningKey,
) -> (Report, Work) {
    let public = writer.verifying_key().to_bytes();
    let witness = witness.verifying_key().to_bytes();
    let expected = verify_with_witness(records, checkpoints, b"key", Some(&public), &witness);
    let (actual, work) = drive(records, checkpoints, b"key", &public, Some(&witness)).unwrap();
    assert_eq!(actual, expected);
    (actual, work)
}

#[test]
fn later_record_error_overrides_early_witness_failure_and_checkpoint_work() {
    let (mut records, mut checkpoints, writer, witness) = fixture();
    checkpoints[0].witness_sig.clear();
    records[2].hmac = "bad".into();
    let (report, work) = compare(&records, &checkpoints, &writer, &witness);
    assert_eq!(report.class, Mismatch::HmacInvalid);
    assert_eq!(report.scope.records, 2);
    assert_eq!(report.scope.checkpoints, 0);
    assert_eq!(report.scope.witnessed_checkpoints, 0);
    assert_eq!(work.base_successes, 3);
    assert_eq!(work.witness_attempts, 3);
}

#[test]
fn later_base_error_overrides_early_witness_failure_with_base_prefix_scope() {
    let (records, mut checkpoints, writer, witness) = fixture();
    checkpoints[0].witness_sig.clear();
    checkpoints[2].root = "different".into();
    checkpoints[2].sign(&writer);
    checkpoints[2].witness_sig =
        hex::encode(witness.sign(&checkpoints[2].signing_bytes()).to_bytes());
    let (report, work) = compare(&records, &checkpoints, &writer, &witness);
    assert_eq!(report.class, Mismatch::CheckpointMismatch);
    assert_eq!(report.scope.records, 3);
    assert_eq!(report.scope.checkpoints, 2);
    assert_eq!(report.scope.witnessed_checkpoints, 0);
    assert_eq!(work.witness_successes, 2);
}

#[test]
fn witness_failure_reports_all_base_successes_but_only_prior_witness_successes() {
    let (records, mut checkpoints, writer, witness) = fixture();
    checkpoints[1].witness_sig.clear();
    let (report, work) = compare(&records, &checkpoints, &writer, &witness);
    assert_eq!(report.class, Mismatch::WitnessInvalid);
    assert_eq!(report.scope.checkpoints, 3);
    assert_eq!(report.scope.witnessed_checkpoints, 1);
    assert_eq!(work.witness_successes, 2);
    assert_eq!(work.base_successes, 3);
}

#[test]
fn first_record_failure_preserves_prefix_scope_while_later_checks_continue() {
    let (mut records, checkpoints, writer, witness) = fixture();
    records[1].seq = 5;
    records[1].prev_hash = "wrong".into();
    records[1].hmac.clear();
    let (report, work) = compare(&records, &checkpoints, &writer, &witness);
    assert_eq!(report.class, Mismatch::SequenceGap);
    assert_eq!(report.at_seq, 5);
    assert_eq!(report.scope.records, 1);
    assert_eq!(work.record_successes, 2);
}

#[derive(Deserialize)]
struct Case {
    records: Vec<Record>,
    checkpoints: Vec<Checkpoint>,
    key: String,
    public: String,
    witness: String,
    expected: serde_json::Value,
}

#[test]
fn frozen_go_reports_and_sequential_scope_match_interleaved_accumulation() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../../../tests/fixtures/verification.json")).unwrap();
    assert_eq!(cases.len(), 840);
    for case in cases {
        let key = hex::decode(case.key).unwrap();
        let public: [u8; 32] = hex::decode(case.public).unwrap().try_into().unwrap();
        let witness: Option<[u8; 32]> = (!case.witness.is_empty())
            .then(|| hex::decode(case.witness).unwrap().try_into().unwrap());
        let expected = match witness.as_ref() {
            Some(witness) => verify_with_witness(
                &case.records,
                &case.checkpoints,
                &key,
                Some(&public),
                witness,
            ),
            None => verify(&case.records, &case.checkpoints, &key, Some(&public)),
        };
        let (actual, _) = drive(
            &case.records,
            &case.checkpoints,
            &key,
            &public,
            witness.as_ref(),
        )
        .unwrap();
        assert_eq!(serde_json::to_value(&actual).unwrap(), case.expected);
        assert_eq!(actual, expected);
    }
}

#[test]
fn resource_errors_do_not_become_mismatch_or_success_reports() {
    let (records, checkpoints, writer, _) = fixture();
    assert_eq!(
        checks::record(&records[0], 1, "", b"key", 0, 4096)
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::FileTooLarge
    );
    assert_eq!(
        checkpoints[0]
            .verify_signature_bounded(&writer.verifying_key().to_bytes(), &checkpoints[0].sig, 0)
            .unwrap_err()
            .kind(),
        io::ErrorKind::FileTooLarge
    );
    let mut accumulator = Accumulator::new(true);
    accumulator.work.record_attempts = usize::MAX;
    assert_eq!(
        accumulator.record(None).unwrap_err().kind(),
        io::ErrorKind::FileTooLarge
    );
    assert!(accumulator.checkpoint(None).is_err());
    assert!(accumulator.finish().is_err());
}
