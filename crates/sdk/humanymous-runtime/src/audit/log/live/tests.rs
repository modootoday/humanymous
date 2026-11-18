use super::*;
use crate::audit::{
    log::Config,
    record::events::{EVENT_ENF_ALLOW, EVENT_INSTANCE_STARTUP},
    storage::{Wal, WalOptions},
    verification::Mismatch,
};
use ed25519_dalek::SigningKey;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
fn dir() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "humanymous-live-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&p).unwrap();
    p
}
fn clean(dir: &Path) {
    for e in fs::read_dir(dir).unwrap() {
        fs::remove_file(e.unwrap().path()).unwrap();
    }
    fs::remove_dir(dir).unwrap();
}
/// Go `walLog`: node n1, HMAC key "k", a checkpoint every four records and a
/// fixed persisted signing seed.
fn config() -> Config {
    Config {
        node_id: "n1".into(),
        hmac_key: b"k".to_vec(),
        signing_seed: [0; 32],
        checkpoint_every: 4,
        witness: None,
    }
}
fn witnessed() -> Config {
    let mut config = config();
    let writer = SigningKey::from_bytes(&config.signing_seed)
        .verifying_key()
        .to_bytes();
    config.witness = Some(Witness::from_seed(&[9; 32], writer, "n1".into()));
    config
}
fn open(dir: &Path, config: Config, allow_new: bool) -> Log {
    Log::open(
        Wal::open(dir, WalOptions::default()).unwrap(),
        config,
        None,
        allow_new,
    )
    .unwrap()
}
/// Go `sampleRecord(i)` shape: a distinct enforcement record per index.
fn sample(i: usize) -> Record {
    Record {
        event_type: EVENT_ENF_ALLOW.into(),
        request_id: format!("req-{i}"),
        risk_score: i64::try_from(i).unwrap(),
        ..Default::default()
    }
}

/// Go TestWALRingBoundsMemory (internal/audit/wal_test.go at 981d25c): with
/// RingCap 8 and 40 appends the in-memory window stays within twice the cap
/// while the full chain stays on disk, and Recent(5) returns five records.
#[test]
fn ring_window_is_bounded_while_the_wal_keeps_the_full_chain() {
    let dir = dir();
    let mut log = open(&dir, config(), true);
    log.set_ring_capacity(8);
    for i in 0..40 {
        log.append(sample(i)).unwrap();
    }
    assert!(
        log.ring.len() <= 2 * 8,
        "window not bounded: {}",
        log.ring.len()
    );
    assert_eq!(log.len(), 40);
    assert_eq!(log.records().unwrap().len(), 40);
    let recent = log.recent(5);
    assert_eq!(recent.len(), 5);
    assert_eq!(
        recent.iter().map(|r| r.seq).collect::<Vec<_>>(),
        [36, 37, 38, 39, 40]
    );
    // Go Recent: n <= 0 or n beyond the window returns the whole window.
    assert_eq!(log.recent(0), log.recent(usize::MAX));
    assert_eq!(log.recent(0).len(), log.ring.len());
    assert_eq!(log.recent(0).last().unwrap().seq, 40);
    drop(log);
    clean(&dir);
}

/// Go replayWAL refilled the window with the newest RingCap records, and Len
/// resumed from the replayed sequence (TestWALReplayAcrossRestart).
#[test]
fn restart_refills_the_window_from_the_authenticated_wal() {
    let dir = dir();
    let mut log = open(&dir, config(), true);
    assert!(log.is_empty());
    assert!(log.recent(3).is_empty());
    for i in 0..6 {
        log.append(sample(i)).unwrap();
    }
    log.flush().unwrap();
    drop(log);
    let mut log = open(&dir, config(), false);
    assert_eq!(log.len(), 6);
    assert!(!log.is_empty());
    assert_eq!(log.recent(0), log.records().unwrap());
    log.set_ring_capacity(4);
    assert_eq!(
        log.recent(0).iter().map(|r| r.seq).collect::<Vec<_>>(),
        [3, 4, 5, 6]
    );
    for i in 6..12 {
        log.append(sample(i)).unwrap();
    }
    assert_eq!(log.len(), 12);
    let window = log.recent(0);
    assert!(window.len() <= 8);
    assert_eq!(window.last().unwrap().seq, 12);
    assert_eq!(window.last(), log.records().unwrap().last());
    log.set_ring_capacity(0);
    assert_eq!(log.ring_capacity, DEFAULT_RING_CAPACITY);
    assert_eq!(DEFAULT_RING_CAPACITY, 4096);
    assert_eq!(log.self_verify().class, Mismatch::Ok);
    drop(log);
    clean(&dir);
}

