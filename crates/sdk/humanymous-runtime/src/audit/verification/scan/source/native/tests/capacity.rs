//! Explicitly allocated observations; no universal RSS/allocator guarantee.
use super::super::*;
use crate::audit::{
    checkpoint::Checkpoint,
    merkle::frontier::Frontier,
    record::{Record, Signal, record_hash, record_mac},
    storage::{RecoveryPolicy, Wal, WalOptions, framing::preflight},
};
use ed25519_dalek::SigningKey;
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const NODE: &str = "capacity-node";
const KEY: &[u8] = b"capacity-fixture-hmac";
const FRAME: usize = 4 << 20;

struct Fixture {
    dir: PathBuf,
    head: Checkpoint,
    records: u64,
    checkpoints: usize,
    record_bytes: u64,
}
impl Fixture {
    fn write(
        label: &str,
        mut record: Record,
        count: u64,
        cadence: u64,
        rotate: u64,
    ) -> io::Result<Self> {
        if count == 0 || cadence == 0 || !count.is_multiple_of(cadence) {
            return Err(invalid("invalid bounded capacity fixture shape"));
        }
        let dir = directory(label)?;
        let mut wal = Wal::open(
            &dir,
            WalOptions {
                rotate_bytes: rotate,
                max_frame: FRAME,
            },
        )?;
        let writer = SigningKey::from_bytes(&[7; 32]);
        let mut frontier = Frontier::new();
        let mut head = String::new();
        let mut previous_signature = String::new();
        let mut latest = None;
        let mut checkpoints = 0usize;
        record.node_id = NODE.into();
        record.event_type = "capacity-fixture".into();
        // One reused record, one frontier and scalar/link state; never Log,
        // history-sized Vecs or a copied canonical/cryptographic implementation.
        for seq in 1..=count {
            record.seq = seq;
            record.prev_hash = std::mem::take(&mut head);
            record.record_hash = record_hash(&record, &record.prev_hash);
            record.hmac = record_mac(&record.record_hash, KEY).map_err(io::Error::other)?;
            wal.append_record(&record)?;
            frontier
                .append(record.record_hash.as_bytes())
                .map_err(|_| resource("capacity fixture frontier exhausted"))?;
            head = std::mem::take(&mut record.record_hash);
            if seq.is_multiple_of(cadence) {
                let mut checkpoint = Checkpoint {
                    node_id: NODE.into(),
                    tree_size: seq,
                    root: head.clone(),
                    merkle_root: hex::encode(frontier.root()),
                    prev_checkpoint_hash: std::mem::take(&mut previous_signature),
                    ..Checkpoint::default()
                };
                checkpoint.sign(&writer);
                wal.append_checkpoint(&checkpoint)?;
                previous_signature = checkpoint.sig.clone();
                latest = Some(checkpoint);
                checkpoints += 1;
            }
        }
        wal.flush()?;
        drop(wal);
        drop(record);
        let mut record_bytes = 0u64;
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name();
            if name.as_bytes().starts_with(b"audit-") && name.as_bytes().ends_with(b".log") {
                record_bytes = record_bytes
                    .checked_add(entry.metadata()?.len())
                    .ok_or_else(|| resource("capacity fixture bytes overflow"))?;
            }
        }
        Ok(Self {
            dir,
            head: latest.ok_or_else(|| invalid("capacity fixture has no head"))?,
            records: count,
            checkpoints,
            record_bytes,
        })
    }
    fn policy<'a>(&'a self, public: &'a [u8; 32]) -> RecoveryPolicy<'a> {
        RecoveryPolicy {
            node_id: NODE,
            hmac_key: KEY,
            writer_public: public,
            witness_public: None,
            trusted_head: Some(&self.head),
            allow_new: false,
        }
    }
    fn inspect(&self) {
        assert!(
            descriptor_count().unwrap() <= 128,
            "other-process reservation precondition not met"
        );
        let cancel = AtomicBool::new(false);
        let control = Control::new(&cancel, Duration::from_secs(300)).unwrap();
        let limits = profile();
        let public = SigningKey::from_bytes(&[7; 32]).verifying_key().to_bytes();
        let diagnostic = crate::audit::verification::scan::inspect_native(
            &self.dir,
            &self.policy(&public),
            &limits,
            &control,
            reservation(),
        )
        .unwrap();
        assert!(diagnostic.report.unwrap().ok);
        assert_eq!(
            diagnostic.work.record_successes,
            usize::try_from(self.records).unwrap()
        );
        assert_eq!(diagnostic.work.base_successes, self.checkpoints);
        eprintln!(
            "capacity records={} checkpoints={} record_bytes={}",
            self.records, self.checkpoints, self.record_bytes
        );
    }
}

