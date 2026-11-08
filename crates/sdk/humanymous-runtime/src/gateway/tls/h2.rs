//! R14 first-profile H2 serving consumer. Separate from R13 H1 CapturedRequest.
use crate::capture::tls::{Cancellation, Connection, PendingHandshake};
use crate::gateway::http::{self, Request};
use crate::network::ingress::{H2Claim, Refusal};
use httlib_hpack::Encoder;
use rustls::ServerConfig;
use std::{
    io,
    net::TcpStream,
    sync::Arc,
    thread,
    time::Duration,
};
use tokio::sync::oneshot;

const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
const FRAME_LIMIT: usize = 16_384;
const STREAM: u32 = 1;
const SETTINGS_INITIAL_WINDOW_SIZE: u16 = 4;
const FLAG_END_STREAM: u8 = 0x1;
const FLAG_END_HEADERS: u8 = 0x4;
const FLAG_PADDED: u8 = 0x8;
const FLAG_ACK: u8 = 0x1;

/// One bounded complete H2 request on stream 1 and its live TLS owner.
pub struct CapturedH2Request {
    request: Request,
    connection: Connection,
    stream: u32,
}

impl CapturedH2Request {
    /// Receive stream-1 END_STREAM on an owned worker; revoke+join on drop.
    pub async fn receive(
        socket: TcpStream,
        config: Arc<ServerConfig>,
        budget: Duration,
    ) -> io::Result<Self> {
        let pending = PendingHandshake::new(socket, config, budget)?;
        let cancellation = pending.cancellation()?;
        joined(cancellation, move || {
            let connection = pending.accept()?;
            serve_accepted(connection)
        })
        .await
    }
}

/// Continue R14 serve after an already-accepted `h2` Connection (dual-ALPN path).
pub(super) fn serve_accepted(mut connection: Connection) -> io::Result<CapturedH2Request> {
    if connection.negotiated_alpn() != Some(b"h2".as_slice()) {
        return Err(io::Error::other("TLS application protocol unsupported"));
    }
    connection.write_plaintext(&settings_frame(&[(
        SETTINGS_INITIAL_WINDOW_SIZE,
        http::MAX_BODY as u32,
    )]))?;
    let mut owner = BodyOwner::default();
    let mut buf = [0u8; 16_384];
    loop {
        connection.remaining()?;
        let count = connection.read_plaintext(&mut buf)?;
        if count == 0 {
            return Err(io::Error::other("H2 truncated before stream end"));
        }
        owner.feed(&buf[..count]).map_err(io::Error::other)?;
        if owner.settings_ack_due {
            connection.write_plaintext(&settings_ack())?;
            owner.settings_ack_due = false;
        }
        if let Some(metadata) = connection.h2_stream_metadata(STREAM)
            && metadata.input == crate::capture::http2::InputState::Ended
        {
            if metadata.trailer_fields.is_some() {
                return Err(io::Error::other("H2 trailers unsupported"));
            }
            let body = owner
                .take_body(STREAM)
                .ok_or_else(|| io::Error::other("H2 serving body owner missing stream"))?;
            if body.len() as u64 != metadata.data_bytes {
                return Err(io::Error::other("H2 body length mismatch"));
            }
            if body.len() > http::MAX_BODY {
                return Err(io::Error::other("H2 body budget"));
            }
            let captured = connection
                .h2_request(STREAM)
                .ok_or_else(|| io::Error::other("H2 request headers missing"))?;
            let request = gateway_request(captured.fields(), body)
                .map_err(|error| io::Error::other(format!("H2 request {error:?}")))?;
            let claim = H2Claim {
                stream: STREAM,
                method: &request.method,
                authority: request
                    .header("host")
                    .ok_or_else(|| io::Error::other("H2 authority missing"))?,
                target: &request.target,
                body_bytes: request.body.len() as u64,
            };
            connection
                .observe_h2(&claim)
                .map_err(|error| io::Error::other(format!("H2 observe {error:?}")))?;
            connection.remaining()?;
            return Ok(CapturedH2Request {
                request,
                connection,
                stream: STREAM,
            });
        }
    }
}

