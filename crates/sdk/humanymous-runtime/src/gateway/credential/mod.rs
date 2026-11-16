//! Pure gate credential verifier kernels (shipping row B25; legacy
//! internal/gate webbotauth.go, privacypass.go and webauthn.go at baseline
//! 981d25c).
//!
//! Each kernel turns operator-configured trust anchors plus request-carried
//! credential bytes into a verdict. Runtime owns the clock, request parsing,
//! audit and composition; nothing here is wired into routing, and no verdict
//! changes detection scoring. A verified credential is at most a trust signal.
//! An unconfigured verifier, a malformed credential or any check that was not
//! actually performed never yields a success verdict.
//!
//! Signature primitives come from existing owners only: Ed25519 from
//! `ed25519-dalek`, and RSA-PSS-SHA384 / ECDSA-P256-SHA256 from the *ring*
//! verification algorithms that the gateway's `rustls` dependency already
//! ships. No algorithm is reimplemented here.
//!
//! Shared deliberate differences from Go, each pinned by tests:
//! - Configuration is parsed strictly: a malformed line, duplicate identifier,
//!   invalid key or empty configuration is a typed [`ConfigError`]; Go skipped
//!   such lines silently and accepted an empty trust store.
//! - Verdicts carry static reason codes and only echo identifiers that matched
//!   an operator-configured entry, never unmatched attacker-supplied bytes.
//! - Base64 decoding rejects non-canonical trailing bits; Go's default
//!   decoders tolerate them.
//! - Inputs and caches are bounded; oversize input is a refusal.
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use rustls::SignatureScheme;
use rustls::pki_types::AlgorithmIdentifier;
use std::sync::LazyLock;

pub mod privacy_pass;
pub mod web_bot_auth;
pub mod webauthn;

/// Bound on configured trust-anchor entries per verifier.
pub const MAX_ENTRIES: usize = 4096;
/// Bound on a configured identifier (keyid or credential id) in bytes.
pub const MAX_ID: usize = 256;
/// Bound on a configuration document in bytes.
pub const MAX_CONFIG: usize = 4 << 20;

/// Static configuration refusals; no configuration bytes are interpolated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    /// Configuration exceeds `MAX_CONFIG` bytes or `MAX_ENTRIES` entries.
    Capacity,
    /// A line, PEM block or identifier is not in the expected shape.
    Syntax,
    /// A key does not decode to a supported public key.
    Key,
    /// The same identifier or key is configured twice.
    Duplicate,
    /// No trust anchor is configured; an empty verifier would only refuse.
    Empty,
}

/// Parse `identifier value` lines; blanks and `#` comments are skipped.
fn parse_lines(spec: &str) -> Result<Vec<(&str, &str)>, ConfigError> {
    if spec.len() > MAX_CONFIG {
        return Err(ConfigError::Capacity);
    }
    let mut out = Vec::new();
    for line in spec.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_ascii_whitespace();
        let (Some(id), Some(value), None) = (fields.next(), fields.next(), fields.next()) else {
            return Err(ConfigError::Syntax);
        };
        if id.len() > MAX_ID || !id.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(ConfigError::Syntax);
        }
        if out.len() == MAX_ENTRIES {
            return Err(ConfigError::Capacity);
        }
        out.push((id, value));
    }
    if out.is_empty() {
        return Err(ConfigError::Empty);
    }
    Ok(out)
}

/// Decode padded standard base64, falling back to the unpadded form
/// (Go `decodeB64Loose`).
fn decode_b64_loose(text: &str) -> Option<Vec<u8>> {
    STANDARD
        .decode(text)
        .or_else(|_| STANDARD_NO_PAD.decode(text))
        .ok()
}

