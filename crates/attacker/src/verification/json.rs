//! Report JSON syntax boundary: decoded object keys must be unique at every depth.
use humanymous_defender::lab::LabError;
use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};

struct ReportValue(Value);

impl<'de> Deserialize<'de> for ReportValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ReportVisitor)
    }
}

struct ReportVisitor;
impl<'de> Visitor<'de> for ReportVisitor {
    type Value = ReportValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("report JSON with unique decoded object keys")
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(ReportValue(Value::Bool(value)))
    }
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(ReportValue(Value::Number(value.into())))
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(ReportValue(Value::Number(value.into())))
    }
    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Number::from_f64(value)
            .map(|value| ReportValue(Value::Number(value)))
            .ok_or_else(|| E::custom("non-finite report number"))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(ReportValue(Value::String(value.into())))
    }
    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(ReportValue(Value::String(value)))
    }
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(ReportValue(Value::Null))
    }
    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(ReportValue(Value::Null))
    }
    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        ReportValue::deserialize(deserializer)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<ReportValue>()? {
            values.push(value.0);
        }
        Ok(ReportValue(Value::Array(values)))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Self::Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = object.next_key::<String>()? {
            if values.contains_key(&key) {
                // Do not copy a caller-controlled key into diagnostic logs.
                return Err(de::Error::custom("duplicate report object key"));
            }
            values.insert(key, object.next_value::<ReportValue>()?.0);
        }
        Ok(ReportValue(Value::Object(values)))
    }
}

pub(super) fn decode(bytes: &[u8]) -> Result<Value, LabError> {
    serde_json::from_slice::<ReportValue>(bytes)
        .map(|value| value.0)
        .map_err(|_| LabError("report JSON syntax or duplicate object key"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoded_keys_are_unique_inside_arrays_objects_and_dynamic_maps() {
        for raw in [
            br#"{"seed":0,"seed":42}"#.as_slice(),
            br#"{"schema":1,"\u0073chema":1}"#,
            br#"{"rows":[{"totals":{"a":0,"a":0}}]}"#,
            br#"{"by_class":{"attack":{},"attack":{}}}"#,
        ] {
            assert!(decode(raw).is_err());
        }
        let valid = br#" {"seed":18446744073709551615,"rows":[null,true,-1,0.5,"text"]} "#;
        let value = decode(valid).unwrap();
        assert_eq!(value["seed"].as_u64(), Some(u64::MAX));
        assert!(decode(b"{} trailing").is_err());
        assert!(decode(b"[NaN]").is_err());
        assert!(decode(b"[1e9999]").is_err());
    }
}
