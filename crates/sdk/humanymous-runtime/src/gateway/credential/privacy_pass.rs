//! Privacy Pass Private Access Tokens: RFC 9578 token type 0x0002, publicly
//! verifiable Blind RSA (legacy internal/gate/privacypass.go).
//!
//! To the relying party the unblinded RSABSSA authenticator is an ordinary
//! RSASSA-PSS signature with SHA-384 over the token input
//! `type || nonce || challenge_digest || token_key_id`, so no blind-signature
//! code is needed here. A valid token from a configured issuer is a trust
//! signal; an absent or invalid token is never a deny. A verified token is
//! single-use within `SPENT_TTL` (anti double-spend).
//!
//! Deliberate fail-closed differences from Go, each pinned by tests:
//! - The PSS salt length must be 48 bytes, as RFC 9578 fixes
//!   RSABSSA-SHA384-PSS-Deterministic; Go's `PSSSaltLengthAuto` also accepted
//!   zero-salt signatures from other variants.
//! - The authenticator must be exactly the issuer modulus length and the
//!   modulus 2048 to 8192 bits (the *ring* bounds); Go accepted any length.
//! - Issuer PEM must contain only `PUBLIC KEY` blocks with canonical-DER
//!   `rsaEncryption` SubjectPublicKeyInfo, at least one; Go skipped other
//!   blocks and keys silently. `token_key_id` is SHA-256 of that DER, which is
//!   byte-identical to Go's re-marshalled key.
//! - A header that names `PrivateToken` without a token, or exceeds
//!   `MAX_HEADER`, is `Invalid(Malformed)`; Go reported "no token".
//! - The caller supplies the clock; Go read the wall clock.
//! - [`Verifier::accept_challenges`] optionally binds `challenge_digest` to
//!   challenges the Runtime issued. Go never checked it; it stays off by
//!   default for parity, which is a documented gap until Runtime issues
//!   `WWW-Authenticate: PrivateToken` challenges.
use super::{ConfigError, Der, decode_b64_loose, pem_public_keys, ring_verify, spki_parts};
use rustls::SignatureScheme;
use rustls::pki_types::alg_id;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;

/// RFC 9578 Blind RSA token type.
pub const TOKEN_TYPE: u16 = 0x0002;
/// Spent-nonce retention (Go `ttl`).
pub const SPENT_TTL: Duration = Duration::from_secs(10 * 60);
/// Bound on retained spent nonces (Go `patMaxSeen`).
pub const MAX_SPENT: usize = 1 << 20;
/// Bound on the Authorization header value in bytes.
pub const MAX_HEADER: usize = 4096;
/// Bound on configured challenge digests.
pub const MAX_CHALLENGES: usize = 1024;
/// Signed prefix length: type, nonce, challenge digest and key id.
const PREFIX: usize = 2 + 32 + 32 + 32;
/// DER body of the rsaEncryption AlgorithmIdentifier (OID plus NULL).
const RSA_ENCRYPTION: &[u8] = &[
    0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01, 0x05, 0x00,
];

/// One configured issuer key.
struct Issuer {
    /// DER RSAPublicKey (the SubjectPublicKeyInfo key bits).
    public_key: Vec<u8>,
    /// Modulus length in bytes, which is the authenticator length.
    modulus_len: usize,
}

/// Why a presented token confers no trust; static, never token bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// No verifier is configured, so nothing could be verified.
    NotConfigured,
    /// Header, encoding or token structure is malformed or oversize.
    Malformed,
    /// Token type is not 0x0002.
    TokenType,
    /// `token_key_id` names no configured issuer.
    UnknownIssuer,
    /// `challenge_digest` is not one the Runtime accepts.
    Challenge,
    /// RSA-PSS verification failed.
    BadSignature,
    /// Token nonce was already spent.
    DoubleSpend,
    /// Spent-nonce store is saturated; fail safe without an upgrade.
    Capacity,
}

/// Outcome of verifying one Private Access Token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// No token header.
    Absent,
    /// Valid, unspent token from a configured issuer.
    Verified {
        /// Lowercase hex `token_key_id` of the configured issuer.
        key_id: String,
    },
    /// Present but unverifiable; never a deny.
    Invalid {
        /// Static reason.
        reason: Reason,
        /// Hex key id only when it names a configured issuer.
        key_id: Option<String>,
    },
}

fn invalid(reason: Reason) -> Verdict {
    Verdict::Invalid {
        reason,
        key_id: None,
    }
}

/// Configured issuers plus the single-owner spent-nonce store.
pub struct Verifier {
    issuers: BTreeMap<[u8; 32], Issuer>,
    challenges: Option<BTreeSet<[u8; 32]>>,
    spent: HashMap<[u8; 32], Duration>,
    last_sweep: Duration,
}

impl Verifier {
    /// Parse PEM `PUBLIC KEY` blocks of RSA issuer keys (Go `NewPATVerifier`).
    pub fn from_pem(pem: &str) -> Result<Self, ConfigError> {
        let mut issuers = BTreeMap::new();
        for spki in pem_public_keys(pem)? {
            let issuer = rsa_issuer(&spki).ok_or(ConfigError::Key)?;
            if issuers
                .insert(Sha256::digest(&spki).into(), issuer)
                .is_some()
            {
                return Err(ConfigError::Duplicate);
            }
        }
        Ok(Self {
            issuers,
            challenges: None,
            spent: HashMap::new(),
            last_sweep: Duration::ZERO,
        })
    }

