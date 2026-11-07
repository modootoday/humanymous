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
    fs::remove_dir_all(dir).unwrap();
}
