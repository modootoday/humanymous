//! RFC 8805 geofeed CSV: the first column is the prefix; `#` lines are comments.
use super::{SourceError, prefix};
use crate::network::intel::prefix_set::Prefix;

pub(in crate::network::intel::source) fn parse(text: &str) -> Result<Vec<Prefix>, SourceError> {
    text.lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
        .map(|(number, line)| {
            let first = line.split(',').next().unwrap_or("");
            prefix(first, number)
        })
        .collect()
}

#[cfg(test)]
mod tests;
