use super::{
    frames::{Budget, Frames},
    limits::{Control, Limits},
    metadata::{self, Torn},
    source::{self, Source},
};
use crate::{
    checkpoint::Checkpoint,
    record::Record,
    storage::{RecoveryPolicy, framing::preflight},
};
use std::{
    cell::Cell,
    io::{self, Cursor},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
mod crypto;

struct Fixture {
    segments: Vec<Vec<u8>>,
    sidecar: Option<Vec<u8>>,
    checks: Cell<usize>,
    opens: Cell<usize>,
    changed_on: Option<usize>,
    change_membership: bool,
    cancel_on_final: Option<Arc<AtomicBool>>,
    cancel_at: usize,
    second_pass_segments: Option<Vec<Vec<u8>>>,
    second_pass_sidecar: Option<Option<Vec<u8>>>,
}
impl source::sealed::Sealed for Fixture {}
impl Source for Fixture {
    type Reader = Cursor<Vec<u8>>;
    fn segment_count(&self) -> io::Result<usize> {
        let segments = if self.checks.get() >= 3 {
            self.second_pass_segments.as_ref().unwrap_or(&self.segments)
        } else {
            &self.segments
        };
        Ok(segments.len() + usize::from(self.change_membership && self.checks.get() >= 2))
    }
    fn segment(&self, index: usize) -> io::Result<Self::Reader> {
        self.opens.set(self.opens.get() + 1);
        let segments = if self.checks.get() >= 3 {
            self.second_pass_segments.as_ref().unwrap_or(&self.segments)
        } else {
            &self.segments
        };
        Ok(Cursor::new(segments[index - 1].clone()))
    }
    fn checkpoints(&self) -> io::Result<Option<Self::Reader>> {
        let sidecar = if self.checks.get() >= 3 {
            self.second_pass_sidecar
                .clone()
                .unwrap_or_else(|| self.sidecar.clone())
        } else {
            self.sidecar.clone()
        };
        Ok(sidecar.map(Cursor::new))
    }
    fn unchanged(&self) -> io::Result<()> {
        let next = self.checks.get() + 1;
        self.checks.set(next);
        if next == self.cancel_at
            && let Some(cancel) = &self.cancel_on_final
        {
            cancel.store(true, Ordering::Release);
        }
        if self.changed_on == Some(next) {
            return Err(io::Error::other("injected source change"));
        }
        Ok(())
    }
}
fn fixture(segments: Vec<Vec<u8>>, sidecar: Option<Vec<u8>>) -> Fixture {
    Fixture {
        segments,
        sidecar,
        checks: Cell::new(0),
        opens: Cell::new(0),
        changed_on: None,
        change_membership: false,
        cancel_on_final: None,
        cancel_at: 2,
        second_pass_segments: None,
        second_pass_sidecar: None,
    }
}
pub(super) fn limits() -> Limits {
    Limits {
        frame: 4096,
        json: preflight::Limits {
            strings: 4096,
            array_items: 4096,
            values: 65536,
            depth: 64,
        },
        segments: 8,
        records: 100,
        checkpoints: 100,
        record_bytes: 65536,
        checkpoint_bytes: 65536,
        node_bytes: 256,
        canonical_bytes: 16 << 20,
        signing_bytes: 16384,
    }
}
fn policy(head: Option<&Checkpoint>) -> RecoveryPolicy<'_> {
    RecoveryPolicy {
        node_id: "node",
        hmac_key: b"key",
        writer_public: &[7; 32],
        witness_public: None,
        trusted_head: head,
        allow_new: false,
    }
}
fn inspect(
    source: &Fixture,
    policy: &RecoveryPolicy<'_>,
    limits: &Limits,
) -> io::Result<metadata::Metadata> {
    let cancel = AtomicBool::new(false);
    let control = Control::new(&cancel, Duration::from_secs(30))?;
    metadata::inspect(source, policy, limits, &control)
}
fn record(node: &str) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&Record {
        node_id: node.into(),
        seq: 99,
        ..Default::default()
    })
    .unwrap();
    bytes.push(b'\n');
    bytes
}
fn checkpoint() -> Checkpoint {
    Checkpoint {
        node_id: "node".into(),
        tree_size: 1,
        merkle_root: "0".repeat(64),
        ..Default::default()
    }
}
fn sidecar(checkpoint: &Checkpoint) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(checkpoint).unwrap();
    bytes.push(b'\n');
    bytes
}

