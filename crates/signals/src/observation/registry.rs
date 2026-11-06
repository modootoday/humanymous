use super::Layer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;

/// Frozen static metadata exported from the baseline Go registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Definition {
    /// Stable dotted signal identity.
    #[serde(rename = "ID")]
    pub id: String,
    /// Owning detection layer.
    #[serde(rename = "Layer")]
    pub layer: Layer,
    /// Frozen maximum risk contribution.
    #[serde(rename = "Weight")]
    pub weight: f64,
    /// Expected human value, or JSON null when unspecified.
    #[serde(rename = "ExpectedHuman")]
    pub expected_human: Value,
    /// Correlation group for deduplication.
    #[serde(rename = "Group")]
    pub group: String,
    /// Existing signal description.
    #[serde(rename = "Desc")]
    pub description: String,
}

/// All registered definitions in stable ID order.
pub fn all() -> &'static [Definition] {
    static DEFINITIONS: OnceLock<Vec<Definition>> = OnceLock::new();
    DEFINITIONS.get_or_init(|| {
        serde_json::from_str(include_str!("registry.json"))
            .expect("validated embedded signal registry")
    })
}

/// Look up an exact signal ID; unknown IDs have no definition.
pub fn lookup(id: &str) -> Option<&'static Definition> {
    let definitions = all();
    definitions
        .binary_search_by(|definition| definition.id.as_str().cmp(id))
        .ok()
        .map(|index| &definitions[index])
}

/// Correlation group of an exact ID, or the empty string.
pub fn group_of(id: &str) -> &'static str {
    lookup(id)
        .map(|definition| definition.group.as_str())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_unique_sorted_ids_and_known_frozen_weights() {
        assert_eq!(all().len(), 143);
        assert!(all().windows(2).all(|pair| pair[0].id < pair[1].id));
        assert_eq!(lookup("l1.navigator.webdriver").unwrap().weight, 15.0);
        assert_eq!(group_of("l1.navigator.webdriver"), "webdriver");
        assert!(lookup("not.registered").is_none());
        assert_eq!(group_of("not.registered"), "");
        for definition in all() {
            assert_eq!(&definition.id[1..2], &definition.layer.as_str()[1..2]);
            assert!((0.0..=100.0).contains(&definition.weight));
        }
        for id in [
            "l4.ml.behavioral",
            "l5.correlation.proxy_rotation",
            "l5.correlation.shared_fingerprint",
            "l5.correlation.fp_churn_proxy",
            "l5.correlation.ip_velocity",
        ] {
            assert_eq!(
                lookup(id).unwrap().weight,
                0.0,
                "audit-only residual changed policy: {id}"
            );
        }
    }
}
