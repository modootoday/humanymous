//! Web Bot Auth request signatures: RFC 9421 HTTP Message Signatures with the
//! `web-bot-auth` tag and Ed25519 (legacy internal/gate/webbotauth.go).
//!
//! A valid signature from an operator-allowlisted key is a trust signal; a
//! signature that claims an allowlisted keyid but fails is a forgery; anything
//! else is neutral. The covered set must be exactly a permutation of
//! `@authority`, `@method` and `@path` so a lifted signature cannot be replayed
//! across hosts, methods or paths, lifetime is bounded to one hour, and a
//! single-use `nonce` blocks same-request replay. The live JWKS directory fetch
//! remains a future seam; [`KeyDirectory`] is the operator allowlist.
//!
//! Deliberate fail-closed differences from Go, each pinned by tests:
//! - Without a configured directory a presented signature is
//!   `Neutral(NotConfigured)` (shipping row B25); Go reported "no signature".
//! - Only one signature label is parsed; commas outside quoted strings,
//!   duplicate parameters or components, non-decimal `created`/`expires`,
//!   unquoted string parameters and control bytes are `Malformed`. Go parsed
//!   leniently and read an unparsable timestamp as absent.
//! - Ed25519 uses `verify_strict` (no small-order keys or non-canonical
//!   points); weak directory keys are refused at configuration.
//! - Exactly one of the two headers is `Neutral(Malformed)`; Go reported "no
//!   signature" unless both were non-empty.
//! - `created=0` is a present timestamp; Go read zero as absent.
//! - The keyid is only reported when it names a directory entry.
use super::{ConfigError, parse_lines};
use crate::gateway::token::Nonces;
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, VerifyingKey};
use std::collections::BTreeMap;
use std::time::Duration;

/// Allowed clock skew for `created` and `expires`, in seconds.
pub const SKEW: u64 = 60;
/// Longest accepted signature lifetime, in seconds.
pub const MAX_LIFETIME: u64 = 3600;
/// Bound on the `Signature-Input` field value in bytes.
pub const MAX_SIGNATURE_INPUT: usize = 4096;
/// Bound on the `Signature` field value in bytes.
pub const MAX_SIGNATURE: usize = 1024;
/// Bound on request authority, method and path bytes.
pub const MAX_REQUEST_FIELD: usize = 8192;
/// Signature profile tag.
pub const TAG: &str = "web-bot-auth";

/// Operator-configured allowlist of trusted agent keys.
pub struct KeyDirectory {
    keys: BTreeMap<String, VerifyingKey>,
}

impl KeyDirectory {
    /// Parse `keyid base64url-ed25519-public-key` lines; every entry is trusted.
    pub fn parse(spec: &str) -> Result<Self, ConfigError> {
        let mut keys = BTreeMap::new();
        for (id, value) in parse_lines(spec)? {
            let raw = URL_SAFE_NO_PAD
                .decode(value)
                .map_err(|_| ConfigError::Key)?;
            let bytes: [u8; 32] = raw.try_into().map_err(|_| ConfigError::Key)?;
            let key = VerifyingKey::from_bytes(&bytes).map_err(|_| ConfigError::Key)?;
            if key.is_weak() {
                return Err(ConfigError::Key);
            }
            if keys.insert(id.to_owned(), key).is_some() {
                return Err(ConfigError::Duplicate);
            }
        }
        Ok(Self { keys })
    }
    fn lookup(&self, keyid: &str) -> Option<(&String, &VerifyingKey)> {
        self.keys.get_key_value(keyid)
    }
}

/// The request line a signature must cover, as Runtime parsed it.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    /// Request authority (Host); compared lowercase.
    pub authority: &'a str,
    /// Request method; compared uppercase.
    pub method: &'a str,
    /// Request path as received.
    pub path: &'a str,
    /// `Signature-Input` field value, if any.
    pub signature_input: Option<&'a str>,
    /// `Signature` field value, if any.
    pub signature: Option<&'a str>,
}

/// Why a presented signature confers no trust; never echoes request bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// No directory is configured, so nothing could be verified.
    NotConfigured,
    /// Header or request bytes are not in the accepted shape or size.
    Malformed,
    /// Not an Ed25519 `web-bot-auth` signature.
    Profile,
    /// Not yet valid or expired.
    Stale,
    /// Lifetime is absent or longer than `MAX_LIFETIME`.
    Unbounded,
    /// Covered components do not bind exactly authority, method and path.
    Coverage,
    /// Keyid is not on the allowlist.
    UnknownKey,
    /// Valid signature whose nonce was already consumed.
    Replay,
}

