use super::*;
use serde_json::{Value, json};
mod bounds;
mod envelope;
mod providers;

fn document() -> Value {
    json!({
        "schema":"humanymous.public-role/1", "role":"core_demo",
        "scope":{"installation":"i", "tenant":"t", "origin":"o", "revision":"r",
            "application_authority":"Example.test:443"},
        "binding":{"installation_epoch":"01".repeat(16), "generation":1,
            "base_plan_sha256":"12".repeat(32), "executable_sha256":"34".repeat(32)},
        "required_providers":["durable_audit","external_witness","native_preparation","original_tls"]
    })
}
fn encoded(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
fn parse(value: &Value) -> Result<PublicRoleData, PublicRoleDataError> {
    parse_public_role_data(&encoded(value))
}
fn invalid(bytes: &[u8]) {
    assert!(matches!(
        parse_public_role_data(bytes),
        Err(PublicRoleDataError::Invalid)
    ));
}
fn duplicate(value: &Value, name: &str, escaped: bool) -> String {
    let key = if escaped {
        format!("\\u{:04x}{}", name.as_bytes()[0], &name[1..])
    } else {
        name.into()
    };
    let object = serde_json::to_string(value).unwrap();
    format!("{{\"{key}\":{},{}", value[name], &object[1..])
}
