//! AWS `ip-ranges.json`: `prefixes[].ip_prefix` and `ipv6_prefixes[].ipv6_prefix`.
use super::{SourceError, document, prefix};
use crate::network::intel::prefix_set::Prefix;
use serde::Deserialize;

#[derive(Deserialize)]
struct Document {
    prefixes: Vec<V4>,
    #[serde(default)]
    ipv6_prefixes: Vec<V6>,
}
#[derive(Deserialize)]
struct V4 {
    ip_prefix: String,
}
#[derive(Deserialize)]
struct V6 {
    ipv6_prefix: String,
}

pub(in crate::network::intel::source) fn parse(text: &str) -> Result<Vec<Prefix>, SourceError> {
    let document: Document = document(text)?;
    let v4 = document.prefixes.iter().map(|p| prefix(&p.ip_prefix, 0));
    let v6 = document
        .ipv6_prefixes
        .iter()
        .map(|p| prefix(&p.ipv6_prefix, 0));
    v4.chain(v6).collect()
}

#[cfg(test)]
mod tests;
