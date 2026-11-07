//! Bounded complete origin frames and response-header ownership.
use super::{MAX_BODY, MAX_HEADER, MAX_ORIGIN_FRAME, append_extra, decimal, token};
use crate::Rejection;

/// Validated bounded origin frame. No Debug leaks application header/entity data.
pub(crate) struct Response {
    pub(crate) status: u16,
    status_line: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    reframe_length: bool,
}
impl Response {
    pub(crate) fn parse(bytes: &[u8], head: bool) -> Result<Self, Rejection> {
        let bad = Rejection::new(503, "origin_framing");
        let end = bytes.windows(4).position(|b| b == b"\r\n\r\n").ok_or(bad)?;
        if end + 4 > MAX_HEADER || bytes.len() > MAX_ORIGIN_FRAME {
            return Err(bad);
        }
        let text = std::str::from_utf8(&bytes[..end]).map_err(|_| bad)?;
        let mut lines = text.split("\r\n");
        let status = lines.next().ok_or(bad)?;
        let parts: Vec<_> = status.splitn(3, ' ').collect();
        if parts.len() != 3
            || parts[0] != "HTTP/1.1"
            || parts[1].len() != 3
            || !parts[1].bytes().all(|b| b.is_ascii_digit())
            || status.bytes().any(|b| !(32..127).contains(&b))
        {
            return Err(bad);
        }
        let code: u16 = parts[1].parse().map_err(|_| bad)?;
        if !(200..=599).contains(&code) {
            return Err(bad);
        }
        let mut length = None;
        let mut chunked = false;
        let mut fields = Vec::new();
        for line in lines {
            let (name, value) = line.split_once(':').ok_or(bad)?;
            if fields.len() >= 100
                || name.is_empty()
                || !name.bytes().all(token)
                || value.bytes().any(|b| b < 32 && b != b'\t' || b >= 127)
            {
                return Err(bad);
            }
            fields.push((name.to_owned(), value.to_owned()));
            match name.to_ascii_lowercase().as_str() {
                "content-length" => {
                    if length.is_some() {
                        return Err(bad);
                    }
                    length = Some(decimal(value.trim()).map_err(|_| bad)?);
                }
                "transfer-encoding" => {
                    if chunked || !value.trim().eq_ignore_ascii_case("chunked") {
                        return Err(bad);
                    }
                    chunked = true;
                }
                "upgrade" | "trailer" => return Err(bad),
                "connection"
                    if !value.trim().eq_ignore_ascii_case("close")
                        && !value.trim().eq_ignore_ascii_case("keep-alive") =>
                {
                    return Err(bad);
                }
                _ => {}
            }
        }
        let empty = head || code == 204 || code == 304;
        if chunked && length.is_some() || code == 204 && (length.is_some() || chunked) {
            return Err(bad);
        }
        let entity = &bytes[end + 4..];
        if entity.len() > MAX_BODY * 2 {
            return Err(bad);
        }
        if empty && !entity.is_empty() {
            return Err(bad);
        }
        let body = if empty {
            Vec::new()
        } else if chunked {
            decode_chunks(entity)?
        } else {
            entity.to_vec()
        };
        if body.len() > MAX_BODY
            || !empty && length.is_some_and(|length| usize::try_from(length) != Ok(body.len()))
        {
            return Err(bad);
        }
        Ok(Self {
            status: code,
            status_line: status.to_owned(),
            headers: fields,
            body,
            reframe_length: !empty && (chunked || length.is_none()),
        })
    }

    /// Reframe one already-validated entity, preserving duplicate end-to-end fields
    /// while preventing an origin from forging gateway counter acknowledgements.
    pub(crate) fn encode(&self, extra: &[(String, String)]) -> Result<Vec<u8>, Rejection> {
        let mut result = format!("{}\r\n", self.status_line);
        for (name, value) in &self.headers {
            let lower = name.to_ascii_lowercase();
            if lower.starts_with("x-humanymous-")
                || matches!(
                    lower.as_str(),
                    "connection"
                        | "keep-alive"
                        | "proxy-authenticate"
                        | "proxy-authorization"
                        | "transfer-encoding"
                        | "trailer"
                )
            {
                continue;
            }
            result.push_str(name);
            result.push(':');
            result.push_str(value);
            result.push_str("\r\n");
        }
        if self.reframe_length {
            result.push_str(&format!("Content-Length: {}\r\n", self.body.len()));
        }
        result.push_str("Connection: close\r\n");
        append_extra(&mut result, extra).map_err(|_| Rejection::new(503, "response_metadata"))?;
        result.push_str("\r\n");
        let mut bytes = result.into_bytes();
        bytes.extend_from_slice(&self.body);
        Ok(bytes)
    }
}

fn decode_chunks(mut wire: &[u8]) -> Result<Vec<u8>, Rejection> {
    let bad = Rejection::new(503, "origin_chunk_framing");
    let mut body = Vec::new();
    for _ in 0..=4096 {
        let end = wire
            .windows(2)
            .position(|bytes| bytes == b"\r\n")
            .ok_or(bad)?;
        let size = &wire[..end];
        if size.is_empty() || size.len() > 16 || !size.iter().all(u8::is_ascii_hexdigit) {
            return Err(bad);
        }
        let size = usize::from_str_radix(std::str::from_utf8(size).map_err(|_| bad)?, 16)
            .map_err(|_| bad)?;
        wire = &wire[end + 2..];
        if size == 0 {
            return if wire == b"\r\n" { Ok(body) } else { Err(bad) };
        }
        let length = body
            .len()
            .checked_add(size)
            .filter(|length| *length <= MAX_BODY)
            .ok_or(bad)?;
        let frame = size.checked_add(2).ok_or(bad)?;
        if wire.len() < frame || &wire[size..frame] != b"\r\n" {
            return Err(bad);
        }
        body.reserve(length - body.len());
        body.extend_from_slice(&wire[..size]);
        wire = &wire[frame..];
    }
    Err(bad)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunked_complete_body_reframes_and_preserves_cookie_fields() {
        let response=Response::parse(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nConnection: keep-alive\r\n\r\n2\r\nhe\r\n3\r\nllo\r\n0\r\n\r\n",false).unwrap();
        let encoded = response.encode(&[]).unwrap();
        assert_eq!(encoded,b"HTTP/1.1 200 OK\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello");
        assert!(Response::parse(&encoded, false).is_ok());
    }
    #[test]
    fn chunk_ambiguity_truncation_extensions_and_trailers_fail_closed() {
        for body in [
            "1\r\nx\r\n0\r\n\r\nx",
            "1\r\nx",
            "1;x=y\r\nx\r\n0\r\n\r\n",
            "0\r\nX-Other: x\r\n\r\n",
            "10001\r\n",
            "FFFFFFFFFFFFFFFF\r\n",
        ] {
            let frame = format!("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{body}");
            assert!(Response::parse(frame.as_bytes(), false).is_err());
        }
        assert!(Response::parse(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",false).is_err());
        assert!(
            Response::parse(
                b"HTTP/1.1 204 No Content\r\nTransfer-Encoding: chunked\r\n\r\n",
                false
            )
            .is_err()
        );
    }
    #[test]
    fn close_delimited_body_is_bounded_and_reframed_after_eof() {
        let response =
            Response::parse(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nok", false).unwrap();
        assert_eq!(
            response.encode(&[]).unwrap(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
        );
        let mut frame = b"HTTP/1.1 200 OK\r\n\r\n".to_vec();
        frame.extend(vec![b'x'; MAX_BODY + 1]);
        assert!(Response::parse(&frame, false).is_err());
    }
}
