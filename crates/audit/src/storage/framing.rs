use serde::de::DeserializeOwned;
use std::io::{self, BufRead};
pub(crate) mod cursor;
use cursor::FrameCursor;
// Private S1 primitive awaiting the read-only bounded inspection consumer.
// Keep it outside production decoding until that profile is explicitly wired.
#[cfg(test)]
pub(crate) mod preflight;

/// Successfully framed values and the byte offset safe for append recovery.
pub(super) struct Frames<T> {
    pub values: Vec<T>,
    pub complete_bytes: u64,
    pub torn_tail: bool,
}

/// Bounded JSONL decoder. A final incomplete frame is recoverable only when
/// explicitly allowed; even valid JSON without its newline is uncommitted.
pub(super) fn decode<T: DeserializeOwned>(
    reader: impl BufRead,
    max_frame: usize,
    allow_torn_tail: bool,
) -> io::Result<Frames<T>> {
    let mut cursor = FrameCursor::new(reader, max_frame, allow_torn_tail)?;
    let mut result = Frames {
        values: Vec::new(),
        complete_bytes: 0,
        torn_tail: false,
    };
    while let Some(content) = cursor.next_frame().map_err(|error| {
        if error.kind() == io::ErrorKind::FileTooLarge {
            invalid(error)
        } else {
            error
        }
    })? {
        if !content.is_empty() {
            result
                .values
                .push(serde_json::from_slice(content).map_err(invalid)?);
        }
    }
    result.complete_bytes = cursor.complete_bytes();
    result.torn_tail = cursor.torn_tail();
    Ok(result)
}

fn invalid(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::io::{BufReader, Cursor};

    fn read(bytes: &[u8], limit: usize, recover: bool) -> io::Result<Frames<Value>> {
        decode(
            BufReader::with_capacity(2, Cursor::new(bytes)),
            limit,
            recover,
        )
    }

    #[test]
    fn recovery_keeps_only_newline_committed_frames() {
        for tail in [b"{\"b\":2}".as_slice(), b"{\"b\":"] {
            let bytes = [b"{\"a\":1}\r\n\n".as_slice(), tail].concat();
            let parsed = read(&bytes, 64, true).unwrap();
            assert_eq!(parsed.values, vec![serde_json::json!({"a":1})]);
            assert_eq!(parsed.complete_bytes, 10);
            assert!(parsed.torn_tail);
            assert!(read(&bytes, 64, false).is_err());
        }
    }

    #[test]
    fn framed_corruption_is_never_a_recoverable_tail() {
        for bytes in [b"{bad}\n".as_slice(), b"\xff\n", b"{} {}\n"] {
            assert_eq!(
                read(bytes, 64, true).err().unwrap().kind(),
                io::ErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn bounds_include_newline_and_apply_to_unterminated_frames() {
        assert!(read(b"{}\n", 3, false).is_ok());
        assert!(read(b"{}\n", 2, true).is_err());
        assert_eq!(
            read(b"{}\n", 2, true).err().unwrap().kind(),
            io::ErrorKind::InvalidData
        );
        assert!(read(b"1234", 3, true).is_err());
        assert!(read(b"", 0, true).is_err());
        let clean = read(b"", 3, false).unwrap();
        assert!(!clean.torn_tail);
        assert_eq!(clean.complete_bytes, 0);
    }

    #[test]
    fn ignored_unknown_numbers_do_not_require_machine_numeric_representation() {
        // The semantic decoder skips unknown fields lexically. A resource
        // preflight must not introduce f64 conversion for those fields.
        for number in ["1e400", "-1e400", "184467440737095516160000000000"] {
            let bytes = format!("{{\"unknown\":{number}}}\n");
            let decoded = decode::<crate::record::Record>(bytes.as_bytes(), 256, false).unwrap();
            assert_eq!(decoded.values, vec![crate::record::Record::default()]);
            assert_eq!(decoded.complete_bytes, bytes.len() as u64);
            assert!(!decoded.torn_tail);
        }
        assert!(serde_json::from_str::<serde_json::Value>(r#"{"unknown":1e400}"#).is_err());
    }
}
