//! Join authenticated retained intent with covered effects; never replay grants.
use super::KillTarget;
use crate::startup::SealedAudit;
use humanymous_audit::control::{Action, LedgerAnchor, RetainedControlState, Scope};
use std::io;

/// Inspection only. No mutation API accepts a caller-selected state or boolean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KillState {
    /// Exact authenticated absence of both retained intent and local effect.
    Clear,
    /// Retained intent and a covered permanent effect; never expires or rearms.
    Killed,
    /// Retained intent has no independently covered effect. Remain unavailable.
    Pending,
}
/// Single-owner assessment, not a grant, distributed lease or public assertion.
pub struct KillStatus {
    state: KillState,
    scope: Scope,
    target: String,
    authority_public: [u8; 32],
    control_anchor: LedgerAnchor,
    audit_sequence: u64,
    witnessed_through: u64,
}
impl KillStatus {
    /// The caller must hold issuance/startup/admission serialization throughout
    /// this join. Query the real authority/retainer freshly; cached snapshots or
    /// this returned value must not become a reusable healthy capability.
    pub fn assess(
        target: &KillTarget,
        retained: RetainedControlState,
        authority_public: &[u8; 32],
        audit: &SealedAudit,
    ) -> io::Result<Self> {
        let commitment = target.commitment().map_err(|_| invalid())?;
        if retained.scope() != target.scope()
            || retained.authority_public() != *authority_public
            || retained.action() != Action::KillSwitch
            || retained.target() != commitment
            || !retained.other_consumed_targets().is_empty()
            || !audit.matches_control_scope(target.scope())
        {
            return Err(invalid());
        }
        let through = audit.witnessed_through()?.ok_or_else(invalid)?;
        let mut effect = Effect::Absent;
        for record in audit
            .records()?
            .iter()
            .filter(|record| record.event_type.starts_with("runtime.kill."))
        {
            let operation = record
                .request_id
                .strip_prefix("kill:")
                .ok_or_else(invalid)?;
            if record.event_type != "runtime.kill.engaged"
                || record.event_version != "1"
                || record.node_id != target.scope().node
                || record.correlation_id != commitment
                || record.verdict != "KILLED"
                || record.enforcement_action != "kill_switch"
                || record.enforcement_mode != "enforce"
                || record.fail_mode != "fail_closed"
                || operation.len() != 64
                || !operation
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                || operation.bytes().all(|byte| byte == b'0')
            {
                return Err(invalid());
            }
            if record.seq <= through {
                effect = Effect::Covered;
            } else if effect != Effect::Covered {
                effect = Effect::Ambiguous;
            }
        }
        Ok(Self {
            state: classify(retained.consumed().is_some(), effect)?,
            scope: target.scope().clone(),
            target: commitment,
            authority_public: *authority_public,
            control_anchor: retained.anchor().clone(),
            audit_sequence: audit.sequence(),
            witnessed_through: through,
        })
    }
    /// Only an inspected result, never permission to apply a destructive effect.
    pub fn state(&self) -> KillState {
        self.state
    }
    /// Exact node/installation binding; consumers must not transplant the result.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    /// Exact stable target whose intent and effect were joined.
    pub fn target(&self) -> &str {
        &self.target
    }
    /// Pinned authority identity used for this assessment.
    pub fn authority_public(&self) -> [u8; 32] {
        self.authority_public
    }
    /// Exact independent control commitment that the producer authenticated.
    pub fn control_anchor(&self) -> &LedgerAnchor {
        &self.control_anchor
    }
    /// Actual authenticated audit extent observed during this serialized join.
    pub fn audit_sequence(&self) -> u64 {
        self.audit_sequence
    }
    /// Actual independent coverage, distinct from writer-only durable bytes.
    pub fn witnessed_through(&self) -> u64 {
        self.witnessed_through
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Effect {
    Absent,
    Covered,
    Ambiguous,
}
fn invalid() -> io::Error {
    io::Error::other("kill intent/effect binding unavailable")
}
fn classify(consumed: bool, effect: Effect) -> io::Result<KillState> {
    match (consumed, effect) {
        (false, Effect::Absent) => Ok(KillState::Clear),
        (true, Effect::Covered) => Ok(KillState::Killed),
        (true, Effect::Absent | Effect::Ambiguous) => Ok(KillState::Pending),
        (false, _) => Err(invalid()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spent_intent_without_covered_effect_cannot_become_healthy() {
        assert_eq!(classify(false, Effect::Absent).unwrap(), KillState::Clear);
        assert_eq!(classify(true, Effect::Covered).unwrap(), KillState::Killed);
        assert_eq!(classify(true, Effect::Absent).unwrap(), KillState::Pending);
        assert_eq!(
            classify(true, Effect::Ambiguous).unwrap(),
            KillState::Pending
        );
        assert!(classify(false, Effect::Covered).is_err());
        assert!(classify(false, Effect::Ambiguous).is_err());
    }
}
