//! Live Rotating Integrity Token wiring of the legacy Core (port of
//! cmd/server/rit.go `ritCanonical`, `issueSeed`, `verifyRIT`, `withinTB` and
//! `parseUint` at baseline 981d25c; SoT-07).
//!
//! The canonical signs `sid "\n" n "\n" tb "\n" hex(sha256(body))` with the
//! seed of the session's last counter. Unlike `integrity::request::compute_token`
//! nothing else is appended to the MAC input, so tokens minted by the legacy
//! browser client (web/js/rit.js) verify unchanged. The first tokenless
//! request of a session consumes a one-shot bootstrap grace, exactly as the Go
//! Core did; every signal identity, verdict and note is the frozen one.
use crate::gateway::session::store::CollectorStore;
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use humanymous_core::integrity::request::{seed, session_key, time_bucket};
use humanymous_core::signals::observation::{Signal, Source, Verdict};
use humanymous_core::signals::report::Timestamp;
use serde_json::Value;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Seconds per RIT time bucket (Go `ritWindow`).
pub const WINDOW: u64 = 10;

/// Go `ritCanonical`.
pub fn canonical(sid: &str, n: u64, tb: u64, body: &[u8]) -> String {
    format!("{sid}\n{n}\n{tb}\n{}", hex::encode(Sha256::digest(body)))
}

/// Seed for counter `n` of a session, base64url without padding.
pub fn seed_text(master_key: &[u8], sid: &str, n: u64) -> String {
    URL_SAFE_NO_PAD.encode(seed(&session_key(master_key, sid), n))
}

/// The token a legacy client sends for `canonical` under seed `n`.
pub fn token(master_key: &[u8], sid: &str, n: u64, canonical: &str) -> String {
    let seed = seed(&session_key(master_key, sid), n);
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(&seed) else {
        return String::new();
    };
    mac.update(canonical.as_bytes());
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

/// Go `issueSeed`: the seed for the session's current counter, the counter
/// and the window.
pub fn issue_seed(store: &CollectorStore, master_key: &[u8], sid: &str) -> (String, u64, u64) {
    let n = store.rit_counter(sid);
    (seed_text(master_key, sid, n), n, WINDOW)
}

/// Go `withinTB`.
pub fn within_tb(a: u64, b: u64, tolerance: u64) -> bool {
    a.abs_diff(b) <= tolerance
}

/// Go `strconv.ParseUint(s, 10, 64)` with the error discarded: an empty,
/// signed or non-decimal value is 0 and an overflowing one is `u64::MAX`.
pub fn parse_uint(text: &str) -> u64 {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return 0;
    }
    text.parse().unwrap_or(u64::MAX)
}

fn signal(id: &str, value: Value, verdict: Verdict, notes: &str) -> Signal {
    Signal::new(id, value, verdict, 1.0, Source::Server, notes)
}

/// The RIT headers of one request.
#[derive(Debug, Clone, Copy, Default)]
pub struct Presented<'a> {
    /// `X-HM-Token`.
    pub token: &'a str,
    /// `X-HM-N`.
    pub n: &'a str,
    /// `X-HM-TB`.
    pub tb: &'a str,
}

/// Go `verifyRIT`: the signal for this request; on success the session's
/// counter advances.
pub fn verify(
    store: &mut CollectorStore,
    master_key: &[u8],
    sid: &str,
    presented: Presented<'_>,
    body: &[u8],
    now: Timestamp,
) -> Signal {
    let last = store.rit_counter(sid);
    let presented_n = parse_uint(presented.n);
    let presented_tb = parse_uint(presented.tb);
    let current_tb = time_bucket(now.0.unix_timestamp(), WINDOW as i64);
    if last == 0 && presented.token.is_empty() {
        if store.claim_rit_bootstrap(sid, now) {
            return signal(
                "l5.rit.bootstrap",
                Value::from("bootstrap"),
                Verdict::Ok,
                "first request, token grace",
            );
        }
        return signal(
            "l5.rit.absent",
            Value::Bool(true),
            Verdict::Bot,
            "repeated tokenless API call (grace already consumed)",
        );
    }
    if presented.token.is_empty() {
        return signal(
            "l5.rit.absent",
            Value::Bool(true),
            Verdict::Bot,
            "no RIT token on API call",
        );
    }
    if last.wrapping_add(1) != presented_n {
        return signal(
            "l5.rit.stale_replay",
            Value::from(presented_n),
            Verdict::Suspicious,
            "RIT counter out of sequence (replay)",
        );
    }
    if !within_tb(presented_tb, current_tb, 1) {
        return signal(
            "l5.rit.stale_replay",
            Value::from(presented_tb),
            Verdict::Suspicious,
            "RIT time bucket outside window (replay)",
        );
    }
    let expected = token(
        master_key,
        sid,
        last,
        &canonical(sid, presented_n, presented_tb, body),
    );
    if !bool::from(expected.as_bytes().ct_eq(presented.token.as_bytes())) {
        return signal(
            "l5.rit.header_tampered",
            Value::Bool(true),
            Verdict::Bot,
            "signed request != observed (invalid HMAC / body or token tampered)",
        );
    }
    store.advance_rit(sid, now);
    signal("l5.rit.ok", Value::Bool(true), Verdict::Ok, "RIT valid")
}
