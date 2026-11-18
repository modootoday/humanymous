//! Shadow accounting: what a policy would have done next to the verdict that
//! was actually served. Aggregate counts only; no identifiers or vectors.
use super::{ShadowOutcome, SimilarityError, policy::Policy};
use serde::Serialize;

/// Verdict the frozen engine actually served.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActualVerdict {
    /// Allowed.
    Allow,
    /// Challenged.
    Challenge,
    /// Blocked.
    Block,
}

/// Events observed when the outcome was first reached, bucketed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LatencyBucket {
    /// At most 16 events.
    UpTo16,
    /// 17 to 64 events.
    UpTo64,
    /// 65 to 256 events.
    UpTo256,
    /// More than 256 events.
    Over256,
}

impl LatencyBucket {
    /// Bucket for an event count.
    pub fn of(events: u32) -> Self {
        match events {
            0..=16 => Self::UpTo16,
            17..=64 => Self::UpTo64,
            65..=256 => Self::UpTo256,
            _ => Self::Over256,
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// One de-identified evaluation input, kept only for policy preview.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// Similarity to the nearest matchable cluster, if any.
    pub similarity: Option<f64>,
    /// Cluster recurrence within the policy window.
    pub recurrence: u64,
    /// Subject repetition within the policy window.
    pub subject_repeats: u64,
    /// Verdict actually served.
    pub actual: ActualVerdict,
    /// Behavior events observed when the sample was taken.
    pub events: u32,
}

/// Aggregate shadow statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowStats {
    /// Samples evaluated.
    pub n: u64,
    /// Outcomes: no change, challenge, temporary block, permanent proposal.
    pub outcomes: [u64; 4],
    /// Shadow asked for more than the served verdict.
    pub stricter: u64,
    /// Shadow asked for less than the served verdict (served stands anyway).
    pub looser: u64,
    /// Shadow and served verdict agree.
    pub agree: u64,
    /// Event-count buckets at a non-`NoChange` shadow outcome.
    pub latency: [u64; 4],
    /// Samples rejected as invalid; not part of any other count.
    pub rejected: u64,
}

fn severity(outcome: ShadowOutcome) -> u8 {
    match outcome {
        ShadowOutcome::NoChange => 0,
        ShadowOutcome::Challenge => 1,
        ShadowOutcome::TemporaryBlock | ShadowOutcome::PermanentBanProposal => 2,
    }
}

fn served(actual: ActualVerdict) -> u8 {
    match actual {
        ActualVerdict::Allow => 0,
        ActualVerdict::Challenge => 1,
        ActualVerdict::Block => 2,
    }
}

/// Running shadow ledger for one policy. Never returns anything a caller could
/// serve as a verdict; it only counts.
#[derive(Debug, Clone)]
pub struct ShadowLedger {
    policy: Policy,
    stats: ShadowStats,
}

impl ShadowLedger {
    /// Ledger bound to a validated policy.
    pub fn new(policy: Policy) -> Result<Self, SimilarityError> {
        policy.validate()?;
        Ok(Self {
            policy,
            stats: ShadowStats::default(),
        })
    }

    /// Record one sample.
    pub fn observe(&mut self, sample: Sample) -> Result<(), SimilarityError> {
        let outcome =
            match self
                .policy
                .evaluate(sample.similarity, sample.recurrence, sample.subject_repeats)
            {
                Ok(outcome) => outcome,
                Err(error) => {
                    self.stats.rejected = self.stats.rejected.saturating_add(1);
                    return Err(error);
                }
            };
        let stats = &mut self.stats;
        stats.n = stats.n.saturating_add(1);
        let slot = match outcome {
            ShadowOutcome::NoChange => 0,
            ShadowOutcome::Challenge => 1,
            ShadowOutcome::TemporaryBlock => 2,
            ShadowOutcome::PermanentBanProposal => 3,
        };
        stats.outcomes[slot] = stats.outcomes[slot].saturating_add(1);
        let (shadow, served) = (severity(outcome), served(sample.actual));
        let counter = match shadow.cmp(&served) {
            std::cmp::Ordering::Greater => &mut stats.stricter,
            std::cmp::Ordering::Less => &mut stats.looser,
            std::cmp::Ordering::Equal => &mut stats.agree,
        };
        *counter = counter.saturating_add(1);
        if outcome != ShadowOutcome::NoChange {
            let bucket = LatencyBucket::of(sample.events).index();
            stats.latency[bucket] = stats.latency[bucket].saturating_add(1);
        }
        Ok(())
    }

    /// Current statistics.
    pub fn stats(&self) -> ShadowStats {
        self.stats
    }

    /// Digest of the bound policy.
    pub fn policy_digest(&self) -> String {
        self.policy.digest()
    }
}

/// Replay held samples under a candidate policy before an operator saves it.
/// Invalid samples are counted in `rejected` and do not abort the preview.
pub fn preview(policy: &Policy, samples: &[Sample]) -> Result<ShadowStats, SimilarityError> {
    let mut ledger = ShadowLedger::new(policy.clone())?;
    for sample in samples {
        let _ = ledger.observe(*sample);
    }
    Ok(ledger.stats())
}

#[cfg(test)]
mod tests;
