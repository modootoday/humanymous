//! Pure externally pinned recovery planning; never invents or replays effects.
use super::{RecoveryPolicy, VerifiedHistory, verify_history};
use crate::audit::{checkpoint::Checkpoint, record::Record};
use serde::Serialize;

/// Trusted operator choice for complete records beyond the external head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TailRecovery {
    /// All complete durable records must already have independent coverage.
    RequireCovered,
    /// Keep the authenticated complete tail without acknowledging its effects.
    PreserveAmbiguousTail,
}
/// Authentication actually required and completed by the recovery verifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RecoveryAuthentication {
    /// Record HMACs were checked with the provisioned key.
    pub hmac_checked: bool,
    /// Writer signatures and committed roots were checked against actual records.
    pub writer_checked: bool,
    /// All checkpoints were checked with the distinct provisioned witness key.
    pub witness_checked: bool,
}
/// Coverage report, not an authorization to repeat a prior request or effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WitnessCoverage {
    /// Last complete authenticated local record, including an ambiguous tail.
    pub durable_sequence: u64,
    /// Last independently witnessed record covered by the trusted external head.
    pub witnessed_through: u64,
    /// Exact independently authenticated and retained head used for planning.
    pub authenticated_external_head: Checkpoint,
    /// Local latest checkpoint before any explicit planned repair.
    pub local_latest_checkpoint: Option<Checkpoint>,
    /// Complete authenticated records not acknowledged by that external head.
    pub ambiguous_tail_len: u64,
    /// True only if every complete authenticated local record is covered.
    pub fully_witnessed: bool,
    /// Cryptographic scope, separate from persistence or deployment isolation.
    pub authentication: RecoveryAuthentication,
}
/// An authenticated candidate history. Planning does not persist a checkpoint.
/// The private pending-append field cannot be constructed by deserializing input.
pub struct WitnessRecoveryPlan {
    history: VerifiedHistory,
    coverage: WitnessCoverage,
    append: Option<Checkpoint>,
}
impl WitnessRecoveryPlan {
    /// Candidate verified coverage; inspect needs_checkpoint_append before
    /// claiming that the current on-disk checkpoint history has this coverage.
    pub fn coverage(&self) -> &WitnessCoverage {
        &self.coverage
    }
    /// Whether explicit recovery must append the authenticated external head.
    pub fn needs_checkpoint_append(&self) -> bool {
        self.append.is_some()
    }
    /// Authenticated candidate history, including a missing checkpoint if planned.
    pub fn history(&self) -> &VerifiedHistory {
        &self.history
    }
}
/// Verify actual record/checkpoint bytes with the existing recovery algorithms.
/// Both witness_public and trusted_head are mandatory and must be provisioned
/// independently. Never obtains a freshness anchor from untrusted local storage.
pub fn plan_witness_recovery(
    records: Vec<Record>,
    mut checkpoints: Vec<Checkpoint>,
    policy: RecoveryPolicy<'_>,
    tail: TailRecovery,
) -> Result<WitnessRecoveryPlan, &'static str> {
    let external = policy
        .trusted_head
        .ok_or("independent latest head required")?;
    let witness = policy
        .witness_public
        .ok_or("independent witness key required")?;
    if witness == policy.writer_public {
        return Err("writer and witness identities must differ");
    }
    if policy.allow_new {
        return Err("witness recovery cannot provision new history");
    }
    let local_latest_checkpoint = checkpoints.last().cloned();
    let append = if local_latest_checkpoint.as_ref() == Some(external) {
        None
    } else {
        if local_latest_checkpoint
            .as_ref()
            .is_some_and(|local| local.tree_size >= external.tree_size)
        {
            return Err("external head is behind or rewrites local checkpoint");
        }
        checkpoints.push(external.clone());
        Some(external.clone())
    };
    let history = verify_history(records, checkpoints, policy)?;
    let ambiguous_tail_len = history
        .sequence()
        .checked_sub(external.tree_size)
        .ok_or("external head exceeds durable records")?;
    if ambiguous_tail_len > 0 && tail == TailRecovery::RequireCovered {
        return Err("complete unwitnessed tail requires explicit preservation");
    }
    let report = history
        .report()
        .ok_or("authenticated history report absent")?;
    if !report.scope.hmac_checked
        || report.scope.checkpoints == 0
        || report.scope.witnessed_checkpoints != report.scope.checkpoints
    {
        return Err("required recovery authentication incomplete");
    }
    let coverage = WitnessCoverage {
        durable_sequence: history.sequence(),
        witnessed_through: external.tree_size,
        authenticated_external_head: external.clone(),
        local_latest_checkpoint,
        ambiguous_tail_len,
        fully_witnessed: ambiguous_tail_len == 0,
        authentication: RecoveryAuthentication {
            hmac_checked: true,
            writer_checked: true,
            witness_checked: true,
        },
    };
    Ok(WitnessRecoveryPlan {
        history,
        coverage,
        append,
    })
}

/// Observed outcome of explicit exclusive-WAL recovery, not effect execution.
#[cfg(any(unix, windows))]
pub struct AppliedWitnessRecovery {
    /// Verified actual records and planned/retained coverage.
    pub coverage: WitnessCoverage,
    /// A missing external checkpoint was appended successfully during this call.
    pub checkpoint_appended: bool,
    /// Actual native persistence strength, present only for a performed append.
    pub persistence: Option<crate::audit::keys::custody::Durability>,
    /// Earlier terminal torn-frame byte recovery, separate from complete tail.
    pub recovered_torn_bytes: u64,
}
#[cfg(any(unix, windows))]
impl super::Wal {
    /// Explicitly recover a witnessed head under this WAL's lifetime writer lock.
    /// No complete record is removed and no checkpoint is fabricated or re-signed.
    /// Append/sync errors retain existing WAL poison semantics and return no receipt.
    pub fn recover_witness_checkpoint(
        &mut self,
        policy: RecoveryPolicy<'_>,
        tail: TailRecovery,
    ) -> std::io::Result<AppliedWitnessRecovery> {
        let plan = plan_witness_recovery(self.records()?, self.checkpoints()?, policy, tail)
            .map_err(std::io::Error::other)?;
        let checkpoint_appended = plan.append.is_some();
        let persistence = if let Some(checkpoint) = plan.append {
            self.append_checkpoint(&checkpoint)?;
            #[cfg(unix)]
            {
                Some(crate::audit::keys::custody::Durability::FileAndDirectory)
            }
            #[cfg(windows)]
            {
                Some(crate::audit::keys::custody::Durability::FileSyncedNamespaceUnconfirmed)
            }
        } else {
            None
        };
        Ok(AppliedWitnessRecovery {
            coverage: plan.coverage,
            checkpoint_appended,
            persistence,
            recovered_torn_bytes: self.recovered_bytes(),
        })
    }
}

#[cfg(test)]
mod tests;
