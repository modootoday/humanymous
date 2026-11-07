//! Bounded connection-ordered frame and HPACK capture. This passive profile
//! does not replace the serving stack's flow control or request/body validation.
mod headers;
use headers::decode;
use humanymous_network::http2::{Fingerprint, Setting};
use std::collections::BTreeMap;

const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
const FRAME_LIMIT: usize = 16_384;
const BLOCK_LIMIT: usize = 65_536;
const STREAM_LIMIT: usize = 128;

/// A bounded profile/protocol failure. Terminal for this capture epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Invalid frame, ordering, stream, HPACK or request headers.
    Malformed,
    /// Configured bytes/frames/streams budget exceeded.
    Budget,
    /// EOF in a preface, frame or header block.
    Truncated,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "H2 capture {self:?}")
    }
}
impl std::error::Error for Error {}

/// Complete initial request header section with transient ordered byte fields.
/// Contains no body completeness or application authorization claim.
pub struct Request {
    fields: Vec<(Vec<u8>, Vec<u8>)>,
    fingerprint: Fingerprint,
}
impl Request {
    /// Original decoded order and duplicate fields before HTTP normalization.
    pub fn fields(&self) -> &[(Vec<u8>, Vec<u8>)] {
        &self.fields
    }
    /// Frozen network transforms consume this stream-specific observation.
    pub fn fingerprint(&self) -> &Fingerprint {
        &self.fingerprint
    }
}
struct Stream {
    request: Option<Request>,
    ended: bool,
    reset: bool,
}
struct Block {
    stream: u32,
    bytes: Vec<u8>,
    priority: String,
    end_stream: bool,
    trailers: bool,
}

