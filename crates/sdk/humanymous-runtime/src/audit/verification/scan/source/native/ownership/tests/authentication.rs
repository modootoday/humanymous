//! Negative controls use the same actual owner and native fixture, not tokens.
use super::*;
use crate::audit::verification::scan::source::native::{Admission, segment_name};

pub(super) fn refusals(path: &Path, key_path: &Path, passphrase: &[u8], head: &Checkpoint) {
    let segment = path.join(segment_name(1));
    let original = fs::read(&segment).unwrap();
    for fault in [
        "wrong-node",
        "witness-alias",
        "bad-hmac",
        "post-auth-change",
    ] {
        if fault == "bad-hmac" {
            let mut record: crate::audit::record::Record =
                serde_json::from_slice(&original).unwrap();
            assert!(record.hmac.is_ascii() && record.hmac.len() == 64);
            let first = if record.hmac.starts_with('0') {
                "1"
            } else {
                "0"
            };
            record.hmac.replace_range(..1, first);
            let mut bytes = serde_json::to_vec(&record).unwrap();
            bytes.push(b'\n');
            fs::write(&segment, bytes).unwrap();
        }
        let limits = crate::audit::verification::scan::tests::limits();
        let cancel = AtomicBool::new(false);
        let control = Control::new(&cancel, Duration::from_secs(30)).unwrap();
        let source = NativeSource::open(
            path,
            &limits,
            &control,
            Admission {
                max_entries: 16,
                max_handles: 64,
                other_handles: 16,
            },
        )
        .unwrap();
        let custody = Custody::acquire(key_path, Owner::CurrentUser).unwrap();
        let writer = ed25519_dalek::SigningKey::from_bytes(&[7; 32])
            .verifying_key()
            .to_bytes();
        let before_wal = snapshot(path);
        let before_keys = snapshot(key_path);
        let opening = OpeningOwner::existing(
            custody,
            passphrase,
            source.into_files().unwrap(),
            Trust {
                node: if fault == "wrong-node" {
                    "foreign-node"
                } else {
                    "node"
                }
                .into(),
                witness: (fault == "witness-alias").then_some(writer),
                head: Some(head.clone()),
            },
        );
        if fault == "witness-alias" {
            assert!(
                matches!(opening, Err(error) if error.kind() == io::ErrorKind::InvalidData
                && error.to_string() == "owned audit witness must be independent")
            );
        } else {
            let result = opening.unwrap().authenticate(&limits, &control);
            if fault == "post-auth-change" {
                let verified = result.unwrap();
                let mut changed = original.clone();
                changed.push(b'\n');
                fs::write(&segment, changed).unwrap();
                let changed_state = snapshot(path);
                assert!(matches!(verified.finalize(), Err(error)
                    if error.kind() == io::ErrorKind::InvalidData));
                // No truncation or other automatic repair of this changed file.
                assert_eq!(snapshot(path), changed_state);
            } else {
                let error = result.err().unwrap();
                assert_eq!(error.kind(), io::ErrorKind::InvalidData);
                if fault == "bad-hmac" {
                    // This exact branch follows a completed diagnostic scan,
                    // not a privacy/path/framing failure masquerading as crypto.
                    assert_eq!(error.to_string(), "owned audit authentication failed");
                }
            }
        }
        if fault != "post-auth-change" {
            assert_eq!(snapshot(path), before_wal);
        }
        assert_eq!(snapshot(key_path), before_keys);
        drop(crate::audit::storage::Wal::open(path, Default::default()).unwrap());
        drop(Custody::acquire(key_path, Owner::CurrentUser).unwrap());
        if matches!(fault, "bad-hmac" | "post-auth-change") {
            fs::write(&segment, &original).unwrap();
        }
    }
    missing_identity_refuses(path, passphrase, head);
    empty_history_is_not_a_cryptographic_report(path, key_path, passphrase);
}

fn missing_identity_refuses(path: &Path, passphrase: &[u8], head: &Checkpoint) {
    let key_path = path.with_extension("owner-missing-keys");
    fs::DirBuilder::new().mode(0o700).create(&key_path).unwrap();
    let custody = Custody::acquire(&key_path, Owner::CurrentUser).unwrap();
    let limits = crate::audit::verification::scan::tests::limits();
    let cancel = AtomicBool::new(false);
    let control = Control::new(&cancel, Duration::from_secs(30)).unwrap();
    let source = NativeSource::open(
        path,
        &limits,
        &control,
        Admission {
            max_entries: 16,
            max_handles: 64,
            other_handles: 16,
        },
    )
    .unwrap();
    let before_wal = snapshot(path);
    let before_keys = snapshot(&key_path);
    let result = OpeningOwner::existing(
        custody,
        passphrase,
        source.into_files().unwrap(),
        Trust {
            node: "node".into(),
            witness: None,
            head: Some(head.clone()),
        },
    );
    assert!(matches!(result, Err(error) if error.kind() == io::ErrorKind::InvalidData));
    assert_eq!(snapshot(path), before_wal);
    assert_eq!(snapshot(&key_path), before_keys);
    assert!(!key_path.join("keys.sealed").exists());
    drop(crate::audit::storage::Wal::open(path, Default::default()).unwrap());
    drop(Custody::acquire(&key_path, Owner::CurrentUser).unwrap());
    fs::remove_dir_all(key_path).unwrap();
}

fn empty_history_is_not_a_cryptographic_report(path: &Path, key_path: &Path, passphrase: &[u8]) {
    let empty = path.with_extension("owner-empty-history");
    fs::DirBuilder::new().mode(0o700).create(&empty).unwrap();
    drop(crate::audit::storage::Wal::open(&empty, Default::default()).unwrap());
    let limits = crate::audit::verification::scan::tests::limits();
    let cancel = AtomicBool::new(false);
    let control = Control::new(&cancel, Duration::from_secs(30)).unwrap();
    let source = NativeSource::open(
        &empty,
        &limits,
        &control,
        Admission {
            max_entries: 16,
            max_handles: 64,
            other_handles: 16,
        },
    )
    .unwrap();
    let writer = ed25519_dalek::SigningKey::from_bytes(&[7; 32])
        .verifying_key()
        .to_bytes();
    let policy = RecoveryPolicy {
        node_id: "node",
        hmac_key: b"key",
        writer_public: &writer,
        witness_public: None,
        trusted_head: None,
        allow_new: true,
    };
    let diagnostic =
        crate::audit::verification::scan::inspect(&source, &policy, &limits, &control).unwrap();
    assert!(diagnostic.report.is_none());
    let custody = Custody::acquire(key_path, Owner::CurrentUser).unwrap();
    let before_wal = snapshot(&empty);
    let before_keys = snapshot(key_path);
    let owner = OpeningOwner::existing(
        custody,
        passphrase,
        source.into_files().unwrap(),
        Trust {
            node: "node".into(),
            witness: None,
            head: None,
        },
    )
    .unwrap();
    let error = owner.authenticate(&limits, &control).err().unwrap();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(error.to_string(), "unexpected empty audit history");
    assert_eq!(snapshot(&empty), before_wal);
    assert_eq!(snapshot(key_path), before_keys);
    drop(crate::audit::storage::Wal::open(&empty, Default::default()).unwrap());
    drop(Custody::acquire(key_path, Owner::CurrentUser).unwrap());
    fs::remove_dir_all(empty).unwrap();
}
