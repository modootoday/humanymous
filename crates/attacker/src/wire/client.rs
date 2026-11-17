//! Loopback-only HTTP/1.1 client for the owned-lab attack families.
//!
//! Defense-only. This client refuses any non-loopback target, so the ported
//! redteam families can only ever exercise this deployment's own lab runtime.
//! It speaks plain HTTP/1.1 with caller-ordered headers so the header-order and
//! RIT families can present their exact wire shape. It carries no TLS/h2 control
//! (that is a recorded gap); its fingerprint is the HTTP/1.1 request plan.
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// A refusal from the loopback client. Carries no captured bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientError(pub &'static str);
impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for ClientError {}

/// A parsed HTTP/1.1 response from the owned runtime.
#[derive(Debug, Clone)]
pub struct Response {
    /// Actual HTTP status.
    pub status: u16,
    /// First `Set-Cookie` value (name=value), if any.
    pub cookie: Option<String>,
    /// `X-HM-Seed` rotated seed header, if present.
    pub rotated_seed: Option<String>,
    /// Response body bytes.
    pub body: Vec<u8>,
}
impl Response {
    /// Explicit JSON `verdict`, never inferred from the HTTP status.
    pub fn verdict(&self) -> Option<String> {
        serde_json::from_slice::<serde_json::Value>(&self.body)
            .ok()?
            .get("verdict")?
            .as_str()
            .map(str::to_owned)
    }
}

/// A loopback-restricted target. Construction fails for any non-loopback address.
pub struct LoopbackClient {
    address: SocketAddr,
    authority: String,
}

impl LoopbackClient {
    /// Bind to an owned loopback runtime. Refuses any routable address so no
    /// third-party target can ever be reached (defense-only constraint).
    pub fn new(address: SocketAddr) -> Result<Self, ClientError> {
        if !address.ip().is_loopback() {
            return Err(ClientError("attacker target must be loopback"));
        }
        Ok(Self {
            authority: address.to_string(),
            address,
        })
    }

    /// Send one HTTP/1.1 request with headers written in exactly the given
    /// order (not a map), so wire header order is caller-controlled.
    pub fn send(
        &self,
        method: &str,
        path: &str,
        ordered_headers: &[(String, String)],
        body: &[u8],
    ) -> Result<Response, ClientError> {
        if !self.address.ip().is_loopback() {
            return Err(ClientError("target is no longer loopback"));
        }
        let mut wire = format!("{method} {path} HTTP/1.1\r\nHost: {}\r\n", self.authority);
        for (name, value) in ordered_headers {
            wire.push_str(&format!("{name}: {value}\r\n"));
        }
        if !body.is_empty() {
            wire.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        wire.push_str("Connection: close\r\n\r\n");
        let mut bytes = wire.into_bytes();
        bytes.extend_from_slice(body);

        let timeout = Duration::from_secs(5);
        let mut stream = TcpStream::connect_timeout(&self.address, timeout)
            .map_err(|_| ClientError("connect"))?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|_| ClientError("write timeout"))?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|_| ClientError("read timeout"))?;
        stream.write_all(&bytes).map_err(|_| ClientError("write"))?;
        let mut raw = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = stream.read(&mut buf).map_err(|_| ClientError("read"))?;
            if n == 0 {
                break;
            }
            raw.extend_from_slice(&buf[..n]);
            if raw.len() > 262_144 {
                return Err(ClientError("response budget"));
            }
        }
        parse(&raw)
    }

    /// Bootstrap a session: GET /api/session and capture the cookie and the RIT
    /// seed/counter the server issues (the Go cookie-session bootstrap).
    pub fn session(
        &self,
        ordered_headers: &[(String, String)],
    ) -> Result<(String, Vec<u8>, u64), ClientError> {
        let response = self.send("GET", "/api/session", ordered_headers, &[])?;
        let cookie = response
            .cookie
            .ok_or(ClientError("session issued no cookie"))?;
        let parsed: serde_json::Value =
            serde_json::from_slice(&response.body).map_err(|_| ClientError("session body"))?;
        let seed = parsed
            .get("ritSeed")
            .and_then(|v| v.as_str())
            .map(decode_seed)
            .transpose()?
            .unwrap_or_default();
        let n = parsed
            .get("ritN")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        Ok((cookie, seed, n))
    }
}

fn decode_seed(value: &str) -> Result<Vec<u8>, ClientError> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| ClientError("seed encoding"))
}

fn parse(raw: &[u8]) -> Result<Response, ClientError> {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or(ClientError("response framing"))?;
    let head = std::str::from_utf8(&raw[..split]).map_err(|_| ClientError("response head"))?;
    let mut lines = head.split("\r\n");
    let status_line = lines.next().ok_or(ClientError("status line"))?;
    let status = status_line
        .split(' ')
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or(ClientError("status code"))?;
    let mut cookie = None;
    let mut rotated_seed = None;
    let mut content_length = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("set-cookie") && cookie.is_none() {
            cookie = Some(value.split(';').next().unwrap_or(value).to_owned());
        }
        if name.eq_ignore_ascii_case("x-hm-seed") {
            rotated_seed = Some(value.to_owned());
        }
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value.parse::<usize>().ok();
        }
    }
    // Honor Content-Length so a close-delimited read never leaves trailing bytes
    // on the parsed JSON body.
    let rest = &raw[split + 4..];
    let body = match content_length {
        Some(length) if length <= rest.len() => rest[..length].to_vec(),
        _ => rest.to_vec(),
    };
    Ok(Response {
        status,
        cookie,
        rotated_seed,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn non_loopback_targets_are_refused() {
        let routable = SocketAddr::from((Ipv4Addr::new(93, 184, 216, 34), 443));
        assert!(LoopbackClient::new(routable).is_err());
        let loopback = SocketAddr::from((Ipv4Addr::LOCALHOST, 8443));
        assert!(LoopbackClient::new(loopback).is_ok());
    }

    #[test]
    fn response_parser_extracts_status_cookie_seed_and_verdict() {
        // Content-Length 19 truncates the trailing 'x' from the close-delimited read.
        let raw = b"HTTP/1.1 200 OK\r\nSet-Cookie: hm=abc; Path=/\r\nX-HM-Seed: seed123\r\nContent-Length: 19\r\n\r\n{\"verdict\":\"ALLOW\"}x";
        let response = parse(raw).unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.cookie.as_deref(), Some("hm=abc"));
        assert_eq!(response.rotated_seed.as_deref(), Some("seed123"));
        assert_eq!(response.verdict().as_deref(), Some("ALLOW"));
    }
}
