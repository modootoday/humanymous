use super::{LabError, OwnedRuntime, Request, now_ms};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use humanymous_core::integrity::request::{compute_token, seed};
use humanymous_core::signals::report::ClientReport;
use humanymous_runtime::gateway::{configuration::Configuration, session::canonical};
use serde::Deserialize;
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};
use zeroize::{Zeroize, Zeroizing};

/// Volatile issued capability. Intentionally neither Debug nor Serialize.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    session: Zeroizing<String>,
    key: Zeroizing<String>,
    counter: u64,
    expires: u64,
}
impl Session {
    /// Bind a synthetic report to the issued capability without exposing the key.
    pub fn bind_report(&self, report: &mut ClientReport) {
        report.session_id = self.session.to_string();
    }
}

/// Bounded response bytes; not serializable because session replies are secrets.
pub struct Response {
    /// Actual HTTP status, not a derived policy verdict.
    pub status: u16,
    /// Ordered headers, preserving duplicate Set-Cookie.
    pub headers: Vec<(String, String)>,
    /// Bounded raw body; callers must not log session issue replies.
    pub body: Vec<u8>,
}
impl Response {
    /// Explicit JSON policy verdict if present; never infers from HTTP status.
    pub fn verdict(&self) -> Option<String> {
        serde_json::from_slice::<serde_json::Value>(&self.body)
            .ok()?
            .get("verdict")?
            .as_str()
            .map(str::to_owned)
    }
}

fn request(config: &Configuration, target: &str, body: Vec<u8>) -> Request {
    Request {
        method: "POST".into(),
        target: target.into(),
        headers: vec![
            ("Host".into(), config.scope.authority.clone()),
            ("Content-Length".into(), body.len().to_string()),
            ("Content-Type".into(), "application/json".into()),
            ("sec-ch-ua".into(), "Chromium".into()),
            (
                "User-Agent".into(),
                "Mozilla/5.0 Chrome/126.0 Safari/537.36".into(),
            ),
            ("Accept".into(), "*/*".into()),
            ("Sec-Fetch-Site".into(), "same-origin".into()),
            ("Accept-Encoding".into(), "gzip, deflate, br, zstd".into()),
        ],
        body,
    }
}

fn sign(
    config: &Configuration,
    request: &mut Request,
    session: &Session,
    counter: u64,
    now: u64,
    deadline: u64,
) -> Result<(), LabError> {
    if counter == 0
        || request.headers.iter().any(|(n, _)| {
            n.eq_ignore_ascii_case("authorization")
                || n.to_ascii_lowercase().starts_with("x-humanymous-")
        })
    {
        return Err(LabError("request already signed or invalid counter"));
    }
    request.headers.extend([
        (
            "Authorization".into(),
            format!("Bearer {}", session.session.as_str()),
        ),
        ("x-humanymous-counter".into(), counter.to_string()),
        ("x-humanymous-bucket".into(), (now / 10000).to_string()),
        ("x-humanymous-deadline".into(), deadline.to_string()),
    ]);
    let key = Zeroizing::new(
        URL_SAFE_NO_PAD
            .decode(session.key.as_bytes())
            .map_err(|_| LabError("session key encoding"))?,
    );
    if key.len() != 32 {
        return Err(LabError("session key width"));
    }
    let text = Zeroizing::new(canonical(config, request, &session.session, deadline));
    request.headers.push((
        "x-humanymous-token".into(),
        compute_token(&seed(&key, counter - 1), &text, counter, now / 10000),
    ));
    Ok(())
}