fn directory(label: &str) -> io::Result<PathBuf> {
    let base = std::env::var_os("HUMANYMOUS_AUDIT_NATIVE_SOURCE_PARENT")
        .ok_or_else(|| invalid("capacity case requires allocated fixture parent"))?;
    let dir = PathBuf::from(base).join(format!(
        "audit-capacity-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos()
    ));
    fs::DirBuilder::new().mode(0o700).create(&dir)?;
    Ok(dir)
}

fn profile() -> Limits {
    Limits {
        frame: FRAME,
        json: preflight::Limits {
            strings: FRAME,
            array_items: 4096,
            values: 65536,
            depth: 64,
        },
        segments: 4096,
        records: 10_000_000,
        checkpoints: 10_000_000,
        record_bytes: 64 << 30,
        checkpoint_bytes: 4 << 30,
        node_bytes: 256,
        canonical_bytes: 16 << 20,
        signing_bytes: 16 << 10,
    }
}
fn reservation() -> Admission {
    Admission {
        max_entries: 4100,
        max_handles: 8192,
        other_handles: 128,
    }
}
fn descriptor_count() -> io::Result<usize> {
    fs::read_dir("/proc/self/fd")?.try_fold(0usize, |count, entry| {
        entry?;
        count
            .checked_add(1)
            .ok_or_else(|| resource("descriptor observation overflow"))
    })
}

#[test]
#[ignore = "requires explicit Linux capacity/RSS allocation and ext-family fixture parent"]
fn sparse_history_exceeds_working_set_target_without_history_collection() {
    let record = Record {
        triggered_rules: Some(vec!["r".repeat(16 << 10)]),
        ..Record::default()
    };
    let fixture = Fixture::write("sparse", record, 16_384, 1024, 4 << 20).unwrap();
    assert!(fixture.record_bytes > 128 << 20);
    fixture.inspect();
    fs::remove_dir_all(fixture.dir).unwrap();
}

#[test]
#[ignore = "requires explicit Linux capacity/RSS allocation and ext-family fixture parent"]
fn large_unicode_and_maximum_schema_arrays() {
    let record = Record {
        fail_reason: format!("a{}", "\u{301}".repeat(1_500_000)),
        triggered_rules: Some(
            (0..4096)
                .rev()
                .map(|index| format!("rule-{index:04}"))
                .collect(),
        ),
        top_signals: Some(
            (0..4096)
                .rev()
                .map(|index| Signal {
                    id: format!("signal-{index:04}"),
                    verdict: "neutral".into(),
                    conf: 0.5,
                })
                .collect(),
        ),
        ..Record::default()
    };
    let fixture = Fixture::write("unicode-arrays", record, 1, 1, 4 << 20).unwrap();
    assert!(fixture.record_bytes > 3_000_000);
    assert!(fixture.record_bytes <= FRAME as u64);
    fixture.inspect();
    fs::remove_dir_all(fixture.dir).unwrap();
}

