//! Small request-derivation helpers (legacy internal/gate/request.go at
//! baseline 981d25c): route class for policy and audit, saturating latency,
//! per-request correlation ids and a stable upstream error class.
//!
//! The client subnet lives in `token::client_subnet`; the session id comes from
//! signed RIT headers, not the legacy `hsid` cookie.
use super::Rejection;
use std::io;
use std::time::Duration;

/// Coarse route class (SoT-19 §7); not a detection signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteClass {
    /// WebSocket upgrade.
    Upgrade,
    /// Document navigation or HTML accept.
    Html,
    /// JSON or fetch with an empty destination.
    Api,
    /// Everything else.
    Static,
}

impl RouteClass {
    /// Legacy audit spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Upgrade => "upgrade",
            Self::Html => "html",
            Self::Api => "api",
            Self::Static => "static",
        }
    }
}

fn first<'a>(headers: &'a [(String, String)], name: &str) -> &'a str {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map_or("", |(_, v)| v.as_str())
}

/// Classify exactly like Go: first header value per name, in priority order.
pub fn route_class(headers: &[(String, String)]) -> RouteClass {
    if first(headers, "Upgrade").eq_ignore_ascii_case("websocket")
        || !first(headers, "Sec-WebSocket-Key").is_empty()
    {
        return RouteClass::Upgrade;
    }
    let accept = first(headers, "Accept");
    let dest = first(headers, "Sec-Fetch-Dest");
    if dest == "document" || accept.contains("text/html") {
        return RouteClass::Html;
    }
    if accept.contains("application/json") || dest == "empty" {
        return RouteClass::Api;
    }
    RouteClass::Static
}

/// Microseconds since `start`, saturating into `u32` and never negative.
/// `None` start (unknown) is zero, as Go's zero time was.
pub fn latency_us(start: Option<Duration>, now: Duration) -> u32 {
    let Some(start) = start else { return 0 };
    u32::try_from(now.saturating_sub(start).as_micros()).unwrap_or(u32::MAX)
}

/// Short random per-request id; entropy failure fails closed.
pub fn correlation_id() -> Result<String, Rejection> {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).map_err(|_| Rejection::new(503, "entropy"))?;
    Ok(hex::encode(bytes))
}

/// Stable audit class for an upstream transport error; never the raw text.
pub fn upstream_error_class(error: &io::Error) -> &'static str {
    match error.kind() {
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => "timeout",
        io::ErrorKind::Interrupted => "canceled",
        io::ErrorKind::ConnectionRefused => "connection_refused",
        io::ErrorKind::UnexpectedEof => "eof",
        _ if error.to_string().contains("failed to lookup address") => "dns",
        _ => "unreachable",
    }
}

#[cfg(test)]
mod tests;
