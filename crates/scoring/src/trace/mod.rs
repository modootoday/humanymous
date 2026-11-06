//! Additive explanations produced by the same math and rule table as scoring.
use serde::Serialize;

/// One positive contribution before group deduplication.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContributionTrace {
    /// Signal or cross-check identity.
    pub id: String,
    /// Detection layer wire name.
    pub layer: String,
    /// Correlation group.
    pub group: String,
    /// Effective contribution after overlays and mitigation.
    pub score: f64,
}
/// A group member displaced by its highest-scoring peer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DedupTrace {
    /// Detection layer.
    pub layer: String,
    /// Correlation group.
    pub group: String,
    /// Retained identity.
    pub kept_id: String,
    /// Displaced identity.
    pub dropped_id: String,
    /// Retained contribution.
    pub kept_score: f64,
    /// Displaced contribution.
    pub dropped_score: f64,
}
/// One layer's within-layer noisy-OR probability and cap.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerTrace {
    /// Layer name.
    pub layer: String,
    /// Probability before the layer cap.
    pub raw_prob: f64,
    /// Probability after the layer cap.
    pub capped_prob: f64,
    /// Whether the cap reduced the probability.
    pub cap_hit: bool,
}
/// Evaluation of one entry in the ordered promotion table.
#[derive(Debug, Clone, Serialize)]
pub struct HardRuleEval {
    /// Stable rule identity.
    pub rule: String,
    /// Rule outcome if enforced.
    pub verdict: String,
    /// Frozen explanatory rationale.
    pub why: String,
    /// Whether the predicate matched.
    pub matched: bool,
    /// Whether this enforce-mode entry won.
    pub won: bool,
}
/// Policy snapshot used for one evaluation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyView {
    /// Policy identity.
    pub version: String,
    /// Single-layer cap.
    pub layer_cap: f64,
    /// Verification threshold.
    pub challenge_at: f64,
    /// Denial threshold.
    pub deny_at: f64,
}
/// Full additive explanation; absent drop/layer lists preserve legacy nulls.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreTrace {
    /// Positive pre-deduplication observations.
    pub contributions: Vec<ContributionTrace>,
    /// Displaced group members.
    pub dedup_dropped: Option<Vec<DedupTrace>>,
    /// Per-layer probabilities.
    pub per_layer: Option<Vec<LayerTrace>>,
    /// Every ordered rule evaluation.
    #[serde(rename = "hardRuleEval")]
    pub hard_rules: Vec<HardRuleEval>,
    /// Rounded result risk.
    pub score: f64,
    /// Final session outcome.
    pub verdict: String,
    /// Winning rule or trust upgrade.
    pub hard_rule_fired: String,
    /// Policy snapshot.
    pub policy: PolicyView,
}
