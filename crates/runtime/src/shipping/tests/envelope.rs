use super::*;

#[test]
fn both_roles_preserve_immutable_scope_binding_and_original_input() {
    for (name, role) in [
        ("core_demo", PublicRole::CoreDemo),
        ("gate_edge", PublicRole::GateEdge),
    ] {
        let mut value = document();
        value["role"] = json!(name);
        let input = encoded(&value);
        let before = input.clone();
        let data = parse_public_role_data(&input).unwrap();
        assert!(data.role() == role);
        assert_eq!(data.scope().installation(), "i");
        assert_eq!(data.scope().tenant(), "t");
        assert_eq!(data.scope().origin(), "o");
        assert_eq!(data.scope().revision(), "r");
        assert_eq!(data.scope().application_authority(), "Example.test:443");
        assert_eq!(data.binding().installation_epoch(), &[1; 16]);
        assert_eq!(data.binding().generation(), 1);
        assert_eq!(data.binding().base_plan_sha256(), &[0x12; 32]);
        assert_eq!(data.binding().executable_sha256(), &[0x34; 32]);
        assert_eq!(input, before);
    }
}

#[test]
fn standard_whitespace_and_decoded_escaped_keys_and_tokens_are_valid() {
    let text = String::from_utf8(encoded(&document()))
        .unwrap()
        .replace("\"schema\"", "\"\\u0073chema\"")
        .replace("\"scope\"", "\"\\u0073cope\"")
        .replace("\"generation\"", "\"\\u0067eneration\"")
        .replace("core_demo", "core_\\u0064emo");
    assert!(parse_public_role_data(format!(" \n{text}\r\n\t").as_bytes()).is_ok());
}

#[test]
fn duplicate_decoded_fields_refuse_at_every_object_level() {
    let value = document();
    for escaped in [false, true] {
        for key in value.as_object().unwrap().keys() {
            invalid(duplicate(&value, key, escaped).as_bytes());
        }
        for parent in ["scope", "binding"] {
            for key in value[parent].as_object().unwrap().keys() {
                let old = serde_json::to_string(&value[parent]).unwrap();
                let changed = String::from_utf8(encoded(&value))
                    .unwrap()
                    .replace(&old, &duplicate(&value[parent], key, escaped));
                invalid(changed.as_bytes());
            }
        }
    }
}

#[test]
fn unknown_fields_refuse_at_every_object_level() {
    for parent in [None, Some("scope"), Some("binding")] {
        let mut value = document();
        let object = match parent {
            Some(p) => &mut value[p],
            None => &mut value,
        };
        object["extra"] = json!("never-accepted");
        invalid(&encoded(&value));
    }
}

#[test]
fn every_required_root_scope_and_binding_field_is_required() {
    let original = document();
    for parent in [None, Some("scope"), Some("binding")] {
        let object = match parent {
            Some(p) => &original[p],
            None => &original,
        };
        for key in object.as_object().unwrap().keys() {
            let mut value = original.clone();
            let object = match parent {
                Some(p) => &mut value[p],
                None => &mut value,
            };
            object.as_object_mut().unwrap().remove(key);
            invalid(&encoded(&value));
        }
    }
}

