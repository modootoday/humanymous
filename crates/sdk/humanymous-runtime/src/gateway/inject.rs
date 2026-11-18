//! Streaming, single-pass HTML loader injector, ported from legacy
//! `internal/gate/inject.go` (oracle 981d25c, plan B15).
//!
//! [`InjectingReader`] yields the upstream body with one snippet inserted at
//! the first head boundary: right after the first `<head ...>` tag, else
//! before `</head>`, else before `<body ...>`. It is add-only (origin bytes are
//! never rewritten or narrowed), idempotent (a body that already carries
//! [`MARKER`] passes through untouched) and bounded: at most [`MAX_LOOKAHEAD`]
//! bytes plus one read chunk are buffered while hunting for the insertion
//! point, after which the body passes through un-injected. Callers ask the
//! origin for an identity-encoded body so the bytes are plain HTML.
//!
//! One deliberate difference from Go: the tag search is ASCII
//! case-insensitive on the original bytes. Go searched a `bytes.ToLower`
//! copy, whose offsets can drift from the original when non-ASCII text
//! changes byte length under Unicode lowering, splitting a character.
//!
//! Composition into a served response path (content-type selection, the
//! identity request, CSP and the audit event fed by [`Outcome`]) is not done
//! here.
use std::io::{self, Read};

/// Idempotency marker; its presence means the body is already injected.
pub const MARKER: &str = "<!--hmn-injected-->";
/// Most bytes buffered before giving up (Go `maxLookahead`, 64 KiB).
pub const MAX_LOOKAHEAD: usize = 64 << 10;
const CHUNK: usize = 8 << 10;

/// Why the snippet was not inserted (Go `injectResult.Reason`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// The marker is already present.
    AlreadyPresent,
    /// No head boundary within the look-ahead budget or before end of body.
    PointNotFound,
}

impl Reason {
    /// Legacy audit wording.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyPresent => "already_present",
            Self::PointNotFound => "point_not_found",
        }
    }
}

/// What the injector did, once decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing has been read yet.
    Pending,
    /// The snippet was inserted once.
    Injected,
    /// The body passes through unchanged.
    NotInjected(Reason),
}

/// Reader that inserts a snippet into an HTML stream (Go `injectingReader`).
pub struct InjectingReader<R> {
    source: R,
    snippet: Vec<u8>,
    lookahead: Vec<u8>,
    staged: Vec<u8>,
    staged_at: usize,
    outcome: Outcome,
    source_ended: bool,
    pending_error: Option<io::Error>,
}

impl<R> std::fmt::Debug for InjectingReader<R> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InjectingReader")
            .field("outcome", &self.outcome)
            .finish_non_exhaustive()
    }
}

impl<R: Read> InjectingReader<R> {
    /// Wrap `source`; the snippet should start with [`MARKER`] so a second
    /// pass over the output is a no-op.
    pub fn new(source: R, snippet: impl Into<Vec<u8>>) -> Self {
        Self {
            source,
            snippet: snippet.into(),
            lookahead: Vec::new(),
            staged: Vec::new(),
            staged_at: 0,
            outcome: Outcome::Pending,
            source_ended: false,
            pending_error: None,
        }
    }

    /// The decision so far.
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }

    /// The wrapped source.
    pub fn into_inner(self) -> R {
        self.source
    }

    fn decide(&mut self) {
        let marker = MARKER.as_bytes();
        loop {
            if contains(&self.lookahead, marker) {
                return self.pass(Reason::AlreadyPresent);
            }
            if let Some(at) = insertion_point(&self.lookahead) {
                // Wait for the bytes right after the point unless the body
                // ended: a marker there means the page is already injected.
                let waiting = self.lookahead.len() < at + marker.len() && !self.source_ended;
                if !waiting {
                    if self.lookahead[at..].starts_with(marker) {
                        return self.pass(Reason::AlreadyPresent);
                    }
                    return self.inject(at);
                }
            }
            if self.lookahead.len() >= MAX_LOOKAHEAD {
                return self.pass(Reason::PointNotFound);
            }
            let mut chunk = [0u8; CHUNK];
            match self.source.read(&mut chunk) {
                Ok(0) => {
                    self.source_ended = true;
                    return match insertion_point(&self.lookahead) {
                        Some(at) => self.inject(at),
                        None => self.pass(Reason::PointNotFound),
                    };
                }
                Ok(read) => self.lookahead.extend_from_slice(&chunk[..read]),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => {
                    // Surface the upstream error after the buffered bytes.
                    self.pending_error = Some(error);
                    return self.pass(Reason::PointNotFound);
                }
            }
        }
    }

    /// Stage the look-ahead with the snippet inserted at `at`.
    fn inject(&mut self, at: usize) {
        let lookahead = std::mem::take(&mut self.lookahead);
        let mut staged = Vec::with_capacity(lookahead.len() + self.snippet.len());
        staged.extend_from_slice(&lookahead[..at]);
        staged.extend_from_slice(&self.snippet);
        staged.extend_from_slice(&lookahead[at..]);
        self.staged = staged;
        self.outcome = Outcome::Injected;
    }

    /// Stage the look-ahead unchanged.
    fn pass(&mut self, reason: Reason) {
        self.staged = std::mem::take(&mut self.lookahead);
        self.outcome = Outcome::NotInjected(reason);
    }
}

impl<R: Read> Read for InjectingReader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.outcome == Outcome::Pending {
            self.decide();
        }
        if self.staged_at < self.staged.len() {
            let copied = (self.staged.len() - self.staged_at).min(out.len());
            out[..copied].copy_from_slice(&self.staged[self.staged_at..self.staged_at + copied]);
            self.staged_at += copied;
            if self.staged_at == self.staged.len() {
                self.staged = Vec::new();
                self.staged_at = 0;
            }
            return Ok(copied);
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

/// Byte offset to insert at (Go `findInsertionPoint`): right after the first
/// `<head ...>` tag, else before `</head>`, else before `<body ...>`. A
/// `<head` whose `>` has not arrived yet answers `None` so the caller waits.
pub fn insertion_point(bytes: &[u8]) -> Option<usize> {
    if let Some(head) = find(bytes, b"<head") {
        return bytes[head..]
            .iter()
            .position(|byte| *byte == b'>')
            .map(|close| head + close + 1);
    }
    find(bytes, b"</head>").or_else(|| find(bytes, b"<body"))
}

/// First ASCII case-insensitive occurrence of a lowercase `needle`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[cfg(test)]
mod tests;
