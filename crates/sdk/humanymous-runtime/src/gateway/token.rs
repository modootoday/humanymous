//! Verdict tokens, step-up proofs and receipts, epochs, single-use nonces and
//! opaque incident handles and their resolution (shipping row B17; legacy internal/gate token.go,
//! epoch.go and incident.go at baseline 981d25c).
//!
//! The wire format matches Go: `b64url(payload) "." b64url(HMAC-SHA256(key,
//! payload))` with `|`-separated payload fields and no algorithm field. Issuing
//! or accepting a token is a trust upgrade; this kernel enables none by itself.
//!
//! Deliberate fail-closed differences, each pinned by tests:
//! - An empty expected session is a refusal; Go skipped the session check.
//! - Fields containing `|`, empty fields and non-canonical expiry are refused
//!   at issue and verification; Go treated an unparsable expiry as zero.
//! - An empty nonce is a refusal; Go accepted it as an older client.
//! - Keys shorter than 32 bytes are refused.
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;
use subtle::ConstantTimeEq;

/// Verdict-token epoch bucket in seconds.
pub const EPOCH_WINDOW: u64 = 15 * 60;
/// Clock-skew leeway for step-up receipts minted on another host.
pub const RECEIPT_SKEW: u64 = 120;
/// Bound on retained single-use nonces.
pub const MAX_NONCES: usize = 1 << 20;
/// Minimum server key length.
pub const MIN_KEY: usize = 32;

/// Why a token or receipt failed; static, never echoes token bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Not `payload.mac` base64url, wrong arity, domain or field shape.
    Malformed,
    /// MAC does not verify under the server key.
    BadSignature,
    /// Past expiry or outside the accepted epochs.
    Expired,
    /// Valid token for another session or client binding.
    BindingMismatch,
}

/// Issue-time refusals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Server key is shorter than `MIN_KEY`.
    Key,
    /// A field is empty or contains the `|` separator.
    Field,
}

/// Server token key; never Debug or Clone.
pub struct TokenKey(Vec<u8>);

