use super::*;
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    io::{self, Cursor, Read},
    time::Duration,
};

#[derive(Default)]
struct FixedClock;
impl Clock for FixedClock {
    fn elapsed(&mut self) -> Duration {
        Duration::ZERO
    }
}
struct ScriptClock {
    values: VecDeque<Duration>,
}
impl Clock for ScriptClock {
    fn elapsed(&mut self) -> Duration {
        self.values.pop_front().expect("declared clock calls")
    }
}
fn limits(file: u64, total: u64) -> Limits {
    Limits {
        max_file_bytes: file,
        max_total_read_bytes: total,
        ..Limits::default()
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn accepted_defaults_are_finite_and_explicit() {
    let l = Limits::default();
    assert_eq!((l.max_files, l.max_roots), (5_000, 64));
    assert_eq!(l.max_total_read_bytes, 2 * 1024 * 1024 * 1024);
    assert_eq!(l.max_file_bytes, 512 * 1024 * 1024);
    assert_eq!(l.max_elapsed, Duration::from_secs(30));
}

#[test]
fn empty_reader_observes_eof_without_consuming_bytes() {
    let r = ReadBudget::new(limits(0, 1))
        .unwrap()
        .read(&mut Cursor::new([]), &mut FixedClock);
    assert!(r.complete());
    assert_eq!((r.actual_read_bytes, r.probe_bytes), (0, 0));
    assert_eq!(r.sha256, Some(hash(b"")));
}

#[test]
fn exact_file_cap_requires_a_real_eof_probe() {
    let r = ReadBudget::new(limits(3, 4))
        .unwrap()
        .read(&mut Cursor::new(b"abc"), &mut FixedClock);
    assert!(r.complete());
    assert_eq!(r.sha256, Some(hash(b"abc")));
    assert_eq!((r.actual_read_bytes, r.probe_bytes), (3, 0));
}

#[test]
fn cap_plus_one_is_accounted_and_has_no_full_digest() {
    let mut input = Cursor::new(b"abcdextra");
    let mut budget = ReadBudget::new(limits(3, 8)).unwrap();
    let r = budget.read(&mut input, &mut FixedClock);
    assert_eq!(r.status, ReadStatus::FileLimit);
    assert_eq!(
        (r.actual_read_bytes, r.probe_bytes, input.position()),
        (4, 1, 4)
    );
    assert_eq!(budget.actual_read_bytes(), 4);
    assert!(r.sha256.is_none());
}

#[test]
fn exact_aggregate_cap_is_unknown_without_probe_budget() {
    let mut input = Cursor::new(b"abc");
    let r = ReadBudget::new(limits(3, 3))
        .unwrap()
        .read(&mut input, &mut FixedClock);
    assert_eq!(r.status, ReadStatus::AggregateLimit);
    assert_eq!(r.actual_read_bytes, 3);
    assert!(r.sha256.is_none());
}

#[test]
fn multiple_files_share_content_and_probe_budget() {
    let mut budget = ReadBudget::new(limits(2, 5)).unwrap();
    let a = budget.read(&mut Cursor::new(b"ab"), &mut FixedClock);
    let b = budget.read(&mut Cursor::new(b"cde"), &mut FixedClock);
    assert!(a.complete());
    assert_eq!(b.status, ReadStatus::FileLimit);
    assert_eq!((budget.actual_read_bytes(), budget.probe_bytes()), (5, 1));
    let mut untouched = Cursor::new(b"x");
    assert_eq!(
        budget.read(&mut untouched, &mut FixedClock).status,
        ReadStatus::AggregateLimit
    );
    assert_eq!(untouched.position(), 0);
}

struct ShortReader(Cursor<Vec<u8>>);
impl Read for ShortReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.0.read(&mut out[..1])
    }
}
#[test]
fn short_reads_are_not_mistaken_for_eof() {
    let r = ReadBudget::new(limits(8, 16)).unwrap().read(
        &mut ShortReader(Cursor::new(b"abc".to_vec())),
        &mut FixedClock,
    );
    assert!(r.complete());
    assert_eq!(r.sha256, Some(hash(b"abc")));
}

struct InterruptedOnce {
    first: bool,
    content: Cursor<Vec<u8>>,
}
impl Read for InterruptedOnce {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.first {
            self.first = false;
            Err(io::ErrorKind::Interrupted.into())
        } else {
            self.content.read(out)
        }
    }
}
#[test]
fn interrupted_read_retries_without_counting_phantom_bytes() {
    let mut reader = InterruptedOnce {
        first: true,
        content: Cursor::new(b"abc".to_vec()),
    };
    let r = ReadBudget::new(limits(8, 16))
        .unwrap()
        .read(&mut reader, &mut FixedClock);
    assert!(r.complete());
    assert_eq!(r.actual_read_bytes, 3);
}

struct DataThenError(bool);
impl Read for DataThenError {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.0 {
            Err(io::ErrorKind::PermissionDenied.into())
        } else {
            self.0 = true;
            out[0] = b'a';
            Ok(1)
        }
    }
}
#[test]
fn io_error_keeps_partial_count_and_no_digest() {
    let r = ReadBudget::new(limits(8, 16))
        .unwrap()
        .read(&mut DataThenError(false), &mut FixedClock);
    assert_eq!(r.status, ReadStatus::IoError);
    assert_eq!(r.actual_read_bytes, 1);
    assert!(r.sha256.is_none());
}

