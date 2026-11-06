//! Strict single-request HTTP/1.1 framing over authenticated local IPC.
use crate::Rejection;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Header/body budget for the local service envelope, not origin entity bytes.
pub const MAX_BODY: usize = 1_600_000;
const MAX_HEADER: usize = 16_384;
/// Parsed service request; secrets intentionally have no Debug implementation.
pub struct Request {
    /// Exact service credential header (never an origin Authorization value).
    pub authorization: String,
    /// Still-untrusted JSON; the shared integrity parser owns its validation.
    pub body: Vec<u8>,
}
fn token(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}
fn headers(bytes: &[u8]) -> Result<(String, usize), Rejection> {
    let bad = Rejection::new(400, "http_framing");
    let text = std::str::from_utf8(bytes).map_err(|_| bad)?;
    let mut lines = text.split("\r\n");
    let start = lines.next().ok_or(bad)?;
    if start != "POST /adapter/v0.1/decision HTTP/1.1" {
        return Err(Rejection::new(403, "decision_endpoint_only"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut authorization = None;
    let mut length = None;
    let mut content_type = false;
    let mut host = false;
    for (i, line) in lines.enumerate() {
        if i >= 100 {
            return Err(bad);
        }
        let (name, value) = line.split_once(':').ok_or(bad)?;
        if name.is_empty()
            || !name.bytes().all(token)
            || value
                .bytes()
                .any(|b| b < 32 && b != b'\t' || b == 127 || b >= 128)
        {
            return Err(bad);
        }
        let name = name.to_ascii_lowercase();
        // Reject all repeated service-envelope headers, even innocuous ones.
        if !seen.insert(name.clone()) {
            return Err(bad);
        }
        let value = value.trim_matches([' ', '\t']);
        match name.as_str() {
            "authorization" => {
                if value.len() > 263 {
                    return Err(Rejection::new(401, "credentials"));
                }
                authorization = Some(value.to_owned());
            }
            "content-length" => {
                if value.is_empty()
                    || value.len() > 7
                    || !value.bytes().all(|b| b.is_ascii_digit())
                    || (value.len() > 1 && value.starts_with('0'))
                {
                    return Err(bad);
                }
                let n: usize = value.parse().map_err(|_| bad)?;
                if n > MAX_BODY {
                    return Err(Rejection::new(413, "envelope_size"));
                }
                length = Some(n);
            }
            "content-type" => {
                if !value.eq_ignore_ascii_case("application/json") {
                    return Err(bad);
                }
                content_type = true;
            }
            "host" => {
                if value.is_empty()
                    || !value
                        .bytes()
                        .all(|b| (33..=126).contains(&b) && !b"@/#?\\,".contains(&b))
                {
                    return Err(bad);
                }
                host = true;
            }
            "connection"
                if value.eq_ignore_ascii_case("close")
                    || value.eq_ignore_ascii_case("keep-alive") => {}
            "connection" | "transfer-encoding" | "upgrade" | "expect" | "trailer" | "te" => {
                return Err(bad);
            }
            _ => {}
        }
    }
    if !host || !content_type {
        return Err(bad);
    }
    Ok((
        authorization.ok_or(Rejection::new(401, "credentials"))?,
        length.ok_or(bad)?,
    ))
}
/// Consume a single bounded request. Caller supplies a whole-connection timeout.
/// Already-buffered trailing bytes are rejected; connection is always closed,
/// so late pipelining can never invoke a second decision.
pub async fn read(stream: &mut (impl AsyncRead + Unpin)) -> Result<Request, Rejection> {
    read_frame(stream, true).await
}

/// Drain one valid rejected frame using bounded scratch storage, so closing a
/// Unix stream with unread inbound bytes cannot reset a completed 429 response.
pub async fn discard(stream: &mut (impl AsyncRead + Unpin)) -> Result<(), Rejection> {
    read_frame(stream, false).await.map(|_| ())
}

async fn read_frame(
    stream: &mut (impl AsyncRead + Unpin),
    retain: bool,
) -> Result<Request, Rejection> {
    let mut buffer = Vec::with_capacity(4096);
    let end = loop {
        if let Some(i) = buffer.windows(4).position(|b| b == b"\r\n\r\n") {
            if i + 4 > MAX_HEADER {
                return Err(Rejection::new(413, "header_size"));
            }
            break i;
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
    let (authorization, length) = headers(&buffer[..end])?;
    let mut body = buffer.split_off(end + 4);
    if body.len() > length {
        return Err(Rejection::new(400, "trailing_frame"));
    }
    let got = body.len();
    if !retain {
        let mut remaining = length - got;
        let mut scratch = [0u8; 4096];
        while remaining > 0 {
            let chunk = remaining.min(scratch.len());
            stream
                .read_exact(&mut scratch[..chunk])
                .await
                .map_err(|_| Rejection::new(400, "truncated_body"))?;
            remaining -= chunk;
        }
        body.clear();
        return Ok(Request {
            authorization,
            body,
        });
    }
    body.resize(length, 0);
    stream
        .read_exact(&mut body[got..])
        .await
        .map_err(|_| Rejection::new(400, "truncated_body"))?;
    Ok(Request {
        authorization,
        body,
    })
}
/// Emit one bounded response with close/no-store semantics and no reflected data.
pub async fn write(
    stream: &mut (impl AsyncWrite + Unpin),
    status: u16,
    body: &[u8],
) -> std::io::Result<()> {
    if body.len() > 65_536 {
        return Err(std::io::Error::other("response exceeds budget"));
    }
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
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}
/// Bounded static errors only; never serialize rejected envelope contents.
pub fn error_body(error: Rejection) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"error": error.code})).expect("static error serializes")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(extra: &str, body: &str) -> Vec<u8> {
        format!("POST /adapter/v0.1/decision HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer x\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}\r\n{body}", body.len()).into_bytes()
    }
    #[tokio::test]
    async fn bounds_framing_and_single_endpoint() {
        let bytes = request("Connection: close\r\n", "{}");
        assert_eq!(read(&mut bytes.as_slice()).await.unwrap().body, b"{}");
        for bytes in [
            request("Transfer-Encoding: chunked\r\n", "{}"),
            request("Content-Length: 2\r\n", "{}"),
            request("Expect: 100-continue\r\n", "{}"),
            request(" X-Fold: value\r\n", "{}"),
            request("Connection: upgrade\r\n", "{}"),
            request("Bad : value\r\n", "{}"),
            request("X: value\nInjected: y\r\n", "{}"),
            request("", "{}")
                .iter()
                .copied()
                .chain(b"extra".iter().copied())
                .collect(),
            bytes.iter().copied().take(bytes.len() - 1).collect(),
            String::from_utf8(bytes)
                .unwrap()
                .replace("/adapter/v0.1/decision", "/admin/erase")
                .into_bytes(),
        ] {
            assert!(read(&mut bytes.as_slice()).await.is_err());
        }
        let oversized = b"POST /adapter/v0.1/decision HTTP/1.1\r\nHost: local\r\nAuthorization: Bearer x\r\nContent-Type: application/json\r\nContent-Length: 1600001\r\n\r\n";
        assert_eq!(
            read(&mut oversized.as_slice()).await.err().unwrap().status,
            413
        );
        assert!(
            read(&mut vec![b'a'; MAX_HEADER + 1].as_slice())
                .await
                .is_err()
        );
    }
}