impl TokenKey {
    /// Take ownership of a key of at least `MIN_KEY` bytes.
    pub fn new(bytes: Vec<u8>) -> Result<Self, Refusal> {
        if bytes.len() < MIN_KEY {
            return Err(Refusal::Key);
        }
        Ok(Self(bytes))
    }
    fn mac(&self, payload: &[u8]) -> Vec<u8> {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&self.0).expect("HMAC accepts any key");
        mac.update(payload);
        mac.finalize().into_bytes().to_vec()
    }
    fn seal(&self, fields: &[&str]) -> Result<String, Refusal> {
        if fields.iter().any(|f| f.is_empty() || f.contains('|')) {
            return Err(Refusal::Field);
        }
        let payload = fields.join("|");
        Ok(format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(payload.as_bytes()),
            URL_SAFE_NO_PAD.encode(self.mac(payload.as_bytes()))
        ))
    }
    fn open(&self, token: &str) -> Result<Vec<String>, Reason> {
        let (payload, mac) = token.split_once('.').ok_or(Reason::Malformed)?;
        let payload = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| Reason::Malformed)?;
        let mac = URL_SAFE_NO_PAD.decode(mac).map_err(|_| Reason::Malformed)?;
        if !bool::from(mac.ct_eq(&self.mac(&payload))) {
            return Err(Reason::BadSignature);
        }
        let payload = String::from_utf8(payload).map_err(|_| Reason::Malformed)?;
        Ok(payload.split('|').map(str::to_owned).collect())
    }

    /// Opaque, non-enumerable incident handle for an audit sequence number.
    pub fn incident_handle(&self, seq: u64) -> String {
        let mut payload = b"incident|".to_vec();
        payload.extend_from_slice(&seq.to_be_bytes());
        format!("INC-{}", hex::encode_upper(&self.mac(&payload)[..5]))
    }

    /// Resolve an opaque incident handle against `(seq, record)` pairs (Go
    /// `resolveIncident`): the first record whose sequence produces `handle`,
    /// or `None`. The handle is recomputed at read time and never stored, so
    /// a handle from another key resolves to nothing. Per-operator lookup caps
    /// belong to the admin API that calls this.
    pub fn resolve_incident<R>(
        &self,
        handle: &str,
        records: impl IntoIterator<Item = (u64, R)>,
    ) -> Option<(u64, R)> {
        records.into_iter().find(|(seq, _)| {
            bool::from(
                self.incident_handle(*seq)
                    .as_bytes()
                    .ct_eq(handle.as_bytes()),
            )
        })
    }

    /// Mint a verdict token bound to session, client binding, expiry and epoch.
    pub fn issue_verdict(
        &self,
        sid: &str,
        bind: &str,
        epoch: &str,
        exp: u64,
    ) -> Result<String, Refusal> {
        self.seal(&[sid, bind, &exp.to_string(), epoch])
    }

    /// Verify a verdict token for this session and binding at `now`.
    pub fn verify_verdict(
        &self,
        token: &str,
        bind: &str,
        expect_sid: &str,
        now: u64,
        epochs: &[String],
    ) -> Result<(), Reason> {
        let parts = self.open(token)?;
        let [sid, bound, exp, epoch] = parts.as_slice() else {
            return Err(Reason::Malformed);
        };
        if sid == "su" || sid == "sur" {
            return Err(Reason::Malformed);
        }
        check(sid, bound, exp, epoch, bind, expect_sid, now, epochs)
    }

    /// Mint a step-up proof, domain-separated from verdict tokens by `su`.
    pub fn issue_step_up(
        &self,
        sid: &str,
        bind: &str,
        epoch: &str,
        exp: u64,
    ) -> Result<String, Refusal> {
        self.seal(&["su", sid, bind, &exp.to_string(), epoch])
    }

    /// Verify a step-up proof; a verdict token never verifies here.
    pub fn verify_step_up(
        &self,
        token: &str,
        bind: &str,
        expect_sid: &str,
        now: u64,
        epochs: &[String],
    ) -> Result<(), Reason> {
        let parts = self.open(token)?;
        let [domain, sid, bound, exp, epoch] = parts.as_slice() else {
            return Err(Reason::Malformed);
        };
        if domain != "su" {
            return Err(Reason::Malformed);
        }
        check(sid, bound, exp, epoch, bind, expect_sid, now, epochs)
    }

    /// Mint a session-bound step-up receipt (grants nothing by itself).
    pub fn issue_receipt(&self, sid: &str, exp: u64) -> Result<String, Refusal> {
        self.seal(&["sur", sid, &exp.to_string()])
    }

    /// Verify a step-up receipt with `RECEIPT_SKEW` leeway.
    pub fn verify_receipt(&self, token: &str, expect_sid: &str, now: u64) -> Result<(), Reason> {
        let parts = self.open(token)?;
        let [domain, sid, exp] = parts.as_slice() else {
            return Err(Reason::Malformed);
        };
        if domain != "sur" {
            return Err(Reason::Malformed);
        }
        if now.saturating_sub(RECEIPT_SKEW) > expiry(exp)? {
            return Err(Reason::Expired);
        }
        if expect_sid.is_empty() || !bool::from(sid.as_bytes().ct_eq(expect_sid.as_bytes())) {
            return Err(Reason::BindingMismatch);
        }
        Ok(())
    }
}

fn expiry(text: &str) -> Result<u64, Reason> {
    if text.is_empty()
        || text.len() > 20
        || !text.bytes().all(|b| b.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return Err(Reason::Malformed);
    }
    text.parse().map_err(|_| Reason::Malformed)
}

