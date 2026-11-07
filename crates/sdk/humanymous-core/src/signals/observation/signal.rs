use super::{Layer, Source, Verdict, compute_score, lookup};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One atomic observation, serialized with the frozen public JSON field names.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Signal {
    /// Stable dotted identity.
    #[serde(deserialize_with = "crate::signals::wire::decode::null_default")]
    pub id: String,
    /// Owning detection layer.
    #[serde(deserialize_with = "crate::signals::wire::decode::null_default")]
    pub layer: Layer,
    /// Observed value.
    #[serde(deserialize_with = "crate::signals::wire::decode::untyped_value")]
    pub value: Value,
    /// Expected human baseline; omitted when unspecified.
    #[serde(
        rename = "expectedHuman",
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::signals::wire::decode::optional_untyped_value"
    )]
    pub expected_human: Option<Value>,
    /// Per-observation judgment.
    #[serde(deserialize_with = "crate::signals::wire::decode::null_default")]
    pub verdict: Verdict,
    /// Maximum contribution.
    #[serde(deserialize_with = "crate::signals::wire::decode::null_default")]
    pub weight: f64,
    /// Actual contribution.
    #[serde(deserialize_with = "crate::signals::wire::decode::null_default")]
    pub score: f64,
    /// Observation confidence.
    #[serde(deserialize_with = "crate::signals::wire::decode::null_default")]
    pub confidence: f64,
    /// Collection plane.
    #[serde(deserialize_with = "crate::signals::wire::decode::null_default")]
    pub collected: Source,
    /// Optional explanation.
    #[serde(
        skip_serializing_if = "String::is_empty",
        deserialize_with = "crate::signals::wire::decode::null_default"
    )]
    pub notes: String,
}

impl Signal {
    /// Construct from the shared registry. Unknown IDs retain zero weight and an empty layer.
    pub fn new(
        id: &str,
        value: Value,
        verdict: Verdict,
        confidence: f64,
        collected: Source,
        notes: &str,
    ) -> Self {
        let definition = lookup(id);
        let weight = definition
            .map(|definition| definition.weight)
            .unwrap_or_default();
        Self {
            id: id.to_owned(),
            layer: definition
                .map(|definition| definition.layer.clone())
                .unwrap_or_default(),
            value,
            expected_human: definition.and_then(|definition| {
                (!definition.expected_human.is_null()).then(|| definition.expected_human.clone())
            }),
            score: compute_score(weight, confidence, &verdict),
            verdict,
            weight,
            confidence,
            collected,
            notes: notes.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn constructor_uses_registry_and_unknown_ids_do_not_gain_weight() {
        let signal = Signal::new(
            "l1.navigator.webdriver",
            json!(true),
            Verdict::Bot,
            0.8,
            Source::Wasm,
            "",
        );
        assert_eq!(signal.score, 12.0);
        assert_eq!(signal.expected_human, Some(json!(false)));
        let unknown = Signal::new(
            "future.id",
            json!(true),
            Verdict::Bot,
            1.0,
            Source::Server,
            "",
        );
        assert_eq!(unknown.weight, 0.0);
        assert_eq!(unknown.layer.as_str(), "");
        let encoded = serde_json::to_value(unknown).unwrap();
        assert!(encoded.get("expectedHuman").is_none());
        assert!(encoded.get("notes").is_none());
    }
}
