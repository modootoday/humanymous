//! Legacy live rotating-integrity-token kernels (legacy cmd/server/rit.go at
//! baseline 981d25c, SoT-07).
//!
//! The legacy server signed the fetch/XHR-controllable surface of an `/api/*`
//! call: `sid "\n" n "\n" tb "\n" sha256hex(body)`, with the token
//! `b64url(HMAC-SHA256(seed_lastN, canonical))` and ten-second time buckets.
//! Session keys and seeds reuse `humanymous_core::integrity::request`.
//!
//! These are pure decisions. The caller owns the per-session counter and the
//! one-shot bootstrap grace, and maps an [`Outcome`] to the frozen `l5.rit.*`
//! signals; that mapping and the collect-path wiring stay with the serving
//! composition. The accepted bootstrap difference (owner decision 2026-11-16)
//! is not decided here: [`Outcome::Bootstrap`] only reports that the grace was
//! claimed.
//!
//! Deliberate fail-closed difference: an empty master key or session ID is a
//! refusal; Go derived keys from them anyway.
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use humanymous_core::integrity::request::{seed, session_key, time_bucket};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Seconds per time bucket (Go `ritWindow`).
pub const WINDOW: i64 = 10;
/// Accepted distance between presented and current bucket.
pub const BUCKET_TOLERANCE: u64 = 1;

/// Missing required input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The server master key is empty.
    MasterKey,
    /// The session ID is empty.
    Session,
}

/// Seed for the client's next signed request (Go `issueSeed`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedSeed {
    /// `b64url(seed_n)` without padding.
    pub seed: String,
    /// The session's current counter `n`.
    pub counter: u64,
    /// Seconds per time bucket.
    pub window: i64,
}

/// Request headers as presented: `X-HM-Token`, `X-HM-N`, `X-HM-TB`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Presented<'a> {
    /// Token header, empty when absent.
    pub token: &'a str,
    /// Counter header text.
    pub counter: &'a str,
    /// Time-bucket header text.
    pub bucket: &'a str,
}

/// Verification result, in Go's check order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// First tokenless request; the caller's one-shot grace was claimed.
    Bootstrap,
    /// No token: either the grace was already consumed or the session has
    /// signed before.
    Absent,
    /// Counter is not `last + 1` (replay); carries the presented counter.
    StaleCounter(u64),
    /// Time bucket is outside the tolerance (replay); carries the presented bucket.
    StaleBucket(u64),
    /// HMAC over the observed canonical does not match (header, body or token
    /// tampering).
    Tampered,
    /// Valid; the caller advances the session counter.
    Valid,
}

fn mac(key: &[u8]) -> Hmac<Sha256> {
    <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts keys of any length")
}

fn session_seed(master: &[u8], sid: &str, counter: u64) -> Result<[u8; 32], Refusal> {
    if master.is_empty() {
        return Err(Refusal::MasterKey);
    }
    if sid.is_empty() {
        return Err(Refusal::Session);
    }
    Ok(seed(&session_key(master, sid), counter))
}

/// The live canonical string (Go `ritCanonical`).
pub fn canonical(sid: &str, counter: u64, bucket: u64, body: &[u8]) -> String {
    format!(
        "{sid}\n{counter}\n{bucket}\n{}",
        hex::encode(Sha256::digest(body))
    )
}

/// `b64url(HMAC-SHA256(seed, canonical))` without padding.
pub fn token(seed: &[u8], canonical: &str) -> String {
    let mut value = mac(seed);
    value.update(canonical.as_bytes());
    URL_SAFE_NO_PAD.encode(value.finalize().into_bytes())
}

/// Whether two buckets are at most `tolerance` apart (Go `withinTB`).
pub fn within_bucket(a: u64, b: u64, tolerance: u64) -> bool {
    a.abs_diff(b) <= tolerance
}

/// Go `strconv.ParseUint(s, 10, 64)` with errors read as zero: only ASCII
/// digits, no sign, no separators, and overflow is zero.
pub fn parse_counter(text: &str) -> u64 {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return 0;
    }
    text.parse().unwrap_or(0)
}

/// Seed for counter `counter` of session `sid` (Go `issueSeed`).
pub fn issue_seed(master: &[u8], sid: &str, counter: u64) -> Result<IssuedSeed, Refusal> {
    Ok(IssuedSeed {
        seed: URL_SAFE_NO_PAD.encode(session_seed(master, sid, counter)?),
        counter,
        window: WINDOW,
    })
}

/// Verify one `/api/*` request (Go `verifyRIT`).
///
/// `last` is the session's current counter, `now_unix` the wall clock in
/// seconds. `claim_bootstrap` is called only for a tokenless request on a
/// session that has never signed, and must return true at most once per
/// session.
pub fn verify(
    master: &[u8],
    sid: &str,
    last: u64,
    presented: Presented<'_>,
    now_unix: i64,
    body: &[u8],
    claim_bootstrap: impl FnOnce() -> bool,
) -> Result<Outcome, Refusal> {
    let seed = session_seed(master, sid, last)?;
    if presented.token.is_empty() {
        return Ok(if last == 0 && claim_bootstrap() {
            Outcome::Bootstrap
        } else {
            Outcome::Absent
        });
    }
    let counter = parse_counter(presented.counter);
    let bucket = parse_counter(presented.bucket);
    if counter != last.wrapping_add(1) {
        return Ok(Outcome::StaleCounter(counter));
    }
    if !within_bucket(bucket, time_bucket(now_unix, WINDOW), BUCKET_TOLERANCE) {
        return Ok(Outcome::StaleBucket(bucket));
    }
    let expect = token(&seed, &canonical(sid, counter, bucket, body));
    if bool::from(expect.as_bytes().ct_eq(presented.token.as_bytes())) {
        Ok(Outcome::Valid)
    } else {
        Ok(Outcome::Tampered)
    }
}

#[cfg(test)]
mod tests;
