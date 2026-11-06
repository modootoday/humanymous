use serde::{Deserialize, Deserializer};

// Newtype prevents serde's special missing-Option behavior: null is permitted,
// omission is not. No defaults are used anywhere in this wire schema.
pub(super) struct Nullable(pub Option<String>);
impl<'de> Deserialize<'de> for Nullable {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = Nullable;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("an explicitly present string or null")
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(Nullable(None))
            }
            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Self::Value, E> {
                Ok(Nullable(Some(s.to_owned())))
            }
            fn visit_string<E: serde::de::Error>(self, s: String) -> Result<Self::Value, E> {
                Ok(Nullable(Some(s)))
            }
        }
        d.deserialize_any(Visitor)
    }
}
// Explicit unit decoding accepts only JSON null, including when nested.
impl<'de> Deserialize<'de> for OnlyNull {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        <()>::deserialize(d).map(|()| Self)
    }
}
pub(super) struct OnlyNull;
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Request {
    pub protocol: String,
    pub id: String,
    pub tenant: String,
    pub origin: String,
    pub revision: String,
    pub deadline: u64,
    pub method: String,
    pub target: String,
    pub authority: String,
    pub peer: Nullable,
    pub headers: Vec<[String; 2]>,
    pub header_mode: String,
    pub body: Body,
    pub capabilities: Capabilities,
    #[serde(rename = "stream")]
    pub _stream: OnlyNull,
    pub adapter: Adapter,
    pub connection_id: Nullable,
    #[serde(rename = "capture")]
    pub _capture: OnlyNull,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Adapter {
    pub id: String,
    pub version: String,
    pub installation: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Body {
    pub layer: String,
    pub complete: bool,
    pub length: u64,
    pub sha256: String,
    pub base64: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Capabilities {
    pub raw_tls: bool,
    pub original_h2: bool,
    pub raw_header_order: bool,
    pub complete_body: bool,
}
