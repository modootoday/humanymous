//! Actual private-owner controls inside the allocated native fixture.
use super::*;
use crate::{keys::custody::Owner, verification::scan::limits::ManualClock};
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

fn snapshot(path: &Path) -> Vec<(String, super::super::Stamp, Vec<u8>)> {
    let mut names: Vec<_> = fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    let root = super::super::absolute(path).unwrap();
    let mut state = vec![(
        String::new(),
        super::super::observe(&root, super::super::Role::Directory).unwrap(),
        Vec::new(),
    )];
    for name in names {
        let file = super::super::relative(&root, &name, false, false).unwrap();
        let stamp = super::super::observe(&file, super::super::Role::Payload).unwrap();
        state.push((name.clone(), stamp, fs::read(path.join(name)).unwrap()));
    }
    state
}

pub(in super::super) fn exercise(path: &Path, head: &Checkpoint) {
    let key_path = path.with_extension("owner-keys");
    fs::DirBuilder::new().mode(0o700).create(&key_path).unwrap();
    let passphrase = b"native-owner-fixture-passphrase";
    let mut custody = Custody::acquire(&key_path, Owner::CurrentUser).unwrap();
    custody
        .load_or_create(passphrase, || {
            KeyMaterial::new([7; 32], b"key".to_vec(), None, None)
        })
        .unwrap();
    drop(custody);
    for fault in ["none", "cancel", "expiry"] {
        let limits = crate::verification::scan::tests::limits();
        let cancel = AtomicBool::new(false);
        let clock = ManualClock::new();
        let control = Control::with_clock(&cancel, Duration::from_secs(30), &clock).unwrap();
        let source = NativeSource::open(
            path,
            &limits,
            &control,
            super::super::Admission {
                max_entries: 16,
                max_handles: 64,
                other_handles: 16,
            },
        )
        .unwrap();
        let custody = Custody::acquire(&key_path, Owner::CurrentUser).unwrap();
        let owner = OpeningOwner::existing(
            custody,
            passphrase,
            source.into_files().unwrap(),
            Trust {
                node: "node".into(),
                witness: None,
                head: Some(head.clone()),
                allow_new: false,
            },
        )
        .unwrap();
        let before_wal = snapshot(path);
        let before_keys = snapshot(&key_path);
        let verified = owner.authenticate(&limits, &control).unwrap();
        let fresh_cancel = AtomicBool::new(false);
        let fresh = Control::new(&fresh_cancel, Duration::from_secs(30)).unwrap();
        match fault {
            "cancel" => cancel.store(true, Ordering::Release),
            "expiry" => clock.advance(Duration::from_secs(30)).unwrap(),
            _ => {}
        }
        assert!(fresh.check().is_ok());
        let result = verified.finalize();
        if fault == "none" {
            let readonly = result.unwrap();
            assert!(readonly.diagnostic.report.as_ref().unwrap().ok);
            assert_eq!(readonly.owner.material.hmac_key(), b"key");
            assert!(
                matches!(Custody::acquire(&key_path, Owner::CurrentUser), Err(error)
                if error.kind() == io::ErrorKind::WouldBlock)
            );
            assert!(
                matches!(crate::storage::Wal::open(path, Default::default()), Err(error)
                if error.kind() == io::ErrorKind::WouldBlock)
            );
            drop(readonly);
        } else {
            let error = result.err().unwrap();
            assert_eq!(
                error.kind(),
                if fault == "cancel" {
                    io::ErrorKind::Interrupted
                } else {
                    io::ErrorKind::TimedOut
                }
            );
        }
        assert_eq!(snapshot(path), before_wal);
        assert_eq!(snapshot(&key_path), before_keys);
        drop(crate::storage::Wal::open(path, Default::default()).unwrap());
        drop(Custody::acquire(&key_path, Owner::CurrentUser).unwrap());
    }
    fs::remove_dir_all(key_path).unwrap();
}