#[test]
fn scalar_types_and_positional_objects_cannot_bypass_closed_schema() {
    use super::super::wire;
    let original = document();
    for pointer in [
        "/schema",
        "/role",
        "/scope/installation",
        "/scope/tenant",
        "/scope/origin",
        "/scope/revision",
        "/scope/application_authority",
        "/binding/installation_epoch",
        "/binding/base_plan_sha256",
        "/binding/executable_sha256",
    ] {
        for bad in [Value::Null, json!(false), json!(1), json!([]), json!({})] {
            let mut v = original.clone();
            *v.pointer_mut(pointer).unwrap() = bad;
            invalid(&encoded(&v));
        }
    }
    for pointer in ["/scope", "/binding"] {
        for bad in [Value::Null, json!(false), json!(1), json!("x"), json!([])] {
            let mut v = original.clone();
            *v.pointer_mut(pointer).unwrap() = bad;
            invalid(&encoded(&v));
        }
    }

    // Every value is valid and in the actual derived wire declaration order.
    // Direct derived controls isolate rejection by the map-only wrappers from
    // unrelated missing fields, scalar types, generation or scope bounds.
    let map = parse(&original).unwrap();
    let scope = &original["scope"];
    let scope_array = json!([
        scope["installation"],
        scope["tenant"],
        scope["origin"],
        scope["revision"],
        scope["application_authority"]
    ]);
    let derived_scope: wire::Scope = serde_json::from_value(scope_array.clone()).unwrap();
    assert_eq!(derived_scope.installation, map.scope().installation());
    assert_eq!(
        derived_scope.application_authority,
        map.scope().application_authority()
    );
    let mut v = original.clone();
    v["scope"] = scope_array;
    invalid(&encoded(&v));

    let binding = &original["binding"];
    let binding_array = json!([
        binding["installation_epoch"],
        binding["generation"],
        binding["base_plan_sha256"],
        binding["executable_sha256"]
    ]);
    let derived_binding: wire::Binding = serde_json::from_value(binding_array.clone()).unwrap();
    assert_eq!(derived_binding.generation, map.binding().generation());
    assert_eq!(
        derived_binding.installation_epoch,
        binding["installation_epoch"].as_str().unwrap()
    );
    let mut v = original.clone();
    v["binding"] = binding_array;
    invalid(&encoded(&v));

    let root_array = json!([
        original["schema"],
        original["role"],
        original["scope"],
        original["binding"],
        original["required_providers"]
    ]);
    let derived_root: wire::Envelope = serde_json::from_value(root_array.clone()).unwrap();
    assert_eq!(derived_root.schema, "humanymous.public-role/1");
    assert_eq!(derived_root.role, "core_demo");
    assert_eq!(
        derived_root.required_providers.0.len(),
        map.required_providers().len()
    );
    invalid(&encoded(&root_array));

    let mut v = original;
    v["role"] = json!({"core_demo":null});
    invalid(&encoded(&v));
}

#[test]
fn role_schema_aliases_and_trailing_documents_are_rejected() {
    for role in [
        "CoreDemo",
        "core",
        "gate",
        "gate_edge_tls_v1",
        "",
        "core_demo ",
    ] {
        let mut v = document();
        v["role"] = json!(role);
        invalid(&encoded(&v));
    }
    for schema in [
        "humanymous.public-role/0",
        "humanymous.public-role/2",
        "",
        "humanymous.public-role/1 ",
    ] {
        let mut v = document();
        v["schema"] = json!(schema);
        invalid(&encoded(&v));
    }
    let mut bytes = encoded(&document());
    bytes.extend_from_slice(b" {} ");
    invalid(&bytes);
    invalid(&[]);
    invalid(br#"{"schema":"#);
}

#[test]
fn original_raw_budget_is_enforced_before_json_parsing() {
    let mut bytes = encoded(&document());
    bytes.resize(32_768, b' ');
    assert!(parse_public_role_data(&bytes).is_ok());
    bytes.push(b' ');
    assert!(matches!(
        parse_public_role_data(&bytes),
        Err(PublicRoleDataError::Size)
    ));
    assert!(matches!(
        parse_public_role_data(&vec![b'!'; 32_769]),
        Err(PublicRoleDataError::Size)
    ));
}

#[test]
fn public_errors_are_fixed_redacted_and_have_no_parser_source() {
    use std::error::Error as _;
    let input = br#"{"secret":"do-not-echo-this-credential"}"#;
    let error = parse_public_role_data(input).err().unwrap();
    assert_eq!(error.to_string(), "public role data rejected");
    assert_eq!(format!("{error:?}"), "Invalid");
    assert!(error.source().is_none());
    assert_eq!(PublicRoleDataError::Size.to_string(), error.to_string());
}
