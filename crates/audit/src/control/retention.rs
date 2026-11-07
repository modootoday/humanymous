//! Independent storage boundary for the original signed control-ledger anchor.
use super::{LedgerAnchor, Scope};
use crate::{
    keys::custody::{Durability, Owner},
    storage::PrivateState,
};
use serde::{Deserialize, Serialize};
use std::{io, path::Path};

/// Persistence required before issuing an externally retained action grant.
#[derive(Clone, Copy)]
pub enum RetentionRequirement {
    /// Accept explicit file-sync strength, including Windows namespace uncertainty.
    FileSynced,
    /// Require successful file and directory sync; Windows cannot satisfy this.
    FileAndDirectory,
}
impl RetentionRequirement {
    pub(crate) fn accepts(self, actual: Durability) -> bool {
        matches!(self, Self::FileSynced) || actual == Durability::FileAndDirectory
    }
}
/// Exact durable retention acknowledgement, distinct from action completion.
pub struct RetainedControlAnchor {
    scope: Scope,
    authority_public: [u8; 32],
    anchor: LedgerAnchor,
    durability: Durability,
}
impl RetainedControlAnchor {
    /// Validate a trusted authenticated service's acknowledgement. This checks
    /// signed identity and bytes, not the peer's claim to have synced storage.
    pub fn from_acknowledgement(
        scope: Scope,
        authority_public: [u8; 32],
        expected: &LedgerAnchor,
        acknowledged: LedgerAnchor,
        durability: Durability,
    ) -> io::Result<Self> {
        super::policy::valid_scope(&scope).map_err(|_| invalid())?;
        if acknowledged != *expected || !acknowledged.verify(&authority_public) {
            return Err(invalid());
        }
        Ok(Self {
            scope,
            authority_public,
            anchor: acknowledged,
            durability,
        })
    }
    /// Original signed authority anchor retained by the independent peer.
    pub fn anchor(&self) -> &LedgerAnchor {
        &self.anchor
    }
    /// Actual reported persistence strength; no stronger guarantee is inferred.
    pub fn durability(&self) -> Durability {
        self.durability
    }
    pub(crate) fn matches(&self, scope: &Scope, public: &[u8; 32], anchor: &LedgerAnchor) -> bool {
        &self.scope == scope && &self.authority_public == public && &self.anchor == anchor
    }
}
/// Trusted independently authenticated storage adapter. Implementations must
/// persist before acknowledgement; local construction does not prove isolation.
pub trait ControlAnchorRetainer {
    /// Provisioned scope and authority key must have a one-to-one association.
    fn scope(&self) -> &Scope;
    /// Original authority verification key; retention needs no signing seed.
    fn authority_public(&self) -> [u8; 32];
    /// Read the authenticated latest independently retained anchor.
    fn current(&mut self) -> io::Result<LedgerAnchor>;
    /// Compare exact parent, verify revision+1, persist, then acknowledge.
    fn retain(
        &mut self,
        previous: &LedgerAnchor,
        next: &LedgerAnchor,
    ) -> io::Result<RetainedControlAnchor>;
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    scope: Scope,
    authority_public: [u8; 32],
    anchor: LedgerAnchor,
}
/// Verification-only native owner intended for the independent service process.
/// Its filesystem and operating-system owner must be provisioned independently.
pub struct NativeAnchorRetainer {
    store: PrivateState,
    scope: Scope,
    public: [u8; 32],
    anchor: LedgerAnchor,
}
fn invalid() -> io::Error {
    io::Error::other("control anchor retention authentication or transition failure")
}
pub(super) fn transition(
    public: &[u8; 32],
    previous: &LedgerAnchor,
    next: &LedgerAnchor,
) -> io::Result<()> {
    if !previous.verify(public)
        || !next.verify(public)
        || previous.revision.checked_add(1) != Some(next.revision)
    {
        return Err(invalid());
    }
    Ok(())
}
impl NativeAnchorRetainer {
    /// Promote only a signed single transition from the exact external old pin
    /// to an independently confirmed pending pin. Never discard pending state.
    pub fn recover_pending(
        directory: &Path,
        owner: Owner,
        scope: Scope,
        public: [u8; 32],
        previous: &LedgerAnchor,
        next: &LedgerAnchor,
    ) -> io::Result<(Self, RetainedControlAnchor)> {
        super::policy::valid_scope(&scope).map_err(|_| invalid())?;
        transition(&public, previous, next)?;
        let mut store = PrivateState::open(directory, owner, "control-anchor")?;
        let old: Snapshot = serde_json::from_slice(&store.committed()?.ok_or_else(invalid)?)
            .map_err(|_| invalid())?;
        let pending: Snapshot = serde_json::from_slice(&store.pending()?.ok_or_else(invalid)?)
            .map_err(|_| invalid())?;
        if old.scope != scope
            || pending.scope != scope
            || old.authority_public != public
            || pending.authority_public != public
            || old.anchor != *previous
            || pending.anchor != *next
        {
            return Err(invalid());
        }
        let durability = store.promote_pending()?;
        let receipt = RetainedControlAnchor::from_acknowledgement(
            scope.clone(),
            public,
            next,
            next.clone(),
            durability,
        )?;
        Ok((
            Self {
                store,
                scope,
                public,
                anchor: next.clone(),
            },
            receipt,
        ))
    }
    /// Explicitly provision revision zero, or restore exact independently pinned
    /// latest state. Existing state never becomes fresh merely by opening it.
    pub fn open(
        directory: &Path,
        owner: Owner,
        scope: Scope,
        public: [u8; 32],
        pin: &LedgerAnchor,
        provision: bool,
    ) -> io::Result<Self> {
        super::policy::valid_scope(&scope).map_err(|_| invalid())?;
        if !pin.verify(&public) {
            return Err(invalid());
        }
        let mut store = PrivateState::open(directory, owner, "control-anchor")?;
        match store.read()? {
            None if provision && pin.revision == 0 => {
                let snapshot = Snapshot {
                    scope: scope.clone(),
                    authority_public: public,
                    anchor: pin.clone(),
                };
                store.commit(&serde_json::to_vec(&snapshot).map_err(|_| invalid())?)?;
            }
            Some(bytes) if !provision => {
                let saved: Snapshot = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
                if saved.scope != scope || saved.authority_public != public || saved.anchor != *pin
                {
                    return Err(invalid());
                }
            }
            _ => return Err(invalid()),
        }
        Ok(Self {
            store,
            scope,
            public,
            anchor: pin.clone(),
        })
    }
}

