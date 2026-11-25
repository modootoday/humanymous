//! Small request-derivation helpers (legacy internal/gate/request.go at
//! baseline 981d25c): route class for policy and audit, saturating latency,
//! per-request correlation ids and a stable upstream error class, plus the
//! watermark day bucket, keyed IP pseudonym and JA4 stable part from
//! cmd/server/helpers.go (its `readFile` lives with `resource::read_asset`).
//!
//! The client subnet lives in `token::client_subnet`; the session id comes from
//! signed RIT headers, not the legacy `hsid` cookie.
use super::Rejection;
use crate::network::tls::{ClientHello, ja4};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::io;
use std::net::IpAddr;
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

/// Stable audit class for an origin refusal code when no transport error was
/// observed: the overall deadline is `timeout`, a short origin frame is `eof`,
/// and any other malformed or over-budget frame is `unreachable`, Go's
/// fallback bucket for a non-dial error.
pub fn origin_error_class(code: &str) -> &'static str {
    if code == "origin_timeout" {
        "timeout"
    } else if code.contains("truncated") {
        "eof"
    } else {
        "unreachable"
    }
}

/// Unix day used as the low-resolution watermark issue time (legacy
/// cmd/server/helpers.go `dayBucket`). Times before the epoch clamp to day 0.
pub fn day_bucket(now_unix: i64) -> u64 {
    u64::try_from(now_unix / 86_400).unwrap_or(0)
}

/// Keyed pseudonym of the socket peer for the watermark ledger (legacy
/// `trafficIPHash`, PRIV-1): first 8 bytes of HMAC-SHA256(key, "ipwm|" + ip),
/// hex. `None` for an empty key, which would make the pseudonym reversible.
pub fn traffic_ip_hash(key: &[u8], ip: IpAddr) -> Option<String> {
    if key.is_empty() {
        return None;
    }
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(key).ok()?;
    mac.update(b"ipwm|");
    mac.update(ip.to_string().as_bytes());
    Some(hex::encode(&mac.finalize().into_bytes()[..8]))
}

/// Permutation-invariant `a_b` part of a connection's JA4 (legacy
/// `ja4Stable`, the network half of correlation keys): the hashed JA4 up to
/// its second underscore, or empty without a ClientHello.
pub fn ja4_stable(hello: Option<&ClientHello>) -> String {
    let Some(hello) = hello else {
        return String::new();
    };
    let (mut ja4, _) = ja4(hello);
    if let Some((index, _)) = ja4.match_indices('_').nth(1) {
        ja4.truncate(index);
    }
    ja4
}

#[cfg(test)]
mod tests;
