//! Signed tree heads cover both linear-chain and RFC 6962 Merkle commitments.
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use humanymous_core::signals::wire::decode::null_default;
use serde::{Deserialize, Serialize};
mod signing;
mod witness;
pub use witness::{Witness, WitnessError};
#[cfg(any(unix, windows))]
mod durable;
#[cfg(any(unix, windows))]
pub use durable::{CheckpointWitness, DurableWitness, WitnessBootstrap, WitnessReceipt, service};

/// Persisted signed tree head. Writer and independent witness sign the same
/// frozen bytes; neither signature is itself included in those bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Checkpoint {
    /// Node owning the chain.
    #[serde(deserialize_with = "null_default")]
    pub node_id: String,
    /// Number of records covered.
    #[serde(deserialize_with = "null_default")]
    pub tree_size: u64,
    /// Last linear-chain record hash.
    #[serde(deserialize_with = "null_default")]
    pub root: String,
    /// RFC 6962 root over record-hash text bytes.
    #[serde(deserialize_with = "null_default")]
    pub merkle_root: String,
    /// Previous writer signature, despite the legacy field's hash name.
    #[serde(deserialize_with = "null_default")]
    pub prev_checkpoint_hash: String,
    /// Hex Ed25519 writer signature.
    #[serde(deserialize_with = "null_default")]
    pub sig: String,
    /// Hex Ed25519 independent witness signature.
    #[serde(deserialize_with = "null_default")]
    pub witness_sig: String,
}

impl Checkpoint {
    /// Fixed-order Go JSON bytes, including its HTML/U+2028/U+2029 escaping.
    /// These strings are not NFC-normalized by the checkpoint wire contract.
    pub fn signing_bytes(&self) -> Vec<u8> {
        signing::bounded(self, usize::MAX)
            .expect("in-memory checkpoint signing bytes fit address space")
    }

    /// Seal the writer commitment and discard any now-stale witness signature.
    pub fn sign(&mut self, key: &SigningKey) {
        self.sig = hex::encode(key.sign(&self.signing_bytes()).to_bytes());
        self.witness_sig.clear();
    }

    /// Verify writer signature with a public key only, rejecting malformed and
    /// weak/noncanonical signatures through strict Ed25519 validation.
    pub fn verify_writer(&self, public: &[u8; 32]) -> bool {
        self.verify_signature(public, &self.sig)
    }

    /// Verify an existing witness signature. This alone does not prove that
    /// the witness followed an append-only policy or lives independently.
    pub fn verify_witness(&self, public: &[u8; 32]) -> bool {
        self.verify_signature(public, &self.witness_sig)
    }

    fn verify_signature(&self, public: &[u8; 32], encoded: &str) -> bool {
        self.verify_signature_bounded(public, encoded, usize::MAX)
            .unwrap_or(false)
    }

    pub(crate) fn verify_signature_bounded(
        &self,
        public: &[u8; 32],
        encoded: &str,
        limit: usize,
    ) -> std::io::Result<bool> {
        let Ok(key) = VerifyingKey::from_bytes(public) else {
            return Ok(false);
        };
        let mut bytes = [0; 64];
        if hex::decode_to_slice(encoded, &mut bytes).is_err() {
            return Ok(false);
        }
        let Ok(signature) = Signature::from_slice(&bytes) else {
            return Ok(false);
        };
        let message = signing::bounded(self, limit)?;
        Ok(key.verify_strict(&message, &signature).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signatures_cover_both_roots_and_legacy_json_escaping() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let mut checkpoint = Checkpoint {
            node_id: "<&>\u{2028}\u{2029}".into(),
            tree_size: 1,
            root: "linear".into(),
            merkle_root: "tree".into(),
            ..Default::default()
        };
        assert!(
            String::from_utf8(checkpoint.signing_bytes())
                .unwrap()
                .contains("\\u003c\\u0026\\u003e\\u2028\\u2029")
        );
        checkpoint.sign(&key);
        let public = key.verifying_key().to_bytes();
        assert!(checkpoint.verify_writer(&public));
        checkpoint.witness_sig = checkpoint.sig.clone();
        assert!(checkpoint.verify_witness(&public));
        checkpoint.merkle_root.push('x');
        assert!(!checkpoint.verify_writer(&public));
        checkpoint.sign(&key);
        assert!(checkpoint.witness_sig.is_empty());
        checkpoint.sig = "invalid".into();
        assert!(!checkpoint.verify_writer(&public));
    }
}