#[test]
fn metadata_is_not_crypto_and_raw_commitments_cover_framing_and_torn_tail() {
    let bytes = record("node");
    let mut torn = bytes.clone();
    torn.extend_from_slice(b"partial");
    let source = fixture(vec![torn.clone()], None);
    let metadata = inspect(&source, &policy(None), &limits()).unwrap();
    assert_eq!(metadata.segments, 1);
    assert_eq!(metadata.records, 1);
    assert_eq!(metadata.checkpoints, 0);
    assert_eq!(metadata.record_bytes, torn.len() as u64);
    assert_eq!(metadata.checkpoint_bytes, 0);
    assert!(!metadata.has_sidecar);
    assert!(metadata.physical_latest.is_none());
    assert_eq!(
        metadata.torn,
        Some(Torn {
            segment: 1,
            complete: bytes.len() as u64,
            bytes: 7
        })
    );
    let plain = inspect(
        &fixture(vec![bytes.clone()], None),
        &policy(None),
        &limits(),
    )
    .unwrap();
    assert_ne!(metadata.record_digest, plain.record_digest);
    let mut crlf = bytes[..bytes.len() - 1].to_vec();
    crlf.extend_from_slice(b"\r\n");
    let crlf = inspect(
        &fixture(vec![crlf], Some(Vec::new())),
        &policy(None),
        &limits(),
    )
    .unwrap();
    assert_ne!(plain.record_digest, crlf.record_digest);
    assert_ne!(plain.checkpoint_digest, crlf.checkpoint_digest);
    // seq99 and empty hashes/signatures deliberately pass metadata only.
    assert_eq!(source.checks.get(), 2);
}

#[test]
fn segments_never_join_torn_bytes_and_sidecar_never_accepts_incomplete_tail() {
    let first = br#"{"node_id":"#.to_vec();
    let second = b"\"node\"}\n".to_vec();
    let joined = [first.clone(), second.clone()].concat();
    assert!(serde_json::from_slice::<Record>(&joined).is_ok());
    assert!(
        inspect(
            &fixture(vec![first, second], None),
            &policy(None),
            &limits()
        )
        .is_err()
    );
    let mut cp = sidecar(&checkpoint());
    cp.pop();
    assert!(
        inspect(
            &fixture(vec![record("node")], Some(cp)),
            &policy(None),
            &limits()
        )
        .is_err()
    );
}

#[test]
fn complete_metadata_checks_node_progression_and_exact_physical_pin() {
    let cp = checkpoint();
    let source = fixture(vec![record("node")], Some(sidecar(&cp)));
    let metadata = inspect(&source, &policy(Some(&cp)), &limits()).unwrap();
    assert_eq!(metadata.physical_latest, Some(cp.clone()));
    let mut foreign = cp.clone();
    foreign.witness_sig = "other".into();
    assert_eq!(
        inspect(&source, &policy(Some(&foreign)), &limits())
            .err()
            .unwrap()
            .to_string(),
        "trusted audit head missing: rollback or fork"
    );
    let duplicate = [sidecar(&cp), sidecar(&cp)].concat();
    assert_eq!(
        inspect(
            &fixture(vec![record("node")], Some(duplicate.clone())),
            &policy(None),
            &limits()
        )
        .err()
        .unwrap()
        .to_string(),
        "invalid audit checkpoint progression or Merkle commitment"
    );
    let later_node = [record("node"), record("other")].concat();
    assert_eq!(
        inspect(
            &fixture(vec![later_node], Some(duplicate)),
            &policy(None),
            &limits()
        )
        .err()
        .unwrap()
        .to_string(),
        "audit history belongs to another node"
    );
}

