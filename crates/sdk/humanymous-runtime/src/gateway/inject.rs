//! Streaming, single-pass HTML loader injector (legacy internal/gate/inject.go
//! at baseline 981d25c, SoT-20).
//!
//! The snippet is inserted exactly once at the first head boundary using a
//! bounded look-ahead. Injection is idempotent (a marker prevents a second
//! insertion) and add-only: origin bytes are never rewritten or narrowed, one
//! tag block is inserted. Callers force `Accept-Encoding: identity` upstream so
//! the body is plain.
use std::io::{self, Read};

/// Marker that makes injection idempotent (SoT-20 §2).
pub const MARKER: &str = "<!--hmn-injected-->";

/// Look-ahead budget before giving up, so a body that never closes its head
/// cannot make the injector buffer without limit (SoT-20 §8, HR-27a).
pub const MAX_LOOKAHEAD: usize = 64 << 10;

const READ_CHUNK: usize = 8 << 10;

/// Why the snippet was not inserted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skipped {
    /// The page already carries [`MARKER`].
    AlreadyPresent,
    /// No insertion point within the budget, or the upstream read failed first.
    PointNotFound,
}

impl Skipped {
    /// Audit reason string (Go `injectResult.Reason`).
    pub fn reason(self) -> &'static str {
        match self {
            Self::AlreadyPresent => "already_present",
            Self::PointNotFound => "point_not_found",
        }
    }
}

/// What the injector decided; feeds the audit event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The snippet was inserted once.
    Injected,
    /// The body passes through unchanged.
    Skipped(Skipped),
}

/// Wraps an upstream body and yields the same bytes with the snippet inserted
/// once at the first head boundary.
pub struct Injector<R> {
    source: R,
    snippet: Vec<u8>,
    pending: Vec<u8>,
    offset: usize,
    lookahead: Vec<u8>,
    outcome: Option<Outcome>,
    source_ended: bool,
    pending_error: Option<io::Error>,
}

impl<R: Read> Injector<R> {
    /// Injector inserting `snippet` into the HTML read from `source`.
    pub fn new(source: R, snippet: impl Into<Vec<u8>>) -> Self {
        Self {
            source,
            snippet: snippet.into(),
            pending: Vec::new(),
            offset: 0,
            lookahead: Vec::new(),
            outcome: None,
            source_ended: false,
            pending_error: None,
        }
    }

    /// The decision, once made (after the first read that needed it).
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    fn resolve(&mut self, at: Option<usize>, skipped: Skipped) {
        let lookahead = std::mem::take(&mut self.lookahead);
        match at {
            Some(index) => {
                let mut out = Vec::with_capacity(lookahead.len() + self.snippet.len());
                out.extend_from_slice(&lookahead[..index]);
                out.extend_from_slice(&self.snippet);
                out.extend_from_slice(&lookahead[index..]);
                self.pending = out;
                self.outcome = Some(Outcome::Injected);
            }
            None => {
                self.pending = lookahead;
                self.outcome = Some(Outcome::Skipped(skipped));
            }
        }
        self.offset = 0;
    }

    fn drain(&mut self, out: &mut [u8]) -> usize {
        let rest = &self.pending[self.offset..];
        let count = rest.len().min(out.len());
        out[..count].copy_from_slice(&rest[..count]);
        self.offset += count;
        count
    }

    fn decide(&mut self) {
        let marker = MARKER.as_bytes();
        while self.outcome.is_none() {
            // An existing marker always sits right after an earlier insertion
            // point: the page is already injected, so pass it through.
            if contains(&self.lookahead, marker) {
                self.resolve(None, Skipped::AlreadyPresent);
                return;
            }
            if let Some(index) = insertion_point(&self.lookahead) {
                // Wait for the bytes right after the point before deciding,
                // unless upstream already ended.
                let undecided = self.lookahead.len() < index + marker.len() && !self.source_ended;
                if !undecided {
                    if self.lookahead[index..].starts_with(marker) {
                        self.resolve(None, Skipped::AlreadyPresent);
                    } else {
                        self.resolve(Some(index), Skipped::PointNotFound);
                    }
                    return;
                }
            }
            if self.lookahead.len() >= MAX_LOOKAHEAD {
                self.resolve(None, Skipped::PointNotFound);
                return;
            }
            let mut chunk = [0; READ_CHUNK];
            match self.source.read(&mut chunk) {
                Ok(0) => {
                    // End before a point was confirmed: inject at the point if
                    // this looks like HTML, otherwise pass through.
                    self.source_ended = true;
                    let point = insertion_point(&self.lookahead);
                    self.resolve(point, Skipped::PointNotFound);
                    return;
                }
                Ok(count) => self.lookahead.extend_from_slice(&chunk[..count]),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => {
                    self.resolve(None, Skipped::PointNotFound);
                    self.pending_error = Some(error);
                    return;
                }
            }
        }
    }
}

impl<R: Read> Read for Injector<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        if self.offset < self.pending.len() {
            return Ok(self.drain(out));
        }
        if self.outcome.is_none() {
            self.decide();
            if self.offset < self.pending.len() {
                return Ok(self.drain(out));
            }
        }
        if let Some(error) = self.pending_error.take() {
            return Err(error);
        }
        if self.source_ended {
            return Ok(0);
        }
        self.source.read(out)
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    find(haystack, needle).is_some()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Offset to insert at: right after the first `<head ...>` opening tag, else
/// right before `</head>`, else before `<body ...>`; `None` when there is no
/// point yet (including an unclosed `<head`). Case-insensitive and add-only
/// (SoT-20 §6). Like the oracle, `<head` also matches a `<header` prefix.
pub fn insertion_point(bytes: &[u8]) -> Option<usize> {
    let lower = bytes.to_ascii_lowercase();
    if let Some(start) = find(&lower, b"<head") {
        return bytes[start..]
            .iter()
            .position(|byte| *byte == b'>')
            .map(|close| start + close + 1);
    }
    find(&lower, b"</head>").or_else(|| find(&lower, b"<body"))
}

#[cfg(test)]
mod tests;
