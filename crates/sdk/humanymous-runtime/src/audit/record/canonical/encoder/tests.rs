use super::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    input: Record,
    canonical: String,
}
#[derive(Deserialize)]
struct Corpus {
    records: Vec<Case>,
}

#[test]
fn exact_budget_emission_matches_all_frozen_canonical_records() {
    let corpus: Corpus = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/audit/records.json"
    )))
    .unwrap();
    assert_eq!(corpus.records.len(), 92);
    for case in corpus.records {
        let expected = hex::decode(case.canonical).unwrap();
        let before = case.input.clone();
        let mut out = Vec::new();
        write_record(
            &case.input,
            &mut out,
            Limits {
                bytes: expected.len(),
                items: 4096,
            },
        )
        .unwrap();
        assert_eq!(out, expected);
        assert_eq!(case.input, before);
        let mut short = Vec::new();
        let error = write_record(
            &case.input,
            &mut short,
            Limits {
                bytes: expected.len() - 1,
                items: 4096,
            },
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::FileTooLarge);
        assert!(short.len() < expected.len());
    }
}

#[test]
fn item_budget_refuses_before_any_field_output() {
    let record = Record {
        triggered_rules: Some(vec!["raw-rule".into()]),
        ..Record::default()
    };
    let mut out = Vec::new();
    assert_eq!(
        write_record(
            &record,
            &mut out,
            Limits {
                bytes: 4096,
                items: 0
            }
        )
        .unwrap_err()
        .kind(),
        io::ErrorKind::FileTooLarge
    );
    assert!(out.is_empty());
    let mut out = Vec::new();
    assert_eq!(
        write_record(&Record::default(), &mut out, Limits { bytes: 0, items: 0 })
            .unwrap_err()
            .kind(),
        io::ErrorKind::FileTooLarge
    );
    assert!(out.is_empty());
}

#[test]
fn count_replay_disagreement_never_returns_success() {
    let calls = std::cell::Cell::new(0);
    let mut bytes = Vec::new();
    let mut out = Limited {
        inner: &mut bytes,
        written: 0,
        limit: 64,
    };
    let error = field(&mut out, "k", |sink| {
        let prior = calls.get();
        calls.set(prior + 1);
        sink.write_all(if prior == 0 { b"a" } else { b"bb" })
    })
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(calls.get(), 2);
    assert_eq!(bytes.len(), 9); // Header only: oversized replay payload was refused.
}

#[test]
fn length_counter_overflow_refuses_without_large_allocation() {
    let mut out = Count {
        bytes: usize::MAX,
        limit: usize::MAX,
    };
    assert_eq!(
        out.write(b"x").unwrap_err().kind(),
        io::ErrorKind::FileTooLarge
    );
    assert_eq!(out.bytes, usize::MAX);
    let mut out = Count {
        bytes: u32::MAX as usize,
        limit: u32::MAX as usize,
    };
    assert_eq!(
        out.write(b"x").unwrap_err().kind(),
        io::ErrorKind::FileTooLarge
    );
}

#[test]
fn partial_sink_writes_and_io_failure_cannot_be_reported_as_complete() {
    struct Failing {
        bytes: Vec<u8>,
        remaining: usize,
    }
    impl Write for Failing {
        fn write(&mut self, input: &[u8]) -> io::Result<usize> {
            if self.remaining == 0 {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            let count = input.len().min(self.remaining).min(3);
            self.bytes.extend_from_slice(&input[..count]);
            self.remaining -= count;
            Ok(count)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut out = Failing {
        bytes: Vec::new(),
        remaining: 7,
    };
    assert_eq!(
        write_record(&Record::default(), &mut out, Limits::compatibility())
            .unwrap_err()
            .kind(),
        io::ErrorKind::BrokenPipe
    );
    assert_eq!(out.bytes.len(), 7);
}
