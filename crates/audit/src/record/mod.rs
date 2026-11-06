//! Frozen audit record schema and canonical bytes, distinct from request tokens.
mod canonical;
mod normalization;
mod ordering;
pub use canonical::{EmptyMacKey, record_mac, verify_record_mac};
pub use canonical::{canonical_bytes, record_hash};
use humanymous_signals::wire::decode::{null_default, nullable_vec};
use serde::{Deserialize, Serialize};

/// Semantic event and chain-integrity fields. Identifiers must already be
/// pseudonymized by the producer; this type does not sanitize raw personal data.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    /// Display/correlation UUIDv7, not the sequence/deduplication authority.
    #[serde(deserialize_with = "null_default")]
    pub event_id: String,
    /// Per-node monotonic sequence.
    #[serde(deserialize_with = "null_default")]
    pub seq: u64,
    /// Chain-owning node identity.
    #[serde(deserialize_with = "null_default")]
    pub node_id: String,
    /// Observational RFC3339 nanosecond timestamp.
    #[serde(deserialize_with = "null_default")]
    pub ts: String,
    /// Versioned event taxonomy code.
    #[serde(deserialize_with = "null_default")]
    pub event_type: String,
    /// Event schema version.
    #[serde(deserialize_with = "null_default")]
    pub event_version: String,
    /// Pseudonymized actor.
    #[serde(deserialize_with = "null_default")]
    pub actor: Actor,
    /// Tenant isolation boundary.
    #[serde(deserialize_with = "null_default")]
    pub tenant_id: String,
    /// Subject-linked session pseudonym.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub session_pseudonym: String,
    /// Request correlation handle.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub request_id: String,
    /// Additional correlation handle.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub correlation_id: String,
    /// Origin host observation.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub host: String,
    /// html/api/upgrade/control/static class.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub route_class: String,
    /// Event-level verdict vocabulary.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub verdict: String,
    /// Integer audit risk score.
    #[serde(deserialize_with = "null_default")]
    pub risk_score: i64,
    /// Triggered rules; canonical bytes sort, without mutating this record.
    #[serde(deserialize_with = "nullable_vec")]
    pub triggered_rules: Option<Vec<String>>,
    /// Minimal signal observations; canonical bytes sort by original ID.
    #[serde(deserialize_with = "nullable_vec")]
    pub top_signals: Option<Vec<Signal>>,
    /// Enforcement action.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub enforcement_action: String,
    /// monitor/shadow/enforce.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub enforcement_mode: String,
    /// none/fail_open/fail_closed/degraded.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub fail_mode: String,
    /// Bounded reason code or explanation.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub fail_reason: String,
    /// Observed latency in microseconds.
    #[serde(deserialize_with = "null_default")]
    pub latency_us: u32,
    /// Optional upstream result; Some(default) differs from absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<Upstream>,
    /// Optional pseudonymized transport facts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsPseudonyms>,
    /// Approved configuration version.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub config_version: String,
    /// Signing/pepper key version.
    #[serde(deserialize_with = "null_default")]
    pub key_id: String,
    /// Data classification.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub data_class: String,
    /// Previous chained record hash; excluded from semantic canonical body.
    #[serde(deserialize_with = "null_default")]
    pub prev_hash: String,
    /// This chained record hash; excluded from semantic canonical body.
    #[serde(deserialize_with = "null_default")]
    pub record_hash: String,
    /// Secondary record authentication; excluded from semantic canonical body.
    #[serde(deserialize_with = "null_default")]
    pub hmac: String,
    /// Read-time opaque incident handle, never part of stored canonical body.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub incident: String,
}

/// Pseudonymized actor, not a raw IP, user agent or account identifier.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Actor {
    /// system/subject/operator.
    #[serde(deserialize_with = "null_default")]
    pub kind: String,
    /// Purpose-scoped pseudonym.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub id_pseudonym: String,
}
/// Minimal signal commitment fields.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Signal {
    /// Stable signal ID.
    #[serde(deserialize_with = "null_default")]
    pub id: String,
    /// Wire verdict.
    #[serde(deserialize_with = "null_default")]
    pub verdict: String,
    /// Confidence, formatted to three fixed decimal places in canonical bytes.
    #[serde(deserialize_with = "null_default")]
    pub conf: f64,
}
/// Upstream result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Upstream {
    /// HTTP status, including explicit zero when a result object is present.
    #[serde(deserialize_with = "null_default")]
    pub status: i64,
    /// Bounded error class.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub error_class: String,
}
/// Purpose-pseudonymized TLS observations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TlsPseudonyms {
    /// JA4 pseudonym.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub ja4_pseudonym: String,
    /// HTTP/2 fingerprint pseudonym.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub h2fp_pseudonym: String,
    /// Server-name pseudonym.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "null_default"
    )]
    pub sni_pseudonym: String,
}
