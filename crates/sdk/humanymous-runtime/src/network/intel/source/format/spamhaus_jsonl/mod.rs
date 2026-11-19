//! Spamhaus DROP JSON lines: `{"cidr":...}` records and one `{"type":"metadata"}`
//! record. Any other record shape refuses the body.
use super::{SourceError, prefix};
use crate::network::intel::prefix_set::Prefix;
use serde::Deserialize;

#[derive(Deserialize)]
struct Record {
    cidr: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
}

pub(in crate::network::intel::source) fn parse(text: &str) -> Result<Vec<Prefix>, SourceError> {
    let mut prefixes = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if line.trim().is_empty() {
            continue;
        }
        let record: Record =
            serde_json::from_str(line).map_err(|_| SourceError::Malformed { line: number })?;
        match (record.cidr, record.kind.as_deref()) {
            (Some(cidr), None) => prefixes.push(prefix(&cidr, number)?),
            (None, Some("metadata")) => {}
            _ => return Err(SourceError::Malformed { line: number }),
        }
    }
    Ok(prefixes)
}

#[cfg(test)]
mod tests;
