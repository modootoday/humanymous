//! Lab bundle identity lifecycle: stage, promote and roll back.
//!
//! Stage hashes, parses and schema-checks ONE bounded immutable byte buffer,
//! so the verified bytes are the parsed bytes (no path re-read). Promote and
//! rollback move only the lab identities (staged, active, previous).
//!
//! Deliberate deltas from legacy: transitions never call an inference
//! `Runtime` setter, so no model is activated; production activation requires a
//! separately accepted design. This package has no Ed25519 implementation, so
//! the signed policy takes a caller-supplied [`BundleVerifier`] via
//! [`BundleManager::with_verifier`] (the Runtime supplies the Ed25519 one);
//! [`BundleManager::with_public_key`] still fails closed here. The unsigned lab
//! policy is explicit via [`BundleManager::unsigned_lab`]. A digest pin is
//! integrity only, not an authenticated supply chain.
use super::{CorrectionError, lock};
use crate::behavior::features::schema_hash;
use crate::model::inference::{Mlp, Predictor};
use sha2::{Digest, Sha256};
use std::sync::Mutex;

/// Largest bundle buffer accepted by [`BundleManager::stage`].
pub const MAX_BUNDLE_BYTES: usize = 1 << 20;

/// Canonical bundle digest: lowercase hex SHA-256 of the exact bytes.
pub fn digest(bundle_bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bundle_bytes))
}

/// Audit and rollback identity of a verified bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleIdentity {
    /// Artifact version recorded inside the bundle.
    pub version: String,
    /// Lowercase hex SHA-256 of the staged bytes.
    pub digest: String,
}

#[derive(Debug, Default)]
struct Lifecycle {
    active: Option<BundleIdentity>,
    previous: Option<BundleIdentity>,
    staged: Option<BundleIdentity>,
}

/// Detached-signature check over the exact bundle bytes (legacy
/// `ed25519.Verify(pubkey, raw, sig)`). Implementations must be pure and
/// return `false` for any malformed signature.
pub trait BundleVerifier: Send + Sync {
    /// Whether `signature` authenticates exactly `bundle_bytes`.
    fn verify(&self, bundle_bytes: &[u8], signature: &[u8]) -> bool;
}

/// Lab lifecycle manager. Safe for concurrent use; every transition is atomic.
#[derive(Default)]
pub struct BundleManager {
    state: Mutex<Lifecycle>,
    verifier: Option<Box<dyn BundleVerifier>>,
}

impl std::fmt::Debug for BundleManager {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BundleManager")
            .field("state", &self.state)
            .field("signed", &self.signed())
            .finish()
    }
}

impl BundleManager {
    /// Explicit unsigned lab policy: schema pin and optional digest pin only.
    pub fn unsigned_lab() -> Self {
        Self::default()
    }

    /// Signed policy request. Always fails closed with `SignatureUnsupported`
    /// because Ed25519 verification is not available here; legacy accepted a
    /// PKIX PEM key and then required a valid signature to stage.
    pub fn with_public_key(_public_key_pem: &[u8]) -> Result<Self, CorrectionError> {
        Err(CorrectionError::SignatureUnsupported)
    }

    /// Signed policy: every stage requires a detached signature that
    /// `verifier` accepts over the exact staged bytes (legacy behavior once a
    /// public key was configured).
    pub fn with_verifier(verifier: Box<dyn BundleVerifier>) -> Self {
        Self {
            state: Mutex::default(),
            verifier: Some(verifier),
        }
    }

    /// Whether a signature is required to stage.
    pub fn signed(&self) -> bool {
        self.verifier.is_some()
    }

    /// Verify and stage a candidate without serving it: size bound, optional
    /// exact digest pin, shape-valid decode and running-schema match.
    ///
    /// Stricter than legacy: empty or oversized buffers are `Limit` errors and
    /// a pinned digest must match exactly (an empty pin is a mismatch rather
    /// than "no pin"; pass `None` for no pin).
    pub fn stage(
        &self,
        bundle_bytes: &[u8],
        expected_digest: Option<&str>,
    ) -> Result<BundleIdentity, CorrectionError> {
        self.stage_signed(bundle_bytes, expected_digest, None)
    }

    /// [`Self::stage`] with an optional detached signature, checked in the
    /// legacy order: size, digest pin, signature, shape decode, schema pin.
    /// Under the signed policy a missing or invalid signature is a
    /// `Signature` error; under the unsigned lab policy it is ignored, as in
    /// legacy when no public key was configured.
    pub fn stage_signed(
        &self,
        bundle_bytes: &[u8],
        expected_digest: Option<&str>,
        signature: Option<&[u8]>,
    ) -> Result<BundleIdentity, CorrectionError> {
        if bundle_bytes.is_empty() || bundle_bytes.len() > MAX_BUNDLE_BYTES {
            return Err(CorrectionError::Limit);
        }
        let digest = digest(bundle_bytes);
        if expected_digest.is_some_and(|expected| expected != digest) {
            return Err(CorrectionError::Digest);
        }
        if let Some(verifier) = &self.verifier
            && !signature.is_some_and(|signature| verifier.verify(bundle_bytes, signature))
        {
            return Err(CorrectionError::Signature);
        }
        let model = Mlp::from_json(bundle_bytes).map_err(|_| CorrectionError::InvalidBundle)?;
        if model.schema_hash() != schema_hash() {
            return Err(CorrectionError::StaleSchema);
        }
        let identity = BundleIdentity {
            version: model.bundle_version().to_owned(),
            digest,
        };
        lock(&self.state).staged = Some(identity.clone());
        Ok(identity)
    }

    /// Verified candidate awaiting promotion, if any.
    pub fn staged(&self) -> Option<BundleIdentity> {
        lock(&self.state).staged.clone()
    }

    /// Move the staged identity to active, retaining the prior active one.
    pub fn promote(&self) -> Result<BundleIdentity, CorrectionError> {
        let mut state = lock(&self.state);
        let staged = state.staged.take().ok_or(CorrectionError::NoStaged)?;
        state.previous = state.active.replace(staged.clone());
        Ok(staged)
    }

    /// Revert to the previous identity, or to no model when none was retained.
    /// Returns the identity now active.
    pub fn rollback(&self) -> Result<Option<BundleIdentity>, CorrectionError> {
        let mut state = lock(&self.state);
        if state.active.is_none() && state.previous.is_none() {
            return Err(CorrectionError::NoPrevious);
        }
        state.active = state.previous.take();
        Ok(state.active.clone())
    }

    /// Lab-active identity; `None` corresponds to legacy `"none"` abstention.
    pub fn active(&self) -> Option<BundleIdentity> {
        lock(&self.state).active.clone()
    }
}

#[cfg(test)]
mod tests;
