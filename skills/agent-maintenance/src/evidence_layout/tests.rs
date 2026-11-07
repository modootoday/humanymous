use super::*;
#[path = "../../tests/support/layout_request.rs"]
mod fixture;

fn value() -> serde_json::Value {
    fixture::request(&std::env::temp_dir().join("hmn-layout-pure-fixture"))
}
fn inspect(value: &serde_json::Value) -> Plan {
    decode_and_plan(&serde_json::to_vec(value).unwrap()).unwrap()
}
fn refuses(value: &serde_json::Value, code: &str) {
    let plan = inspect(value);
    assert!(!plan.structurally_valid);
    assert!(!plan.retirement_authorized);
    assert!(plan.errors.contains(&code), "{:?}", plan.errors);
}

#[test]
fn complete_assertions_never_become_independently_verified_authority() {
    let plan = inspect(&value());
    assert!(plan.structurally_valid, "{:?}", plan.errors);
    assert_eq!(plan.state, "proposed");
    assert!(plan.supplied_assertions_only);
    assert!(!plan.retirement_authorized && !plan.execution_authorized);
    assert!(!plan.owner_authority_verified && !plan.terminal_handles_independently_verified);
    assert!(
        !plan.candidate_filesystem_verified
            && !plan.archive_bytes_verified
            && !plan.readback_independently_verified
    );
    assert_eq!(plan.proposed_member_bytes, 3);
    assert_eq!(plan.proposed_archive_bytes, 7);
}
#[test]
fn unknown_duplicate_missing_and_arbitrary_fields_fail_closed() {
    let mut unknown = value();
    unknown["owner"]["extra"] = true.into();
    assert!(matches!(
        decode_and_plan(&serde_json::to_vec(&unknown).unwrap()),
        Err(RequestError::InvalidJson { .. })
    ));
    let mut missing = value();
    missing.as_object_mut().unwrap().remove("retention");
    assert!(decode_and_plan(&serde_json::to_vec(&missing).unwrap()).is_err());
    assert!(decode_and_plan(br#"{"schema_version":1,"schema_version":1}"#).is_err());
    let mut action = value();
    action["proposed_action"] = "delete_now".into();
    assert!(decode_and_plan(&serde_json::to_vec(&action).unwrap()).is_err());
    let mut limits = value();
    limits["timeout_seconds"] = 120.into();
    assert!(decode_and_plan(&serde_json::to_vec(&limits).unwrap()).is_err());
}
#[test]
fn schema_and_request_bytes_are_bounded_without_value_disclosure() {
    let error = decode_and_plan(br#"{"schema_version":"PRIVATE_SENTINEL"}"#)
        .err()
        .unwrap();
    assert!(!error.to_string().contains("PRIVATE_SENTINEL"));
    assert!(matches!(
        decode_and_plan(&vec![b' '; MAX_REQUEST_BYTES as usize + 1]),
        Err(RequestError::TooLarge)
    ));
    let mut version = value();
    version["schema_version"] = 2.into();
    refuses(&version, "unsupported_version");
}
#[test]
fn unknown_owners_active_handles_and_incomplete_inventory_are_refused() {
    let mut owner = value();
    owner["owner"]["session"] = "unknown".into();
    refuses(&owner, "owner_assertion");
    let mut active = value();
    active["consumers"]["handles"][0]["state"] = "active".into();
    refuses(&active, "terminal_handle_assertion");
    let mut exit = value();
    exit["consumers"]["handles"][0]["exit_code"] = serde_json::Value::Null;
    refuses(&exit, "terminal_handle_assertion");
    let mut coverage = value();
    coverage["consumers"]["inventory_complete_asserted"] = false.into();
    refuses(&coverage, "consumer_coverage_unknown");
    let mut duplicate = value();
    let handle = duplicate["consumers"]["handles"][0].clone();
    duplicate["consumers"]["handles"]
        .as_array_mut()
        .unwrap()
        .push(handle);
    refuses(&duplicate, "terminal_handle_assertion");
}
#[test]
fn escapes_protected_roots_and_canonical_scratch_are_refused() {
    let mut escape = value();
    escape["members"][0]["path"] = "../source".into();
    refuses(&escape, "member_path_or_case_collision");
    let mut scratch = value();
    scratch["scratch_path"] = scratch["retained_root"].clone();
    refuses(&scratch, "scratch_inside_canonical_tree");
    let mut protected = value();
    protected["materialization_path"] = protected["repository_root"].clone();
    refuses(&protected, "protected_path_overlap");
    let mut outside = value();
    outside["archive"]["parts"][0]["path"] = outside["scratch_path"].clone();
    refuses(&outside, "archive_part_assertion");
}
#[test]
fn member_case_aliases_and_link_ancestors_are_refused() {
    let mut duplicate = value();
    let mut member = duplicate["members"][0].clone();
    member["path"] = "SOURCE.BIN".into();
    duplicate["members"].as_array_mut().unwrap().push(member);
    refuses(&duplicate, "member_path_or_case_collision");
    let mut ancestor = value();
    ancestor["members"].as_array_mut().unwrap().push(serde_json::json!({"path":"source.bin/child","kind":"file","bytes":0,"sha256":"a".repeat(64),"link_target":"","mode":420}));
    refuses(&ancestor, "non_directory_ancestor");
    let mut link = value();
    link["members"][0] = serde_json::json!({"path":"link","kind":"link","bytes":0,"sha256":"","link_target":"../outside","mode":420});
    refuses(&link, "member_metadata");
}
#[test]
fn archive_order_missing_parts_and_declared_byte_overflow_are_refused() {
    let mut order = value();
    order["archive"]["parts"][0]["ordinal"] = 1.into();
    refuses(&order, "archive_part_assertion");
    let mut missing = value();
    missing["archive"]["parts"] = serde_json::json!([]);
    refuses(&missing, "archive_part_count");
    let mut bytes = value();
    bytes["archive"]["whole_bytes"] = 8.into();
    refuses(&bytes, "archive_byte_coverage");
    let mut overflow = value();
    overflow["archive"]["parts"][0]["bytes"] = u64::MAX.into();
    let extra = overflow["archive"]["parts"][0].clone();
    overflow["archive"]["parts"]
        .as_array_mut()
        .unwrap()
        .push(extra);
    refuses(&overflow, "byte_overflow");
}
#[test]
fn coverage_and_readback_hashes_are_assertions_with_exact_bindings() {
    let mut count = value();
    count["coverage"]["member_count"] = 2.into();
    refuses(&count, "member_coverage_assertion");
    let mut changed = value();
    changed["members"][0]["sha256"] = "c".repeat(64).into();
    refuses(&changed, "readback_assertion");
    let mut archive = value();
    archive["readback"]["archive_sha256"] = "c".repeat(64).into();
    refuses(&archive, "readback_assertion");
    let mut retention = value();
    retention["retention"]["rollback_receipt"] = "refused".into();
    refuses(&retention, "retention_assertion");
}

#[test]
fn member_handle_and_archive_counts_are_finite() {
    let mut members = value();
    let member = members["members"][0].clone();
    members["members"] = serde_json::json!(vec![member; 5001]);
    refuses(&members, "member_count");
    let mut handles = value();
    let handle = handles["consumers"]["handles"][0].clone();
    handles["consumers"]["handles"] = serde_json::json!(vec![handle; 65]);
    refuses(&handles, "handle_count");
    let mut parts = value();
    let part = parts["archive"]["parts"][0].clone();
    parts["archive"]["parts"] = serde_json::json!(vec![part; 65]);
    refuses(&parts, "archive_part_count");
}
