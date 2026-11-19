//! Upstream wire-format normalizers, one module per format.
use super::SourceError;
use crate::network::intel::prefix_set::Prefix;

pub(super) mod aws_json;
pub(super) mod azure_tags;
pub(super) mod gcp_json;
pub(super) mod geofeed;
pub(super) mod netset;
pub(super) mod oracle_json;
pub(super) mod spamhaus_jsonl;

/// One upstream prefix token; host bits are masked. `line` is 1-based or zero.
fn prefix(token: &str, line: usize) -> Result<Prefix, SourceError> {
    Prefix::parse_masked(token.trim()).map_err(|_| SourceError::Prefix { line })
}

/// One upstream JSON document; unknown upstream fields are ignored.
fn document<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, SourceError> {
    serde_json::from_str(text).map_err(|_| SourceError::Malformed { line: 0 })
}
