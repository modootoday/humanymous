//! Strict bounded single-frame native HTTP/1.1. No normalized proxy provenance.
use crate::{Rejection, configuration};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Maximum request entity size for this complete-body profile.
pub const MAX_BODY: usize = 65_536;
const MAX_HEADER: usize = 16_384;
/// A frame observed directly on the accepted socket; credentials are not Debug.
pub struct Request {
    /// Exact method.
    pub method: String,
    /// Exact origin-form target, including query.
    pub target: String,
    /// Original header name/order/casing, after surrounding OWS removal.
    pub headers: Vec<(String, String)>,
    /// Complete original entity bytes.
    pub body: Vec<u8>,
}
impl Request {
    /// Unique case-insensitive header lookup.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
    /// Canonical decimal integer; alternate encodings are rejected.
    pub fn number(&self, name: &str) -> Result<u64, Rejection> {
        decimal(
            self.header(name)
                .ok_or(Rejection::new(400, "missing_integer"))?,
        )
    }
}

fn token(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}
fn decimal(value: &str) -> Result<u64, Rejection> {
    if value.is_empty()
        || value.len() > 20
        || !value.bytes().all(|b| b.is_ascii_digit())
        || value.len() > 1 && value.starts_with('0')
    {
        return Err(Rejection::new(400, "integer"));
    }
    value.parse().map_err(|_| Rejection::new(400, "integer"))
}
fn parse_headers(bytes: &[u8]) -> Result<(Request, usize), Rejection> {
    let bad = Rejection::new(400, "http_framing");
    let text = std::str::from_utf8(bytes).map_err(|_| bad)?;
    let mut lines = text.split("\r\n");
    let mut start = lines.next().ok_or(bad)?.split(' ');
    let method = start.next().ok_or(bad)?;
    let target = start.next().ok_or(bad)?;
    if start.next() != Some("HTTP/1.1")
        || start.next().is_some()
        || !configuration::method(method)
        || !configuration::target(target)
    {
        return Err(bad);
    }
    let mut request = Request {
        method: method.into(),
        target: target.into(),
        headers: Vec::new(),
        body: Vec::new(),
    };
    let mut seen = std::collections::HashSet::new();
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(bad)?;
        if request.headers.len() >= 100
            || name.is_empty()
            || !name.bytes().all(token)
            || value.bytes().any(|b| b < 32 && b != b'\t' || b >= 127)
            || !seen.insert(name.to_ascii_lowercase())
        {
            return Err(bad);
        }
        let value = value.trim_matches([' ', '\t']);
        match name.to_ascii_lowercase().as_str() {
            "transfer-encoding" | "upgrade" | "expect" | "trailer" | "te" | "proxy-connection" => {
                return Err(bad);
            }
            "connection"
                if !value.eq_ignore_ascii_case("close")
                    && !value.eq_ignore_ascii_case("keep-alive") =>
            {
                return Err(bad);
            }
            _ => {}
        }
        request.headers.push((name.into(), value.into()));
    }
    if request.header("host").is_none_or(str::is_empty) {
        return Err(bad);
    }
    let length = if request.header("content-length").is_none()
        && matches!(request.method.as_str(), "GET" | "HEAD")
        && request.header("x-humanymous-protocol")
            == Some(humanymous_integrity::request::browser::PROTOCOL)
    {
        0
    } else {
        usize::try_from(request.number("content-length")?).map_err(|_| bad)?
    };
    if length > MAX_BODY {
        return Err(Rejection::new(413, "body_size"));
    }
    Ok((request, length))
}

/// Read one bounded complete frame. Caller supplies a fixed overall deadline.
pub async fn read(stream: &mut (impl AsyncRead + Unpin)) -> Result<Request, Rejection> {
    let mut bytes = Vec::with_capacity(4096);
    let end = loop {
        if let Some(i) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
            if i + 4 > MAX_HEADER {
                return Err(Rejection::new(413, "header_size"));
            }
            break i;
        }
        if bytes.len() >= MAX_HEADER {
            return Err(Rejection::new(413, "header_size"));
        }
        let mut chunk = [0; 1024];
        let n = stream
            .read(&mut chunk)
            .await
            .map_err(|_| Rejection::new(400, "http_read"))?;
        if n == 0 {
            return Err(Rejection::new(400, "truncated_headers"));
        }
        bytes.extend_from_slice(&chunk[..n]);
    };
    let (mut request, length) = parse_headers(&bytes[..end])?;
    let mut body = bytes.split_off(end + 4);
    if body.len() > length {
        return Err(Rejection::new(400, "trailing_frame"));
    }
    let got = body.len();
    body.resize(length, 0);
    stream
        .read_exact(&mut body[got..])
        .await
        .map_err(|_| Rejection::new(400, "truncated_body"))?;
    request.body = body;
    Ok(request)
}

/// Small local JSON response, always close/no-store; no reflected header values.
pub async fn write(
    stream: &mut (impl AsyncWrite + Unpin),
    status: u16,
    body: &[u8],
) -> std::io::Result<()> {
    write_headers(stream, status, body, &[]).await
}

