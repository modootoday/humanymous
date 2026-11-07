//! Read-only authoritative control intent, never a grant or distributed lease.
use super::{Action, LedgerAnchor, Proposal, Scope};

/// In-process view obtained only after ledger authentication and exact current
/// independent anchor comparison. Absence is authenticated for this exact query.
/// No public constructor, deserializer, clone or effect method is exposed.
/// The trusted caller must serialize startup admission with control issuance.
pub struct RetainedControlState {
    scope: Scope,
    authority_public: [u8; 32],
    anchor: LedgerAnchor,
    action: Action,
    target: String,
    consumed: Option<Proposal>,
    other_consumed_targets: Vec<String>,
}
impl RetainedControlState {
    pub(super) fn checked(
        scope: Scope,
        authority_public: [u8; 32],
        anchor: LedgerAnchor,
        action: Action,
        target: String,
        consumed: Option<Proposal>,
        other_consumed_targets: Vec<String>,
    ) -> Self {
        Self {
            scope,
            authority_public,
            anchor,
            action,
            target,
            consumed,
            other_consumed_targets,
        }
    }
    /// Exact provisioned logical scope, not a request's claimed identity.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    /// Authority verification identity provisioned with the independent retainer.
    pub fn authority_public(&self) -> [u8; 32] {
        self.authority_public
    }
    /// Current authenticated ledger commitment at the time of the query.
    pub fn anchor(&self) -> &LedgerAnchor {
        &self.anchor
    }
    /// Exact action whose durable consumption was queried.
    pub fn action(&self) -> Action {
        self.action
    }
    /// Exact canonical target commitment queried, including on absent results.
    pub fn target(&self) -> &str {
        &self.target
    }
    /// One matching consumed proposal, never a reusable execution capability.
    /// A proposal's later expiry does not undo its durable consumption or intent.
    pub fn consumed(&self) -> Option<&Proposal> {
        self.consumed.as_ref()
    }
    /// Other distinct consumed targets for this action in the same signed scope.
    /// A one-way kill consumer must refuse unknown targets as a migration issue,
    /// not interpret a changed target protocol as an authenticated healthy reset.
    pub fn other_consumed_targets(&self) -> &[String] {
        &self.other_consumed_targets
    }
}
