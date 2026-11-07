//! Actual retained resources and keys; private read-only authentication only.
//! No result exposes a writer, repair, witness acknowledgement or Ready state.
use super::{NativeFiles, NativeSource};
use crate::audit::{
    checkpoint::Checkpoint,
    keys::{KeyError, KeyMaterial, custody::Custody},
    storage::RecoveryPolicy,
    verification::scan::{
        Diagnostic,
        limits::{Control, Limits, invalid},
    },
};
use std::{io, sync::Arc};

struct Trust {
    node: String,
    witness: Option<[u8; 32]>,
    head: Option<Checkpoint>,
}

struct OpeningOwner {
    files: Arc<NativeFiles>,
    material: KeyMaterial,
    trust: Trust,
    // Retain key custody until all owned material/resources have been dropped.
    _custody: Custody,
}
struct VerifiedScan<'op, 'cancel> {
    owner: OpeningOwner,
    limits: &'op Limits,
    control: &'op Control<'cancel>,
    diagnostic: Diagnostic,
}
struct ReadOnlyOwner {
    owner: OpeningOwner,
    diagnostic: Diagnostic,
}

impl OpeningOwner {
    // The caller already holds the actual native WAL resources and key custody.
    // Only load an existing sealed identity; never provision from this path.
    fn existing(
        mut custody: Custody,
        passphrase: &[u8],
        files: Arc<NativeFiles>,
        trust: Trust,
    ) -> io::Result<Self> {
        let (material, _) =
            custody.load_or_create(passphrase, || Err(KeyError::InvalidMaterial))?;
        if trust.node.is_empty() {
            return Err(invalid("missing owned audit node"));
        }
        let writer = ed25519_dalek::SigningKey::from_bytes(material.signing_seed())
            .verifying_key()
            .to_bytes();
        if trust.witness.as_ref() == Some(&writer) {
            return Err(invalid("owned audit witness must be independent"));
        }
        Ok(Self {
            files,
            material,
            trust,
            _custody: custody,
        })
    }

    fn inspect(&self, limits: &Limits, control: &Control<'_>) -> io::Result<Diagnostic> {
        let writer = ed25519_dalek::SigningKey::from_bytes(self.material.signing_seed())
            .verifying_key()
            .to_bytes();
        // Borrow owned secrets only during the complete scanner call. No policy
        // borrowing moved key fields is stored beside the owning material.
        let policy = RecoveryPolicy {
            node_id: &self.trust.node,
            hmac_key: self.material.hmac_key(),
            writer_public: &writer,
            witness_public: self.trust.witness.as_ref(),
            trusted_head: self.trust.head.as_ref(),
            // This slice authenticates existing physical history only. Empty
            // initialization needs its separately accepted bootstrap contract.
            allow_new: false,
        };
        let source = NativeSource::operation(Arc::clone(&self.files), limits, control)?;
        let diagnostic =
            crate::audit::verification::scan::inspect(&source, &policy, limits, control)?;
        // Require every operation reader to be gone before moving the owner.
        drop(source.into_files()?);
        if !diagnostic.report.as_ref().is_some_and(|report| report.ok) {
            return Err(invalid("owned audit authentication failed"));
        }
        control.check()?;
        Ok(diagnostic)
    }

    fn authenticate<'op, 'cancel>(
        self,
        limits: &'op Limits,
        control: &'op Control<'cancel>,
    ) -> io::Result<VerifiedScan<'op, 'cancel>> {
        let diagnostic = self.inspect(limits, control)?;
        Ok(VerifiedScan {
            owner: self,
            limits,
            control,
            diagnostic,
        })
    }
}

impl VerifiedScan<'_, '_> {
    fn finalize(self) -> io::Result<ReadOnlyOwner> {
        // No replacement arguments. Repeat full authenticated EOF with the SAME
        // resources, keys and original absolute bounds before this read-only move.
        self.control.check()?;
        let diagnostic = self.owner.inspect(self.limits, self.control)?;
        if diagnostic.report != self.diagnostic.report || diagnostic.work != self.diagnostic.work {
            return Err(invalid(
                "owned audit diagnostic changed before finalization",
            ));
        }
        self.control.check()?;
        Ok(ReadOnlyOwner {
            owner: self.owner,
            diagnostic,
        })
    }
}

#[cfg(test)]
pub(super) mod tests;
