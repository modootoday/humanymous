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
//! - Spoof-header gate: `spoof_gate` is the pure port of legacy gates.go
//!   `spoofHeaderGate` (strip, HR-27b record, deny when enforcing).
//!
//! `http::origin_request` drops internal headers before forwarding and, with
//! a configured `edge.origin_key_hex`, attaches the origin-cloak token for the
//! current epoch; `Application` runs `spoof_gate` on the served path
//! (`application::edge`). Deliberate fail-closed differences, pinned by tests: an origin
//! key shorter than `MIN_ORIGIN_KEY` bytes is refused (Go accepted any), and
//! `spoof_gate` strips internal headers even when none carried a value.
use crate::audit::record::Record;
use hmac::{Hmac, KeyInit, Mac};
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

/// Incident pointer header set on every gate deny (legacy `deny` and
/// `enforceBan`); the detail lives only in the audit chain.
pub const INCIDENT_HEADER: (&str, &str) = ("X-Hmn-Incident", "see-audit-log");

/// Static body of a request the enforcing spoof gate denies (legacy `deny`).
pub const SPOOF_DENY_BODY: &str = "Request blocked by humanymous (automation detected).";

/// Spoof-gate finding: the HR-27b record to append before acting, and whether
/// the request is denied (403, never forwarded) or only audited.
#[derive(Debug, Clone, PartialEq)]
pub struct Spoof {
    /// HR-27b record; mode `monitor` when not enforcing.
    pub record: Record,
    /// Deny instead of forwarding.
    pub deny: bool,
}

impl Spoof {
    /// Response metadata of a deny: the incident pointer. Empty when the
    /// request continues (monitor), since nothing is answered locally.
    pub fn headers(&self) -> Vec<(&'static str, String)> {
        if self.deny {
            vec![(INCIDENT_HEADER.0, INCIDENT_HEADER.1.to_owned())]
        } else {
            Vec::new()
        }
    }
}

/// Legacy `spoofHeaderGate`: strip every internal header and, when any carried
/// a value, return the HR-27b record. Monitor and shadow both record mode
/// `monitor` and let the stripped request continue, as Go did.
pub fn spoof_gate(
    headers: &mut Vec<(String, String)>,
    enforce: bool,
    tenant: &str,
    session_pseudonym: &str,
) -> Option<Spoof> {
    let found = spoof_scan(headers);
    strip_inbound(headers);
    if found.is_empty() {
        return None;
    }
    let mut record = crate::gateway::smuggle::spoof_record(tenant, session_pseudonym, &found);
    if !enforce {
        record.enforcement_mode = "monitor".into();
    }
    Some(Spoof {
        record,
        deny: enforce,
    })
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
        let mut mac =
            <Hmac<Sha256> as KeyInit>::new_from_slice(&self.0).expect("HMAC accepts any key");
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
