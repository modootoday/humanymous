//! WebAuthn ES256 assertion verification for returning users (legacy
//! internal/gate/webauthn.go).
//!
//! A registered credential proves possession by an ECDSA P-256 / SHA-256
//! signature over `authenticatorData || SHA-256(clientDataJSON)`. A valid,
//! fresh assertion is a modest trust signal; anything else is never a deny.
//!
//! Anti-replay caveat (kept from Go): assertions are header-presented without
//! a server-issued per-request challenge, so replay protection rests entirely
//! on the signature counter. A zero or non-advancing counter is refused.
//! Binding the WebAuthn challenge to a Runtime-issued nonce is a documented gap.
//!
//! Deliberate fail-closed differences from Go, each pinned by tests:
//! - Only P-256 `ecPublicKey` SubjectPublicKeyInfo keys are accepted (ES256);
//!   Go also parsed other curves. Registry lines must be well formed, ids and
//!   keys unique, and at least one entry present.
//! - The authenticator data must assert user presence (flag bit 0), as the
//!   WebAuthn assertion procedure requires; Go did not check flags.
//! - JSON field names are matched exactly and duplicates are refused; Go's
//!   decoder matched names case-insensitively and kept the last duplicate.
//! - The header is bounded by `MAX_HEADER`; the credential id is only reported
//!   when it names a registered credential.
use super::{ConfigError, decode_b64_loose, parse_lines, pem_public_keys, ring_verify, spki_parts};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use rustls::SignatureScheme;
use rustls::pki_types::alg_id;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Bound on the assertion header value in bytes.
pub const MAX_HEADER: usize = 16 * 1024;
/// Minimum authenticator data: rpIdHash, flags and signCount.
const MIN_AUTH_DATA: usize = 37;
/// User-present flag bit.
const FLAG_UP: u8 = 0x01;
/// DER body of the ecPublicKey / prime256v1 AlgorithmIdentifier.
const EC_P256: &[u8] = &[
    0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d,
    0x03, 0x01, 0x07,
];

/// One registered credential.
struct Credential {
    /// Uncompressed SEC1 P-256 point.
    point: Vec<u8>,
    /// Highest signature counter seen.
    counter: u32,
}

/// Why an assertion confers no trust; static, never assertion bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// No registry is configured, so nothing could be verified.
    NotConfigured,
    /// Encoding, JSON or authenticator data shape is malformed or oversize.
    Malformed,
    /// Credential id is not registered.
    UnknownCredential,
    /// Client data is not a `webauthn.get` assertion.
    ClientDataType,
    /// Client data origin differs from the bound origin.
    Origin,
    /// rpIdHash differs from the bound relying party.
    RelyingParty,
    /// User-present flag is not set.
    UserPresence,
    /// ECDSA verification failed.
    BadSignature,
    /// Counter is zero or did not advance.
    Replay,
}

/// Outcome of verifying one assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// No assertion header.
    Absent,
    /// Valid, fresh assertion from a registered credential.
    Verified {
        /// Registered credential id.
        credential_id: String,
    },
    /// Present but unverifiable, replayed or unknown; never a deny.
    Invalid {
        /// Static reason.
        reason: Reason,
        /// Credential id only when it names a registered credential.
        credential_id: Option<String>,
    },
}

fn invalid(reason: Reason) -> Verdict {
    Verdict::Invalid {
        reason,
        credential_id: None,
    }
}

/// Client assertion as carried in the header (WebAuthn `get()` result).
#[derive(Deserialize)]
struct Assertion {
    #[serde(rename = "credentialId")]
    credential_id: String,
    #[serde(rename = "authenticatorData")]
    authenticator_data: String,
    #[serde(rename = "clientDataJSON")]
    client_data_json: String,
    signature: String,
}

/// The client data fields this verifier checks; others are ignored.
#[derive(Deserialize)]
struct ClientData {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    origin: String,
}

/// Registered credentials plus optional origin and RP-ID binding.
pub struct Registry {
    credentials: BTreeMap<String, Credential>,
    origin: Option<String>,
    rp_id_hash: Option<[u8; 32]>,
}

