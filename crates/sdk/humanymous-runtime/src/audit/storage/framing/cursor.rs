//! One borrowed newline-committed frame at a time; no history collection.
use super::invalid;
use std::io::{self, BufRead};

pub(in crate::audit) struct FrameCursor<R> {
    reader: R,
    line: Vec<u8>,
    max_frame: usize,
    allow_torn_tail: bool,
    complete_bytes: u64,
    torn_tail: bool,
    finished: bool,
    failed: bool,
}

impl<R: BufRead> FrameCursor<R> {
    pub fn new(reader: R, max_frame: usize, allow_torn_tail: bool) -> io::Result<Self> {
        if max_frame == 0 {
            return Err(invalid("zero frame limit"));
        }
        Ok(Self {
            reader,
            line: Vec::new(),
            max_frame,
            allow_torn_tail,
            complete_bytes: 0,
            torn_tail: false,
            finished: false,
            failed: false,
        })
    }

    pub fn complete_bytes(&self) -> u64 {
        self.complete_bytes
    }

    pub fn torn_tail(&self) -> bool {
        self.torn_tail
    }

    /// Empty committed lines are returned as empty slices. Only LF and one
    /// preceding CR are removed. A caller must decode/authenticate every frame.
    pub fn next_frame(&mut self) -> io::Result<Option<&[u8]>> {
        self.next_raw_frame().map(|frame| frame.map(content))
    }

    #[cfg(test)]
    pub fn raw_tail(&self) -> &[u8] {
        if self.finished && self.torn_tail {
            &self.line
        } else {
            &[]
        }
    }

    pub fn next_raw_frame(&mut self) -> io::Result<Option<&[u8]>> {
        if self.failed {
            return Err(invalid("JSONL cursor already failed"));
        }
        if self.finished {
            return Ok(None);
        }
        // Any failure poisons this cursor, including reader/allocation failures.
        // A retry must restart the scan, never resume after uncertain input.
        self.failed = true;
        self.line.clear();
        loop {
            let available = self.reader.fill_buf()?;
            if available.is_empty() {
                if !self.line.is_empty() {
                    if !self.allow_torn_tail {
                        return Err(invalid("incomplete JSONL frame"));
                    }
                    self.torn_tail = true;
                }
                self.finished = true;
                self.failed = false;
                return Ok(None);
            }
            let end = available.iter().position(|&byte| byte == b'\n');
            let count = end.map_or(available.len(), |index| index + 1);
            let length = self
                .line
                .len()
                .checked_add(count)
                .filter(|length| *length <= self.max_frame)
                .ok_or_else(|| resource("JSONL frame exceeds configured limit"))?;
            if length > self.line.capacity() {
                let capacity = self
                    .line
                    .capacity()
                    .checked_mul(2)
                    .unwrap_or(self.max_frame)
                    .max(length)
                    .min(self.max_frame);
                self.line
                    .try_reserve_exact(capacity - self.line.len())
                    .map_err(|_| io::Error::other("JSONL frame allocation refused"))?;
            }
            self.line.extend_from_slice(&available[..count]);
            self.reader.consume(count);
            if end.is_some() {
                self.complete_bytes = self
                    .complete_bytes
                    .checked_add(
                        u64::try_from(length).map_err(|_| resource("JSONL offset overflow"))?,
                    )
                    .ok_or_else(|| resource("JSONL offset overflow"))?;
                self.failed = false;
                return Ok(Some(&self.line));
            }
        }
    }
}

pub(in crate::audit) fn content(raw: &[u8]) -> &[u8] {
    let content = raw.strip_suffix(b"\n").expect("framed newline");
    content.strip_suffix(b"\r").unwrap_or(content)
}

fn resource(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::FileTooLarge, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor, Read};

    #[test]
    fn fragmented_frames_preserve_blank_crlf_and_torn_offsets() {
        let reader = BufReader::with_capacity(1, Cursor::new(b"{}\r\n\n\r\npartial"));
        let mut cursor = FrameCursor::new(reader, 8, true).unwrap();
        assert_eq!(cursor.next_frame().unwrap(), Some(b"{}".as_slice()));
        assert_eq!(cursor.complete_bytes(), 4);
        assert_eq!(cursor.next_frame().unwrap(), Some(b"".as_slice()));
        assert_eq!(cursor.next_frame().unwrap(), Some(b"".as_slice()));
        assert_eq!(cursor.next_frame().unwrap(), None);
        assert_eq!(cursor.complete_bytes(), 7);
        assert!(cursor.torn_tail());
        assert_eq!(cursor.next_frame().unwrap(), None);
    }

    #[test]
    fn oversized_frame_refuses_without_consuming_offending_input_or_resuming() {
        let reader = Cursor::new(b"{}\nlong\n{}\n");
        let mut cursor = FrameCursor::new(reader, 3, true).unwrap();
        assert_eq!(cursor.next_frame().unwrap(), Some(b"{}".as_slice()));
        assert!(cursor.next_frame().is_err());
        assert_eq!(cursor.reader.position(), 3);
        assert_eq!(cursor.complete_bytes(), 3);
        assert!(cursor.next_frame().is_err());
        assert!(!cursor.torn_tail());
    }

    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("injected read failure"))
        }
    }

    #[test]
    fn reader_failure_after_valid_prefix_never_becomes_clean_eof() {
        let reader = BufReader::with_capacity(2, Cursor::new(b"{}\n{").chain(Broken));
        let mut cursor = FrameCursor::new(reader, 8, true).unwrap();
        assert_eq!(cursor.next_frame().unwrap(), Some(b"{}".as_slice()));
        assert_eq!(
            cursor.next_frame().unwrap_err().kind(),
            io::ErrorKind::Other
        );
        assert_eq!(cursor.complete_bytes(), 3);
        assert!(cursor.next_frame().is_err());
        assert!(!cursor.torn_tail());
    }

    #[test]
    fn offset_overflow_never_commits_or_resumes() {
        let mut cursor = FrameCursor::new(Cursor::new(b"{}\n"), 3, false).unwrap();
        cursor.complete_bytes = u64::MAX - 1;
        assert!(cursor.next_frame().is_err());
        assert_eq!(cursor.complete_bytes(), u64::MAX - 1);
        assert!(cursor.next_frame().is_err());
        assert!(!cursor.torn_tail());
    }
}
