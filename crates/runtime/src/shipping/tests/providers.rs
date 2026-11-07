use super::*;

#[test]
fn both_roles_must_declare_each_of_four_providers() {
    for role in ["core_demo", "gate_edge"] {
        for index in 0..4 {
            let mut v = document();
            v["role"] = json!(role);
            v["required_providers"]
                .as_array_mut()
                .unwrap()
                .remove(index);
            invalid(&encoded(&v));
        }
        let mut v = document();
        v["role"] = json!(role);
        v["required_providers"] = json!([]);
        invalid(&encoded(&v));
    }
}

#[test]
fn bounded_optional_declarations_preserve_order_without_provider_observations() {
    let mut v = document();
    v["required_providers"] = json!([
        "collector",
        "original_h2",
        "original_tls",
        "native_preparation",
        "external_witness",
        "durable_audit"
    ]);
    let data = parse(&v).unwrap();
    assert_eq!(data.required_providers().len(), 6);
    assert!(data.required_providers()[0] == RequiredProvider::Collector);
    assert!(data.required_providers()[1] == RequiredProvider::OriginalH2);
    assert!(data.required_providers()[5] == RequiredProvider::DurableAudit);
    assert_eq!(parse(&document()).unwrap().required_providers().len(), 4);
}

#[test]
fn duplicate_unknown_and_wrong_type_provider_declarations_refuse() {
    for bad in [
        json!("durable_audit"),
        json!("unknown-provider"),
        json!("OriginalTls"),
        json!(false),
        json!(1),
        Value::Null,
        json!({}),
        json!([]),
    ] {
        let mut v = document();
        v["required_providers"].as_array_mut().unwrap().push(bad);
        invalid(&encoded(&v));
    }
    for bad in [
        json!(null),
        json!("durable_audit"),
        json!({}),
        json!(false),
        json!(1),
    ] {
        let mut v = document();
        v["required_providers"] = bad;
        invalid(&encoded(&v));
    }
    let original = String::from_utf8(encoded(&document())).unwrap();
    invalid(
        original
            .replace(
                "\"original_tls\"]",
                "\"original_tls\",\"\\u0064urable_audit\"]",
            )
            .as_bytes(),
    );
}

#[test]
fn seventh_provider_is_refused_before_decoding_or_collecting_its_value() {
    use serde::de::{
        DeserializeSeed,
        value::{BoolDeserializer, Error},
    };
    let error = super::super::wire::ProviderSeed { full: true }
        .deserialize(BoolDeserializer::<Error>::new(false))
        .err()
        .unwrap();
    assert_eq!(error.to_string(), "provider budget");
    let mut v = document();
    v["required_providers"].as_array_mut().unwrap().extend([
        json!("original_h2"),
        json!("collector"),
        json!({"never":"decoded"}),
    ]);
    invalid(&encoded(&v));
}

#[test]
fn repeated_epoch_generation_and_digests_remain_data_not_replay_authority() {
    let value = document();
    let first = parse(&value).unwrap();
    let second = parse(&value).unwrap();
    assert_eq!(first.binding().generation(), second.binding().generation());
    assert_eq!(
        first.binding().installation_epoch(),
        second.binding().installation_epoch()
    );
    assert_eq!(
        first.binding().base_plan_sha256(),
        second.binding().base_plan_sha256()
    );
    assert_eq!(
        first.binding().executable_sha256(),
        second.binding().executable_sha256()
    );
}
