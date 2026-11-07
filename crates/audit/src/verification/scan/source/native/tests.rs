use super::*;
use crate::verification::scan::tests::limits;
use std::{sync::atomic::AtomicBool, time::Duration};

#[test]
fn descriptor_admission_counts_pins_temporaries_and_other_process_handles() {
    let policy = Admission { max_entries: 4100, max_handles: 4113, other_handles: 10 };
    assert!(policy.check(4096).is_ok());
    assert!(Admission { max_handles: 4112, ..policy }.check(4096).is_err());
    assert!(Admission { max_entries: 4101, max_handles: 4113, other_handles: 10 }
        .check(4096).is_err());
    assert!(Admission { max_entries: 4100, max_handles: usize::MAX - 1, other_handles: usize::MAX - 1 }
        .check(1).is_err());
}

#[test]
fn admission_and_cancellation_refuse_before_any_missing_directory_open() {
    let limits = limits();
    let cancel = AtomicBool::new(false);
    let control = Control::new(&cancel, Duration::from_secs(30)).unwrap();
    let missing = Path::new("/humanymous-missing-native-admission-probe");
    let result = NativeSource::open(missing, &limits, &control,
        Admission { max_entries: 16, max_handles: 1, other_handles: 0 });
    assert!(matches!(result, Err(error) if error.kind() == io::ErrorKind::FileTooLarge));
    cancel.store(true, Ordering::Release);
    let result = NativeSource::open(missing, &limits, &control,
        Admission { max_entries: 16, max_handles: 64, other_handles: 0 });
    assert!(matches!(result, Err(error) if error.kind() == io::ErrorKind::Interrupted));
}

#[test]
fn path_aliases_are_refused_before_native_open() {
    let limits = limits();
    let cancel = AtomicBool::new(false);
    let control = Control::new(&cancel, Duration::from_secs(30)).unwrap();
    for path in ["relative", "/humanymous/./missing", "/humanymous/../missing"] {
        let result = NativeSource::open(Path::new(path), &limits, &control,
            Admission { max_entries: 16, max_handles: 64, other_handles: 0 });
        assert!(matches!(result, Err(error) if error.kind() == io::ErrorKind::InvalidData));
    }
}

