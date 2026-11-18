//! Operator policy: similarity, recurrence and subject repetition to a shadow
//! outcome, with fixed safety floors an operator cannot lower.
//!
//! Safety floors (sot:root/abuse-similarity, section "Policy bounds"):
//! - behavior similarity alone can reach at most CHALLENGE;
//! - any block also requires cluster recurrence and repetition by the same
//!   subject (IP or fingerprint key, counted by the runtime);
//! - a permanent outcome is only a proposal for the dual-control queue.
use super::{PolicyRefusal, SimilarityError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Accepted policy document version.
pub const POLICY_VERSION: u32 = 1;
/// Most rules in one policy.
pub const MAX_RULES: usize = 16;
/// Shortest recurrence window (one hour).
pub const MIN_WINDOW_SECS: u64 = 3600;
/// Longest recurrence window (seven days, the ban strike decay).
pub const MAX_WINDOW_SECS: u64 = 7 * 24 * 3600;

/// What a matching rule asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleAction {
    /// Ask for a CHALLENGE.
    Challenge,
    /// Ask for a temporary ban through the progressive strike path.
    TemporaryBlock,
    /// Ask an operator pair to consider a permanent ban (dual control).
    PermanentBanProposal,
}

/// Result of evaluating a policy. Ordered by severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadowOutcome {
    /// No rule matched; the existing verdict stands.
    NoChange,
    /// Would ask for a CHALLENGE.
    Challenge,
    /// Would ask for a temporary ban.
    TemporaryBlock,
    /// Would queue a permanent-ban proposal for dual control.
    PermanentBanProposal,
}

impl From<RuleAction> for ShadowOutcome {
    fn from(action: RuleAction) -> Self {
        match action {
            RuleAction::Challenge => Self::Challenge,
            RuleAction::TemporaryBlock => Self::TemporaryBlock,
            RuleAction::PermanentBanProposal => Self::PermanentBanProposal,
        }
    }
}

/// Minimums a rule for `action` must require: (similarity, cluster
/// recurrence, subject repeats).
pub const fn floor_for(action: RuleAction) -> (f64, u64, u64) {
    match action {
        RuleAction::Challenge => (0.80, 0, 0),
        RuleAction::TemporaryBlock => (0.90, 3, 2),
        RuleAction::PermanentBanProposal => (0.95, 10, 3),
    }
}

/// One operator rule; every minimum must hold for it to match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// Requested action.
    pub action: RuleAction,
    /// Minimum similarity to the nearest confirmed-abuse cluster.
    pub min_similarity: f64,
    /// Minimum matches of that cluster within the policy window.
    pub min_recurrence: u64,
    /// Minimum prior matches by the same subject within the policy window.
    pub min_subject_repeats: u64,
}

/// Validated operator policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Must equal [`POLICY_VERSION`].
    pub version: u32,
    /// Recurrence window in seconds.
    pub window_secs: u64,
    /// Rules; the most severe matching rule wins.
    pub rules: Vec<Rule>,
}

impl Policy {
    /// Parse and validate policy JSON; nothing partial is ever returned.
    pub fn from_json(text: &str) -> Result<Self, SimilarityError> {
        let policy: Self = serde_json::from_str(text).map_err(|_| PolicyRefusal::Syntax)?;
        policy.validate()?;
        Ok(policy)
    }

    /// Check every safety bound.
    pub fn validate(&self) -> Result<(), SimilarityError> {
        if self.version != POLICY_VERSION {
            return Err(PolicyRefusal::Version.into());
        }
        if self.rules.is_empty() || self.rules.len() > MAX_RULES {
            return Err(PolicyRefusal::RuleCount.into());
        }
        if !(MIN_WINDOW_SECS..=MAX_WINDOW_SECS).contains(&self.window_secs) {
            return Err(PolicyRefusal::Window.into());
        }
        for rule in &self.rules {
            if !rule.min_similarity.is_finite() {
                return Err(SimilarityError::Nonfinite);
            }
            if rule.min_similarity > 1.0 {
                return Err(SimilarityError::Range);
            }
            let (similarity, recurrence, subject) = floor_for(rule.action);
            if rule.min_similarity < similarity {
                return Err(PolicyRefusal::SimilarityFloor.into());
            }
            if rule.min_recurrence < recurrence {
                return Err(PolicyRefusal::RecurrenceFloor.into());
            }
            if rule.min_subject_repeats < subject {
                return Err(PolicyRefusal::SubjectFloor.into());
            }
        }
        Ok(())
    }

    /// Most severe outcome among matching rules. `similarity` is `None` when no
    /// matchable cluster exists, which always yields [`ShadowOutcome::NoChange`].
    pub fn evaluate(
        &self,
        similarity: Option<f64>,
        recurrence: u64,
        subject_repeats: u64,
    ) -> Result<ShadowOutcome, SimilarityError> {
        let Some(similarity) = similarity else {
            return Ok(ShadowOutcome::NoChange);
        };
        if !similarity.is_finite() {
            return Err(SimilarityError::Nonfinite);
        }
        if !(0.0..=1.0).contains(&similarity) {
            return Err(SimilarityError::Range);
        }
        Ok(self
            .rules
            .iter()
            .filter(|rule| {
                similarity >= rule.min_similarity
                    && recurrence >= rule.min_recurrence
                    && subject_repeats >= rule.min_subject_repeats
            })
            .map(|rule| ShadowOutcome::from(rule.action))
            .max()
            .unwrap_or(ShadowOutcome::NoChange))
    }

    /// SHA-256 of the canonical JSON, for audit records and preview binding.
    pub fn digest(&self) -> String {
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        hex::encode(Sha256::digest(bytes))
    }
}

#[cfg(test)]
mod tests;
