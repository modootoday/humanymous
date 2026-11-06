use serde::de::DeserializeOwned;
use std::io::{self, BufRead};

/// Successfully framed values and the byte offset safe for append recovery.
pub(super) struct Frames<T> {
    pub values: Vec<T>,
    pub complete_bytes: u64,
    pub torn_tail: bool,
}

/// Bounded JSONL decoder. A final incomplete frame is recoverable only when
/// explicitly allowed; even valid JSON without its newline is uncommitted.
pub(super) fn decode<T: DeserializeOwned>(
    mut reader: impl BufRead,
    max_frame: usize,
    allow_torn_tail: bool,
) -> io::Result<Frames<T>> {
    if max_frame == 0 {
        return Err(invalid("zero frame limit"));
    }
    let mut result = Frames {
        values: Vec::new(),
        complete_bytes: 0,
        torn_tail: false,
    };
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            if !line.is_empty() {
                if !allow_torn_tail {
                    return Err(invalid("incomplete JSONL frame"));
                }
                result.torn_tail = true;
            }
            return Ok(result);
        }
        let end = available.iter().position(|&b| b == b'\n');
        let count = end.map_or(available.len(), |n| n + 1);
        if count > max_frame.saturating_sub(line.len()) {
            return Err(invalid("JSONL frame exceeds configured limit"));
        }
        line.extend_from_slice(&available[..count]);
        reader.consume(count);
        if end.is_some() {
            let content = line.strip_suffix(b"\n").expect("framed newline");
            let content = content.strip_suffix(b"\r").unwrap_or(content);
            if !content.is_empty() {
                result
                    .values
                    .push(serde_json::from_slice(content).map_err(invalid)?);
            }
            result.complete_bytes = result
                .complete_bytes
                .checked_add(line.len() as u64)
                .ok_or_else(|| invalid("JSONL offset overflow"))?;
            line.clear();
        }
    }
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
        assert!(read(b"1234", 3, true).is_err());
        assert!(read(b"", 0, true).is_err());
        let clean = read(b"", 3, false).unwrap();
        assert!(!clean.torn_tail);
        assert_eq!(clean.complete_bytes, 0);
    }
}
