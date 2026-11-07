//! Independently retained erasure capabilities over the existing transaction.
use super::{Custody, Durability, crypto, invalid};
use crate::audit::{
    control::{Action, ControlReceipt, RetainedActionPermit, Scope, target_commitment},
    keys::vault::{ErasurePermit, Vault},
};
use std::io;

impl Custody {
    /// Consume an independently retained grant for this exact scoped subject.
    /// The caller must witness its audit intent before calling, serialize vault
    /// access and stop serving on any error after in-memory shredding. Completion
    /// covers this sealed generation only, never backup or unknown-copy removal.
    pub fn erase_retained(
        &mut self,
        vault: &Vault,
        passphrase: &[u8],
        grant: RetainedActionPermit,
        scope: &Scope,
        now: u64,
        subject: &str,
    ) -> io::Result<(Durability, ControlReceipt)> {
        self.ready()?;
        if subject.is_empty() || self.exists("keys.pending")? {
            return Err(invalid(
                "erasure requires subject and no pending transaction",
            ));
        }
        let current = self.read("keys.sealed", passphrase)?;
        let snapshot = vault.snapshot().map_err(crypto)?;
        if current.vault() != Some(snapshot.as_slice()) {
            return Err(invalid("erasure vault differs from committed generation"));
        }
        drop(current);
        drop(snapshot);
        let target = target_commitment(subject.as_bytes());
        grant.execute(scope, Action::EraseSubject, &target, now, |receipt| {
            let permit = ErasurePermit::issued(subject, &receipt.operation_id, scope);
            vault.shred(subject, &permit).map_err(crypto)?;
            Ok((
                self.persist_erasure(vault, passphrase, &permit)?,
                receipt.clone(),
            ))
        })
    }

    /// Explicitly recover the exact pending subject tombstone using a fresh
    /// retained approval. A consumed old grant is never reconstructed or replayed.
    /// Existing recovery authenticates both generations and unchanged identity.
    pub fn recover_retained_erasure(
        &mut self,
        committed_passphrase: &[u8],
        pending_passphrase: &[u8],
        grant: RetainedActionPermit,
        scope: &Scope,
        now: u64,
        subject: &str,
    ) -> io::Result<(Durability, ControlReceipt)> {
        self.ready()?;
        if subject.is_empty() {
            return Err(invalid("erasure recovery requires subject"));
        }
        // Destructive recovery cannot become initial provisioning when the
        // committed generation was lost. Authenticate it before executing effect.
        self.read("keys.sealed", committed_passphrase)?;
        let target = target_commitment(subject.as_bytes());
        grant.execute(scope, Action::EraseSubject, &target, now, |receipt| {
            let permit = ErasurePermit::issued(subject, &receipt.operation_id, scope);
            Ok((
                self.recover_erasure_pending(
                    committed_passphrase,
                    pending_passphrase,
                    &permit,
                    scope,
                )?,
                receipt.clone(),
            ))
        })
    }
}