/// Pure capture state. Public feed is parsing only; original-hop qualification
/// requires the [`crate::tls::Connection`] owner that feeds its actual plaintext.
pub struct Capture {
    preface_used: usize,
    header: [u8; 9],
    header_used: usize,
    payload: Vec<u8>,
    frames: usize,
    settings_seen: bool,
    window_seen: bool,
    fingerprint: Fingerprint,
    streams: BTreeMap<u32, Stream>,
    max_stream: u32,
    block: Option<Block>,
    decoder: httlib_hpack::Decoder<'static>,
    monitor: humanymous_network::http2::FrameMonitor,
    failed: Option<Error>,
}
impl Default for Capture {
    fn default() -> Self {
        Self {
            preface_used: 0,
            header: [0; 9],
            header_used: 0,
            payload: Vec::new(),
            frames: 0,
            settings_seen: false,
            window_seen: false,
            fingerprint: Fingerprint::default(),
            streams: BTreeMap::new(),
            max_stream: 0,
            block: None,
            decoder: httlib_hpack::Decoder::with_dynamic_size(4096),
            monitor: humanymous_network::http2::FrameMonitor::default(),
            failed: None,
        }
    }
}
impl Capture {
    /// Feed an arbitrary decrypted ingress chunk. No recoverable parse errors.
    pub fn feed(&mut self, input: &[u8]) -> Result<(), Error> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        self.monitor.feed(input);
        let result = self.feed_inner(input);
        if let Err(error) = result {
            self.failed = Some(error);
            self.streams.clear();
            self.block = None;
            self.payload.clear();
        }
        result
    }
    fn feed_inner(&mut self, mut input: &[u8]) -> Result<(), Error> {
        while self.preface_used < PREFACE.len() && !input.is_empty() {
            if input[0] != PREFACE[self.preface_used] {
                return Err(Error::Malformed);
            }
            self.preface_used += 1;
            input = &input[1..];
        }
        while !input.is_empty() {
            if self.header_used < 9 {
                let count = (9 - self.header_used).min(input.len());
                self.header[self.header_used..self.header_used + count]
                    .copy_from_slice(&input[..count]);
                self.header_used += count;
                input = &input[count..];
                if self.header_used < 9 {
                    break;
                }
                if self.length() > FRAME_LIMIT {
                    return Err(Error::Budget);
                }
                self.frames += 1;
                if self.frames > 4096 {
                    return Err(Error::Budget);
                }
            }
            let count = (self.length() - self.payload.len()).min(input.len());
            self.payload.extend_from_slice(&input[..count]);
            input = &input[count..];
            if self.payload.len() != self.length() {
                break;
            }
            self.frame()?;
            self.payload.clear();
            self.header_used = 0;
        }
        Ok(())
    }
    fn length(&self) -> usize {
        (usize::from(self.header[0]) << 16)
            | (usize::from(self.header[1]) << 8)
            | usize::from(self.header[2])
    }
    fn frame(&mut self) -> Result<(), Error> {
        let kind = self.header[3];
        let flags = self.header[4];
        let stream =
            u32::from_be_bytes(self.header[5..9].try_into().map_err(|_| Error::Malformed)?)
                & 0x7fff_ffff;
        if !self.settings_seen && (kind != 4 || flags & 1 != 0) {
            return Err(Error::Malformed);
        }
        if let Some(block) = &self.block
            && (kind != 9 || stream != block.stream)
        {
            return Err(Error::Malformed);
        }
        match kind {
            4 => {
                if stream != 0
                    || flags & 1 != 0 && !self.payload.is_empty()
                    || !self.payload.len().is_multiple_of(6)
                {
                    return Err(Error::Malformed);
                }
                if flags & 1 == 0 {
                    let mut settings = Vec::new();
                    for pair in self.payload.as_chunks::<6>().0 {
                        let id = u16::from_be_bytes([pair[0], pair[1]]);
                        let value = u32::from_be_bytes(
                            pair[2..6].try_into().map_err(|_| Error::Malformed)?,
                        );
                        if id == 2 && value > 1
                            || id == 4 && value > 0x7fff_ffff
                            || id == 5 && !(16_384..=16_777_215).contains(&value)
                        {
                            return Err(Error::Malformed);
                        }
                        settings.push(Setting { id, value });
                    }
                    if !self.settings_seen {
                        self.fingerprint.settings = settings;
                    }
                    self.settings_seen = true;
                }
            }
            8 => {
                if self.payload.len() != 4 {
                    return Err(Error::Malformed);
                }
                let increment =
                    u32::from_be_bytes(self.payload[..].try_into().map_err(|_| Error::Malformed)?)
                        & 0x7fff_ffff;
                if increment == 0 {
                    return Err(Error::Malformed);
                }
                if stream != 0 && stream > self.max_stream {
                    return Err(Error::Malformed);
                }
                if stream == 0 && !self.window_seen {
                    self.fingerprint.window_update = increment;
                    self.window_seen = true;
                }
            }
            2 => {
                if stream == 0 || self.payload.len() != 5 {
                    return Err(Error::Malformed);
                }
                let tuple = priority(stream, &self.payload)?;
                if self.fingerprint.priorities.len() >= 128 {
                    return Err(Error::Budget);
                }
                self.fingerprint
                    .priorities
                    .push(format!("{stream}:{tuple}"));
            }
            1 => self.start_headers(stream, flags)?,
            9 => {
                let block = self.block.as_mut().ok_or(Error::Malformed)?;
                if block.bytes.len() + self.payload.len() > BLOCK_LIMIT {
                    return Err(Error::Budget);
                }
                block.bytes.extend_from_slice(&self.payload);
                if flags & 4 != 0 {
                    self.finish_headers()?;
                }
            }
            0 => {
                let state = self.streams.get_mut(&stream).ok_or(Error::Malformed)?;
                if state.request.is_none() || state.ended || state.reset {
                    return Err(Error::Malformed);
                }
                unpad(&self.payload, flags)?;
                if flags & 1 != 0 {
                    state.ended = true;
                }
            }
            3 => {
                if stream == 0 || self.payload.len() != 4 {
                    return Err(Error::Malformed);
                }
                let state = self.streams.get_mut(&stream).ok_or(Error::Malformed)?;
                state.reset = true;
                state.request = None;
            }
            6 => {
                if stream != 0 || self.payload.len() != 8 {
                    return Err(Error::Malformed);
                }
            }
            7 => {
                // This ingress is no longer available for authorizing requests.
                return Err(Error::Malformed);
            }
            5 => return Err(Error::Malformed), // clients never send PUSH_PROMISE
            _ => {}                            // Unknown extensions are ignored per HTTP/2.
        }
        Ok(())
    }
    fn start_headers(&mut self, stream: u32, flags: u8) -> Result<(), Error> {
        if stream == 0 || stream.is_multiple_of(2) {
            return Err(Error::Malformed);
        }
        let trailers = if let Some(state) = self.streams.get(&stream) {
            if state.ended || state.reset || state.request.is_none() || flags & 1 == 0 {
                return Err(Error::Malformed);
            }
            true
        } else {
            if stream <= self.max_stream {
                return Err(Error::Malformed);
            }
            if self.streams.len() >= STREAM_LIMIT {
                return Err(Error::Budget);
            }
            self.max_stream = stream;
            self.streams.insert(
                stream,
                Stream {
                    request: None,
                    ended: false,
                    reset: false,
                },
            );
            false
        };
        let mut bytes = unpad(&self.payload, flags)?;
        let tuple = if flags & 0x20 != 0 {
            let value = priority(stream, bytes.get(..5).ok_or(Error::Malformed)?)?;
            bytes = &bytes[5..];
            value
        } else {
            String::new()
        };
        self.block = Some(Block {
            stream,
            bytes: bytes.to_vec(),
            priority: tuple,
            end_stream: flags & 1 != 0,
            trailers,
        });
        if flags & 4 != 0 {
            self.finish_headers()?;
        }
        Ok(())
    }
    fn finish_headers(&mut self) -> Result<(), Error> {
        let block = self.block.take().ok_or(Error::Malformed)?;
        let fields = decode(&mut self.decoder, block.bytes, block.trailers)?;
        let state = self
            .streams
            .get_mut(&block.stream)
            .ok_or(Error::Malformed)?;
        if !block.trailers {
            let mut fingerprint = self.fingerprint.clone();
            fingerprint.headers_priority = block.priority;
            for (name, _) in &fields {
                if name.starts_with(b":") {
                    fingerprint.pseudo_order.push(
                        match name.as_slice() {
                            b":method" => "m",
                            b":authority" => "a",
                            b":scheme" => "s",
                            b":path" => "p",
                            _ => return Err(Error::Malformed),
                        }
                        .into(),
                    );
                } else {
                    fingerprint.header_order.push(
                        std::str::from_utf8(name)
                            .map_err(|_| Error::Malformed)?
                            .into(),
                    );
                }
            }
            state.request = Some(Request {
                fields,
                fingerprint,
            });
        }
        state.ended = block.end_stream;
        Ok(())
    }
    /// Borrow an initial request section only after its whole HPACK block has
    /// decoded. Reset streams and terminal connection errors remain unavailable.
    pub fn request(&self, stream: u32) -> Option<&Request> {
        if self.failed.is_some() {
            return None;
        }
        self.streams.get(&stream)?.request.as_ref()
    }
    /// Existing frozen frame-monitor output; this context never changes its
    /// predicates or derives an additional score. Consumers use the signal once.
    pub fn abuse(&self) -> Option<&'static str> {
        self.monitor.abuse()
    }
    /// Reject partial EOF. This does not assert that request bodies completed.
    pub fn finish(&mut self) -> Result<(), Error> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        if self.preface_used != PREFACE.len()
            || !self.settings_seen
            || self.header_used != 0
            || self.block.is_some()
        {
            self.failed = Some(Error::Truncated);
            self.streams.clear();
            return Err(Error::Truncated);
        }
        Ok(())
    }
}
fn unpad(payload: &[u8], flags: u8) -> Result<&[u8], Error> {
    if flags & 8 == 0 {
        return Ok(payload);
    }
    let padding = usize::from(*payload.first().ok_or(Error::Malformed)?);
    let end = payload
        .len()
        .checked_sub(padding)
        .filter(|end| *end >= 1)
        .ok_or(Error::Malformed)?;
    Ok(&payload[1..end])
}
fn priority(stream: u32, bytes: &[u8]) -> Result<String, Error> {
    if bytes.len() != 5 {
        return Err(Error::Malformed);
    }
    let raw = u32::from_be_bytes(bytes[..4].try_into().map_err(|_| Error::Malformed)?);
    let dependency = raw & 0x7fff_ffff;
    if dependency == stream {
        return Err(Error::Malformed);
    }
    // Network's frozen tuple uses the encoded byte, not semantic weight+1.
    Ok(format!("{}:{dependency}:{}", raw >> 31, bytes[4]))
}

#[cfg(test)]
mod tests;
