//! Core HTTP/1 request and response framing (the parts of Go `net/http`
//! `ReadRequest`, `url.ParseRequestURI`, `URL.Query` and `response.finish`
//! the legacy cmd/server handlers observed, Go 1.26.3 oracle at 981d25c).
//!
//! One request is read per connection and the connection is closed after the
//! response (runtime-application SoT). Deliberate differences from Go, each
//! pinned by tests:
//! - Only `Content-Length` framing is accepted; any `Transfer-Encoding` is a
//!   400 (owner decision 2026-11-16).
//! - The request head is limited to [`MAX_HEAD`] bytes (431 above it); Go
//!   allowed 1 MiB.
//! - Header values and decoded paths must be UTF-8; Go kept arbitrary bytes.
//! - Keep-alive, `Expect: 100-continue` and HTTP/2 serving are not offered.
use crate::network::http::PEEK_LIMIT;
use std::collections::BTreeMap;
use std::io::{self, Read, Write};

/// Largest accepted request head, matching the wire-order peek bound.
pub const MAX_HEAD: usize = PEEK_LIMIT;

/// One parsed HTTP/1 request as the Core handlers see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Request method token.
    pub method: String,
    /// Raw request target, as sent.
    pub target: String,
    /// HTTP/1 minor version (0 or 1).
    pub minor: u8,
    /// Header fields in wire order, including `Host`.
    pub headers: Vec<(String, String)>,
    /// Body bytes read (at most the route's limit plus one).
    pub body: Vec<u8>,
    /// Transport peer as `host:port`.
    pub remote_addr: String,
}

/// Why a request could not be framed; the status the transport answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refused(pub u16);

impl Request {
    /// A complete HTTP/1.1 request, for in-process callers and tests.
    pub fn new(
        method: &str,
        target: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        remote_addr: &str,
    ) -> Self {
        Self {
            method: method.to_owned(),
            target: target.to_owned(),
            minor: 1,
            headers,
            body,
            remote_addr: remote_addr.to_owned(),
        }
    }

    /// Go `Request.Host`.
    pub fn host(&self) -> &str {
        crate::network::http::first_value(&self.headers, "Host")
    }

    /// Header fields a Go handler saw: `Host` is promoted out of the map.
    pub fn handler_fields(&self) -> Vec<(String, String)> {
        self.headers
            .iter()
            .filter(|(name, _)| !name.eq_ignore_ascii_case("host"))
            .cloned()
            .collect()
    }

    /// Escaped path and raw query of an origin- or absolute-form target.
    pub(super) fn split_target(&self) -> Option<(&str, &str)> {
        let mut rest = self.target.as_str();
        for scheme in ["http://", "https://"] {
            if rest.len() >= scheme.len() && rest[..scheme.len()].eq_ignore_ascii_case(scheme) {
                let after = &rest[scheme.len()..];
                rest = after.find('/').map_or("/", |at| &after[at..]);
                break;
            }
        }
        if !rest.starts_with('/') {
            return None;
        }
        let rest = rest.split_once('#').map_or(rest, |(path, _)| path);
        Some(rest.split_once('?').unwrap_or((rest, "")))
    }

    /// Go `URL.Path`: the percent-decoded path.
    pub(super) fn path(&self) -> Option<String> {
        let (escaped, _) = self.split_target()?;
        unescape(escaped, false)
    }

    /// Go `URL.Query().Get(key)`.
    pub(super) fn query(&self, key: &str) -> String {
        self.split_target()
            .map(|(_, raw)| query_value(raw, key))
            .unwrap_or_default()
    }
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Go `url.unescape` in path (`plus = false`) or query component mode
/// (`plus = true`). `None` for an invalid escape or non-UTF-8 result.
pub(super) fn unescape(text: &str, plus: bool) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'%' => {
                let high = hex(*bytes.get(at + 1)?)?;
                let low = hex(*bytes.get(at + 2)?)?;
                out.push(high << 4 | low);
                at += 3;
            }
            b'+' if plus => {
                out.push(b' ');
                at += 1;
            }
            other => {
                out.push(other);
                at += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// Go `url.shouldEscape(byte, encodePath)`.
fn path_escapes(byte: u8) -> bool {
    !(byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'-' | b'_'
                | b'.'
                | b'~'
                | b'$'
                | b'&'
                | b'+'
                | b','
                | b'/'
                | b':'
                | b';'
                | b'='
                | b'@'
        ))
}

/// Go `url.escape(text, encodePath)`: what `url.URL{Path: text}.String()`
/// writes for a rooted path. A `%` is escaped too, as Go does.
pub(super) fn escape_path(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for &byte in text.as_bytes() {
        if path_escapes(byte) {
            out.push_str(&format!("%{byte:02X}"));
        } else {
            out.push(char::from(byte));
        }
    }
    out
}

/// Go `URL.EscapedPath` of a request target parsed from `raw`, whose
/// decoded form is `decoded`: `raw` when it is validly encoded, otherwise
/// the canonical escaping of `decoded` (so a literal `\` becomes `%5C`).
pub(super) fn escaped_path(raw: &str, decoded: &str) -> String {
    let valid = raw.bytes().all(|byte| {
        matches!(
            byte,
            b'!' | b'$'
                | b'&'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'+'
                | b','
                | b';'
                | b'='
                | b':'
                | b'@'
                | b'['
                | b']'
                | b'%'
        ) || !path_escapes(byte)
    });
    if valid {
        raw.to_owned()
    } else {
        escape_path(decoded)
    }
}

/// Go `url.ParseQuery(raw)` then `Get(key)`: the first value of `key`;
/// pairs with a semicolon or an invalid escape are skipped.
pub(super) fn query_value(raw: &str, key: &str) -> String {
    raw.split('&')
        .filter(|pair| !pair.is_empty() && !pair.contains(';'))
        .filter_map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            Some((unescape(name, true)?, unescape(value, true)?))
        })
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
        .unwrap_or_default()
}