fn encode(request: &Request) -> Result<Vec<u8>, LabError> {
    if request.body.len() > 65536 || request.headers.len() > 100 {
        return Err(LabError("request budget"));
    }
    let mut bytes = format!("{} {} HTTP/1.1\r\n", request.method, request.target).into_bytes();
    for (name, value) in &request.headers {
        bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    bytes.extend_from_slice(b"\r\n");
    if bytes.len() > 16384 {
        return Err(LabError("request header budget"));
    }
    bytes.extend_from_slice(&request.body);
    Ok(bytes)
}

fn parse(bytes: Vec<u8>) -> Result<Response, LabError> {
    let bad = LabError("response framing");
    let end = bytes.windows(4).position(|w| w == b"\r\n\r\n").ok_or(bad)?;
    if end + 4 > 16384 || bytes.len() > 81920 {
        return Err(bad);
    }
    let head = std::str::from_utf8(&bytes[..end]).map_err(|_| bad)?;
    let mut lines = head.split("\r\n");
    let first = lines.next().ok_or(bad)?;
    let mut parts = first.splitn(3, ' ');
    if parts.next() != Some("HTTP/1.1") {
        return Err(bad);
    }
    let status_text = parts.next().ok_or(bad)?;
    if status_text.len() != 3
        || !status_text.bytes().all(|b| b.is_ascii_digit())
        || parts.next().is_none()
    {
        return Err(bad);
    }
    let status = status_text.parse::<u16>().map_err(|_| bad)?;
    if !(200..600).contains(&status) {
        return Err(bad);
    }
    let mut headers = Vec::new();
    let mut length = None;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(bad)?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
            || value.bytes().any(|b| !(32..127).contains(&b))
            || headers.len() >= 100
        {
            return Err(bad);
        }
        let value = value.trim();
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err(bad);
        }
        if name.eq_ignore_ascii_case("content-length") {
            if length.is_some()
                || value.is_empty()
                || !value.bytes().all(|b| b.is_ascii_digit())
                || value.len() > 1 && value.starts_with('0')
            {
                return Err(bad);
            }
            length = Some(value.parse::<usize>().map_err(|_| bad)?);
        }
        headers.push((name.into(), value.into()));
    }
    if length != Some(bytes.len() - end - 4) || bytes.len() - end - 4 > 65536 {
        return Err(bad);
    }
    Ok(Response {
        status,
        headers,
        body: bytes[end + 4..].to_vec(),
    })
}

