//! Bounded report decoding with unique decoded object keys at every depth.
//! Extracted with owner consent from attacker verification/json.rs (671c439).
use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};
use std::{error::Error, fmt};

/// Input-independent failures. No caller-controlled key or JSON fragment is retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrictJsonError {
    /// The caller's byte budget was exceeded before parsing.
    TooLarge,
    /// Syntax, duplicate decoded key, nesting limit, number or trailing input failed.
    Invalid,
}
impl fmt::Display for StrictJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TooLarge => "JSON byte budget exceeded",
            Self::Invalid => "invalid strict JSON",
        })
    }
}
impl Error for StrictJsonError {}

struct UniqueValue(Value);
impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueVisitor)
    }
}
struct UniqueVisitor;
impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = UniqueValue;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JSON with unique decoded object keys")
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Bool(value)))
    }
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Number(value.into())))
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Number(value.into())))
    }
    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Number::from_f64(value)
            .map(|value| UniqueValue(Value::Number(value)))
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::String(value.into())))
    }
    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::String(value)))
    }
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Null))
    }
    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Null))
    }
    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        UniqueValue::deserialize(deserializer)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<UniqueValue>()? {
            values.push(value.0);
        }
        Ok(UniqueValue(Value::Array(values)))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Self::Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = object.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate JSON object key"));
            }
            values.insert(key, object.next_value::<UniqueValue>()?.0);
        }
        Ok(UniqueValue(Value::Object(values)))
    }
}

/// Reject oversized bytes first, then decode with serde's nesting limit intact.
/// Caller schemas, unknown-field policy and authenticated/replay semantics remain
/// with their owning consumer; this function is only the shared syntax boundary.
pub fn decode_strict_json(bytes: &[u8], max_bytes: usize) -> Result<Value, StrictJsonError> {
    if bytes.len() > max_bytes {
        return Err(StrictJsonError::TooLarge);
    }
    serde_json::from_slice::<UniqueValue>(bytes)
        .map(|value| value.0)
        .map_err(|_| StrictJsonError::Invalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escaped_nested_and_dynamic_decoded_keys_must_be_unique() {
        for bytes in [
            br#"{"seed":0,"seed":42}"#.as_slice(),
            br#"{"schema":1,"\u0073chema":1}"#,
            br#"{"rows":[{"totals":{"a":0,"a":0}}]}"#,
            br#"{"by_class":{"attack":{},"attack":{}}}"#,
        ] {
            assert_eq!(
                decode_strict_json(bytes, bytes.len()),
                Err(StrictJsonError::Invalid)
            );
        }
        let bytes = br#" {"seed":18446744073709551615,"rows":[null,true,-1,0.5,"text"]} "#;
        assert_eq!(
            decode_strict_json(bytes, bytes.len()).unwrap()["seed"].as_u64(),
            Some(u64::MAX)
        );
    }
    #[test]
    fn budget_precedes_parse_and_failures_never_echo_input() {
        assert_eq!(decode_strict_json(b"{}", 1), Err(StrictJsonError::TooLarge));
        assert_eq!(decode_strict_json(b"{}", 2).unwrap(), serde_json::json!({}));
        for bytes in [
            b"{} trailing-private-payload".as_slice(),
            b"[NaN]",
            b"[1e9999]",
            b"\xff",
            b"",
        ] {
            let error = decode_strict_json(bytes, bytes.len()).unwrap_err();
            assert_eq!(error, StrictJsonError::Invalid);
            assert_eq!(error.to_string(), "invalid strict JSON");
        }
        let nested = format!("{}0{}", "[".repeat(256), "]".repeat(256));
        assert_eq!(
            decode_strict_json(nested.as_bytes(), nested.len()),
            Err(StrictJsonError::Invalid)
        );
    }
}
