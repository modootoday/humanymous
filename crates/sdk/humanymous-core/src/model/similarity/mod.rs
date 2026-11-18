//! Shadow-only similarity to confirmed abuse patterns (sot:root/abuse-similarity).
//!
//! Confirmed abusive sessions are aggregated into bounded clusters over the
//! `bv1` behavior feature vector. A new session is compared only against those
//! confirmed-abuse clusters; how far it lies from typical human behavior is
//! never measured or used, so atypical motor or speed patterns cannot raise
//! an outcome. An operator policy maps similarity, cluster recurrence and
//! subject repetition to a *shadow* outcome.
//!
//! Nothing here mutates a verdict, writes a ban or proposes one to the runtime.
//! The detection freeze holds: promotion into enforcement needs an owner
//! decision. A permanent ban can only ever be a proposal for the dual-control
//! queue; no type in this module represents an automatic permanent ban.
//!
//! The store keeps per-cluster means, variances, counts and match times only.
//! It holds no raw vectors and no subject identifiers.
mod policy;
mod shadow;
mod store;

pub use policy::{
    MAX_RULES, MAX_WINDOW_SECS, MIN_WINDOW_SECS, POLICY_VERSION, Policy, Rule, RuleAction,
    ShadowOutcome, floor_for,
};
pub use shadow::{ActualVerdict, LatencyBucket, Sample, ShadowLedger, ShadowStats, preview};
pub use store::{
    ClusterId, ClusterView, Evidence, MAX_CLUSTERS, MAX_MATCH_EVENTS, MIN_SUPPORT, Match,
    PatternStore, RETENTION_SECS, VARIANCE_FLOOR, similarity,
};

/// Bounded failures; diagnostics never carry feature values or identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimilarityError {
    /// A feature value or numeric configuration is NaN or infinite.
    Nonfinite,
    /// A similarity or fraction is outside `[0, 1]`.
    Range,
    /// The caller clock moved backwards.
    ClockRegressed,
    /// The store is full; nothing was evicted.
    Capacity,
    /// The cluster does not exist (expired, retired or never created).
    UnknownCluster,
    /// The policy is malformed or violates a safety bound.
    Policy(PolicyRefusal),
}

/// Why an operator policy was refused. Each variant is a fixed safety bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyRefusal {
    /// The policy version is not [`POLICY_VERSION`].
    Version,
    /// No rules, or more than [`MAX_RULES`].
    RuleCount,
    /// The recurrence window is outside `[MIN_WINDOW_SECS, MAX_WINDOW_SECS]`.
    Window,
    /// A rule's similarity threshold is below the floor for its action.
    SimilarityFloor,
    /// A blocking rule does not require enough cluster recurrence.
    RecurrenceFloor,
    /// A blocking rule does not require enough repetition by the same subject.
    SubjectFloor,
    /// The policy text is not valid policy JSON.
    Syntax,
}

impl std::fmt::Display for SimilarityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "abuse similarity: {self:?}")
    }
}

impl std::error::Error for SimilarityError {}

impl From<PolicyRefusal> for SimilarityError {
    fn from(refusal: PolicyRefusal) -> Self {
        Self::Policy(refusal)
    }
}