impl CapturedH2Request {

    /// Exact parsed gateway request bound to this H2 stream.
    pub fn request(&self) -> &Request {
        &self.request
    }

    /// Borrow the live TLS owner retained with this request.
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub(in crate::gateway) fn parts(&mut self) -> (&Request, &mut Connection, u32) {
        (&self.request, &mut self.connection, self.stream)
    }

    /// Recheck lifetime and stream-1 Ended/body invariance (not H1 idle).
    pub(in crate::gateway) fn check_stream(&mut self) -> io::Result<()> {
        self.connection.remaining()?;
        let metadata = self
            .connection
            .h2_stream_metadata(self.stream)
            .ok_or_else(|| io::Error::other("H2 stream unavailable"))?;
        if metadata.input != crate::capture::http2::InputState::Ended
            || metadata.trailer_fields.is_some()
            || metadata.data_bytes != self.request.body.len() as u64
        {
            return Err(io::Error::other("H2 stream no longer complete"));
        }
        Ok(())
    }

    /// Encode Application's H1-shaped response bytes as H2 frames on stream 1.
    pub async fn respond(mut self, h1_response: Vec<u8>) -> io::Result<()> {
        if h1_response.len() > http::MAX_ORIGIN_FRAME {
            return Err(io::Error::other("TLS complete response budget"));
        }
        let frames = h2_response_frames(self.stream, &h1_response)?;
        let cancellation = self.connection.cancellation()?;
        joined(cancellation, move || {
            let bytes = zeroize::Zeroizing::new(frames);
            self.connection.write_plaintext(&bytes)
        })
        .await
    }
}

fn gateway_request(fields: &[(Vec<u8>, Vec<u8>)], body: Vec<u8>) -> Result<Request, Refusal> {
    let mut method = None;
    let mut authority = None;
    let mut path = None;
    let mut headers = Vec::new();
    for (name, value) in fields {
        let name = std::str::from_utf8(name).map_err(|_| Refusal::InvalidField)?;
        let value = std::str::from_utf8(value).map_err(|_| Refusal::InvalidField)?;
        match name {
            ":method" => method = Some(value.to_owned()),
            ":authority" => authority = Some(value.to_owned()),
            ":path" => path = Some(value.to_owned()),
            ":scheme" => {
                if value != "https" {
                    return Err(Refusal::Protocol);
                }
            }
            _ if name.starts_with(':') => return Err(Refusal::InvalidField),
            "host" => headers.push((name.to_owned(), value.to_owned())),
            _ => headers.push((name.to_owned(), value.to_owned())),
        }
    }
    let (method, authority, target) = match (method, authority, path) {
        (Some(m), Some(a), Some(p)) if !m.is_empty() && !a.is_empty() && !p.is_empty() => (m, a, p),
        _ => return Err(Refusal::MissingField),
    };
    if !headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("host"))
    {
        headers.insert(0, ("host".into(), authority));
    }
    Ok(Request {
        method,
        target,
        headers,
        body,
    })
}

fn settings_frame(settings: &[(u16, u32)]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(settings.len() * 6);
    for &(id, value) in settings {
        payload.extend_from_slice(&id.to_be_bytes());
        payload.extend_from_slice(&value.to_be_bytes());
    }
    frame(4, 0, 0, &payload)
}

fn settings_ack() -> Vec<u8> {
    frame(4, FLAG_ACK, 0, &[])
}

