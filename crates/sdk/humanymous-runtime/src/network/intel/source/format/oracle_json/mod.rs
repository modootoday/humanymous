//! Oracle Cloud `public_ip_ranges.json`: `regions[].cidrs[]` and `ipv6_cidrs[]`.
use super::{SourceError, document, prefix};
use crate::network::intel::prefix_set::Prefix;
use serde::Deserialize;

#[derive(Deserialize)]
struct Document {
    regions: Vec<Region>,
}
#[derive(Deserialize)]
struct Region {
    cidrs: Vec<Cidr>,
    #[serde(default)]
    ipv6_cidrs: Vec<Cidr>,
}
#[derive(Deserialize)]
struct Cidr {
    cidr: String,
}

pub(in crate::network::intel::source) fn parse(text: &str) -> Result<Vec<Prefix>, SourceError> {
    let document: Document = document(text)?;
    document
        .regions
        .iter()
        .flat_map(|region| region.cidrs.iter().chain(&region.ipv6_cidrs))
        .map(|entry| prefix(&entry.cidr, 0))
        .collect()
}

#[cfg(test)]
mod tests;