#[test]
fn resource_limits_precede_owned_decode_and_poison_retries_or_partial_finish() {
    let bytes = [record("node"), b"not-json\n".to_vec()].concat();
    let cancel = AtomicBool::new(false);
    let control = Control::new(&cancel, Duration::from_secs(30)).unwrap();
    let limits = limits();
    let mut frames = Frames::new(Cursor::new(bytes), limits.frame, true).unwrap();
    let mut budget = Budget::new(65536, 1);
    assert!(
        frames
            .next::<Record>(&mut budget, &limits, &control)
            .unwrap()
            .is_some()
    );
    assert_eq!(
        frames
            .next::<Record>(&mut budget, &limits, &control)
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::FileTooLarge
    );
    assert!(
        frames
            .next::<Record>(&mut budget, &limits, &control)
            .is_err()
    );
    assert!(frames.finish().is_err());
    let bytes = record("node");
    let mut budget = Budget::new(bytes.len() as u64 - 1, 100);
    let mut frames = Frames::new(Cursor::new(bytes), limits.frame, true).unwrap();
    assert_eq!(
        frames
            .next::<Record>(&mut budget, &limits, &control)
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::FileTooLarge
    );
    let mut unfinished = Frames::new(Cursor::new(record("node")), limits.frame, true).unwrap();
    assert!(
        unfinished
            .next::<Record>(&mut Budget::new(65536, 100), &limits, &control)
            .unwrap()
            .is_some()
    );
    assert!(unfinished.finish().is_err());
    let mut oversized = Frames::new(Cursor::new(b"{}\n"), 2, true).unwrap();
    assert_eq!(
        oversized
            .next::<Record>(&mut Budget::new(65536, 100), &limits, &control)
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::FileTooLarge
    );
}

#[test]
fn cancellation_and_invalid_profiles_refuse_before_source_access() {
    let source = fixture(vec![record("node")], None);
    let cancel = AtomicBool::new(true);
    let control = Control::new(&cancel, Duration::from_secs(30)).unwrap();
    assert_eq!(
        metadata::inspect(&source, &policy(None), &limits(), &control)
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::Interrupted
    );
    assert_eq!(source.opens.get(), 0);
    assert_eq!(source.checks.get(), 0);
    cancel.store(false, Ordering::Release);
    let mut profile = limits();
    profile.segments = 0;
    assert!(metadata::inspect(&source, &policy(None), &profile, &control).is_err());
    profile = limits();
    profile.json.array_items = usize::MAX;
    assert!(metadata::inspect(&source, &policy(None), &profile, &control).is_err());
    assert_eq!(source.opens.get(), 0);
    assert!(Control::new(&cancel, Duration::ZERO).is_err());
}

#[test]
fn changed_source_or_membership_cannot_publish_metadata_and_native_is_unsupported() {
    let mut source = fixture(vec![record("node")], None);
    source.changed_on = Some(2);
    assert!(inspect(&source, &policy(None), &limits()).is_err());
    let mut source = fixture(vec![record("node")], None);
    source.change_membership = true;
    assert_eq!(
        inspect(&source, &policy(None), &limits())
            .err()
            .unwrap()
            .to_string(),
        "audit source membership changed"
    );
    assert_eq!(
        source::open_native(Path::new("unprovisioned-audit-source"))
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::Unsupported
    );
    let cancel = Arc::new(AtomicBool::new(false));
    let mut source = fixture(vec![record("node")], None);
    source.cancel_on_final = Some(cancel.clone());
    let control = Control::new(cancel.as_ref(), Duration::from_secs(30)).unwrap();
    assert_eq!(
        metadata::inspect(&source, &policy(None), &limits(), &control)
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::Interrupted
    );
}
