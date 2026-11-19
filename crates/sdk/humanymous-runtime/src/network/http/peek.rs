//! HTTP/1 wire header-order peek (legacy `peekH1HeaderOrder`, h1capture.go,
//! Go oracle 981d25c).
//!
//! Reads up to the end of the request header section, returns the lowercased
//! header names in wire order and keeps every consumed byte so the caller can
//! replay them in front of the live stream unchanged. The peek is bounded by
//! [`PEEK_LIMIT`] bytes and, on a socket, by one absolute [`PEEK_DEADLINE`].
//! It does not parse or validate the request: the serving parser still owns
//! framing, and the names are an observation, not a request authorization.
use super::{HeaderInfo, headers::trim_space};
use humanymous_core::signals::wire::simple_lowercase;
use std::io::{self, Chain, Cursor, ErrorKind, Read};
use std::net::TcpStream;
use std::time::{Duration, Instant};

/// Header bytes the peek may consume (Go `1 << 16`).
pub const PEEK_LIMIT: usize = 1 << 16;
/// Absolute socket deadline for the whole peek (Go 750 ms).
pub const PEEK_DEADLINE: Duration = Duration::from_millis(750);

const CHUNK: usize = 4096;

/// Why a peek stopped before the blank line ending the header section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeekError {
    /// End of stream, or [`PEEK_LIMIT`] reached, inside the header section.
    Truncated,
    /// The absolute socket deadline elapsed.
    TimedOut,
    /// Any other read failure.
    Io(ErrorKind),
}

/// Header names observed in wire order by a socket peek. Only [`peek_tcp`]
/// can mint one, so a caller-supplied list cannot claim order (Go peeked only
/// the accepted connection, so this adds no Go-observable difference).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireOrder(Vec<String>);

impl WireOrder {
    /// Lowercased names in wire order, duplicates retained.
    pub fn names(&self) -> &[String] {
        &self.0
    }
    /// Replace the projected sorted name set with this wire order and mark it
    /// order-reliable (legacy `mergeObservation`). Casing stays unreliable.
    pub fn apply(self, header: &mut HeaderInfo) {
        header.names = self.0;
        header.order_reliable = true;
    }
}

/// Result of one peek: observed names, consumed bytes and the stop reason.
#[derive(Debug)]
pub struct Peek {
    names: Vec<String>,
    consumed: Vec<u8>,
    outcome: Result<(), PeekError>,
    socket: bool,
}

impl Peek {
    /// Names seen so far; partial when [`Peek::outcome`] is an error, and empty
    /// when the request line itself did not complete.
    pub fn names(&self) -> &[String] {
        &self.names
    }
    /// Every byte read from the stream, including bytes past the header section.
    pub fn consumed(&self) -> &[u8] {
        &self.consumed
    }
    /// `Ok` only when the blank line ending the header section was read.
    pub fn outcome(&self) -> Result<(), PeekError> {
        self.outcome
    }
    /// Wire order for the observation from any non-empty socket peek,
    /// including a partial one that stopped on EOF, the byte cap, the
    /// deadline or a read error. Go-identical (scoring-freeze row, B1): the
    /// Go server recorded the names whatever the peek error and
    /// `mergeObservation` marked any non-empty order reliable.
    pub fn wire_order(&self) -> Option<WireOrder> {
        (self.socket && !self.names.is_empty()).then(|| WireOrder(self.names.clone()))
    }
    /// The consumed bytes followed by the rest of the stream.
    pub fn into_replay<R: Read>(self, rest: R) -> Chain<Cursor<Vec<u8>>, R> {
        Cursor::new(self.consumed).chain(rest)
    }
}

// Go string conversion of arbitrary bytes as strings.ToLower sees them: each
// invalid UTF-8 byte becomes U+FFFD.
fn go_lossy(bytes: &[u8]) -> String {
    let mut rest = bytes;
    let mut out = String::new();
    loop {
        match std::str::from_utf8(rest) {
            Ok(valid) => {
                out.push_str(valid);
                return out;
            }
            Err(error) => {
                let (valid, tail) = rest.split_at(error.valid_up_to());
                out.push_str(std::str::from_utf8(valid).unwrap_or_default());
                out.push('\u{fffd}');
                rest = &tail[1..];
            }
        }
    }
}

fn header_name(line: &[u8]) -> Option<String> {
    let colon = line.iter().position(|&b| b == b':').filter(|&at| at > 0)?;
    Some(simple_lowercase(trim_space(&go_lossy(&line[..colon]))))
}

fn peek_with(socket: bool, mut read: impl FnMut(&mut [u8]) -> io::Result<usize>) -> Peek {
    let mut peek = Peek {
        names: Vec::new(),
        consumed: Vec::new(),
        outcome: Ok(()),
        socket,
    };
    let (mut start, mut request_line) = (0, false);
    loop {
        if let Some(at) = peek.consumed[start..].iter().position(|&b| b == b'\n') {
            let line = &peek.consumed[start..start + at];
            start += at + 1;
            if !request_line {
                request_line = true;
                continue;
            }
            let end = line
                .iter()
                .rposition(|&b| b != b'\r' && b != b'\n')
                .map_or(0, |last| last + 1);
            if end == 0 {
                return peek;
            }
            if let Some(name) = header_name(&line[..end]) {
                peek.names.push(name);
            }
            continue;
        }
        let room = PEEK_LIMIT - peek.consumed.len();
        let failure = if room == 0 {
            Some(PeekError::Truncated)
        } else {
            let mut chunk = [0u8; CHUNK];
            match read(&mut chunk[..room.min(CHUNK)]) {
                Ok(0) => Some(PeekError::Truncated),
                Ok(n) => {
                    peek.consumed.extend_from_slice(&chunk[..n]);
                    None
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => None,
                Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
                {
                    Some(PeekError::TimedOut)
                }
                Err(error) => Some(PeekError::Io(error.kind())),
            }
        };
        if let Some(error) = failure {
            peek.outcome = Err(error);
            return peek;
        }
    }
}

/// Peek any reader. Such a peek never mints a [`WireOrder`].
pub fn peek_header_order(stream: &mut impl Read) -> Peek {
    peek_with(false, |buffer| stream.read(buffer))
}

/// Peek an accepted socket under one absolute [`PEEK_DEADLINE`], as Go's
/// `SetReadDeadline(now + 750 ms)`; a slow sender cannot extend it byte by
/// byte. The previous read timeout is restored afterwards, where Go cleared
/// the deadline.
pub fn peek_tcp(stream: &TcpStream) -> Peek {
    let previous = match stream.read_timeout() {
        Ok(previous) => previous,
        Err(error) => {
            return Peek {
                names: Vec::new(),
                consumed: Vec::new(),
                outcome: Err(PeekError::Io(error.kind())),
                socket: true,
            };
        }
    };
    let deadline = Instant::now() + PEEK_DEADLINE;
    let mut reader = stream;
    let peek = peek_with(true, |buffer| {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(ErrorKind::TimedOut.into());
        }
        stream.set_read_timeout(Some(left))?;
        reader.read(buffer)
    });
    let restored = stream.set_read_timeout(previous);
    match restored {
        Err(error) if peek.outcome.is_ok() => Peek {
            outcome: Err(PeekError::Io(error.kind())),
            ..peek
        },
        _ => peek,
    }
}