#[test]
#[ignore = "requires explicit Linux malformed-link capacity/RSS allocation"]
fn large_raw_links_refuse_without_a_partial_diagnostic() {
    let dir = directory("raw-links").unwrap();
    let mut wal = Wal::open(
        &dir,
        WalOptions {
            rotate_bytes: 1,
            max_frame: FRAME,
        },
    )
    .unwrap();
    let raw_length = (FRAME - 4096) / 2;
    let mut frontier = Frontier::new();
    let mut previous_hash = String::new();
    let mut previous_signature = String::new();
    for seq in 1..=2 {
        let record = Record {
            seq,
            node_id: NODE.into(),
            event_type: "capacity-invalid".into(),
            prev_hash: std::mem::take(&mut previous_hash),
            record_hash: "h".repeat(raw_length),
            ..Record::default()
        };
        wal.append_record(&record).unwrap();
        frontier.append(record.record_hash.as_bytes()).unwrap();
        let checkpoint = Checkpoint {
            node_id: NODE.into(),
            tree_size: seq,
            root: record.record_hash.clone(),
            merkle_root: hex::encode(frontier.root()),
            prev_checkpoint_hash: std::mem::take(&mut previous_signature),
            sig: if seq == 1 {
                "s".repeat(raw_length)
            } else {
                "0".repeat(128)
            },
            ..Checkpoint::default()
        };
        wal.append_checkpoint(&checkpoint).unwrap();
        previous_hash = record.record_hash;
        previous_signature = checkpoint.sig;
    }
    wal.flush().unwrap();
    drop(wal);
    drop(previous_hash);
    drop(previous_signature);
    assert!(descriptor_count().unwrap() <= 128);
    let cancel = AtomicBool::new(false);
    let control = Control::new(&cancel, Duration::from_secs(300)).unwrap();
    let limits = profile();
    let public = SigningKey::from_bytes(&[7; 32]).verifying_key().to_bytes();
    let policy = RecoveryPolicy {
        node_id: NODE,
        hmac_key: KEY,
        writer_public: &public,
        witness_public: None,
        trusted_head: None,
        allow_new: false,
    };
    let error = crate::audit::verification::scan::inspect_native(
        &dir,
        &policy,
        &limits,
        &control,
        reservation(),
    )
    .err()
    .expect("signing budget must refuse rather than return a partial Report");
    assert_eq!(error.kind(), io::ErrorKind::FileTooLarge);
    eprintln!("capacity outcome=resource_refused raw_link_bytes={raw_length} diagnostic=false");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
#[ignore = "requires explicit Linux FD capacity allocation and ext-family fixture parent"]
fn retained_4096_segment_inventory_and_shared_reader_descriptors() {
    let fixture = Fixture::write("pins", Record::default(), 4096, 4096, 1).unwrap();
    let limits = profile();
    let cancel = AtomicBool::new(false);
    let control = Control::new(&cancel, Duration::from_secs(300)).unwrap();
    // Both observations include their own one directory-observer FD. The
    // subtraction cancels it; this is idle retained cardinality, not peak opens.
    let before = descriptor_count().unwrap();
    assert!(
        before <= 128,
        "other-process reservation precondition not met"
    );
    let source = NativeSource::open(&fixture.dir, &limits, &control, reservation()).unwrap();
    assert_eq!(source.segment_count().unwrap(), 4096);
    let retained = descriptor_count().unwrap();
    let retained_delta = retained.checked_sub(before).unwrap();
    assert_eq!(retained_delta, 4096 + 1 + 4);
    let first = source.segment(1).unwrap();
    let second = source.segment(1).unwrap();
    assert_eq!(descriptor_count().unwrap(), retained);
    let public = SigningKey::from_bytes(&[7; 32]).verifying_key().to_bytes();
    let diagnostic = crate::audit::verification::scan::inspect(
        &source,
        &fixture.policy(&public),
        &limits,
        &control,
    )
    .unwrap();
    assert!(diagnostic.report.unwrap().ok);
    assert_eq!(diagnostic.work.record_successes, 4096);
    drop(source);
    assert_eq!(descriptor_count().unwrap(), retained);
    drop(first);
    drop(second);
    assert_eq!(descriptor_count().unwrap(), before);
    eprintln!(
        "capacity idle_pins=4096 sidecar=1 retained_delta={} observer_included=true",
        retained_delta
    );
    fs::remove_dir_all(fixture.dir).unwrap();
}
