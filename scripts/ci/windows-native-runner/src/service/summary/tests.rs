//! Pure supplied-record checks, not actual service or cleanup qualification.
use super::*;
fn expected(case: &str) -> Expected {
    Expected {
        sha: "a".repeat(40),
        run: 7,
        case: case.into(),
        exit: if case == "normal" { 2 } else { 1 },
    }
}
fn fixture(case: &str) -> (Value, Vec<Logs>) {
    let e = expected(case);
    let mut report = json!({"schema":1,"kind":"native","qualification":"component-only","full_runtime_qualified":false,"source_sha":e.sha,"source":{"source_sha":e.sha,"dirty":false,"github":{"GITHUB_RUN_ID":"7","GITHUB_SHA":e.sha},"files":{"Cargo.lock":"b".repeat(64)}},"skips":0,"errors":0,"status":"blocked","cases":[],"actor":{"sid":"PRIVATE-SID","elevated":false,"enabled_administrator":false},"standard_user_lifecycle":{"actor_case":case,"account_deleted":true,"scratch_deleted":true,"scm_delegation_requested":true,"provisioning":"native-Win32","sid":"PRIVATE-SID","child_launch_attempted":true,"child_exit_code":2,"job_empty_observed":true,"job_kill_on_close":true},"scm_delegation":{"rights":3,"single_generated_sid":true,"restored":true,"original_descriptor_sha256":"c".repeat(64),"sddl":"PRIVATE-DESCRIPTOR"}});
    if case == "after-grant" {
        report["status"] = json!("failed");
        report["errors"] = json!(1);
        report["lifecycle_errors"] = json!(["child execution: injected failure after SCM grant"]);
        let life = &mut report["standard_user_lifecycle"];
        life["child_launch_attempted"] = json!(false);
        for key in ["child_exit_code", "job_empty_observed", "job_kill_on_close"] {
            life[key] = Value::Null;
        }
        return (report, vec![]);
    }
    let mut logs = Vec::new();
    for c in &catalog::CASES {
        let mut stdout = String::new();
        let validated = if c.windows_only {
            let mut raw = json!({"schema":"humanymous.runtime-service-pilot/1","case":c.id,"source_sha":e.sha,"run_id":e.run,"product_qualified":false});
            for key in receipt::FACTS {
                raw[key] = json!(true);
            }
            stdout.push_str(&raw.to_string());
            stdout.push('\n');
            Some(receipt::validate(&stdout, &e.sha, e.run).unwrap())
        } else {
            None
        };
        stdout.push_str("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n");
        let log = Logs {
            list: format!("{}: test\n", c.test).into_bytes(),
            list_error: vec![],
            stdout: stdout.into_bytes(),
            stderr: vec![],
        };
        let mut row = json!({"test":c.test,"validated":true,"exit_code":0,"passed":1,"failed":0,"ignored":0,"measured":0,"discovered":1,"nonselected_tests":0,"stdout_sha256":digest(&log.stdout)});
        if let Some(value) = validated {
            row["runtime_receipt"] = value;
        }
        report[c.field] = row;
        report["cases"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":c.id,"status":"passed"}));
        logs.push(log);
    }
    (report, logs)
}
#[test]
fn normal_is_component_only_exit_two_and_sanitizes_private_fields() {
    let (report, logs) = fixture("normal");
    let e = expected("normal");
    let summary = verify(&report, &e, Some(&logs)).unwrap();
    assert_eq!(summary["observed_exit_code"], 2);
    assert_eq!(summary["verified"], true);
    assert_eq!(summary["full_runtime_qualified"], false);
    assert_eq!(summary["native_producer_qualified"], false);
    assert_eq!(summary["cases"].as_array().unwrap().len(), 2);
    assert!(!summary.to_string().contains("PRIVATE"));
    let mut bad = e;
    bad.exit = 0;
    assert!(verify(&report, &bad, Some(&logs)).is_err());
}
#[test]
fn after_grant_requires_cleanup_and_explicit_no_child_unknown_job() {
    let (report, _) = fixture("after-grant");
    let e = expected("after-grant");
    assert!(verify(&report, &e, None).is_ok());
    for key in ["child_exit_code", "job_empty_observed", "job_kill_on_close"] {
        let mut bad = report.clone();
        bad["standard_user_lifecycle"][key] = json!(true);
        assert!(verify(&bad, &e, None).is_err());
        bad["standard_user_lifecycle"]
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert!(verify(&bad, &e, None).is_err());
    }
    let mut bad = report;
    bad["lifecycle_errors"] = json!(["different failure"]);
    assert!(verify(&bad, &e, None).is_err());
}
#[test]
fn missing_cleanup_or_source_run_binding_never_qualifies_either_profile() {
    for case in ["normal", "after-grant"] {
        let (report, logs) = fixture(case);
        let e = expected(case);
        let ls = if case == "normal" {
            Some(logs.as_slice())
        } else {
            None
        };
        for key in ["account_deleted", "scratch_deleted"] {
            let mut bad = report.clone();
            bad["standard_user_lifecycle"][key] = json!(false);
            assert!(verify(&bad, &e, ls).is_err());
        }
        for (parent, key, value) in [
            ("scm_delegation", "restored", json!(false)),
            ("source", "dirty", json!(true)),
        ] {
            let mut bad = report.clone();
            bad[parent][key] = value;
            assert!(verify(&bad, &e, ls).is_err());
        }
        let mut bad = report.clone();
        bad["source"]["github"]["GITHUB_RUN_ID"] = json!("8");
        assert!(verify(&bad, &e, ls).is_err());
        let mut bad = report;
        bad["source_sha"] = json!("d".repeat(40));
        assert!(verify(&bad, &e, ls).is_err());
    }
}
#[test]
fn sole_runtime_receipt_policy_rejects_every_missing_fact() {
    for key in receipt::FACTS {
        let (mut report, mut logs) = fixture("normal");
        let e = expected("normal");
        let text = std::str::from_utf8(&logs[1].stdout).unwrap();
        let (first, footer) = text.split_once('\n').unwrap();
        let mut raw: Value = serde_json::from_str(first).unwrap();
        raw[key] = json!(false);
        logs[1].stdout = format!("{raw}\n{footer}").into_bytes();
        report[catalog::CASES[1].field]["stdout_sha256"] = json!(digest(&logs[1].stdout));
        assert!(verify(&report, &e, Some(&logs)).is_err());
    }
}
#[test]
fn inventory_footer_log_and_report_must_agree() {
    for mode in 0..4 {
        let (mut report, mut logs) = fixture("normal");
        let e = expected("normal");
        match mode {
            0 => logs[0].list = b"other: test\n".to_vec(),
            1 => logs[0].stderr = b"error".to_vec(),
            2 => {
                let copy = logs[0].stdout.clone();
                logs[0].stdout.extend(copy);
                report[catalog::CASES[0].field]["stdout_sha256"] = json!(digest(&logs[0].stdout));
            }
            _ => report[catalog::CASES[0].field]["validated"] = json!(false),
        }
        assert!(verify(&report, &e, Some(&logs)).is_err());
    }
}
#[test]
fn closed_cli_refuses_missing_duplicate_or_unknown_arguments_before_io() {
    assert!(run(std::iter::empty()).is_err());
    for args in [
        vec!["--unknown", "x"],
        vec!["--source", "a", "--source", "b"],
        vec!["--source"],
    ] {
        assert!(run(args.into_iter().map(str::to_owned)).is_err());
    }
}
#[test]
fn failed_footer_with_success_counts_and_matching_hash_is_refused() {
    let (mut report, mut logs) = fixture("normal");
    let text = String::from_utf8(logs[0].stdout.clone()).unwrap();
    logs[0].stdout = text
        .replace("test result: ok.", "test result: FAILED.")
        .into_bytes();
    report[catalog::CASES[0].field]["stdout_sha256"] = json!(digest(&logs[0].stdout));
    assert!(harness::totals(std::str::from_utf8(&logs[0].stdout).unwrap()).is_ok());
    assert!(verify(&report, &expected("normal"), Some(&logs)).is_err());
}
#[test]
fn run_binding_requires_canonical_positive_decimal_string() {
    for value in [
        json!(7),
        json!("+7"),
        json!("07"),
        json!("0"),
        json!(" 7"),
        json!("18446744073709551616"),
    ] {
        let (mut report, logs) = fixture("normal");
        report["source"]["github"]["GITHUB_RUN_ID"] = value;
        assert!(verify(&report, &expected("normal"), Some(&logs)).is_err());
    }
    assert_eq!(run_id(&json!("7")), Some(7));
}
