// Translated from the frozen Go wire contract by scripts/migration/contracts.

/// CrossCheck is an L6 consistency evaluation between signals/layers (SoT-00 §3.3).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CrossCheck {
    /// Wire field `id`.
    #[serde(rename = "id", deserialize_with = "crate::wire::decode::null_default")]
    pub id: String,
    /// Wire field `inputs`.
    #[serde(
        rename = "inputs",
        deserialize_with = "crate::wire::decode::nullable_vec"
    )]
    pub inputs: Option<Vec<String>>,
    /// Wire field `claim`.
    #[serde(
        rename = "claim",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub claim: String,
    /// Wire field `observed`.
    #[serde(
        rename = "observed",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub observed: String,
    /// Wire field `consistent`.
    #[serde(
        rename = "consistent",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub consistent: bool,
    /// Wire field `weight`.
    #[serde(
        rename = "weight",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub weight: f64,
    /// Wire field `score`.
    #[serde(
        rename = "score",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub score: f64,
    /// Wire field `notes`.
    #[serde(
        rename = "notes",
        deserialize_with = "crate::wire::decode::null_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub notes: String,
}

/// ScoreResult is the L7 output (SoT-00 §3.2 scoring).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ScoreResult {
    /// 0..100
    #[serde(
        rename = "riskScore",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub risk_score: f64,
    /// ALLOW|CHALLENGE|DENY
    #[serde(
        rename = "verdict",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub verdict: String,
    /// Wire field `hardRuleFired`.
    #[serde(
        rename = "hardRuleFired",
        deserialize_with = "crate::wire::decode::null_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub hard_rule_fired: String,
    /// Wire field `topContributors`.
    #[serde(
        rename = "topContributors",
        deserialize_with = "crate::wire::decode::nullable_vec"
    )]
    pub top_contributors: Option<Vec<Contributor>>,
    /// Wire field `policyVersion`.
    #[serde(
        rename = "policyVersion",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub policy_version: String,
}

/// Contributor is one signal's contribution to the final score.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Contributor {
    /// Wire field `id`.
    #[serde(rename = "id", deserialize_with = "crate::wire::decode::null_default")]
    pub id: String,
    /// Wire field `score`.
    #[serde(
        rename = "score",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub score: f64,
}