/// Decode every `PUBLIC KEY` PEM block; any other non-blank text is refused.
fn pem_public_keys(text: &str) -> Result<Vec<Vec<u8>>, ConfigError> {
    const BEGIN: &str = "-----BEGIN PUBLIC KEY-----";
    const END: &str = "-----END PUBLIC KEY-----";
    if text.len() > MAX_CONFIG {
        return Err(ConfigError::Capacity);
    }
    let mut keys = Vec::new();
    let mut body: Option<String> = None;
    for line in text.lines().map(str::trim) {
        match body.as_mut() {
            None if line.is_empty() => {}
            None if line == BEGIN => body = Some(String::new()),
            None => return Err(ConfigError::Syntax),
            Some(b64) if line == END => {
                let der = STANDARD
                    .decode(b64.as_bytes())
                    .map_err(|_| ConfigError::Syntax)?;
                if keys.len() == MAX_ENTRIES {
                    return Err(ConfigError::Capacity);
                }
                keys.push(der);
                body = None;
            }
            Some(b64) => b64.push_str(line),
        }
    }
    if body.is_some() {
        return Err(ConfigError::Syntax);
    }
    if keys.is_empty() {
        return Err(ConfigError::Empty);
    }
    Ok(keys)
}

/// Strict DER reader: definite minimal lengths up to 64 KiB.
struct Der<'a>(&'a [u8]);

impl<'a> Der<'a> {
    fn read(&mut self, tag: u8) -> Option<&'a [u8]> {
        let (&t, rest) = self.0.split_first()?;
        if t != tag {
            return None;
        }
        let (&first, rest) = rest.split_first()?;
        let (len, rest) = if first < 0x80 {
            (usize::from(first), rest)
        } else {
            let n = usize::from(first & 0x7f);
            if !(1..=2).contains(&n) || rest.len() < n || rest[0] == 0 {
                return None;
            }
            let len = rest[..n]
                .iter()
                .fold(0usize, |acc, b| (acc << 8) | usize::from(*b));
            if len < 0x80 {
                return None;
            }
            (len, &rest[n..])
        };
        if rest.len() < len {
            return None;
        }
        let (value, rest) = rest.split_at(len);
        self.0 = rest;
        Some(value)
    }
    /// A minimal, positive INTEGER without its sign-padding byte.
    fn positive_integer(&mut self) -> Option<&'a [u8]> {
        let value = self.read(0x02)?;
        match value {
            [] => None,
            [first, ..] if first & 0x80 != 0 => None,
            [0, next, ..] if next & 0x80 != 0 => Some(&value[1..]),
            [0, ..] => None,
            _ => Some(value),
        }
    }
    fn finish(&self) -> Option<()> {
        self.0.is_empty().then_some(())
    }
}

/// Split a DER SubjectPublicKeyInfo into its algorithm body and key bytes.
fn spki_parts(spki: &[u8]) -> Option<(&[u8], &[u8])> {
    let mut outer = Der(spki);
    let mut info = Der(outer.read(0x30)?);
    outer.finish()?;
    let algorithm = info.read(0x30)?;
    let bits = info.read(0x03)?;
    info.finish()?;
    let (&0, key) = bits.split_first()? else {
        return None;
    };
    Some((algorithm, key))
}

/// The *ring* verification algorithm for a TLS signature scheme and key type.
fn ring_algorithm(
    scheme: SignatureScheme,
    key: AlgorithmIdentifier,
) -> Option<&'static dyn rustls::pki_types::SignatureVerificationAlgorithm> {
    static PROVIDER: LazyLock<rustls::crypto::CryptoProvider> =
        LazyLock::new(rustls::crypto::ring::default_provider);
    PROVIDER
        .signature_verification_algorithms
        .mapping
        .iter()
        .filter(|(candidate, _)| *candidate == scheme)
        .flat_map(|(_, algorithms)| algorithms.iter().copied())
        .find(|algorithm| algorithm.public_key_alg_id() == key)
}

/// Verify `signature` over `message`; any missing algorithm is a failure.
fn ring_verify(
    scheme: SignatureScheme,
    key: AlgorithmIdentifier,
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> bool {
    ring_algorithm(scheme, key).is_some_and(|algorithm| {
        algorithm
            .verify_signature(public_key, message, signature)
            .is_ok()
    })
}

#[cfg(test)]
mod tests;
