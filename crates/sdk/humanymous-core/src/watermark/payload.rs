//! Exact legacy-v1 HMAC input and carrier layout, with explicit issuance checks.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::watermark::Error;

/// Truncated legacy-v1 HMAC length in bytes (80 bits).
pub const TAG_LEN: usize = 10;
/// Big-endian u32 ID followed by the ten-byte tag.
pub const CARRIER_LEN: usize = 4 + TAG_LEN;
/// Maximum UTF-8 length of each new session/asset identifier.
pub const MAX_IDENTIFIER_BYTES: usize = 1024;
/// Upper bound on owned secret material; not a minimum-entropy assertion.
pub const MAX_KEY_BYTES: usize = 4096;

/// Issuance context. IDs are public lookup keys and are not bound by the v1 MAC.
#[derive(Clone, Eq, PartialEq)]
pub struct Payload {
    /// Nonzero, non-reused ledger identifier for new issuance.
    pub tag_id: u32,
    /// Trusted runtime session identifier; never a client identity assertion.
    pub session_id: String,
    /// Canonical asset identifier supplied by the owning runtime.
    pub asset_id: String,
    /// Low-resolution issue time bound by the legacy MAC.
    pub bucket: u64,
}

impl Payload {
    /// Validate a new issuance without changing legacy verification semantics.
    pub fn validate_issuance(&self) -> Result<(), Error> {
        if self.tag_id == 0
            || [&self.session_id, &self.asset_id]
                .iter()
                .any(|value| value.len() > MAX_IDENTIFIER_BYTES || value.contains('\0'))
        {
            return Err(Error::InvalidPayload);
        }
        Ok(())
    }
}

/// Owned, zeroizing server secret. Debug and Clone intentionally are absent.
pub struct WatermarkKey(Zeroizing<Vec<u8>>);

impl WatermarkKey {
    /// Adopt secret bytes and erase them on destruction, including rejected input.
    /// The caller remains responsible for copies created before this transfer.
    pub fn new(bytes: Vec<u8>) -> Result<Self, Error> {
        let bytes = Zeroizing::new(bytes);
        if bytes.is_empty() || bytes.len() > MAX_KEY_BYTES {
            return Err(Error::InvalidKey);
        }
        Ok(Self(bytes))
    }

    /// Build exact v1 bytes after validating new-issuance identifiers.
    pub fn carrier(&self, payload: &Payload) -> Result<[u8; CARRIER_LEN], Error> {
        payload.validate_issuance()?;
        let mut carrier = [0; CARRIER_LEN];
        carrier[..4].copy_from_slice(&payload.tag_id.to_be_bytes());
        carrier[4..].copy_from_slice(&self.tag(payload)?);
        Ok(carrier)
    }

    /// Check a retained v1 record's tag in constant time.
    ///
    /// This deliberately permits legacy NUL-containing identifiers. New issuance
    /// rejects them. The MAC omits tag_id; callers must enforce ledger ownership.
    /// This does not authenticate raw identifiers received from a client.
    pub fn verify_legacy(&self, payload: &Payload, tag: &[u8]) -> Result<bool, Error> {
        if payload.session_id.len() > MAX_IDENTIFIER_BYTES
            || payload.asset_id.len() > MAX_IDENTIFIER_BYTES
        {
            return Err(Error::InvalidPayload);
        }
        let expected = self.tag(payload)?;
        Ok(bool::from(expected.as_slice().ct_eq(tag)))
    }

    fn tag(&self, payload: &Payload) -> Result<[u8; TAG_LEN], Error> {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(self.0.as_slice()).map_err(|_| Error::InvalidKey)?;
        mac.update(payload.session_id.as_bytes());
        mac.update(&[0]);
        mac.update(payload.asset_id.as_bytes());
        mac.update(&[0]);
        mac.update(&payload.bucket.to_be_bytes());
        let digest = mac.finalize().into_bytes();
        let mut tag = [0; TAG_LEN];
        tag.copy_from_slice(&digest[..TAG_LEN]);
        Ok(tag)
    }
}

/// Borrowed interpretation of a recovered legacy carrier.
#[derive(Debug, Eq, PartialEq)]
pub struct Carrier<'a> {
    /// Public ledger lookup identifier, not independently authenticated.
    pub tag_id: u32,
    /// Exactly ten bytes to verify against the corresponding retained record.
    pub tag: &'a [u8],
    /// Whether the explicit compatibility path discarded historical suffix bytes.
    pub ignored_suffix: bool,
}

/// Parse exactly fourteen bytes for new, unambiguous extraction.
pub fn split_carrier(bytes: &[u8]) -> Result<Carrier<'_>, Error> {
    if bytes.len() != CARRIER_LEN {
        return Err(Error::InvalidCarrier);
    }
    split_legacy_carrier(bytes)
}

/// Preserve historical SplitCarrier suffix tolerance, visibly marked to callers.
pub fn split_legacy_carrier(bytes: &[u8]) -> Result<Carrier<'_>, Error> {
    if bytes.len() < CARRIER_LEN {
        return Err(Error::InvalidCarrier);
    }
    Ok(Carrier {
        tag_id: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        tag: &bytes[4..CARRIER_LEN],
        ignored_suffix: bytes.len() > CARRIER_LEN,
    })
}

#[cfg(test)]
mod tests;
