//! Retained capability boundary over the existing local retirement transaction.
use super::{BackupInventory, BackupReceipt, BackupStatus, authenticate, invalid};
use crate::control::{Action, ControlAuthority, ControlReceipt, Proposal, RetainedActionPermit};
use std::io;

impl BackupInventory {
    fn retained_binding(
        &self,
        authority: &ControlAuthority,
        grant: &RetainedActionPermit,
    ) -> io::Result<String> {
        self.healthy()?;
        let bytes = self.store.read()?.ok_or_else(invalid)?;
        let actual = authenticate(&bytes, &self.key, &self.body.scope, self.capacity)?;
        if actual.anchor != self.anchor
            || !grant.retention().matches(
                &self.body.scope,
                &authority.public_key(),
                authority.anchor(),
            )
            || !grant.retention().anchor().verify(&authority.public_key())
        {
            return Err(invalid());
        }
        self.verify_retirement(authority, grant.proposal())
    }

    /// Retire only an Active local file using the exact independently retained
    /// grant. Legacy retire remains local-only compatibility. Caller must witness
    /// audit intent before apply and retain the returned inventory anchor before
    /// acknowledging completion. No receipt proves physical/unknown-copy erasure.
    pub fn retire_retained(
        &mut self,
        authority: &ControlAuthority,
        grant: RetainedActionPermit,
        now: u64,
    ) -> io::Result<(BackupReceipt, ControlReceipt)> {
        let digest = self.retained_binding(authority, &grant)?;
        let proposal = grant.proposal().clone();
        let entry = self.body.entries.get(&digest).ok_or_else(invalid)?;
        if entry.status != BackupStatus::Active {
            return Err(invalid());
        }
        self.sealed(&digest, entry.bytes)?.ok_or_else(invalid)?;
        let scope = self.body.scope.clone();
        grant.execute(
            &scope,
            Action::RetireBackup,
            &proposal.target,
            now,
            |receipt| {
                Ok((
                    self.reconcile_retirement(authority, &proposal)?,
                    receipt.clone(),
                ))
            },
        )
    }

    /// Reconcile only an exact already-Retiring operation with a different fresh
    /// retained approval. Audit correlation must contain both operation IDs,
    /// exact scope/digest and before/after inventory anchors. The final inventory
    /// retains the prior operation ID; the returned control receipt is fresh.
    /// Pending inventory and unacknowledged newer pins require explicit recovery.
    pub fn reconcile_retained(
        &mut self,
        authority: &ControlAuthority,
        prior: &Proposal,
        fresh: RetainedActionPermit,
        now: u64,
    ) -> io::Result<(BackupReceipt, ControlReceipt)> {
        let digest = self.retained_binding(authority, &fresh)?;
        if fresh.proposal().operation_id == prior.operation_id
            || self.verify_retirement(authority, prior)? != digest
            || !matches!(
                &self.body.entries.get(&digest).ok_or_else(invalid)?.status,
                BackupStatus::Retiring { proposal } if proposal == prior
            )
        {
            return Err(invalid());
        }
        let scope = self.body.scope.clone();
        let target = fresh.proposal().target.clone();
        fresh.execute(&scope, Action::RetireBackup, &target, now, |receipt| {
            Ok((
                self.reconcile_retirement(authority, prior)?,
                receipt.clone(),
            ))
        })
    }
}