/// Outcome of verifying one request's Web Bot Auth signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Neither signature header is present.
    Absent,
    /// Valid, fresh signature from an allowlisted key.
    Trusted {
        /// Directory keyid.
        keyid: String,
    },
    /// Present but not verifiable or not fresh; neither trust nor deny.
    Neutral {
        /// Static reason.
        reason: Reason,
        /// Directory keyid when the claimed keyid is allowlisted.
        keyid: Option<String>,
    },
    /// Claims an allowlisted key but the signature does not verify.
    Forged {
        /// Directory keyid.
        keyid: String,
    },
}

fn neutral(reason: Reason) -> Verdict {
    Verdict::Neutral {
        reason,
        keyid: None,
    }
}

/// Parsed parameters for the single signature label.
#[derive(Debug, Default, PartialEq, Eq)]
struct Params<'a> {
    label: &'a str,
    components: Vec<&'a str>,
    /// Raw inner list plus parameters: the `@signature-params` value.
    inner: &'a str,
    created: Option<u64>,
    expires: Option<u64>,
    keyid: Option<&'a str>,
    alg: Option<&'a str>,
    tag: Option<&'a str>,
    nonce: Option<&'a str>,
}

/// Verify a request's Web Bot Auth signature at `now` (since the Unix epoch).
pub fn verify(
    request: &Request<'_>,
    directory: Option<&KeyDirectory>,
    nonces: &mut Nonces,
    now: Duration,
) -> Verdict {
    let (Some(input), Some(signature)) = (request.signature_input, request.signature) else {
        return if request.signature_input.is_none() && request.signature.is_none() {
            Verdict::Absent
        } else {
            neutral(Reason::Malformed)
        };
    };
    let Some(directory) = directory else {
        return neutral(Reason::NotConfigured);
    };
    if [request.authority, request.method, request.path]
        .iter()
        .any(|field| field.len() > MAX_REQUEST_FIELD || !printable(field))
    {
        return neutral(Reason::Malformed);
    }
    let Some(params) = parse_signature_input(input) else {
        return neutral(Reason::Malformed);
    };
    let Some(raw) = parse_signature_value(signature, params.label) else {
        return neutral(Reason::Malformed);
    };
    if params.alg != Some("ed25519") || params.tag != Some(TAG) {
        return neutral(Reason::Profile);
    }
    if let Err(reason) = fresh(&params, now.as_secs()) {
        return neutral(reason);
    }
    if !covers_request_line(&params.components) {
        return neutral(Reason::Coverage);
    }
    let Some((keyid, key)) = params.keyid.and_then(|id| directory.lookup(id)) else {
        return neutral(Reason::UnknownKey);
    };
    let base = signature_base(request, &params.components, params.inner);
    let verified = Signature::from_slice(&raw)
        .is_ok_and(|signature| key.verify_strict(base.as_bytes(), &signature).is_ok());
    if !verified {
        return Verdict::Forged {
            keyid: keyid.clone(),
        };
    }
    if let Some(nonce) = params.nonce
        && !nonces.use_once(&format!("wba|{keyid}|{nonce}"), now)
    {
        return Verdict::Neutral {
            reason: Reason::Replay,
            keyid: Some(keyid.clone()),
        };
    }
    Verdict::Trusted {
        keyid: keyid.clone(),
    }
}

fn printable(text: &str) -> bool {
    text.bytes().all(|b| (0x20..0x7f).contains(&b))
}

/// Go freshness rules: not before `created - SKEW`, not after
/// `expires + SKEW`, and a lifetime bounded by `MAX_LIFETIME`.
fn fresh(params: &Params<'_>, now: u64) -> Result<(), Reason> {
    if params
        .created
        .is_some_and(|created| created > now.saturating_add(SKEW))
    {
        return Err(Reason::Stale);
    }
    match (params.created, params.expires) {
        (_, Some(expires)) if expires < now.saturating_sub(SKEW) => Err(Reason::Stale),
        (_, Some(expires)) if expires.saturating_sub(now) > MAX_LIFETIME => Err(Reason::Unbounded),
        (Some(created), Some(expires)) if expires.saturating_sub(created) > MAX_LIFETIME => {
            Err(Reason::Unbounded)
        }
        (_, Some(_)) => Ok(()),
        (Some(created), None) if now.saturating_sub(created) > MAX_LIFETIME => Err(Reason::Stale),
        (Some(_), None) => Ok(()),
        (None, None) => Err(Reason::Unbounded),
    }
}

