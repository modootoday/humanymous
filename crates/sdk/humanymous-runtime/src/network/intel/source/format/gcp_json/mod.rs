//! Google Cloud `cloud.json`: `prefixes[]` with `ipv4Prefix` or `ipv6Prefix`.
use super::{SourceError, document, prefix};
use crate::network::intel::prefix_set::Prefix;
use serde::Deserialize;

#[derive(Deserialize)]
struct Document {
    prefixes: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    ipv4_prefix: Option<String>,
    ipv6_prefix: Option<String>,
}

pub(in crate::network::intel::source) fn parse(text: &str) -> Result<Vec<Prefix>, SourceError> {
    let document: Document = document(text)?;
    document
        .prefixes
        .iter()
        .map(|entry| match (&entry.ipv4_prefix, &entry.ipv6_prefix) {
            (Some(text), None) | (None, Some(text)) => prefix(text, 0),
            _ => Err(SourceError::Malformed { line: 0 }),
        })
        .collect()
}

#[cfg(test)]
mod tests;
