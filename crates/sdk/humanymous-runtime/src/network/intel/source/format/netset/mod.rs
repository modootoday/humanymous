//! Line lists: one address or CIDR per line, with `#` or `;` comments and
//! optional trailing annotations (FireHOL netset/ipset, Tor bulk exit list,
//! X4BNet outputs). Bare addresses become host routes.
use super::{SourceError, prefix};
use crate::network::intel::prefix_set::Prefix;

pub(in crate::network::intel::source) fn parse(text: &str) -> Result<Vec<Prefix>, SourceError> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let entry = line.split(['#', ';']).next().unwrap_or("").trim();
            let token = entry.split_whitespace().next()?;
            Some(prefix(token, index + 1))
        })
        .collect()
}

#[cfg(test)]
mod tests;