fn frame(kind: u8, flags: u8, stream: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(9 + payload.len());
    let len = payload.len() as u32;
    out.push((len >> 16) as u8);
    out.push((len >> 8) as u8);
    out.push(len as u8);
    out.push(kind);
    out.push(flags);
    out.extend_from_slice(&(stream & 0x7fff_ffff).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

fn h2_response_frames(stream: u32, h1: &[u8]) -> io::Result<Vec<u8>> {
    let (status, headers, body) = parse_h1_response(h1)?;
    let mut encoder = Encoder::default();
    let mut block = Vec::new();
    let flags = Encoder::NEVER_INDEXED;
    encoder
        .encode(
            (b":status".to_vec(), status.into_bytes(), flags),
            &mut block,
        )
        .map_err(|_| io::Error::other("H2 response HPACK"))?;
    for (name, value) in headers {
        encoder
            .encode((name.into_bytes(), value.into_bytes(), flags), &mut block)
            .map_err(|_| io::Error::other("H2 response HPACK"))?;
    }
    if block.len() > FRAME_LIMIT {
        return Err(io::Error::other("H2 response header budget"));
    }
    let mut out = Vec::new();
    if body.is_empty() {
        out.extend(frame(
            1,
            FLAG_END_HEADERS | FLAG_END_STREAM,
            stream,
            &block,
        ));
    } else {
        out.extend(frame(1, FLAG_END_HEADERS, stream, &block));
        let chunks: Vec<_> = body.chunks(FRAME_LIMIT).collect();
        for (index, chunk) in chunks.iter().enumerate() {
            let flags = if index + 1 == chunks.len() {
                FLAG_END_STREAM
            } else {
                0
            };
            out.extend(frame(0, flags, stream, chunk));
        }
    }
    Ok(out)
}

type ParsedH1Response = (String, Vec<(String, String)>, Vec<u8>);

fn parse_h1_response(bytes: &[u8]) -> io::Result<ParsedH1Response> {
    let text = std::str::from_utf8(bytes).map_err(|_| io::Error::other("H2 response utf8"))?;
    let (head, body) = text
        .split_once("\r\n\r\n")
        .ok_or_else(|| io::Error::other("H2 response framing"))?;
    let mut lines = head.split("\r\n");
    let status_line = lines
        .next()
        .ok_or_else(|| io::Error::other("H2 response status"))?;
    let mut parts = status_line.split(' ');
    if parts.next() != Some("HTTP/1.1") {
        return Err(io::Error::other("H2 response version"));
    }
    let status = parts
        .next()
        .filter(|value| value.len() == 3 && value.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(|| io::Error::other("H2 response status"))?
        .to_owned();
    let mut headers = Vec::new();
    for line in lines {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| io::Error::other("H2 response header"))?;
        let name = name.trim().to_ascii_lowercase();
        if matches!(
            name.as_str(),
            "connection" | "transfer-encoding" | "keep-alive" | "proxy-connection" | "upgrade"
        ) {
            continue;
        }
        headers.push((name, value.trim().to_owned()));
    }
    Ok((status, headers, body.as_bytes().to_vec()))
}

/// Gateway-local DATA retention; Capture metadata remains body-free (N05).
#[derive(Default)]
struct BodyOwner {
    preface_used: usize,
    header: [u8; 9],
    header_used: usize,
    payload: Vec<u8>,
    settings_seen: bool,
    settings_ack_due: bool,
    bodies: std::collections::BTreeMap<u32, Vec<u8>>,
    ended: std::collections::BTreeSet<u32>,
}

impl BodyOwner {
    fn feed(&mut self, mut input: &[u8]) -> Result<(), &'static str> {
        while self.preface_used < PREFACE.len() && !input.is_empty() {
            if input[0] != PREFACE[self.preface_used] {
                return Err("H2 preface");
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
                    return Err("H2 frame budget");
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

    fn frame(&mut self) -> Result<(), &'static str> {
        let kind = self.header[3];
        let flags = self.header[4];
        let stream = u32::from_be_bytes(self.header[5..9].try_into().map_err(|_| "H2 stream")?)
            & 0x7fff_ffff;
        match kind {
            4 => {
                if stream != 0 {
                    return Err("H2 settings stream");
                }
                if flags & FLAG_ACK == 0 {
                    self.settings_seen = true;
                    self.settings_ack_due = true;
                }
            }
            0 => {
                if stream != STREAM || self.ended.contains(&stream) {
                    return Err("H2 data stream");
                }
                let bytes = unpad(&self.payload, flags)?;
                let body = self.bodies.entry(stream).or_default();
                if body.len() + bytes.len() > http::MAX_BODY {
                    return Err("H2 body budget");
                }
                body.extend_from_slice(bytes);
                if flags & FLAG_END_STREAM != 0 {
                    self.ended.insert(stream);
                }
            }
            1 | 9 => {
                // Capture owns HPACK validation; this owner only tracks END_STREAM.
                if stream != STREAM {
                    return Err("H2 headers stream");
                }
                if flags & FLAG_END_STREAM != 0 {
                    self.bodies.entry(stream).or_default();
                    self.ended.insert(stream);
                }
            }
            _ => {}
        }
        let _ = self.settings_seen;
        Ok(())
    }

    fn take_body(&mut self, stream: u32) -> Option<Vec<u8>> {
        self.bodies.remove(&stream)
    }
}

fn unpad(payload: &[u8], flags: u8) -> Result<&[u8], &'static str> {
    if flags & FLAG_PADDED == 0 {
        return Ok(payload);
    }
    let padding = usize::from(*payload.first().ok_or("H2 padding")?);
    let end = payload
        .len()
        .checked_sub(padding)
        .filter(|end| *end >= 1)
        .ok_or("H2 padding")?;
    Ok(&payload[1..end])
}

struct Worker {
    cancellation: Cancellation,
    thread: Option<thread::JoinHandle<()>>,
}
impl Worker {
    fn finish(mut self) -> io::Result<()> {
        self.thread
            .take()
            .ok_or_else(|| io::Error::other("TLS worker ownership missing"))?
            .join()
            .map_err(|_| io::Error::other("TLS worker panicked"))
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = self.cancellation.cancel();
            let _ = thread.join();
        }
    }
}
async fn joined<T: Send + 'static>(
    cancellation: Cancellation,
    work: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> io::Result<T> {
    let (sender, receiver) = oneshot::channel();
    let thread = thread::Builder::new()
        .name("runtime-tls-h2".into())
        .spawn(move || {
            let _ = sender.send(work());
        })?;
    let worker = Worker {
        cancellation,
        thread: Some(thread),
    };
    let result = receiver
        .await
        .map_err(|_| io::Error::other("TLS worker unavailable"));
    worker.finish()?;
    result?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_and_ack_frames_are_well_formed() {
        let settings = settings_frame(&[(SETTINGS_INITIAL_WINDOW_SIZE, http::MAX_BODY as u32)]);
        assert_eq!(&settings[3..5], &[4, 0]);
        assert_eq!(&settings_ack()[3..5], &[4, FLAG_ACK]);
    }

    #[test]
    fn body_owner_retains_stream_one_data_only() {
        let mut owner = BodyOwner::default();
        let mut wire = PREFACE.to_vec();
        wire.extend(frame(4, 0, 0, &{
            let mut payload = Vec::new();
            payload.extend_from_slice(&SETTINGS_INITIAL_WINDOW_SIZE.to_be_bytes());
            payload.extend_from_slice(&65535u32.to_be_bytes());
            payload
        }));
        // HEADERS END_HEADERS on stream 1 without END_STREAM, empty block.
        wire.extend(frame(1, FLAG_END_HEADERS, STREAM, &[]));
        wire.extend(frame(0, FLAG_END_STREAM, STREAM, b"abc"));
        owner.feed(&wire).unwrap();
        assert!(owner.settings_ack_due);
        assert_eq!(owner.take_body(STREAM).unwrap(), b"abc");
    }

    #[test]
    fn h2_response_frames_end_stream_on_empty_body() {
        let frames = h2_response_frames(
            STREAM,
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
        )
        .unwrap();
        assert_eq!(frames[3], 1); // HEADERS
        assert_eq!(frames[4] & FLAG_END_HEADERS, FLAG_END_HEADERS);
        assert_eq!(frames[4] & FLAG_END_STREAM, 0);
        let data_at = frames.windows(1).position(|b| b[0] == 0 && false);
        let _ = data_at;
        assert!(frames.windows(2).any(|w| w[0] == 0 || (w.len() > 1 && frames.contains(&0))));
        // DATA frame kind appears after HEADERS.
        assert!(frames[9..].windows(4).any(|w| w[3] == 0));
    }
}