#[allow(clippy::too_many_arguments)]
fn check(
    sid: &str,
    bound: &str,
    exp: &str,
    epoch: &str,
    bind: &str,
    expect_sid: &str,
    now: u64,
    epochs: &[String],
) -> Result<(), Reason> {
    if now > expiry(exp)? || !epochs.iter().any(|e| e == epoch) {
        return Err(Reason::Expired);
    }
    if expect_sid.is_empty()
        || !bool::from(sid.as_bytes().ct_eq(expect_sid.as_bytes()))
        || !bool::from(bound.as_bytes().ct_eq(bind.as_bytes()))
    {
        return Err(Reason::BindingMismatch);
    }
    Ok(())
}

/// Wall-clock epoch derivation shared by every node, plus a per-node bump.
#[derive(Debug, Default)]
pub struct Epochs {
    bump: u64,
}

impl Epochs {
    fn at(&self, now: u64, back: u64) -> String {
        format!("e{}", (now / EPOCH_WINDOW + self.bump).saturating_sub(back))
    }
    /// Epoch new tokens are minted under.
    pub fn current(&self, now: u64) -> String {
        self.at(now, 0)
    }
    /// Accepted epochs: current and previous bucket.
    pub fn accepted(&self, now: u64) -> Vec<String> {
        vec![self.at(now, 0), self.at(now, 1)]
    }
    /// Per-node emergency bump; not a fleet-wide revocation by itself.
    pub fn advance(&mut self) {
        self.bump = self.bump.saturating_add(1);
    }
}

/// Header fingerprint surrogate; `None` without a User-Agent.
pub fn bind_key(user_agent: &str, accept_language: &str, sec_ch_ua: &str) -> Option<String> {
    if user_agent.is_empty() {
        return None;
    }
    let mut hash = Sha256::new();
    for (i, part) in [user_agent, accept_language, sec_ch_ua].iter().enumerate() {
        if i > 0 {
            hash.update([0]);
        }
        hash.update(part.as_bytes());
    }
    Some(URL_SAFE_NO_PAD.encode(&hash.finalize()[..12]))
}

/// Socket-peer network anchor: IPv4 /24 or IPv6 /48, Go text form.
pub fn client_subnet(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, c, _] = v4.octets();
            format!("{a}.{b}.{c}.0/24")
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return client_subnet(IpAddr::V4(v4));
            }
            let masked = u128::from(v6) & !(u128::MAX >> 48);
            format!("{}/48", std::net::Ipv6Addr::from(masked))
        }
    }
}

/// Verdict-token binding: header fingerprint plus socket subnet.
pub fn token_bind(bind_key: Option<&str>, subnet: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(bind_key.unwrap_or("").as_bytes());
    hash.update([0]);
    hash.update(subnet.as_bytes());
    URL_SAFE_NO_PAD.encode(&hash.finalize()[..16])
}

/// Single-use proof nonces; a nonce is accepted once across all bindings.
pub struct Nonces {
    seen: HashMap<String, Duration>,
    ttl: Duration,
    last_sweep: Duration,
    capacity: usize,
}

impl Nonces {
    /// Bounded cache; capacity is clamped to `MAX_NONCES`.
    pub fn new(ttl: Duration, capacity: usize) -> Self {
        Self {
            seen: HashMap::new(),
            ttl,
            last_sweep: Duration::ZERO,
            capacity: capacity.clamp(1, MAX_NONCES),
        }
    }
    /// Consume a nonce. Returns false for replay, empty nonce or saturation.
    pub fn use_once(&mut self, nonce: &str, now: Duration) -> bool {
        if nonce.is_empty() {
            return false;
        }
        if now.saturating_sub(self.last_sweep) > self.ttl / 4 || self.seen.len() >= self.capacity {
            let ttl = self.ttl;
            self.seen.retain(|_, t| now.saturating_sub(*t) <= ttl);
            self.last_sweep = now;
        }
        if self.seen.contains_key(nonce) || self.seen.len() >= self.capacity {
            return false;
        }
        self.seen.insert(nonce.into(), now);
        true
    }
}

#[cfg(test)]
mod tests;
