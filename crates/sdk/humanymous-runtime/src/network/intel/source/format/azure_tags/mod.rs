//! Azure Service Tags: union of `values[].properties.addressPrefixes[]`.
use super::{SourceError, document, prefix};
use crate::network::intel::prefix_set::Prefix;
use serde::Deserialize;

#[derive(Deserialize)]
struct Document {
    values: Vec<Tag>,
}
#[derive(Deserialize)]
struct Tag {
    properties: Properties,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Properties {
    address_prefixes: Vec<String>,
}

pub(in crate::network::intel::source) fn parse(text: &str) -> Result<Vec<Prefix>, SourceError> {
    let document: Document = document(text)?;
    document
        .values
        .iter()
        .flat_map(|tag| &tag.properties.address_prefixes)
        .map(|text| prefix(text, 0))
        .collect()
}

#[cfg(test)]
mod tests;
