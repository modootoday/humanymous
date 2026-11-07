use super::RequiredProvider;
use serde::{
    Deserialize, Deserializer,
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor, value::MapAccessDeserializer},
};
use std::{fmt, marker::PhantomData};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Envelope {
    pub schema: String,
    pub role: String,
    #[serde(deserialize_with = "object")]
    pub scope: Scope,
    #[serde(deserialize_with = "object")]
    pub binding: Binding,
    pub required_providers: Providers,
}
// deserialize_map rejects positional arrays that a derived struct visitor might
// otherwise accept. The derived inner visitor still enforces decoded duplicate,
// unknown and missing fields, without routing through an overwriting JSON map.
pub(super) fn object<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<T, D::Error> {
    struct ObjectVisitor<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
        type Value = T;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("closed object")
        }
        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
            T::deserialize(MapAccessDeserializer::new(map))
        }
    }
    d.deserialize_map(ObjectVisitor(PhantomData))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Scope {
    pub installation: String,
    pub tenant: String,
    pub origin: String,
    pub revision: String,
    pub application_authority: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    pub installation_epoch: String,
    pub generation: u64,
    pub base_plan_sha256: String,
    pub executable_sha256: String,
}
pub(super) struct Providers(pub Vec<RequiredProvider>);
impl<'de> Deserialize<'de> for Providers {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_seq(ProviderVisitor)
    }
}
struct ProviderVisitor;
impl<'de> Visitor<'de> for ProviderVisitor {
    type Value = Providers;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bounded required provider declarations")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::with_capacity(6);
        while let Some(value) = seq.next_element_seed(ProviderSeed {
            full: values.len() == 6,
        })? {
            if values.contains(&value) {
                return Err(de::Error::custom("duplicate provider"));
            }
            values.push(value);
        }
        for required in [
            RequiredProvider::DurableAudit,
            RequiredProvider::ExternalWitness,
            RequiredProvider::NativePreparation,
            RequiredProvider::OriginalTls,
        ] {
            if !values.contains(&required) {
                return Err(de::Error::custom("missing required provider"));
            }
        }
        Ok(Providers(values))
    }
}
pub(super) struct ProviderSeed {
    pub(super) full: bool,
}
impl<'de> DeserializeSeed<'de> for ProviderSeed {
    type Value = RequiredProvider;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        // SeqAccess invokes this only for an actual element. Reject the seventh
        // before decoding its value or collecting anything based on size_hint.
        if self.full {
            return Err(de::Error::custom("provider budget"));
        }
        let token = String::deserialize(d)?;
        match token.as_str() {
            "durable_audit" => Ok(RequiredProvider::DurableAudit),
            "external_witness" => Ok(RequiredProvider::ExternalWitness),
            "native_preparation" => Ok(RequiredProvider::NativePreparation),
            "original_tls" => Ok(RequiredProvider::OriginalTls),
            "original_h2" => Ok(RequiredProvider::OriginalH2),
            "collector" => Ok(RequiredProvider::Collector),
            _ => Err(de::Error::custom("unknown provider")),
        }
    }
}