    /// Require `challenge_digest` to be SHA-256 of one of these challenges.
    pub fn accept_challenges(&mut self, digests: &[[u8; 32]]) -> Result<(), ConfigError> {
        if digests.is_empty() {
            return Err(ConfigError::Empty);
        }
        if digests.len() > MAX_CHALLENGES {
            return Err(ConfigError::Capacity);
        }
        self.challenges = Some(digests.iter().copied().collect());
        Ok(())
    }

    /// Verify a `PrivateToken token="..."` Authorization value at `now`.
    pub fn verify(&mut self, header: Option<&str>, now: Duration) -> Verdict {
        let Some(header) = header else {
            return Verdict::Absent;
        };
        let Some(raw) = token_bytes(header) else {
            return invalid(Reason::Malformed);
        };
        if raw.len() <= PREFIX {
            return invalid(Reason::Malformed);
        }
        if u16::from_be_bytes([raw[0], raw[1]]) != TOKEN_TYPE {
            return invalid(Reason::TokenType);
        }
        let key_id: [u8; 32] = raw[66..98].try_into().expect("slice length is 32");
        let Some(issuer) = self.issuers.get(&key_id) else {
            return invalid(Reason::UnknownIssuer);
        };
        let rejected = |reason| Verdict::Invalid {
            reason,
            key_id: Some(hex::encode(key_id)),
        };
        let digest: [u8; 32] = raw[34..66].try_into().expect("slice length is 32");
        if self
            .challenges
            .as_ref()
            .is_some_and(|accepted| !accepted.contains(&digest))
        {
            return rejected(Reason::Challenge);
        }
        let (input, authenticator) = raw.split_at(PREFIX);
        if authenticator.len() != issuer.modulus_len
            || !ring_verify(
                SignatureScheme::RSA_PSS_SHA384,
                alg_id::RSA_ENCRYPTION,
                &issuer.public_key,
                input,
                authenticator,
            )
        {
            return rejected(Reason::BadSignature);
        }
        let nonce: [u8; 32] = raw[2..34].try_into().expect("slice length is 32");
        if let Err(reason) = self.spend(nonce, now) {
            return rejected(reason);
        }
        Verdict::Verified {
            key_id: hex::encode(key_id),
        }
    }

    /// Record a nonce as spent with Go's amortized TTL sweep.
    fn spend(&mut self, nonce: [u8; 32], now: Duration) -> Result<(), Reason> {
        if now.saturating_sub(self.last_sweep) > SPENT_TTL / 4 || self.spent.len() >= MAX_SPENT {
            self.spent
                .retain(|_, seen| now.saturating_sub(*seen) <= SPENT_TTL);
            self.last_sweep = now;
        }
        if self.spent.contains_key(&nonce) {
            return Err(Reason::DoubleSpend);
        }
        if self.spent.len() >= MAX_SPENT {
            return Err(Reason::Capacity);
        }
        self.spent.insert(nonce, now);
        Ok(())
    }
}

/// Verify with an optional verifier; a presented token without a configured
/// verifier is refused, never passed (shipping row B25).
pub fn verify(verifier: Option<&mut Verifier>, header: Option<&str>, now: Duration) -> Verdict {
    match (verifier, header) {
        (_, None) => Verdict::Absent,
        (None, Some(_)) => invalid(Reason::NotConfigured),
        (Some(verifier), header) => verifier.verify(header, now),
    }
}

/// Validate an rsaEncryption SPKI and keep its RSAPublicKey bytes.
fn rsa_issuer(spki: &[u8]) -> Option<Issuer> {
    let (algorithm, key) = spki_parts(spki)?;
    if algorithm != RSA_ENCRYPTION {
        return None;
    }
    let mut outer = Der(key);
    let mut fields = Der(outer.read(0x30)?);
    outer.finish()?;
    let modulus = fields.positive_integer()?;
    let exponent = fields.positive_integer()?;
    fields.finish()?;
    let bits = modulus.len() * 8 - modulus[0].leading_zeros() as usize;
    let exponent_ok = exponent.len() <= 5
        && exponent.last().is_some_and(|b| b & 1 == 1)
        && exponent
            .iter()
            .fold(0u64, |acc, b| (acc << 8) | u64::from(*b))
            >= 3;
    ((2048..=8192).contains(&bits) && modulus[modulus.len() - 1] & 1 == 1 && exponent_ok).then(
        || Issuer {
            public_key: key.to_vec(),
            modulus_len: modulus.len(),
        },
    )
}

/// Extract and decode the token (Go `parsePrivateTokenHeader` plus
/// `decodeB64Loose`); the bare base64 form is tolerated as in Go.
fn token_bytes(header: &str) -> Option<Vec<u8>> {
    let header = header.trim();
    if header.is_empty() || header.len() > MAX_HEADER {
        return None;
    }
    let token = if let Some(at) = header.find("token=") {
        let value = header[at + "token=".len()..].trim().trim_matches('"');
        value
            .find([',', ';', ' '])
            .map_or(value, |end| &value[..end])
    } else if header.starts_with("PrivateToken") {
        return None;
    } else {
        header
    };
    decode_b64_loose(token)
}

#[cfg(test)]
pub(super) mod tests;