/// Emit gateway-owned fixed-name response metadata after one consumed request.
pub(crate) async fn write_headers(
    stream: &mut (impl AsyncWrite + Unpin),
    status: u16,
    body: &[u8],
    extra: &[(String, String)],
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        409 => "Conflict",
        413 => "Content Too Large",
        422 => "Unprocessable Content",
        429 => "Too Many Requests",
        _ => "Service Unavailable",
    };
    let mut header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n",
        body.len()
    );
    append_extra(&mut header, extra)?;
    header.push_str("\r\n");
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.shutdown().await
}

fn append_extra(header: &mut String, extra: &[(String, String)]) -> std::io::Result<()> {
    for (name, value) in extra {
        if !name.to_ascii_lowercase().starts_with("x-humanymous-")
            || !name.bytes().all(token)
            || value.bytes().any(|b| !(32..127).contains(&b))
        {
            return Err(std::io::Error::other("invalid gateway response metadata"));
        }
        header.push_str(name);
        header.push_str(": ");
        header.push_str(value);
        header.push_str("\r\n");
    }
    Ok(())
}

/// Rebuild only framing; preserve complete entity and safe end-to-end fields.
pub fn origin_request(request: &Request, authority: &str) -> Vec<u8> {
    let mut bytes = format!(
        "{} {} HTTP/1.1\r\nHost: {authority}\r\nContent-Length: {}\r\nConnection: close\r\n",
        request.method,
        request.target,
        request.body.len()
    )
    .into_bytes();
    for (name, value) in &request.headers {
        let lower = name.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "host"
                | "content-length"
                | "connection"
                | "keep-alive"
                | "proxy-authorization"
                | "proxy-authenticate"
        ) || lower.starts_with("x-humanymous-")
            || lower == "authorization"
                && !matches!(
                    crate::browser::profile(request),
                    Ok(configuration::RitProfile::BrowserAuthOnly)
                )
        {
            continue;
        }
        bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(&request.body);
    bytes
}

/// Validate a bounded opaque origin response without coalescing Set-Cookie.
/// Close-delimited and transfer-coded responses are outside this pilot profile.
pub fn response(bytes: &[u8], head: bool) -> Result<(), Rejection> {
    Response::parse(bytes, head).map(|_| ())
}

/// Validated bounded origin frame. No Debug leaks application header/entity data.
pub(crate) struct Response {
    pub(crate) status: u16,
    status_line: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}
impl Response {
    pub(crate) fn parse(bytes: &[u8], head: bool) -> Result<Self, Rejection> {
        let bad = Rejection::new(503, "origin_framing");
        let end = bytes.windows(4).position(|b| b == b"\r\n\r\n").ok_or(bad)?;
        if end + 4 > MAX_HEADER || bytes.len() > MAX_BODY + MAX_HEADER {
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
                "transfer-encoding" | "upgrade" | "trailer" => return Err(bad),
                "connection" if !value.trim().eq_ignore_ascii_case("close") => return Err(bad),
                _ => {}
            }
        }
        let empty = head || code == 204 || code == 304;
        if code == 204 && length.is_some() {
            return Err(bad);
        }
        let length = if empty {
            length.unwrap_or(0)
        } else {
            length.ok_or(bad)?
        };
        if (head || code == 204 || code == 304) && bytes.len() != end + 4 {
            return Err(bad);
        }
        if !head && code != 204 && code != 304 && bytes.len() - (end + 4) != length as usize {
            return Err(bad);
        }
        Ok(Self {
            status: code,
            status_line: status.to_owned(),
            headers: fields,
            body: bytes[end + 4..].to_vec(),
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
                    "connection" | "keep-alive" | "proxy-authenticate" | "proxy-authorization"
                )
            {
                continue;
            }
            result.push_str(name);
            result.push(':');
            result.push_str(value);
            result.push_str("\r\n");
        }
        result.push_str("Connection: close\r\n");
        append_extra(&mut result, extra).map_err(|_| Rejection::new(503, "response_metadata"))?;
        result.push_str("\r\n");
        let mut bytes = result.into_bytes();
        bytes.extend_from_slice(&self.body);
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_framing_rejects_smuggling_and_normalization() {
        for frame in [
            "GET / HTTP/1.1\r\nHost: app\r\nContent-Length: 00",
            "GET / HTTP/1.1\r\nHost: app\r\nContent-Length: 0\r\ncontent-length: 0",
            "GET /%2e HTTP/1.1\r\nHost: app\r\nContent-Length: 0",
            "GET / HTTP/1.1\r\nHost: app\r\nContent-Length: 0\r\nTransfer-Encoding: chunked",
            "GET / HTTP/1.1\r\nHost: app\r\nContent-Length: 0\r\nConnection: x-secret",
        ] {
            assert!(parse_headers(frame.as_bytes()).is_err(), "{frame}");
        }
    }
    #[test]
    fn origin_response_preserves_cookie_bytes_and_rejects_ambiguity() {
        let bytes=b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nConnection: close\r\n\r\nok";
        assert!(response(bytes, false).is_ok());
        assert!(response(b"HTTP/1.1 101 Upgrade\r\nContent-Length: 0\r\n\r\n", false).is_err());
        assert!(
            response(
                b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nContent-Length: 0\r\n\r\n",
                false
            )
            .is_err()
        );
    }
}
