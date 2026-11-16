//! Ports of legacy `internal/mltrain/sink_test.go` plus fail-closed refusals.
use super::*;
use crate::model::training::record::read_jsonl;
use crate::signals::report::BehaviorSummary;
use std::io::BufReader;
use std::path::PathBuf;

/// Owned scratch directory removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "humanymous-training-sink-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn record(ts: i64, cohort: Option<&str>) -> Record {
    Record {
        label: 0,
        behavior: BehaviorSummary {
            duration_s: 8.0,
            ..BehaviorSummary::default()
        },
        ts: Some(ts),
        cohort: cohort.map(str::to_owned),
        source: Some("pass".into()),
    }
}

fn read(path: &Path) -> Vec<Record> {
    let file = File::open(path).unwrap();
    read_jsonl(BufReader::new(file), &schema_hash(), Limits::default())
        .unwrap()
        .into_iter()
        .map(|sample| sample.record)
        .collect()
}

/// TestTraceSink_AppendRoundTrip: the missing parent is created and the file
/// reads back through the shared training reader as exactly what was written.
#[test]
fn append_round_trip_creates_parent_and_reads_back() {
    let scratch = Scratch::new();
    let path = scratch.0.join("sub").join("oracle.jsonl");
    let mut sink = TraceSink::open(&path).unwrap();
    sink.append(&record(100, None)).unwrap();
    sink.append(&record(101, Some("typical"))).unwrap();
    assert_eq!(sink.count(), 2);
    sink.close().unwrap();
    let got = read(&path);
    assert_eq!(got, vec![record(100, None), record(101, Some("typical"))]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(path.parent().unwrap()), 0o700);
    }
}

/// TestTraceSink_AppendsAcrossReopen: reopening appends, never truncates.
#[test]
fn reopen_appends_and_count_is_per_handle() {
    let scratch = Scratch::new();
    let path = scratch.0.join("oracle.jsonl");
    for ts in [1, 2] {
        let mut sink = TraceSink::open(&path).unwrap();
        sink.append(&record(ts, None)).unwrap();
        assert_eq!(sink.count(), 1);
        sink.close().unwrap();
    }
    let lines = std::fs::read(&path).unwrap();
    assert_eq!(lines.iter().filter(|byte| **byte == b'\n').count(), 2);
    assert_eq!(read(&path), vec![record(1, None), record(2, None)]);
}

#[test]
fn relative_parent_directory_and_symlink_targets_are_refused() {
    let scratch = Scratch::new();
    assert_eq!(
        TraceSink::open(Path::new("oracle.jsonl")).unwrap_err(),
        SinkError::Path
    );
    assert_eq!(
        TraceSink::open(&scratch.0.join("a").join("..").join("oracle.jsonl")).unwrap_err(),
        SinkError::Path
    );
    assert_eq!(
        TraceSink::open(&scratch.0).unwrap_err(),
        SinkError::NotRegular
    );
    #[cfg(unix)]
    {
        let target = scratch.0.join("target.jsonl");
        std::fs::write(&target, b"").unwrap();
        let link = scratch.0.join("link.jsonl");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(TraceSink::open(&link).unwrap_err(), SinkError::NotRegular);
    }
}

#[test]
fn inadmissible_records_are_refused_without_writing() {
    let scratch = Scratch::new();
    let path = scratch.0.join("oracle.jsonl");
    let mut sink = TraceSink::open(&path).unwrap();
    let mut bad = record(1, None);
    bad.label = 2;
    assert_eq!(
        sink.append(&bad).unwrap_err(),
        SinkError::Invalid(RecordError::InvalidLabel)
    );
    let mut nonfinite = record(1, None);
    nonfinite.behavior.duration_s = f64::NAN;
    assert!(matches!(
        sink.append(&nonfinite).unwrap_err(),
        SinkError::Invalid(_)
    ));
    assert_eq!(sink.count(), 0);
    sink.close().unwrap();
    assert!(std::fs::read(&path).unwrap().is_empty());
}
