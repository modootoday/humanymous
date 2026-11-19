//! Strict bounded single-frame native HTTP/1.1. No normalized proxy provenance.
use crate::gateway::{Rejection, configuration};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Maximum request entity size for this complete-body profile.
pub const MAX_BODY: usize = 65_536;
const MAX_HEADER: usize = 16_384;
/// Complete origin frame cap, including chunk framing overhead and response head.
pub(in crate::gateway) const MAX_ORIGIN_FRAME: usize = MAX_BODY * 2 + MAX_HEADER;
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
            == Some(humanymous_core::integrity::request::browser::PROTOCOL)
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
pub(in crate::gateway) async fn write_headers(
    stream: &mut (impl AsyncWrite + Unpin),
    status: u16,
    body: &[u8],
    extra: &[(String, String)],
) -> std::io::Result<()> {
    let reason = reason(status);
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

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        409 => "Conflict",
        413 => "Content Too Large",
        422 => "Unprocessable Content",
        429 => "Too Many Requests",
        _ => "Service Unavailable",
    }
}

/// Edge gate refusal in Go `http.Error` form: `text/plain; charset=utf-8`,
/// `X-Content-Type-Options: nosniff` and the static body plus a newline,
/// followed by the gate's fixed fields (incident pointer, Retry-After). A HEAD
/// request gets the same head and Content-Length without the body, as Go's
/// server wrote it.
pub(in crate::gateway) async fn write_text(
    stream: &mut (impl AsyncWrite + Unpin),
    status: u16,
    body: &str,
    fields: &[(&'static str, String)],
    head: bool,
) -> std::io::Result<()> {
    stream
        .write_all(&text_frame(status, body, fields, head)?)
        .await?;
    stream.shutdown().await
}

/// Gateway probe answer: the fixed JSON body with the local response head
/// (`application/json`, close, no-store). HEAD keeps Content-Length and
/// omits the body, as Go's server did for its control-plane probes.
pub(in crate::gateway) fn probe_frame(status: u16, body: &str, head: bool) -> Vec<u8> {
    let mut bytes = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n",
        reason(status),
        body.len()
    )
    .into_bytes();
    if !head {
        bytes.extend_from_slice(body.as_bytes());
    }
    bytes
}

/// Encoded [`write_text`] frame; field values are printable ASCII only.
pub(in crate::gateway) fn text_frame(
    status: u16,
    body: &str,
    fields: &[(&'static str, String)],
    head: bool,
) -> std::io::Result<Vec<u8>> {
    let mut header = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: text/plain; charset=utf-8\r\nX-Content-Type-Options: nosniff\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n",
        reason(status),
        body.len() + 1
    );
    for (name, value) in fields {
        if !name.bytes().all(token) || value.bytes().any(|b| !(32..127).contains(&b)) {
            return Err(std::io::Error::other("invalid gate response field"));
        }
        header.push_str(&format!("{name}: {value}\r\n"));
    }
    header.push_str("\r\n");
    let mut bytes = header.into_bytes();
    if !head {
        bytes.extend_from_slice(body.as_bytes());
        bytes.push(b'\n');
    }
    Ok(bytes)
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
/// Every internal trust header is dropped; `origin_auth`, when cloaking is
/// enabled, is then attached as the one `X-Hmny-Origin-Auth` field (legacy
/// gate.go `Rewrite`, SoT-23 §1, HR-24).
pub fn origin_request(request: &Request, authority: &str, origin_auth: Option<&str>) -> Vec<u8> {
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
            || crate::gateway::guard::is_internal(name)
            || lower == "authorization"
                && !matches!(
                    crate::gateway::browser::profile(request),
                    Ok(configuration::RitProfile::BrowserAuthOnly)
                )
        {
            continue;
        }
        bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    if let Some(token) = origin_auth {
        bytes.extend_from_slice(
            format!("{}: {token}\r\n", crate::gateway::guard::ORIGIN_HEADER).as_bytes(),
        );
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(&request.body);
    bytes
}

/// Validate a bounded opaque origin response without coalescing Set-Cookie.
/// Chunk extensions and nonempty trailers remain outside this complete-body profile.
pub fn response(bytes: &[u8], head: bool) -> Result<(), Rejection> {
    Response::parse(bytes, head).map(|_| ())
}

mod origin_response;
pub(in crate::gateway) mod stream;
pub(in crate::gateway) use origin_response::Response;

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
    /// Every legacy Go `smuggleScan` case (internal/gate round3_test.go and
    /// smuggle_wargame_test.go at baseline 981d25c) replayed as wire bytes.
    /// Go's single "chunked" Transfer-Encoding passes its scan; this profile
    /// refuses every Transfer-Encoding, which is stricter, never looser.
    #[test]
    fn legacy_go_smuggle_scan_cases_are_refused_on_the_wire() {
        const HEAD: &str = "POST / HTTP/1.1\r\nHost: app\r\n";
        let clean = format!("{HEAD}Content-Length: 5");
        assert!(parse_headers(clean.as_bytes()).is_ok(), "clean");
        for (name, fields) in [
            ("cl+te", "Content-Length: 5\r\nTransfer-Encoding: chunked"),
            ("dup-cl", "Content-Length: 5\r\nContent-Length: 6"),
            ("cl-identical-dup", "Content-Length: 5\r\nContent-Length: 5"),
            ("te-not-chunked", "Transfer-Encoding: chunked, chunked"),
            (
                "te-multi-chunked",
                "Transfer-Encoding: chunked\r\nTransfer-Encoding: chunked",
            ),
            ("te-empty", "Transfer-Encoding: "),
            ("te-identity", "Transfer-Encoding: identity"),
            ("te-gzip-chunked", "Transfer-Encoding: gzip, chunked"),
            ("te-chunked-alone", "Transfer-Encoding: chunked"),
            (
                "obs-fold",
                "Content-Length: 0\r\nX-Custom: ok\r\n Injected: yes",
            ),
            (
                "obs-fold-tab",
                "Content-Length: 0\r\nX-Custom: ok\r\n\tInjected: yes",
            ),
            ("obs-fold-lf-only", "Content-Length: 0\r\nX-Foo: a\nb"),
            ("bare-cr", "Content-Length: 0\r\nX-Foo: a\rb"),
        ] {
            let frame = format!("{HEAD}{fields}");
            let refusal = parse_headers(frame.as_bytes()).err();
            assert_eq!(refusal, Some(Rejection::new(400, "http_framing")), "{name}");
        }
    }
    /// Legacy cmd/server/h1capture.go peeked lowercased header names in wire
    /// order before net/http lost it; this parser keeps order and casing itself.
    #[test]
    fn header_wire_order_and_casing_are_retained() {
        let frame = "GET /a?b=1 HTTP/1.1\r\nUser-Agent: x\r\nhost: app\r\nAccept-Language: en\r\nACCEPT: */*\r\nContent-Length: 0";
        let (request, length) = parse_headers(frame.as_bytes()).unwrap();
        assert_eq!(length, 0);
        let names: Vec<&str> = request.headers.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            [
                "User-Agent",
                "host",
                "Accept-Language",
                "ACCEPT",
                "Content-Length"
            ]
        );
        let peeked: Vec<String> = names.iter().map(|n| n.to_ascii_lowercase()).collect();
        assert_eq!(
            peeked,
            [
                "user-agent",
                "host",
                "accept-language",
                "accept",
                "content-length"
            ]
        );
        assert_eq!(request.header("Accept"), Some("*/*"));
        assert_eq!(request.target, "/a?b=1");
    }
    #[test]
    fn origin_request_drops_forged_trust_headers() {
        let frame = "GET / HTTP/1.1\r\nHost: app\r\nX-Forwarded-For: 1.2.3.4\r\nforwarded: for=5.6.7.8\r\nX-Real-IP: 9.9.9.9\r\nCF-Connecting-IP: 1.1.1.1\r\nX-Hmny-Origin-Auth: forged\r\nX-Hmn-Verdict: ALLOW\r\nAccept: */*\r\nContent-Length: 0";
        let (request, _) = parse_headers(frame.as_bytes()).unwrap();
        let text = String::from_utf8(origin_request(&request, "origin", None)).unwrap();
        assert_eq!(
            text,
            "GET / HTTP/1.1\r\nHost: origin\r\nContent-Length: 0\r\nConnection: close\r\nAccept: */*\r\n\r\n"
        );
        // Cloaking replaces the forged value with exactly one gateway token.
        let text = String::from_utf8(origin_request(&request, "origin", Some("t0k"))).unwrap();
        assert_eq!(
            text,
            "GET / HTTP/1.1\r\nHost: origin\r\nContent-Length: 0\r\nConnection: close\r\nAccept: */*\r\nX-Hmny-Origin-Auth: t0k\r\n\r\n"
        );
    }
    #[test]
    fn gate_text_frame_matches_go_http_error_and_refuses_unsafe_fields() {
        let head = "HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain; charset=utf-8\r\nX-Content-Type-Options: nosniff\r\nContent-Length: 8\r\nConnection: close\r\nCache-Control: no-store\r\nRetry-After: 5\r\n\r\n";
        let bytes = text_frame(403, "blocked", &[("Retry-After", "5".into())], false).unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            format!("{head}blocked\n")
        );
        // HEAD: same head and Content-Length, no body bytes.
        let bytes = text_frame(403, "blocked", &[("Retry-After", "5".into())], true).unwrap();
        assert_eq!(String::from_utf8(bytes).unwrap(), head);
        assert!(text_frame(403, "x", &[("Retry-After", "1\r\nX: y".into())], false).is_err());
        assert!(text_frame(403, "x", &[("Bad Name", "1".into())], false).is_err());
    }
    #[test]
    fn probe_frame_keeps_content_length_and_drops_the_body_for_head() {
        let body = "{\"plane\":\"gate\",\"status\":\"ok\"}\n";
        let head = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 31\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n";
        assert_eq!(
            probe_frame(200, body, false),
            format!("{head}{body}").into_bytes()
        );
        assert_eq!(probe_frame(200, body, true), head.as_bytes());
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
