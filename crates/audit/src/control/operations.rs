use super::{Action, Approval, ControlAuthority, ControlReceipt, Proposal, Scope};
use std::io;
/// One immutable signed administrative request plus trusted observer time.
pub struct ApprovalRequest<'a> {
    /// Complete action, scope, target, nonce and bounded time window.
    pub proposal: &'a Proposal,
    /// Independent pinned administrator signatures.
    pub approvals: &'a [Approval],
    /// Trusted UTC Unix seconds; never accept a remote request's claimed clock.
    pub now: u64,
}
/// A noncloneable, nondeserializable destructive action capability. Ownership
/// consumption invokes at most one effect; durable ledger prevents reissuance.
pub struct ActionPermit {
    proposal: Proposal,
    receipt: ControlReceipt,
}
impl ActionPermit {
    /// Inspect the signed immutable binding before selecting a runtime handler.
    pub fn proposal(&self) -> &Proposal {
        &self.proposal
    }
    /// Inspect durable consumption, distinct from external effect completion.
    pub fn receipt(&self) -> &ControlReceipt {
        &self.receipt
    }
    /// Consume this capability for the exact scoped action and target digest.
    /// The callback must durably audit the resulting runtime change itself.
    pub fn execute<T>(
        self,
        scope: &Scope,
        action: Action,
        target: &str,
        now: u64,
        effect: impl FnOnce(&ControlReceipt) -> io::Result<T>,
    ) -> io::Result<T> {
        if &self.proposal.scope != scope
            || self.proposal.action != action
            || self.proposal.target != target
            || now < self.proposal.issued_at
            || now >= self.proposal.expires_at
        {
            return Err(io::Error::other(
                "approved action binding or expiry mismatch",
            ));
        }
        effect(&self.receipt)
    }
}
impl ControlAuthority {
    /// Consume a fully signed request and issue a single-owner runtime grant.
    /// Kill switch and permanent ban consumers must use execute's exact binding.
    pub fn authorize_action(&mut self, request: ApprovalRequest<'_>) -> io::Result<ActionPermit> {
        let receipt = self.consume(request.proposal, request.approvals, request.now)?;
        Ok(ActionPermit {
            proposal: request.proposal.clone(),
            receipt,
        })
    }
}