/// Go `path.Clean`.
pub(super) fn clean(path: &str) -> String {
    let rooted = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|last| *last != "..") {
                    parts.pop();
                } else if !rooted {
                    parts.push("..");
                }
            }
            _ => parts.push(part),
        }
    }
    let joined = parts.join("/");
    match (rooted, joined.is_empty()) {
        (true, _) => format!("/{joined}"),
        (false, true) => ".".to_owned(),
        (false, false) => joined,
    }
}

/// Go `cleanPath` used by `ServeMux`: rooted `path.Clean` that keeps a
/// trailing slash.
pub(super) fn clean_mux_path(path: &str) -> String {
    if path.is_empty() {
        return "/".to_owned();
    }
    let rooted = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    };
    let mut cleaned = clean(&rooted);
    if rooted.ends_with('/') && cleaned != "/" {
        cleaned.push('/');
    }
    cleaned
}

fn token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

/// Parse a request head (everything before the blank line).
fn parse_head(head: &[u8], remote_addr: &str) -> Result<(Request, usize), Refused> {
    let bad = Refused(400);
    let text = std::str::from_utf8(head).map_err(|_| bad)?;
    let mut lines = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line));
    let line = lines.next().ok_or(bad)?;
    let mut parts = line.splitn(3, ' ');
    let (method, target, proto) = (
        parts.next().ok_or(bad)?,
        parts.next().ok_or(bad)?,
        parts.next().ok_or(bad)?,
    );
    if method.is_empty() || !method.bytes().all(token) || target.is_empty() {
        return Err(bad);
    }
    let minor = match proto {
        "HTTP/1.1" => 1,
        "HTTP/1.0" => 0,
        _ => return Err(Refused(505)),
    };
    let mut headers = Vec::new();
    for line in lines {
        if line.starts_with([' ', '\t']) {
            return Err(bad); // obsolete line folding
        }
        let (name, value) = line.split_once(':').ok_or(bad)?;
        if name.is_empty() || !name.bytes().all(token) {
            return Err(bad);
        }
        let value = value.trim_matches([' ', '\t']);
        if value.bytes().any(|b| b < 0x20 && b != b'\t' || b == 0x7f) {
            return Err(bad);
        }
        headers.push((name.to_owned(), value.to_owned()));
    }
    let request = Request {
        method: method.to_owned(),
        target: target.to_owned(),
        minor,
        headers,
        body: Vec::new(),
        remote_addr: remote_addr.to_owned(),
    };
    if request.split_target().is_none() || request.path().is_none() {
        return Err(bad);
    }
    if minor == 1
        && !request
            .headers
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case("host"))
    {
        return Err(bad);
    }
    let mut length: Option<u64> = None;
    for (name, value) in &request.headers {
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err(bad);
        }
        if name.eq_ignore_ascii_case("content-length") {
            if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad);
            }
            let parsed = value.parse::<u64>().map_err(|_| bad)?;
            if length.is_some_and(|seen| seen != parsed) {
                return Err(bad);
            }
            length = Some(parsed);
        }
    }
    let length = usize::try_from(length.unwrap_or(0)).map_err(|_| bad)?;
    Ok((request, length))
}