/// Go SelfVerify on a live log (TestWALRoundTripAndVerify and
/// TestVerifyEmptyChainFails): an untouched chain passes with the HMAC layer
/// checked, an empty log is never green, and a checkpoint that no longer
/// matches the records is reported, not hidden.
#[test]
fn self_verify_replays_the_live_chain_and_never_passes_empty() {
    let dir = dir();
    let mut log = open(&dir, config(), true);
    let empty = log.self_verify();
    assert!(!empty.ok);
    assert_eq!(empty.class, Mismatch::EmptyChain);
    for i in 0..10 {
        log.append(sample(i)).unwrap();
    }
    let report = log.self_verify();
    assert!(report.ok, "{report:?}");
    assert_eq!(report.class, Mismatch::Ok);
    assert_eq!(report.scope.records, 10);
    assert!(report.scope.hmac_checked);
    assert_eq!(report.scope.checkpoints, 2);
    log.checkpoints[0].root = "0".repeat(64);
    let tampered = log.self_verify();
    assert!(!tampered.ok);
    assert_eq!(tampered.class, Mismatch::CheckpointMismatch);
    assert_eq!(tampered.at_seq, 4);
    drop(log);
    clean(&dir);
}

/// Go TestSelfVerifyRequiresWitness (verifier_p0_test.go): with a witness
/// configured, stripping the co-signatures fails SelfVerify as
/// witness-invalid even though the writer heads remain valid.
#[test]
fn self_verify_requires_witness_cosignatures_when_configured() {
    let dir = dir();
    let mut log = open(&dir, witnessed(), true);
    for i in 0..8 {
        log.append(sample(i)).unwrap();
    }
    log.checkpoint().unwrap();
    let report = log.self_verify();
    assert!(report.ok, "{report:?}");
    assert_eq!(report.scope.witnessed_checkpoints, report.scope.checkpoints);
    assert!(report.scope.checkpoints > 0);
    for checkpoint in &mut log.checkpoints {
        checkpoint.witness_sig.clear();
    }
    let stripped = log.self_verify();
    assert!(!stripped.ok);
    assert_eq!(stripped.class, Mismatch::WitnessInvalid);
    assert_eq!(stripped.detail, "witness co-signature missing or invalid");
    assert_eq!(stripped.at_seq, log.checkpoints[0].tree_size);
    drop(log);
    clean(&dir);
}

/// Go `Sink.Emit` is `EmitAndAct(r, nil)`: it seals an informational record
/// with no side effect and refuses an empty event type without advancing.
#[test]
fn emit_seals_without_an_action_and_refuses_an_untyped_record() {
    let dir = dir();
    let mut log = open(&dir, config(), true);
    assert!(log.emit(Record::default()).is_err());
    assert!(log.is_empty());
    let sealed = log
        .emit(Record {
            event_type: EVENT_INSTANCE_STARTUP.into(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(sealed.seq, 1);
    assert_eq!(sealed.event_type, "instance.startup");
    assert!(!sealed.record_hash.is_empty() && !sealed.hmac.is_empty());
    assert_eq!(log.records().unwrap(), std::slice::from_ref(&sealed));
    assert_eq!(log.recent(1), [sealed]);
    drop(log);
    clean(&dir);
}
