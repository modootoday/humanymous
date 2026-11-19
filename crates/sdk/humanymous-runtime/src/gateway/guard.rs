//! Proxy-to-origin trust-boundary guards (legacy internal/gate/guard.go and
//! pkg/origincloak at baseline 981d25c; SoT-23, HR-24, HR-27b).
//!
//! - Inbound trust-header hygiene: a client must never send the origin-cloak
//!   header, the internal verdict header or a forwarding header; their presence
//!   is reported by `spoof_scan` and removed by `strip_inbound`.
//! - Origin cloaking: `hex(HMAC-SHA256(key, "hmny-origin|" + epoch))` over a
//!   one-hour wall-clock epoch, verified with a symmetric one-bucket grace. The
//!   wire format matches Go byte for byte so a Go origin can verify it.
//!
//! `http::origin_request` drops internal headers before forwarding; attaching
//! the origin-cloak header and the HR-27b audit record are not wired here. Deliberate fail-closed difference, pinned by tests: an
//! origin key shorter than `MIN_ORIGIN_KEY` bytes is refused (Go accepted any).
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

/// Origin-cloak header name.
pub const ORIGIN_HEADER: &str = "X-Hmny-Origin-Auth";
/// Origin epoch bucket in seconds.
pub const ORIGIN_EPOCH_WINDOW: u64 = 60 * 60;
/// Minimum origin key length.
pub const MIN_ORIGIN_KEY: usize = 32;
/// Headers a client must never send inbound, in legacy scan order.
pub const INTERNAL_HEADERS: [&str; 6] = [
    "X-Hmny-Origin-Auth",
    "X-Hmn-Verdict",
    "X-Forwarded-For",
    "X-Real-Ip",
    "Forwarded",
    "Cf-Connecting-Ip",
];

/// Whether `name` is an internal trust header, case-insensitively.
pub fn is_internal(name: &str) -> bool {
    INTERNAL_HEADERS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(name))
}

/// Internal headers the request carried with a non-empty value, in legacy
/// order and without duplicates. Non-empty means a forged trust claim.
pub fn spoof_scan(headers: &[(String, String)]) -> Vec<&'static str> {
    INTERNAL_HEADERS
        .iter()
        .copied()
        .filter(|known| {
            headers
                .iter()
                .any(|(name, value)| known.eq_ignore_ascii_case(name) && !value.is_empty())
        })
        .collect()
}

/// Remove every internal header, empty or not, case-insensitively.
pub fn strip_inbound(headers: &mut Vec<(String, String)>) {
    headers.retain(|(name, _)| !is_internal(name));
}

/// Wall-clock origin epoch: `e` + unix seconds / 3600, truncating like Go.
pub fn origin_epoch(now_unix: i64) -> String {
    format!("e{}", now_unix / ORIGIN_EPOCH_WINDOW as i64)
}

/// Next, current and previous epochs, tolerating skew in either direction.
pub fn origin_epoch_grace(now_unix: i64) -> [String; 3] {
    let window = ORIGIN_EPOCH_WINDOW as i64;
    [
        origin_epoch(now_unix.saturating_add(window)),
        origin_epoch(now_unix),
        origin_epoch(now_unix.saturating_sub(window)),
    ]
}

/// Error for an origin key below `MIN_ORIGIN_KEY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortKey;

/// Shared origin-cloak secret; never Debug or Clone.
pub struct OriginKey(Vec<u8>);

impl OriginKey {
    /// Take ownership of a key of at least `MIN_ORIGIN_KEY` bytes.
    pub fn new(bytes: Vec<u8>) -> Result<Self, ShortKey> {
        if bytes.len() < MIN_ORIGIN_KEY {
            return Err(ShortKey);
        }
        Ok(Self(bytes))
    }

    /// Token the proxy attaches for `epoch`.
    pub fn token(&self, epoch: &str) -> String {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&self.0).expect("HMAC accepts any key");
        mac.update(b"hmny-origin|");
        mac.update(epoch.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// Constant-time match against any of `epochs`; empty headers never match.
    pub fn valid_at(&self, header: &str, epochs: &[String]) -> bool {
        if header.is_empty() {
            return false;
        }
        epochs.iter().fold(false, |found, epoch| {
            found | bool::from(header.as_bytes().ct_eq(self.token(epoch).as_bytes()))
        })
    }

    /// Origin-side check at `now_unix` with the symmetric one-bucket grace.
    pub fn valid(&self, header: &str, now_unix: i64) -> bool {
        self.valid_at(header, &origin_epoch_grace(now_unix))
    }
}

#[cfg(test)]
mod parity;
#[cfg(test)]
mod tests;