#[test]
#[ignore = "requires explicitly allocated Linux ext-family fixture parent"]
fn controlled_native_inspection_cursor_lifetime_and_replacement() {
    use crate::{log::{Config, Log}, record::Record, storage::{RecoveryPolicy, Wal, WalOptions}};
    use std::{fs, io::Write, os::unix::fs::{DirBuilderExt, OpenOptionsExt}, time::{SystemTime, UNIX_EPOCH}};
    let base = std::env::var_os("HUMANYMOUS_AUDIT_NATIVE_SOURCE_PARENT")
        .expect("controlled native source requires allocated fixture parent");
    let dir = PathBuf::from(base).join(format!("audit-native-{}-{}", std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
    let wal = Wal::open(&dir, WalOptions { rotate_bytes: 1, max_frame: 4096 }).unwrap();
    let mut log = Log::open(wal, Config {
        node_id: "node".into(), hmac_key: b"key".to_vec(), signing_seed: [7; 32],
        checkpoint_every: 1, witness: None,
    }, None, true).unwrap();
    for _ in 0..2 {
        log.append(Record { event_type: "native-source-fixture".into(), ..Record::default() }).unwrap();
    }
    let head = log.checkpoint().unwrap().unwrap();
    drop(log);
    let limits = limits();
    let cancel = AtomicBool::new(false);
    let control = Control::new(&cancel, Duration::from_secs(30)).unwrap();
    let admission = || Admission { max_entries: 16, max_handles: 64, other_handles: 16 };
    let source = NativeSource::open(&dir, &limits, &control, admission()).unwrap();
    let public = ed25519_dalek::SigningKey::from_bytes(&[7; 32]).verifying_key().to_bytes();
    let policy = RecoveryPolicy {
        node_id: "node", hmac_key: b"key", writer_public: &public,
        witness_public: None, trusted_head: Some(&head), allow_new: false,
    };
    let diagnostic = crate::verification::scan::inspect(&source, &policy, &limits, &control).unwrap();
    assert!(diagnostic.report.unwrap().ok);
    assert_eq!(diagnostic.work.record_successes, 2);
    assert_eq!(diagnostic.work.base_successes, 2);
    let raw = fs::read(dir.join(segment_name(1))).unwrap();
    let mut first = source.segment(1).unwrap();
    let mut second = source.segment(1).unwrap();
    let mut prefix = [0; 10];
    first.read_exact(&mut prefix).unwrap();
    let mut all = Vec::new();
    second.read_to_end(&mut all).unwrap();
    assert_eq!(all, raw);
    let mut rest = Vec::new();
    first.read_to_end(&mut rest).unwrap();
    assert_eq!([prefix.as_slice(), rest.as_slice()].concat(), raw);
    drop(source);
    assert!(matches!(Wal::open(&dir, WalOptions::default()), Err(error)
        if error.kind() == io::ErrorKind::WouldBlock));
    drop(first);
    assert!(matches!(Wal::open(&dir, WalOptions::default()), Err(error)
        if error.kind() == io::ErrorKind::WouldBlock));
    drop(second);
    drop(Wal::open(&dir, WalOptions::default()).unwrap());
    let source = NativeSource::open(&dir, &limits, &control, admission()).unwrap();
    let mut reader = source.segment(1).unwrap();
    fs::rename(dir.join(segment_name(1)), dir.join("replaced-original")).unwrap();
    fs::OpenOptions::new().write(true).create_new(true).mode(0o600)
        .open(dir.join(segment_name(1))).unwrap().write_all(&raw).unwrap();
    assert!(source.unchanged().is_err());
    assert!(reader.fill_buf().is_err());
    drop(reader);
    drop(source);
    fs::remove_file(dir.join(segment_name(1))).unwrap();
    fs::rename(dir.join("replaced-original"), dir.join(segment_name(1))).unwrap();

    let segment = dir.join(segment_name(1));
    for fault in ["truncate", "regrow", "append", "sidecar-remove", "sidecar-add", "cancel"] {
        let sidecar = dir.join(SIDECAR);
        let sidecar_bytes = fs::read(&sidecar).unwrap();
        // Deliberately separate the sampled mtime from future mutation, including
        // ext-family filesystems with coarse timestamp resolution. This tests
        // observed regrowth, not adversarial write/restore ABA detection.
        File::options().write(true).open(&segment).unwrap()
            .set_times(fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(1))).unwrap();
        if fault == "sidecar-add" { fs::remove_file(&sidecar).unwrap(); }
        let source = NativeSource::open(&dir, &limits, &control, admission()).unwrap();
        let mut reader = source.segment(1).unwrap();
        match fault {
            "truncate" => File::options().write(true).open(&segment).unwrap().set_len(1).unwrap(),
            "regrow" => {
                let mut file = File::options().write(true).open(&segment).unwrap();
                file.set_len(0).unwrap();
                file.write_all(&raw).unwrap();
            },
            "append" => File::options().append(true).open(&segment).unwrap().write_all(b"\n").unwrap(),
            "sidecar-remove" => fs::remove_file(&sidecar).unwrap(),
            "sidecar-add" => fs::OpenOptions::new().write(true).create_new(true).mode(0o600)
                .open(&sidecar).unwrap().write_all(&sidecar_bytes).unwrap(),
            "cancel" => cancel.store(true, Ordering::Release),
            _ => unreachable!(),
        }
        assert!(source.unchanged().is_err(), "fault {fault} must refuse");
        cancel.store(false, Ordering::Release);
        assert!(reader.fill_buf().is_err(), "fault {fault} must poison subsequent reads");
        drop(reader);
        drop(source);
        fs::write(&segment, &raw).unwrap();
        if !sidecar.exists() {
            fs::OpenOptions::new().write(true).create_new(true).mode(0o600)
                .open(&sidecar).unwrap().write_all(&sidecar_bytes).unwrap();
        }
    }

    let source = NativeSource::open(&dir, &limits, &control, admission()).unwrap();
    let moved = dir.with_file_name(format!("{}-moved", dir.file_name().unwrap().to_str().unwrap()));
    fs::rename(&dir, &moved).unwrap();
    fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
    assert!(source.unchanged().is_err(), "root replacement must refuse");
    drop(source);
    fs::remove_dir(&dir).unwrap();
    fs::rename(&moved, &dir).unwrap();

    for fault in ["wrong-mode", "symlink", "hardlink", "fifo", "gap", "unknown"] {
        use std::os::unix::fs::{PermissionsExt, symlink};
        match fault {
            "wrong-mode" => fs::set_permissions(&segment, fs::Permissions::from_mode(0o644)).unwrap(),
            "symlink" => {
                fs::remove_file(&segment).unwrap();
                symlink(dir.join(segment_name(2)), &segment).unwrap();
            },
            "hardlink" => {
                fs::remove_file(&segment).unwrap();
                fs::hard_link(dir.join(segment_name(2)), &segment).unwrap();
            },
            "fifo" => {
                fs::remove_file(&segment).unwrap();
                rustix::fs::mkfifoat(rustix::fs::CWD, &segment, Mode::RUSR | Mode::WUSR).unwrap();
            },
            "gap" => fs::rename(&segment, dir.join(segment_name(3))).unwrap(),
            "unknown" => fs::OpenOptions::new().write(true).create_new(true).mode(0o600)
                .open(dir.join("unapproved-member")).unwrap().write_all(b"fixture").unwrap(),
            _ => unreachable!(),
        }
        assert!(NativeSource::open(&dir, &limits, &control, admission()).is_err(),
            "constructor fault {fault} must refuse");
        match fault {
            "wrong-mode" => {
                assert_eq!(fs::metadata(&segment).unwrap().mode() & 0o7777, 0o644);
                fs::set_permissions(&segment, fs::Permissions::from_mode(0o600)).unwrap();
            },
            "symlink" | "hardlink" | "fifo" => {
                fs::remove_file(&segment).unwrap();
                fs::OpenOptions::new().write(true).create_new(true).mode(0o600)
                    .open(&segment).unwrap().write_all(&raw).unwrap();
            },
            "gap" => fs::rename(dir.join(segment_name(3)), &segment).unwrap(),
            "unknown" => fs::remove_file(dir.join("unapproved-member")).unwrap(),
            _ => unreachable!(),
        }
        // A failed constructor must release only its own guard and descriptors.
        drop(Wal::open(&dir, WalOptions::default()).unwrap());
    }

    final_epoch_refuses(&dir, &limits, &control, &policy);
    fs::remove_dir_all(dir).unwrap();
}

fn final_epoch_refuses(path: &Path, limits: &Limits, control: &Control<'_>,
    policy: &crate::storage::RecoveryPolicy<'_>)
{
    use std::{cell::Cell, io::Write};
    struct FinalMutation<'scan, 'cancel> {
        source: NativeSource<'scan, 'cancel>,
        calls: Cell<usize>,
        segment: PathBuf,
    }
    impl sealed::Sealed for FinalMutation<'_, '_> {}
    impl<'scan, 'cancel> Source for FinalMutation<'scan, 'cancel> {
        type Reader = NativeReader<'scan, 'cancel>;
        fn segment_count(&self) -> io::Result<usize> { self.source.segment_count() }
        fn segment(&self, index: usize) -> io::Result<Self::Reader> { self.source.segment(index) }
        fn checkpoints(&self) -> io::Result<Option<Self::Reader>> { self.source.checkpoints() }
        fn unchanged(&self) -> io::Result<()> {
            let call = self.calls.get() + 1;
            self.calls.set(call);
            // A-start/A-end/B-start/B-end: mutate only at final publication gate.
            if call == 4 {
                File::options().append(true).open(&self.segment)?.write_all(b"\n")?;
            }
            self.source.unchanged()
        }
    }
    let source = NativeSource::open(path, limits, control,
        Admission { max_entries: 16, max_handles: 64, other_handles: 16 }).unwrap();
    let fault = FinalMutation { source, calls: Cell::new(0), segment: path.join(segment_name(1)) };
    assert!(crate::verification::scan::inspect(&fault, policy, limits, control).is_err());
    assert_eq!(fault.calls.get(), 4, "mutation must occur after both complete passes");
}