/// Read exactly one request. `body_limit` sees the parsed head and returns how
/// many body bytes the route reads (Go's per-handler `MaxBytesReader`); one
/// extra byte is read so an oversize body is observable to the handler.
pub fn read_request(
    stream: &mut impl Read,
    remote_addr: &str,
    body_limit: impl FnOnce(&Request) -> usize,
) -> Result<Request, Refused> {
    let mut bytes = Vec::with_capacity(4096);
    let end = loop {
        if let Some(at) = find_head_end(&bytes) {
            break at;
        }
        if bytes.len() >= MAX_HEAD {
            return Err(Refused(431));
        }
        let mut chunk = [0u8; 4096];
        let room = (MAX_HEAD + 4 - bytes.len()).min(chunk.len());
        match stream.read(&mut chunk[..room]) {
            Ok(0) => return Err(Refused(400)),
            Ok(n) => bytes.extend_from_slice(&chunk[..n]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return Err(Refused(400)),
        }
    };
    let (head_len, separator) = end;
    if head_len > MAX_HEAD {
        return Err(Refused(431));
    }
    let (mut request, length) = parse_head(&bytes[..head_len], remote_addr)?;
    let wanted = length.min(body_limit(&request).saturating_add(1));
    let mut body = bytes.split_off(head_len + separator);
    body.truncate(wanted);
    while body.len() < wanted {
        let mut chunk = vec![0u8; (wanted - body.len()).min(64 * 1024)];
        match stream.read(&mut chunk) {
            Ok(0) => return Err(Refused(400)),
            Ok(n) => body.extend_from_slice(&chunk[..n]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return Err(Refused(400)),
        }
    }
    request.body = body;
    Ok(request)
}

// Offset of the blank line ending the head and the separator length.
fn find_head_end(bytes: &[u8]) -> Option<(usize, usize)> {
    let mut at = 0;
    while let Some(found) = bytes[at..].iter().position(|&b| b == b'\n') {
        let newline = at + found;
        let next = &bytes[newline + 1..];
        if next.starts_with(b"\r\n") {
            return Some((newline, 3));
        }
        if next.starts_with(b"\n") {
            return Some((newline, 2));
        }
        at = newline + 1;
    }
    None
}

/// A finished response ready for the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// HTTP status.
    pub status: u16,
    /// Canonical header name to values, written in sorted order as Go does.
    pub headers: BTreeMap<String, Vec<String>>,
    /// Body bytes (suppressed on the wire for HEAD, 204 and 304).
    pub body: Vec<u8>,
}

impl Response {
    /// First value of a header.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&crate::network::http::canonical_name(name))
            .and_then(|values| values.first())
            .map(String::as_str)
    }
}

/// Go `http.StatusText` for the statuses this server writes.
pub(super) fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        301 => "Moved Permanently",
        307 => "Temporary Redirect",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        431 => "Request Header Fields Too Large",
        451 => "Unavailable For Legal Reasons",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        505 => "HTTP Version Not Supported",
        _ => "",
    }
}

fn body_allowed(status: u16) -> bool {
    !(100..200).contains(&status) && status != 204 && status != 304
}

/// Serialize a response the way Go's server finished a handler that wrote a
/// complete small body: sorted headers with `Content-Length`, `Date` and
/// `Connection: close`; no body for HEAD.
pub fn encode(response: &Response, head: bool, date: &str) -> Vec<u8> {
    let mut headers = response.headers.clone();
    if body_allowed(response.status) {
        headers
            .entry("Content-Length".to_owned())
            .or_insert_with(|| vec![response.body.len().to_string()]);
    } else {
        headers.remove("Content-Length");
    }
    headers.insert("Date".to_owned(), vec![date.to_owned()]);
    headers.insert("Connection".to_owned(), vec!["close".to_owned()]);
    let mut out = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status,
        reason(response.status)
    )
    .into_bytes();
    for (name, values) in &headers {
        for value in values {
            let value: String = value
                .chars()
                .map(|c| if c == '\r' || c == '\n' { ' ' } else { c })
                .collect();
            out.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
        }
    }
    out.extend_from_slice(b"\r\n");
    if !head && body_allowed(response.status) {
        out.extend_from_slice(&response.body);
    }
    out
}

/// Write an encoded response and flush.
pub fn write_response(
    stream: &mut impl Write,
    response: &Response,
    head: bool,
    date: &str,
) -> io::Result<()> {
    stream.write_all(&encode(response, head, date))?;
    stream.flush()
}

/// Go `http.TimeFormat` of a Unix time.
pub(super) fn http_date(unix: i64) -> String {
    let at =
        time::OffsetDateTime::from_unix_timestamp(unix).unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    let weekday = &format!("{:?}", at.weekday())[..3];
    let month = &format!("{:?}", at.month())[..3];
    format!(
        "{weekday}, {:02} {month} {:04} {:02}:{:02}:{:02} GMT",
        at.day(),
        at.year(),
        at.hour(),
        at.minute(),
        at.second()
    )
}