impl ControlAnchorRetainer for NativeAnchorRetainer {
    fn scope(&self) -> &Scope {
        &self.scope
    }
    fn authority_public(&self) -> [u8; 32] {
        self.public
    }
    fn current(&mut self) -> io::Result<LedgerAnchor> {
        let bytes = self.store.read()?.ok_or_else(invalid)?;
        let saved: Snapshot = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
        if saved.scope != self.scope
            || saved.authority_public != self.public
            || saved.anchor != self.anchor
            || !saved.anchor.verify(&self.public)
        {
            return Err(invalid());
        }
        Ok(saved.anchor)
    }
    fn retain(
        &mut self,
        previous: &LedgerAnchor,
        next: &LedgerAnchor,
    ) -> io::Result<RetainedControlAnchor> {
        if self.current()? != *previous {
            return Err(invalid());
        }
        transition(&self.public, previous, next)?;
        let snapshot = Snapshot {
            scope: self.scope.clone(),
            authority_public: self.public,
            anchor: next.clone(),
        };
        let durability = self
            .store
            .commit(&serde_json::to_vec(&snapshot).map_err(|_| invalid())?)?;
        self.anchor = next.clone();
        RetainedControlAnchor::from_acknowledgement(
            self.scope.clone(),
            self.public,
            next,
            next.clone(),
            durability,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn anchor(key: &SigningKey, revision: u64, byte: u8) -> LedgerAnchor {
        let digest = hex::encode([byte; 32]);
        let bytes = format!("humanymous/control-ledger/v1\0{revision}\0{digest}");
        LedgerAnchor {
            revision,
            digest,
            signature: hex::encode(key.sign(bytes.as_bytes()).to_bytes()),
        }
    }
    fn scope() -> Scope {
        Scope {
            installation: "retention-test".into(),
            node: "node".into(),
        }
    }

    #[test]
    fn signed_transition_requires_exact_successor_and_identity() {
        let key = SigningKey::from_bytes(&[47; 32]);
        let public = key.verifying_key().to_bytes();
        let previous = anchor(&key, 7, 1);
        assert!(transition(&public, &previous, &anchor(&key, 8, 2)).is_ok());
        for revision in [0, 6, 7, 9, u64::MAX] {
            assert!(transition(&public, &previous, &anchor(&key, revision, 2)).is_err());
        }
        let other = SigningKey::from_bytes(&[48; 32]);
        assert!(transition(&public, &previous, &anchor(&other, 8, 2)).is_err());
        assert!(transition(&public, &anchor(&key, u64::MAX, 1), &anchor(&key, 0, 2)).is_err());
    }

    #[test]
    fn acknowledgement_requires_exact_signed_bytes_and_scope_binding() {
        let key = SigningKey::from_bytes(&[49; 32]);
        let public = key.verifying_key().to_bytes();
        let expected = anchor(&key, 1, 1);
        let receipt = RetainedControlAnchor::from_acknowledgement(
            scope(),
            public,
            &expected,
            expected.clone(),
            Durability::FileAndDirectory,
        )
        .expect("exact authenticated ack");
        assert!(receipt.matches(&scope(), &public, &expected));
        let mut wrong_scope = scope();
        wrong_scope.node = "other".into();
        assert!(!receipt.matches(&wrong_scope, &public, &expected));
        assert!(
            RetainedControlAnchor::from_acknowledgement(
                scope(),
                public,
                &expected,
                anchor(&key, 1, 2),
                Durability::FileAndDirectory
            )
            .is_err()
        );
        let mut tampered = expected.clone();
        tampered.digest = hex::encode([3; 32]);
        assert!(
            RetainedControlAnchor::from_acknowledgement(
                scope(),
                public,
                &tampered,
                tampered.clone(),
                Durability::FileAndDirectory
            )
            .is_err()
        );
    }

    #[test]
    fn namespace_uncertainty_cannot_satisfy_directory_sync_requirement() {
        assert!(
            RetentionRequirement::FileSynced.accepts(Durability::FileSyncedNamespaceUnconfirmed)
        );
        assert!(
            !RetentionRequirement::FileAndDirectory
                .accepts(Durability::FileSyncedNamespaceUnconfirmed)
        );
        assert!(RetentionRequirement::FileAndDirectory.accepts(Durability::FileAndDirectory));
    }
}