/// Exactly `@authority`, `@method` and `@path`, each once, in any order.
fn covers_request_line(components: &[&str]) -> bool {
    let mut seen = [false; 3];
    for component in components {
        let slot = match *component {
            "@authority" => 0,
            "@method" => 1,
            "@path" => 2,
            _ => return false,
        };
        if seen[slot] {
            return false;
        }
        seen[slot] = true;
    }
    seen == [true; 3]
}

/// RFC 9421 signature base: one line per covered component in signer order,
/// then the `@signature-params` line (Go `buildSignatureBase`).
fn signature_base(request: &Request<'_>, components: &[&str], inner: &str) -> String {
    let mut base = String::new();
    for component in components {
        let value = match *component {
            "@authority" => request.authority.to_ascii_lowercase(),
            "@method" => request.method.to_ascii_uppercase(),
            "@path" => request.path.to_owned(),
            _ => continue,
        };
        base.push_str(&format!("\"{component}\": {value}\n"));
    }
    base.push_str("\"@signature-params\": ");
    base.push_str(inner);
    base
}

/// Targeted parser for one `label=("c1" "c2");k=v;...` member.
fn parse_signature_input(value: &str) -> Option<Params<'_>> {
    let value = value.trim();
    if value.len() > MAX_SIGNATURE_INPUT || !printable(value) {
        return None;
    }
    let (label, inner) = value.split_once('=')?;
    if !is_key(label) {
        return None;
    }
    let inner = inner.trim();
    let list = inner.strip_prefix('(')?;
    let close = list.find(')')?;
    let mut params = Params {
        label,
        inner,
        ..Params::default()
    };
    for item in list[..close].split(' ').filter(|item| !item.is_empty()) {
        let name = quoted(item)?;
        if params.components.contains(&name) || params.components.len() == 8 {
            return None;
        }
        params.components.push(name);
    }
    let rest = &list[close + 1..];
    if !rest.is_empty() && !rest.starts_with(';') {
        return None;
    }
    for pair in rest.split(';').skip(1) {
        let (key, raw) = pair.split_once('=')?;
        let slot = match key {
            "created" => {
                set(&mut params.created, decimal(raw)?)?;
                continue;
            }
            "expires" => {
                set(&mut params.expires, decimal(raw)?)?;
                continue;
            }
            "keyid" => &mut params.keyid,
            "alg" => &mut params.alg,
            "tag" => &mut params.tag,
            "nonce" => &mut params.nonce,
            _ if is_key(key) => continue,
            _ => return None,
        };
        set(slot, quoted(raw)?)?;
    }
    Some(params)
}

fn set<T>(slot: &mut Option<T>, value: T) -> Option<()> {
    slot.is_none().then(|| *slot = Some(value))
}

/// RFC 8941 key: lowercase alpha or `*`, then lowercase alphanumerics, `_-.*`.
fn is_key(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_lowercase() || b == b'*')
        && bytes.all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-' | b'.' | b'*')
        })
}

/// A quoted string without escapes, quotes or commas inside.
fn quoted(text: &str) -> Option<&str> {
    let inner = text.strip_prefix('"')?.strip_suffix('"')?;
    (!inner.is_empty() && !inner.contains(['"', '\\', ','])).then_some(inner)
}

/// Canonical non-negative decimal (no sign, no leading zeros).
fn decimal(text: &str) -> Option<u64> {
    let canonical = !text.is_empty()
        && text.len() <= 15
        && text.bytes().all(|b| b.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    canonical.then(|| text.parse().ok()).flatten()
}

/// Extract `label=:base64:` signature bytes for `label`.
fn parse_signature_value(value: &str, label: &str) -> Option<Vec<u8>> {
    let value = value.trim();
    if value.len() > MAX_SIGNATURE {
        return None;
    }
    let body = value.strip_prefix(label)?.strip_prefix('=')?;
    let encoded = body.strip_prefix(':')?.strip_suffix(':')?;
    STANDARD.decode(encoded).ok()
}

#[cfg(test)]
mod tests;
