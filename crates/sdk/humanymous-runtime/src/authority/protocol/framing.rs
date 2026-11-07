//! One bounded HTTP head/entity decoder for local requests and responses.
use crate::authority::Rejection;
use tokio::io::{AsyncRead, AsyncReadExt};
pub(super) const MAX_HEADER: usize = 16_384;
pub(in crate::authority) struct Head {
    pub start: String,
    pub headers: Vec<(String, String)>,
}
impl Head {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|h| h.0.eq_ignore_ascii_case(name))
            .map(|h| h.1.as_str())
    }
}
fn token(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}
fn parse(bytes: &[u8], max_body: usize) -> Result<(Head, usize), Rejection> {
    let bad = Rejection::new(400, "http_framing");
    let text = std::str::from_utf8(bytes).map_err(|_| bad)?;
    let mut lines = text.split("\r\n");
    let start = lines.next().ok_or(bad)?;
    if start.is_empty() || start.bytes().any(|b| !(32..127).contains(&b)) {
        return Err(bad);
    }
    let mut headers = Vec::new();
    let mut singletons = std::collections::HashSet::new();
    for line in lines {
        if headers.len() >= 100 {
            return Err(bad);
        }
        let (name, value) = line.split_once(':').ok_or(bad)?;
        if name.is_empty()
            || !name.bytes().all(token)
            || value.bytes().any(|b| b < 32 && b != b'\t' || b >= 127)
        {
            return Err(bad);
        }
        let lower = name.to_ascii_lowercase();
        if [
            "authorization",
            "host",
            "content-length",
            "content-type",
            "connection",
        ]
        .contains(&lower.as_str())
            && !singletons.insert(lower.clone())
        {
            return Err(bad);
        }
        if ["transfer-encoding", "te", "trailer", "expect", "upgrade"].contains(&lower.as_str()) {
            return Err(bad);
        }
        let value = value.trim_matches([' ', '\t']);
        if lower == "connection"
            && !value.eq_ignore_ascii_case("close")
            && !value.eq_ignore_ascii_case("keep-alive")
        {
            return Err(bad);
        }
        headers.push((name.to_owned(), value.to_owned()));
    }
    let head = Head {
        start: start.into(),
        headers,
    };
    if !head
        .header("content-type")
        .is_some_and(|v| v.eq_ignore_ascii_case("application/json"))
    {
        return Err(bad);
    }
    let length = head.header("content-length").ok_or(bad)?;
    if length.is_empty()
        || length.len() > 7
        || !length.bytes().all(|b| b.is_ascii_digit())
        || length.len() > 1 && length.starts_with('0')
    {
        return Err(bad);
    }
    let length: usize = length.parse().map_err(|_| bad)?;
    if length > max_body {
        return Err(Rejection::new(413, "envelope_size"));
    }
    Ok((head, length))
}
pub(in crate::authority) async fn read(
    stream: &mut (impl AsyncRead + Unpin),
    max_body: usize,
    retain: bool,
) -> Result<(Head, Vec<u8>), Rejection> {
    let mut buffer = Vec::with_capacity(4096);
    let end = loop {
        if let Some(end) = buffer.windows(4).position(|b| b == b"\r\n\r\n") {
            if end + 4 > MAX_HEADER {
                return Err(Rejection::new(413, "header_size"));
            }
            break end;
        }
        if buffer.len() >= MAX_HEADER {
            return Err(Rejection::new(413, "header_size"));
        }
        let mut chunk = [0u8; 1024];
        let n = stream
            .read(&mut chunk)
            .await
            .map_err(|_| Rejection::new(400, "http_read"))?;
        if n == 0 {
            return Err(Rejection::new(400, "truncated_headers"));
        }
        buffer.extend_from_slice(&chunk[..n]);
    };
    let (head, length) = parse(&buffer[..end], max_body)?;
    let mut body = buffer.split_off(end + 4);
    if body.len() > length {
        return Err(Rejection::new(400, "trailing_frame"));
    }
    if retain {
        let got = body.len();
        body.resize(length, 0);
        stream
            .read_exact(&mut body[got..])
            .await
            .map_err(|_| Rejection::new(400, "truncated_body"))?;
    } else {
        let mut remaining = length - body.len();
        let mut scratch = [0u8; 4096];
        while remaining > 0 {
            let n = remaining.min(scratch.len());
            stream
                .read_exact(&mut scratch[..n])
                .await
                .map_err(|_| Rejection::new(400, "truncated_body"))?;
            remaining -= n;
        }
        body.clear();
    }
    Ok((head, body))
}