#[test]
fn deadline_after_blocking_read_prevents_a_late_complete_claim() {
    let mut clock = ScriptClock {
        values: [Duration::ZERO, Duration::from_secs(30)].into(),
    };
    let r = ReadBudget::new(limits(8, 16))
        .unwrap()
        .read(&mut Cursor::new([]), &mut clock);
    assert_eq!(r.status, ReadStatus::Deadline);
    assert!(!r.complete());
}

#[test]
fn interrupted_read_checks_the_cooperative_deadline() {
    let mut clock = ScriptClock {
        values: [Duration::ZERO, Duration::from_secs(30)].into(),
    };
    let mut reader = InterruptedOnce {
        first: true,
        content: Cursor::new(vec![]),
    };
    let r = ReadBudget::new(limits(8, 16))
        .unwrap()
        .read(&mut reader, &mut clock);
    assert_eq!(r.status, ReadStatus::Deadline);
}

#[test]
fn shared_clock_regression_is_not_a_per_file_reset() {
    let mut budget = ReadBudget::new(limits(8, 16)).unwrap();
    let mut first = ScriptClock {
        values: [Duration::from_secs(1), Duration::from_secs(2)].into(),
    };
    assert!(budget.read(&mut Cursor::new([]), &mut first).complete());
    assert_eq!(
        budget.read(&mut Cursor::new([]), &mut FixedClock).status,
        ReadStatus::ClockRegression
    );
}

struct InvalidCount;
impl Read for InvalidCount {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Ok(usize::MAX)
    }
}
#[test]
fn invalid_reader_count_is_refused_without_panicking_or_hashing() {
    let mut budget = ReadBudget::new(limits(8, 16)).unwrap();
    let r = budget.read(&mut InvalidCount, &mut FixedClock);
    assert_eq!(r.status, ReadStatus::InvalidReadCount);
    assert_eq!(r.actual_read_bytes, 0);
    assert!(r.sha256.is_none());
    assert!(!r.read_count_known);
    let mut untouched = Cursor::new(b"x");
    assert_eq!(
        budget.read(&mut untouched, &mut FixedClock).status,
        ReadStatus::ProviderUnknown
    );
    assert_eq!(untouched.position(), 0);
}

fn roots() -> Vec<Root> {
    vec![Root {
        id: "r".into(),
        path: std::env::temp_dir().join("hmn-selection"),
    }]
}
fn input(path: &str) -> Input {
    Input {
        root: "r".into(),
        path: path.into(),
    }
}
#[test]
fn frozen_inventory_rejects_aliases_escapes_and_unknown_roots() {
    assert!(matches!(
        FrozenSelection::new(
            roots(),
            vec![input("a"), input("A")],
            true,
            &Limits::default()
        ),
        Err(SelectionError::DuplicateInput)
    ));
    for path in ["../a", "/a", "a//b", "a/./b", "a\\b", "C:a", "a\0b"] {
        assert!(matches!(
            FrozenSelection::new(roots(), vec![input(path)], true, &Limits::default()),
            Err(SelectionError::InvalidPath)
        ));
    }
    assert!(matches!(
        FrozenSelection::new(
            roots(),
            vec![Input {
                root: "unknown".into(),
                path: "a".into()
            }],
            true,
            &Limits::default()
        ),
        Err(SelectionError::UnknownRoot)
    ));
}

#[test]
fn finite_file_and_root_counts_apply_before_observation() {
    let l = Limits {
        max_files: 1,
        max_roots: 1,
        ..Limits::default()
    };
    assert!(matches!(
        FrozenSelection::new(roots(), vec![input("a"), input("b")], true, &l),
        Err(SelectionError::CountLimit)
    ));
    let mut two = roots();
    two.push(Root {
        id: "other".into(),
        path: std::env::temp_dir().join("other"),
    });
    assert!(matches!(
        FrozenSelection::new(two, vec![input("a")], true, &l),
        Err(SelectionError::CountLimit)
    ));
}

#[test]
fn selection_fingerprint_detects_changed_names_and_coverage_declaration() {
    let a = FrozenSelection::new(roots(), vec![input("a")], true, &Limits::default()).unwrap();
    let b = FrozenSelection::new(roots(), vec![input("b")], true, &Limits::default()).unwrap();
    let unknown =
        FrozenSelection::new(roots(), vec![input("a")], false, &Limits::default()).unwrap();
    assert_ne!(a.digest(), b.digest());
    assert_ne!(a.digest(), unknown.digest());
}

#[test]
fn invalid_limits_and_vacuous_selection_are_rejected() {
    assert!(matches!(
        ReadBudget::new(Limits {
            max_elapsed: Duration::ZERO,
            ..Limits::default()
        }),
        Err(SelectionError::InvalidLimits)
    ));
    assert!(matches!(
        FrozenSelection::new(roots(), vec![], true, &Limits::default()),
        Err(SelectionError::EmptySelection)
    ));
}
