//! Destination-bound rotation without retaining plaintext material or passwords.
use super::{Custody, Durability, crypto, invalid};
use crate::control::{Action, ControlReceipt, RetainedActionPermit, Scope, target_commitment};
use sha2::{Digest, Sha256};
use std::io;
use zeroize::Zeroizing;

const DOMAIN: &[u8] = b"humanymous/prepared-custody-rotation/v1\0";

/// Immutable encrypted rotation candidate. No plaintext material, credential,
/// public constructor, serializer, clone or Debug implementation is exposed.
pub struct PreparedRotation {
    source_digest: String,
    target: String,
    ciphertext: Zeroizing<Vec<u8>>,
}
impl PreparedRotation {
    /// Exact normal proposal target commitment signed by both administrators.
    pub fn target(&self) -> &str {
        &self.target
    }
}
fn target(source: &str, candidate: &[u8]) -> String {
    let mut descriptor = Vec::with_capacity(DOMAIN.len() + 128);
    descriptor.extend_from_slice(DOMAIN);
    descriptor.extend_from_slice(source.as_bytes());
    descriptor.extend_from_slice(hex::encode(Sha256::digest(candidate)).as_bytes());
    target_commitment(&descriptor)
}
impl Custody {
    /// Authenticate and encrypt the complete current snapshot before approval.
    /// Borrowed credentials remain caller-owned; neither is retained here.
    pub fn prepare_rotation(
        &mut self,
        old_passphrase: &[u8],
        new_passphrase: &[u8],
    ) -> io::Result<PreparedRotation> {
        self.ready()?;
        if self.exists("keys.pending")? {
            return Err(invalid("pending custody transaction requires recovery"));
        }
        let material = self.read("keys.sealed", old_passphrase)?;
        let source_digest = self.committed_digest()?;
        let ciphertext =
            Zeroizing::new(crate::keys::seal(&material, new_passphrase).map_err(crypto)?);
        drop(material);
        let target = target(&source_digest, &ciphertext);
        Ok(PreparedRotation {
            source_digest,
            target,
            ciphertext,
        })
    }
    /// Apply precisely the approved encrypted candidate after independent
    /// retention. The caller supplies provisioned scope and trusted service time.
    /// No re-encryption, new identity, grant reissue or automatic retry occurs.
    pub fn apply_prepared_rotation(
        &mut self,
        prepared: PreparedRotation,
        grant: RetainedActionPermit,
        scope: &Scope,
        now: u64,
    ) -> io::Result<(Durability, ControlReceipt)> {
        self.ready()?;
        if self.exists("keys.pending")? || self.committed_digest()? != prepared.source_digest {
            return Err(invalid("prepared rotation source generation mismatch"));
        }
        grant.execute(
            scope,
            Action::RotatePassphrase,
            &prepared.target,
            now,
            |receipt| {
                let durability = self.commit_envelope(&prepared.ciphertext)?;
                Ok((durability, receipt.clone()))
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prepared_target_binds_both_ciphertexts_and_differs_from_generation_only_approval() {
        let source = hex::encode([1; 32]);
        let candidate = b"synthetic-encrypted-candidate";
        let commitment = target(&source, candidate);
        assert_eq!(commitment, target(&source, candidate));
        assert_ne!(commitment, target(&hex::encode([2; 32]), candidate));
        assert_ne!(
            commitment,
            target(&source, b"different-encrypted-candidate")
        );
        assert_ne!(commitment, target_commitment(source.as_bytes()));
    }
}
