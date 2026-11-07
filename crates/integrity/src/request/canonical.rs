use humanymous_signals::wire::simple_lowercase;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Signed header names in their frozen order.
pub const SIGNED_HEADERS: [&str; 6] = [
    "user-agent",
    "sec-ch-ua",
    "sec-ch-ua-platform",
    "sec-ch-ua-mobile",
    "accept-language",
    "accept",
];

/// Observed request components; names must already be lowercase.
pub struct CanonicalInput<'a> {
    /// Header name to value map.
    pub headers: &'a BTreeMap<String, String>,
    /// Session cookie used for binding.
    pub cookie_id: &'a str,
    /// Observed body bytes, before any reserialization.
    pub body: &'a [u8],
}

/// Build the byte-stable text signed by the rotating token protocol.
pub fn canonical(input: CanonicalInput<'_>) -> String {
    let mut result = String::new();
    for name in SIGNED_HEADERS {
        result.push_str(name);
        result.push(':');
        let value = input
            .headers
            .get(name)
            .map(String::as_str)
            .unwrap_or_default();
        let mut words = value.split_whitespace();
        if let Some(first) = words.next() {
            result.push_str(first);
            for word in words {
                result.push(' ');
                result.push_str(word);
            }
        }
        result.push('\n');
    }
    result.push_str("cookie:");
    result.push_str(input.cookie_id);
    result.push_str("\nbody:");
    result.push_str(&hex::encode(Sha256::digest(input.body)));
    result
}

/// Lowercase names after sorting original keys, so duplicate-cased keys are deterministic.
pub fn lower_header_map(input: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    input
        .iter()
        .map(|(name, value)| (simple_lowercase(name), value.clone()))
        .collect()
}

#[cfg(test)]
mod tests;