impl Registry {
    /// Parse `credentialId base64-spki` lines (Go `NewWebAuthnRegistry`).
    /// The value is base64url or standard base64 of SPKI DER or of a PEM block.
    pub fn parse(spec: &str) -> Result<Self, ConfigError> {
        let mut credentials = BTreeMap::new();
        for (id, value) in parse_lines(spec)? {
            let decoded = URL_SAFE_NO_PAD
                .decode(value)
                .or_else(|_| STANDARD.decode(value))
                .map_err(|_| ConfigError::Key)?;
            let der = match std::str::from_utf8(&decoded) {
                Ok(text) if text.trim_start().starts_with("-----BEGIN") => {
                    let mut blocks = pem_public_keys(text)?;
                    if blocks.len() != 1 {
                        return Err(ConfigError::Syntax);
                    }
                    blocks.remove(0)
                }
                _ => decoded,
            };
            let point = p256_point(&der).ok_or(ConfigError::Key)?;
            if credentials.values().any(|c: &Credential| c.point == point) {
                return Err(ConfigError::Duplicate);
            }
            let credential = Credential { point, counter: 0 };
            if credentials.insert(id.to_owned(), credential).is_some() {
                return Err(ConfigError::Duplicate);
            }
        }
        Ok(Self {
            credentials,
            origin: None,
            rp_id_hash: None,
        })
    }

    /// Bind assertions to an origin and RP ID; empty values leave a check off
    /// (Go `SetBinding`).
    pub fn set_binding(&mut self, origin: &str, rp_id: &str) {
        self.origin = (!origin.is_empty()).then(|| origin.to_owned());
        self.rp_id_hash = (!rp_id.is_empty()).then(|| Sha256::digest(rp_id.as_bytes()).into());
    }

    /// Verify a base64(JSON) assertion header and advance its counter.
    pub fn verify(&mut self, header: Option<&str>) -> Verdict {
        let Some(header) = header else {
            return Verdict::Absent;
        };
        let header = header.trim();
        if header.is_empty() || header.len() > MAX_HEADER {
            return invalid(Reason::Malformed);
        }
        let Some(assertion) = decode_b64_loose(header)
            .and_then(|json| serde_json::from_slice::<Assertion>(&json).ok())
        else {
            return invalid(Reason::Malformed);
        };
        let Some((id, credential)) = self.credentials.get_key_value(&assertion.credential_id)
        else {
            return invalid(Reason::UnknownCredential);
        };
        let rejected = |reason| Verdict::Invalid {
            reason,
            credential_id: Some(id.clone()),
        };
        let (Ok(auth_data), Ok(client_data), Ok(signature)) = (
            STANDARD.decode(&assertion.authenticator_data),
            STANDARD.decode(&assertion.client_data_json),
            STANDARD.decode(&assertion.signature),
        ) else {
            return rejected(Reason::Malformed);
        };
        if auth_data.len() < MIN_AUTH_DATA {
            return rejected(Reason::Malformed);
        }
        let Ok(client) = serde_json::from_slice::<ClientData>(&client_data) else {
            return rejected(Reason::Malformed);
        };
        if client.kind != "webauthn.get" {
            return rejected(Reason::ClientDataType);
        }
        if self
            .origin
            .as_ref()
            .is_some_and(|origin| *origin != client.origin)
        {
            return rejected(Reason::Origin);
        }
        if self.rp_id_hash.is_some_and(|hash| auth_data[..32] != hash) {
            return rejected(Reason::RelyingParty);
        }
        if auth_data[32] & FLAG_UP == 0 {
            return rejected(Reason::UserPresence);
        }
        let mut signed = auth_data.clone();
        signed.extend_from_slice(&Sha256::digest(&client_data));
        if !ring_verify(
            SignatureScheme::ECDSA_NISTP256_SHA256,
            alg_id::ECDSA_P256,
            &credential.point,
            &signed,
            &signature,
        ) {
            return rejected(Reason::BadSignature);
        }
        let counter =
            u32::from_be_bytes([auth_data[33], auth_data[34], auth_data[35], auth_data[36]]);
        if counter == 0 || counter <= credential.counter {
            return rejected(Reason::Replay);
        }
        let id = id.clone();
        if let Some(credential) = self.credentials.get_mut(&id) {
            credential.counter = counter;
        }
        Verdict::Verified { credential_id: id }
    }
}

/// Validate a P-256 ecPublicKey SPKI and return its uncompressed point.
fn p256_point(spki: &[u8]) -> Option<Vec<u8>> {
    let (algorithm, key) = spki_parts(spki)?;
    (algorithm == EC_P256 && key.len() == 65 && key[0] == 0x04).then(|| key.to_vec())
}

/// Verify with an optional registry; a presented assertion without a
/// configured registry is refused, never passed (shipping row B25).
pub fn verify(registry: Option<&mut Registry>, header: Option<&str>) -> Verdict {
    match (registry, header) {
        (_, None) => Verdict::Absent,
        (None, Some(_)) => invalid(Reason::NotConfigured),
        (Some(registry), header) => registry.verify(header),
    }
}

#[cfg(test)]
pub(in crate::gateway) mod tests;
