use super::*;
use humanymous_core::signals::wire::quote_json_go;
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    checkpoint: Checkpoint,
}

fn legacy(checkpoint: &Checkpoint) -> Vec<u8> {
    format!("{{\"node_id\":{},\"tree_size\":{},\"root\":{},\"merkle_root\":{},\"prev_checkpoint_hash\":{}}}",
        quote_json_go(&checkpoint.node_id), checkpoint.tree_size, quote_json_go(&checkpoint.root),
        quote_json_go(&checkpoint.merkle_root), quote_json_go(&checkpoint.prev_checkpoint_hash)).into_bytes()
}

#[test]
fn all_frozen_checkpoints_preserve_legacy_bytes_and_exact_budget_boundary() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../../../tests/fixtures/checkpoints.json")).unwrap();
    assert_eq!(cases.len(), 180);
    for case in cases {
        let expected = legacy(&case.checkpoint);
        assert_eq!(bounded(&case.checkpoint, expected.len()).unwrap(), expected);
        assert_eq!(
            bounded(&case.checkpoint, expected.len() - 1)
                .unwrap_err()
                .kind(),
            io::ErrorKind::FileTooLarge
        );
        assert_eq!(case.checkpoint.signing_bytes(), expected);
    }
    let checkpoint = Checkpoint {
        node_id: "<&>\u{2028}\u{2029}\0\n\"\\e\u{301}".into(),
        tree_size: u64::MAX,
        root: "é".into(),
        ..Default::default()
    };
    assert_eq!(bounded(&checkpoint, 16384).unwrap(), legacy(&checkpoint));
    assert_eq!(
        bounded(&checkpoint, 0).unwrap_err().kind(),
        io::ErrorKind::FileTooLarge
    );
}

struct Broken {
    remaining: usize,
    bytes: Vec<u8>,
}
impl Write for Broken {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "injected failure",
            ));
        }
        let count = bytes.len().min(self.remaining);
        self.bytes.extend_from_slice(&bytes[..count]);
        self.remaining -= count;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn partial_output_failure_is_returned_without_success() {
    let checkpoint = Checkpoint {
        node_id: "<&é".into(),
        ..Default::default()
    };
    let expected = legacy(&checkpoint);
    for stop in 0..expected.len() {
        let mut out = Broken {
            remaining: stop,
            bytes: Vec::new(),
        };
        assert_eq!(
            encode(&checkpoint, &mut out).unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
        assert_eq!(out.bytes, expected[..stop]);
    }
}

#[test]
fn counter_exhaustion_refuses_before_writing_or_wrapping() {
    let mut out = Limited {
        inner: Vec::new(),
        written: usize::MAX,
        limit: usize::MAX,
    };
    assert_eq!(
        out.write(b"x").unwrap_err().kind(),
        io::ErrorKind::FileTooLarge
    );
    assert!(out.inner.is_empty());
    assert_eq!(out.written, usize::MAX);
}
