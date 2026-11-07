//! Strict single-request HTTP framing over authenticated local IPC.
mod framing;
use crate::Rejection;
#[cfg(test)]
use framing::MAX_HEADER;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
/// Maximum complete service-envelope bytes.
pub const MAX_BODY: usize = 1_600_000;
/// An owned local request. Values never establish native peer or administrative scope.
pub struct Request {
    /// Exact HTTP method.
    pub method: String,
    /// Exact origin-form service target, without URL decoding.
    pub target: String,
    /// Ordered service headers, including ordinary duplicates for handler policy.
    pub headers: Vec<(String, String)>,
    /// Service bearer, never an original application's Authorization observation.
    pub authorization: String,
    /// Complete entity bytes; handler owns schema and endpoint authorization.
    pub body: Vec<u8>,
}
/// Strict legacy decision endpoint; no administration dispatch.
pub async fn read(stream: &mut (impl AsyncRead + Unpin)) -> Result<Request, Rejection> {
    let request = read_local(stream).await?;
    if request.method != "POST" || request.target != "/adapter/v0.1/decision" {
        return Err(Rejection::new(403, "decision_endpoint_only"));
    }
    if request.authorization.is_empty() {
        return Err(Rejection::new(401, "credentials"));
    }
    Ok(request)
}
/// Common complete JSON framing for a separately authenticated local handler.
pub async fn read_local(stream: &mut (impl AsyncRead + Unpin)) -> Result<Request, Rejection> {
    let (head, body) = framing::read(stream, MAX_BODY, true).await?;
    let parts: Vec<_> = head.start.split(' ').collect();
    if parts.len() != 3
        || parts[2] != "HTTP/1.1"
        || parts[0].is_empty()
        || parts[0].len() > 32
        || !parts[0]
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b"!#$%&'*+-.^_`|~0123456789".contains(&b))
        || !parts[1].starts_with('/')
        || parts[1].starts_with("//")
        || parts[1].len() > 8192
        || parts[1]
            .bytes()
            .any(|b| b <= 32 || b >= 127 || b"#\\".contains(&b))
    {
        return Err(Rejection::new(400, "http_framing"));
    }
    let host = head
        .header("host")
        .ok_or(Rejection::new(400, "http_host"))?;
    if host.is_empty()
        || !host
            .bytes()
            .all(|b| (33..127).contains(&b) && !b"@/#?\\,".contains(&b))
    {
        return Err(Rejection::new(400, "http_host"));
    }
    let authorization = head.header("authorization").unwrap_or("").to_owned();
    if authorization.len() > 263 {
        return Err(Rejection::new(401, "credentials"));
    }
    Ok(Request {
        method: parts[0].into(),
        target: parts[1].into(),
        authorization,
        headers: head.headers,
        body,
    })
}
/// Scratch-only drain on overload, under the caller's rejection deadline.
pub async fn discard(stream: &mut (impl AsyncRead + Unpin)) -> Result<(), Rejection> {
    framing::read(stream, MAX_BODY, false).await.map(|_| ())
}
/// Read one bounded JSON response without requiring EOF after its exact entity.
/// The caller closes immediately; buffered extra bytes are rejected and no second response is interpreted.
pub async fn read_response(stream: &mut (impl AsyncRead + Unpin)) -> Result<Response, Rejection> {
    let (head, body) = framing::read(stream, 65_536, true).await?;
    let parts: Vec<_> = head.start.splitn(3, ' ').collect();
    if parts.len() != 3
        || parts[0] != "HTTP/1.1"
        || parts[1].len() != 3
        || !parts[1].bytes().all(|b| b.is_ascii_digit())
        || !head
            .header("connection")
            .is_some_and(|s| s.eq_ignore_ascii_case("close"))
    {
        return Err(Rejection::new(503, "authority_framing"));
    }
    let status = parts[1]
        .parse()
        .map_err(|_| Rejection::new(503, "authority_framing"))?;
    Ok(Response {
        status,
        headers: head.headers,
        body,
    })
}
/// One observed bounded response; no header is forwarded to an application.
pub struct Response {
    /// Actual HTTP status.
    pub status: u16,
    /// Ordered validated headers.
    pub headers: Vec<(String, String)>,
    /// Exact complete JSON entity bytes.
    pub body: Vec<u8>,
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
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        413 => "Content Too Large",
        422 => "Unprocessable Content",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
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
