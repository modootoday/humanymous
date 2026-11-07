// Translated from the frozen Go wire contract by scripts/migration/contracts.
use super::ClientReport;
use super::CrossCheck;
use super::NetworkReport;
use super::ScoreResult;
use super::Timestamp;

/// SessionReport is the full merged record (SoT-00 §3.2).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SessionReport {
    /// Wire field `sessionId`.
    #[serde(
        rename = "sessionId",
        deserialize_with = "crate::signals::wire::decode::null_default"
    )]
    pub session_id: String,
    /// Wire field `ts`.
    #[serde(
        rename = "ts",
        deserialize_with = "crate::signals::wire::decode::null_default"
    )]
    pub timestamp: Timestamp,
    /// Wire field `client`.
    #[serde(
        rename = "client",
        deserialize_with = "crate::signals::wire::decode::null_default"
    )]
    pub client: ClientReport,
    /// Wire field `network`.
    #[serde(
        rename = "network",
        deserialize_with = "crate::signals::wire::decode::null_default"
    )]
    pub network: NetworkReport,
    /// Wire field `crosschecks`.
    #[serde(
        rename = "crosschecks",
        deserialize_with = "crate::signals::wire::decode::nullable_vec"
    )]
    pub cross_checks: Option<Vec<CrossCheck>>,
    /// Wire field `scoring`.
    #[serde(
        rename = "scoring",
        deserialize_with = "crate::signals::wire::decode::null_default"
    )]
    pub scoring: ScoreResult,
    /// Label is optional ground-truth for e2e evaluation ("human"|"bot:<tool>").
    #[serde(
        rename = "label",
        deserialize_with = "crate::signals::wire::decode::null_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub label: String,
}