impl OwnedRuntime {
    /// Construct the fixed synthetic fixture header profile for this installation.
    pub fn request(&self, target: &str, body: Vec<u8>) -> Request {
        request(&self.config, target, body)
    }
    /// Sign exact current bytes with the production canonical owner. No retries.
    pub fn sign(
        &self,
        request: &mut Request,
        session: &Session,
        counter: u64,
    ) -> Result<(), LabError> {
        let now = now_ms()?;
        let deadline = now
            .checked_add(self.config.timeout_ms.saturating_sub(1).min(4000))
            .ok_or(LabError("deadline overflow"))?
            .min(session.expires);
        sign(&self.config, request, session, counter, now, deadline)
    }
    /// Deliberately prepare an expired request for a negative case.
    pub fn sign_expired(
        &self,
        request: &mut Request,
        session: &Session,
        counter: u64,
    ) -> Result<(), LabError> {
        let now = now_ms()?;
        sign(
            &self.config,
            request,
            session,
            counter,
            now,
            now.saturating_sub(1),
        )
    }
    /// Request a fresh volatile session from the owned child; reply is not logged.
    pub fn issue(&mut self) -> Result<Session, LabError> {
        let mut response = self.exchange(&self.request("/_humanymous/session", Vec::new()))?;
        if response.status != 200 {
            return Err(LabError("session issue refused"));
        }
        let session: Session =
            serde_json::from_slice(&response.body).map_err(|_| LabError("session issue reply"))?;
        response.body.zeroize();
        let key = Zeroizing::new(
            URL_SAFE_NO_PAD
                .decode(session.key.as_bytes())
                .map_err(|_| LabError("session key encoding"))?,
        );
        if session.session.len() != 43
            || key.len() != 32
            || session.counter != 0
            || session.expires <= now_ms()?
        {
            return Err(LabError("session issue binding"));
        }
        Ok(session)
    }
    /// One exact framed request against only the owned runtime listener.
    pub fn exchange(&mut self, request: &Request) -> Result<Response, LabError> {
        self.exchange_raw(&encode(request)?)
    }
    /// Bounded malformed-frame testing against the same owned child, never a URL.
    pub fn exchange_raw(&mut self, bytes: &[u8]) -> Result<Response, LabError> {
        self.check_live()?;
        if bytes.len() > 81920 {
            return Err(LabError("wire request budget"));
        }
        let until = Instant::now() + Duration::from_millis(self.config.timeout_ms + 1000);
        let remaining = || {
            until
                .checked_duration_since(Instant::now())
                .filter(|d| !d.is_zero())
                .ok_or(LabError("exchange deadline"))
        };
        let mut stream = TcpStream::connect_timeout(&self.address, remaining()?)
            .map_err(|_| LabError("owned child connect"))?;
        if stream
            .peer_addr()
            .map_err(|_| LabError("owned child peer"))?
            != self.address
        {
            return Err(LabError("owned child peer mismatch"));
        }
        let mut pending = bytes;
        while !pending.is_empty() {
            stream
                .set_write_timeout(Some(remaining()?))
                .map_err(|_| LabError("write timeout"))?;
            let n = stream
                .write(pending)
                .map_err(|_| LabError("request write"))?;
            if n == 0 {
                return Err(LabError("request write closed"));
            }
            pending = &pending[n..];
        }
        let mut response = Vec::new();
        let mut part = [0; 4096];
        loop {
            stream
                .set_read_timeout(Some(remaining()?))
                .map_err(|_| LabError("read timeout"))?;
            let n = stream
                .read(&mut part)
                .map_err(|_| LabError("response read"))?;
            if n == 0 {
                break;
            }
            if response.len() + n > 81920 {
                return Err(LabError("response budget"));
            }
            response.extend_from_slice(&part[..n]);
        }
        parse(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_shared_verifier_accepts_only_the_signed_bytes() {
        let config = super::super::config();
        let session = Session {
            session: Zeroizing::new("s".repeat(43)),
            key: Zeroizing::new(URL_SAFE_NO_PAD.encode([7; 32])),
            counter: 0,
            expires: 10000,
        };
        let mut req = request(&config, "/effect", b"effect".to_vec());
        sign(&config, &mut req, &session, 1, 1000, 5000).unwrap();
        let verify = |req: &Request| {
            humanymous_core::integrity::request::verify(
                humanymous_core::integrity::request::VerifyParams {
                    session_key: &[7; 32],
                    last_n: 0,
                    presented_n: 1,
                    presented_tb: 0,
                    current_tb: 0,
                    token: req.header("x-humanymous-token").unwrap(),
                    canonical: &canonical(&config, req, &session.session, 5000),
                    first_req: false,
                },
            )
            .id
        };
        assert_eq!(verify(&req), "l5.rit.ok");
        req.body[0] = b'X';
        assert_ne!(verify(&req), "l5.rit.ok");
        assert!(sign(&config, &mut req, &session, 2, 1000, 5000).is_err());
    }
    #[test]
    fn response_parser_preserves_cookie_duplicates_and_refuses_ambiguity() {
        let good =
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\n\r\nok";
        let response = parse(good.to_vec()).unwrap();
        assert_eq!(
            response
                .headers
                .iter()
                .filter(|(name, _)| name == "Set-Cookie")
                .count(),
            2
        );
        for bad in [
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\nok".as_slice(),
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\no",
            b"HTTP/1.1 200 OK\r\nContent-Length: 02\r\n\r\nok",
        ] {
            assert!(parse(bad.to_vec()).is_err());
        }
        let challenge = parse(
            b"HTTP/1.1 403 Forbidden\r\nContent-Length: 23\r\n\r\n{\"verdict\":\"CHALLENGE\"}"
                .to_vec(),
        )
        .unwrap();
        assert_eq!(challenge.verdict().as_deref(), Some("CHALLENGE"));
    }
}
