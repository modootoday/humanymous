//! Frozen JSON null, collection and untyped-number decoding across wire contracts.
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::BTreeMap;

fn normalize_untyped_numbers(value: Value) -> Value {
    match value {
        Value::Number(number) => Value::Number(
            serde_json::Number::from_f64(number.as_f64().expect("JSON number fits float64"))
                .expect("finite JSON number"),
        ),
        Value::Array(values) => {
            Value::Array(values.into_iter().map(normalize_untyped_numbers).collect())
        }
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, normalize_untyped_numbers(value)))
                .collect(),
        ),
        value => value,
    }
}

// Go decodes interface{} numbers as float64. Restrict this conversion to wire
// decoding; derived typed integer observations retain their exact integer value.
/// Decode an interface-style JSON value with float64 numbers.
pub fn untyped_value<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Value, D::Error> {
    Ok(normalize_untyped_numbers(Value::deserialize(deserializer)?))
}

/// Decode an optional interface-style value; null remains absent.
pub fn optional_untyped_value<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Value>, D::Error> {
    let value = untyped_value(deserializer)?;
    Ok((!value.is_null()).then_some(value))
}

/// Go's JSON decoder leaves zero-valued scalar/struct fields unchanged on null.
pub fn null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// Whether an optional collection is omitted under the legacy omitempty rule.
pub fn optional_vec_empty<T>(value: &Option<Vec<T>>) -> bool {
    value.as_ref().is_none_or(Vec::is_empty)
}

/// Whether an optional map is omitted under the legacy omitempty rule.
pub fn optional_map_empty<K, V>(value: &Option<BTreeMap<K, V>>) -> bool {
    value.as_ref().is_none_or(BTreeMap::is_empty)
}

/// Preserve a null slice while turning its null elements into zero values.
pub fn nullable_vec<'de, D, T>(deserializer: D) -> Result<Option<Vec<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<Vec<Option<T>>>::deserialize(deserializer)?
        .map(|values| values.into_iter().map(Option::unwrap_or_default).collect()))
}

/// Preserve a null map while turning its null values into zero values.
pub fn nullable_map<'de, D, K, V>(deserializer: D) -> Result<Option<BTreeMap<K, V>>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Ord,
    V: Deserialize<'de> + Default,
{
    Ok(
        Option::<BTreeMap<K, Option<V>>>::deserialize(deserializer)?.map(|values| {
            values
                .into_iter()
                .map(|(key, value)| (key, value.unwrap_or_default()))
                .collect()
        }),
    )
}

/// Decode a required vector, using an empty vector for a null collection.
pub fn vec_default<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(nullable_vec(deserializer)?.unwrap_or_default())
}

/// Decode a matrix with zero-valued null elements and empty null rows.
pub fn matrix_default<'de, D, T>(deserializer: D) -> Result<Vec<Vec<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    let rows: Option<Vec<Option<Vec<Option<T>>>>> = Deserialize::deserialize(deserializer)?;
    Ok(rows
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            row.unwrap_or_default()
                .into_iter()
                .map(Option::unwrap_or_default)
                .collect()
        })
        .collect())
}
