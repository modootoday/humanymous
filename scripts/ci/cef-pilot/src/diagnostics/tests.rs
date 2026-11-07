use super::*;
fn bootstrap_records() -> Vec<Value> {
    let config = bootstrap::config().unwrap();
    let mut records = vec![
        json!({"schema":1,"kind":"profiles","phase":"before","inventory_ok":true,"changed":null,"error_kind":"none"}),
    ];
    for case in config["cases"].as_array().unwrap() {
        records.push(json!({"schema":1,"kind":"case","case_id":case["id"],"spawn_ok":true,"wait_completed":true,
            "deadline_expired":false,"reaped":true,"exit_code":78,"cleanup_attempted":false,
            "spawn_error_kind":"none","wait_error_kind":"none","cleanup_error_kind":"none"}));
    }
    records.push(json!({"schema":1,"kind":"profiles","phase":"after","inventory_ok":true,"changed":false,"error_kind":"none"}));
    records
}
fn marker_bytes(records: &[Value]) -> Vec<u8> {
    let config = bootstrap::config().unwrap();
    let prefix = config["marker_prefix"].as_str().unwrap();
    records
        .iter()
        .map(|record| format!("{prefix}{record}\n"))
        .collect::<String>()
        .into_bytes()
}
fn source() -> Value {
    json!({"files":{"crates/browser/native/bootstrap/src/lib.rs":"a"}})
}
fn message(file: &str) -> Value {
    json!({"reason":"compiler-message","message":{"level":"error","code":{"code":"E0308"},"rendered":"PRIVATE_CANARY","message":"PRIVATE_CANARY","spans":[{"is_primary":true,"file_name":file,"line_start":9,"text":[{"text":"PRIVATE_CANARY"}]}]}})
}
#[test]
fn compiler_projection_keeps_only_codes_and_reviewed_locations() {
    let bytes = message("crates/browser/native/bootstrap/src/lib.rs").to_string();
    let value = project(bytes.as_bytes(), &source(), Some(101), false);
    assert_eq!(value["diagnostics"][0]["error_code"], "E0308");
    assert_eq!(value["diagnostics"][0]["locations"][0]["line"], 9);
    assert!(!value.to_string().contains("PRIVATE_CANARY"));
    for path in [
        "C:/PRIVATE_CANARY/lib.rs",
        "../PRIVATE_CANARY.rs",
        "crates/PRIVATE_CANARY.rs",
    ] {
        let value = project(
            message(path).to_string().as_bytes(),
            &source(),
            Some(101),
            false,
        );
        assert_eq!(value["diagnostics"][0]["locations"], json!([]));
        assert!(!value.to_string().contains("PRIVATE_CANARY"));
    }
}
#[test]
fn malformed_and_unstructured_logs_cannot_invent_diagnostics() {
    let value = project(
        b"PRIVATE_CANARY\n{broken\n{\"reason\":\"build-script-executed\"}\n",
        &source(),
        Some(101),
        false,
    );
    assert_eq!(value["status"], "unknown");
    assert_eq!(value["malformed_json_records"], 1);
    assert!(!value.to_string().contains("PRIVATE_CANARY"));
}
#[test]
fn long_and_excess_records_are_bounded_with_explicit_omissions() {
    let mut bytes = vec![b'x'; MAX_LINE + 1];
    bytes.push(b'\n');
    for _ in 0..70 {
        bytes.extend(
            message("crates/browser/native/bootstrap/src/lib.rs")
                .to_string()
                .bytes(),
        );
        bytes.push(b'\n');
    }
    let value = project(&bytes, &source(), None, true);
    assert_eq!(value["diagnostics"].as_array().unwrap().len(), 64);
    assert_eq!(value["oversized_records"], 1);
    assert_eq!(value["omitted_diagnostics"], 6);
    assert_eq!(value["diagnostic_read_truncated"], true);
    let mut warning = message("crates/browser/native/bootstrap/src/lib.rs");
    warning["message"]["level"] = json!("warning");
    let text = format!(
        "{}{}",
        format!("{warning}\n").repeat(64),
        message("crates/browser/native/bootstrap/src/lib.rs")
    );
    let value = project(text.as_bytes(), &source(), Some(101), false);
    assert_eq!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["level"] == "error")
            .count(),
        1
    );
}
#[test]
fn arbitrary_codes_and_lints_remain_unknown() {
    assert_eq!(code(&json!("PRIVATE_CANARY")), (None, None));
    assert_eq!(code(&json!("E0000PRIVATE_CANARY")), (None, None));
    assert_eq!(
        code(&json!("unused_imports")),
        (None, Some("unused_imports"))
    );
    assert_eq!(
        read(
            Path::new("absent-private-compiler-log"),
            &source(),
            None,
            false
        )["status"],
        "missing_or_unreadable"
    );
}
#[test]
fn public_stage_and_protocol_fields_cannot_echo_private_text() {
    let report = json!({"stage":"PRIVATE_CANARY","private_error":"PRIVATE_CANARY",
        "source":{"commit":"PRIVATE_CANARY","files":{}},
        "protocol_preflight":{"required":"PRIVATE_CANARY","raw":"PRIVATE_CANARY"},
        "native_build":{"status":"PRIVATE_CANARY","private_log_sha256":"PRIVATE_CANARY","raw":"PRIVATE_CANARY",
        "diagnostics":[{"level":"error","error_code":"PRIVATE_CANARY","lint":"PRIVATE_CANARY","locations":[{"file":"PRIVATE_CANARY","line":1}]}]}});
    let value = public(&report);
    assert_eq!(value["stage"], "unknown");
    assert!(value["source_sha"].is_null());
    assert!(value["protocol_preflight"]["required"].is_null());
    assert!(!value.to_string().contains("PRIVATE_CANARY"));
}
#[test]
fn bootstrap_harness_requires_one_exact_case_footer_and_consistent_exit() {
    for (label, status, exit, counts) in [
        ("ok", "ok", 0, "1 passed; 0 failed"),
        ("FAILED", "FAILED", 101, "0 passed; 1 failed"),
    ] {
        let text = format!(
            "test {} ... {label}\ntest result: {status}. {counts}; 0 ignored; 0 measured; 7 filtered out; finished in 0.1s\nPRIVATE_CANARY\n",
            super::super::CASE
        );
        let value = bootstrap_outcome(text.as_bytes(), Some(exit), false);
        assert_eq!(value["executed_tests"], 1);
        assert_eq!(value["filtered"], 7);
        assert!(!value.to_string().contains("PRIVATE_CANARY"));
        assert!(bootstrap_outcome(text.as_bytes(), None, false)["executed_tests"].is_null());
    }
    let mut records = bootstrap_records();
    records[1]["deadline_expired"] = json!(true);
    let value = bootstrap::observations(&marker_bytes(&records), false);
    assert_eq!(value["observations_complete"], true);
    assert_eq!(value["cases"][0]["deadline_expired"], true);
    assert_eq!(bootstrap::public_observations(&value), value);
    let profile = bootstrap::config().unwrap()["protocol_preflight"].clone();
    let names: Vec<_> = profile["test_names"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    let log = format!(
        "{}test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out;",
        names
            .iter()
            .map(|name| format!("test {name} ... ok\n"))
            .collect::<String>()
    );
    assert_eq!(verify_bootstrap_protocol(&log).unwrap()["passed"], 3);
    assert!(verify_bootstrap_protocol(&format!("{log}\n{log}")).is_err());
}
#[test]
fn bootstrap_missing_duplicate_ignored_or_truncated_receipts_stay_unknown() {
    let valid = format!(
        "test {} ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;",
        super::super::CASE
    );
    for text in [
        "PRIVATE_CANARY".to_owned(),
        format!("{valid}\n{valid}"),
        valid.replace("0 ignored", "1 ignored"),
        valid.replace(" ... ok", " ... PRIVATE_CANARY"),
    ] {
        assert!(bootstrap_outcome(text.as_bytes(), Some(0), false)["executed_tests"].is_null());
    }
    assert!(bootstrap_outcome(valid.as_bytes(), Some(101), false)["executed_tests"].is_null());
    assert!(bootstrap_outcome(valid.as_bytes(), Some(0), true)["executed_tests"].is_null());
    let records = bootstrap_records();
    assert_eq!(
        bootstrap::observations(&marker_bytes(&records[..1]), false)["status"],
        "partial"
    );
    assert_eq!(bootstrap::observations(&[], false)["status"], "missing");
    assert_eq!(
        bootstrap::observations(&marker_bytes(&records), true)["observations_complete"],
        false
    );
    let mut unobserved = records.clone();
    unobserved[1]["spawn_ok"] = json!(false);
    unobserved[1]["wait_completed"] = json!(false);
    unobserved[1]["deadline_expired"] = Value::Null;
    unobserved[1]["reaped"] = Value::Null;
    unobserved[1]["exit_code"] = Value::Null;
    unobserved[1]["spawn_error_kind"] = json!("not_found");
    let partial = bootstrap::observations(&marker_bytes(&unobserved[..2]), false);
    assert_eq!(partial["status"], "partial");
    assert!(partial["cases"][0]["reaped"].is_null());
    for mutation in 0..6 {
        let mut changed = records.clone();
        match mutation {
            0 => changed[1]["case_id"] = json!("PRIVATE_CANARY"),
            1 => changed[1]["raw"] = json!("PRIVATE_CANARY"),
            2 => changed[1]["reaped"] = json!(false),
            3 => changed[1]["wait_error_kind"] = json!("PRIVATE_CANARY"),
            4 => changed.push(changed[0].clone()),
            _ => changed[1]["spawn_error_kind"] = json!("PRIVATE_CANARY".repeat(100)),
        }
        let value = bootstrap::observations(&marker_bytes(&changed), false);
        assert_eq!(value["status"], "rejected");
        assert!(!value.to_string().contains("PRIVATE_CANARY"));
    }
    let mut changed = bootstrap::observations(&marker_bytes(&records), false);
    changed["cases"][0]["private"] = json!("PRIVATE_CANARY");
    let value = bootstrap::public_observations(&changed);
    assert_eq!(value["status"], "rejected");
    assert!(!value.to_string().contains("PRIVATE_CANARY"));
}
#[test]
fn duplicate_and_escaped_decoded_marker_keys_are_rejected() {
    let records = bootstrap_records();
    let config = bootstrap::config().unwrap();
    let prefix = config["marker_prefix"].as_str().unwrap();
    let record = records[1].to_string();
    let key = format!("\"case_id\":{}", records[1]["case_id"]);
    for duplicate in [
        format!("\"case_id\":\"PRIVATE_CANARY\",{key}"),
        format!(r#""case_\u0069d":"PRIVATE_CANARY",{key}"#),
    ] {
        let bytes = format!("{prefix}{}\n", record.replacen(&key, &duplicate, 1));
        let result = bootstrap::observations(bytes.as_bytes(), false);
        assert_eq!(result["status"], "rejected");
        assert!(!result.to_string().contains("PRIVATE_CANARY"));
    }
    let duplicate = br#"{"reason":"compiler-message","reason":"compiler-message","message":{"level":"error","code":{"code":"E0308"}}}"#;
    let result = project(duplicate, &source(), Some(101), false);
    assert_eq!(result["malformed_json_records"], 1);
    assert_eq!(result["diagnostics"], json!([]));
}
