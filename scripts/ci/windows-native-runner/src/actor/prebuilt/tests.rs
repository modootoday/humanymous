use super::*;
use serde_json::json;

fn sha(label: &str) -> String {
    format!("{:x}", Sha256::digest(label.as_bytes()))
}
fn input() -> Value {
    let app: Vec<_> = APP_FILES
        .iter()
        .map(|(role, path)| json!({"role":role,"path":path,"bytes":10,"sha256":sha(path)}))
        .collect();
    json!({
        "schema": INPUT_SCHEMA,
        "profile": PROFILE,
        "selector": "hosted",
        "source": {"sha":"a".repeat(40),"manifest_sha256":"b".repeat(64),"cargo_lock_sha256":"c".repeat(64)},
        "harness": {"path":"D:/a/target/debug/deps/hosted-1.exe","bytes":100,"sha256":sha("harness")},
        "managed_source": {"path":"D:/a/src/adapters/dotnet/pilot/Program.cs","bytes":5,"sha256":sha("source")},
        "app": {"root":"D:/a/work/app","files":app},
        "runtime": {"root":"D:/a/sdk/sdk"},
        "provenance": {
            "managed":{"source_sha":"a".repeat(40),"required_source_sha":"a".repeat(40),"receipt_sha256":sha("Humanymous.HostPilot.dll")},
            "native":{"source_sha":"a".repeat(40),"cargo_lock_sha256":"c".repeat(64),"receipt_sha256":sha("humanymous_dotnet_adapter_export.dll")},
            "harness":{"source_sha":"a".repeat(40),"cargo_lock_sha256":"c".repeat(64),"receipt_sha256":sha("harness")},
        },
    })
}
fn parse(value: &Value) -> io::Result<ParsedInput> {
    parse_input(&serde_json::to_vec(value).unwrap())
}

#[test]
fn selectors_fix_exactly_two_inventories_and_five_arguments() {
    assert_eq!(Selector::Hosted.tests(), [KESTREL, GENERIC]);
    assert_eq!(Selector::Package.tests(), [PACKAGE]);
    assert!(Selector::parse("Hosted").is_err());
    assert!(Selector::parse("generic").is_err());
    assert_eq!(
        harness_arguments(Selector::Hosted, GENERIC).unwrap(),
        [
            "--exact",
            GENERIC,
            "--ignored",
            "--test-threads=1",
            "--nocapture"
        ]
    );
    assert!(harness_arguments(Selector::Hosted, PACKAGE).is_err());
    assert!(harness_arguments(Selector::Package, KESTREL).is_err());
    assert!(harness_arguments(Selector::Hosted, "actual_kestrel").is_err());
}

#[test]
fn input_is_closed_and_provenance_names_these_artifacts() {
    let parsed = parse(&input()).unwrap();
    assert_eq!(parsed.selector, Selector::Hosted);
    assert_eq!(parsed.app.len(), 7);
    assert_eq!(parsed.app[6].path, "humanymous_native_abi_export.dll");
    let mut package = input();
    package["selector"] = json!("package");
    assert_eq!(parse(&package).unwrap().selector, Selector::Package);
    let cases: [(&str, Value); 14] = [
        ("/schema", json!("humanymous.dotnet-prebuilt-actor-input/1")),
        ("/profile", json!("standard-user-dotnet")),
        ("/selector", json!("both")),
        ("/source/sha", json!("A".repeat(40))),
        ("/harness/path", json!("relative/hosted.exe")),
        ("/harness/bytes", json!(0)),
        ("/app/root", json!("D:/a/sdk/sdk/app")),
        ("/runtime/root", json!("D:/a/work/app")),
        ("/app/files/0/role", json!("assembly")),
        ("/app/files/6/path", json!("dotnet.exe")),
        (
            "/app/files/2/path",
            json!("../Humanymous.HostPilot.deps.json"),
        ),
        ("/provenance/managed/receipt_sha256", json!(sha("other"))),
        ("/provenance/native/receipt_sha256", json!(sha("other"))),
        ("/provenance/harness/receipt_sha256", json!(sha("other"))),
    ];
    for (pointer, wrong) in cases {
        let mut bad = input();
        *bad.pointer_mut(pointer).unwrap() = wrong;
        assert!(parse(&bad).is_err(), "{pointer}");
    }
    let mut bad = input();
    bad["nonce"] = json!("a".repeat(32));
    assert!(parse(&bad).is_err());
    let mut bad = input();
    bad["app"]["files"].as_array_mut().unwrap().pop();
    assert!(parse(&bad).is_err());
    assert!(parse_input(b"{\"schema\":1,\"schema\":2}").is_err());
}

fn runtime_rows() -> Vec<FileEntry> {
    let mut rows = vec![FileEntry {
        path: "host/fxr/10.0.14/hostfxr.dll".into(),
        bytes: RUNTIME_BYTES - (RUNTIME_FILES as u64 - 1),
        sha256: sha("fxr"),
    }];
    for index in 1..RUNTIME_FILES {
        rows.push(FileEntry {
            path: format!("shared/Microsoft.NETCore.App/10.0.14/f{index}.dll"),
            bytes: 1,
            sha256: sha(&index.to_string()),
        });
    }
    rows
}
#[test]
fn runtime_closure_must_equal_the_pinned_sdk_manifest() {
    let rows = runtime_rows();
    // Right count and total, but not the pinned SDK's bytes: refused.
    assert_eq!(rows.len(), RUNTIME_FILES);
    assert!(runtime_closure_ok(&rows).is_err());
    let mut shorter = rows.clone();
    shorter.pop();
    assert!(runtime_closure_ok(&shorter).is_err());
    let mut outside = rows.clone();
    outside[1].path = "sdk/10.0.403/dotnet.dll".into();
    assert!(runtime_closure_ok(&outside).is_err());
    let mut duplicate = rows;
    duplicate[2].path = duplicate[1].path.to_ascii_uppercase();
    assert!(runtime_closure_ok(&duplicate).is_err());
    assert!(hex(RUNTIME_MANIFEST_SHA256, 64) && hex(MUXER_SHA256, 64));
}
#[test]
fn normalized_manifest_is_order_independent_sorted_and_lf_terminated() {
    let rows = vec![
        FileEntry {
            path: "b".into(),
            bytes: 2,
            sha256: sha("b"),
        },
        FileEntry {
            path: "a".into(),
            bytes: 1,
            sha256: sha("a"),
        },
    ];
    let reversed: Vec<_> = rows.iter().rev().cloned().collect();
    let bytes = normalized_manifest(&rows).unwrap();
    assert_eq!(bytes, normalized_manifest(&reversed).unwrap());
    assert!(bytes.ends_with(b"files/1\"}\n"));
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.starts_with("{\"files\":[{\"bytes\":1,\"path\":\"a\","));
    assert!(text.contains("\"schema\":\"humanymous.dotnet-prebuilt-files/1\""));
}

fn host() -> UnverifiedHostContext {
    serde_json::from_value(json!({
        "schema":"humanymous.dotnet-host-launch/2",
        "apphost":"C:/ProgramData/hmart-1/app/Humanymous.HostPilot.exe",
        "app_root":"C:/ProgramData/hmart-1/app",
        "runtime_root":"C:/ProgramData/hmart-1/runtime",
        "system_root":"C:/Windows",
        "private_work":"C:/ProgramData/hmrun1/work",
        "audit_root":"C:/ProgramData/hmrun1/work/audit",
        "handoff_root":"C:/ProgramData/hmrun1/handoff",
        "actor":{"os":"windows","sid":"S-1-5-21-1-2-3-1001"},
        "provenance":input()["provenance"],
        "budget":{"issued_utc_ms":1000,"run_expires_utc_ms":181000,"cleanup_expires_utc_ms":241000,"parent_elapsed_ms_at_dispatch":10},
    }))
    .unwrap()
}
#[test]
fn host_paths_are_absolute_fixed_and_disjoint() {
    assert!(host_paths_ok(&host()).is_ok());
    let edits: [fn(&mut UnverifiedHostContext); 6] = [
        |c| c.apphost = "C:/ProgramData/hmart-1/app/other.exe".into(),
        |c| c.audit_root = "C:/ProgramData/hmrun1/audit".into(),
        |c| c.handoff_root = "C:/ProgramData/hmrun1/work/handoff".into(),
        |c| c.runtime_root = "C:/ProgramData/hmart-1/app/runtime".into(),
        |c| c.system_root = None,
        |c| c.private_work = "ProgramData/hmrun1/work".into(),
    ];
    for edit in edits {
        let mut context = host();
        edit(&mut context);
        assert!(host_paths_ok(&context).is_err());
    }
}

fn line() -> Value {
    json!({"schema":CHILD_SCHEMA,"selector":"hosted","producer_bound":true,"harness_verified":true,
        "private_created":true,"private_removed":true,"handoff_written":true,"tests":[
        {"name":KESTREL,"exit_code":0,"timed_out":false,"readers_joined":true,"creator_channel":true},
        {"name":GENERIC,"exit_code":0,"timed_out":false,"readers_joined":true,"creator_channel":false}]})
}
#[test]
fn child_line_needs_binding_every_test_and_actor_cleanup() {
    assert!(child_line_ok(&line(), Selector::Hosted));
    assert!(!child_line_ok(&line(), Selector::Package));
    for pointer in [
        "/producer_bound",
        "/harness_verified",
        "/private_created",
        "/private_removed",
        "/handoff_written",
        "/tests/0/readers_joined",
        "/tests/1/timed_out",
        "/tests/1/creator_channel",
    ] {
        let mut bad = line();
        let value = bad.pointer_mut(pointer).unwrap();
        *value = json!(!value.as_bool().unwrap());
        assert!(!child_line_ok(&bad, Selector::Hosted), "{pointer}");
    }
    let mut bad = line();
    bad["tests"][0]["exit_code"] = json!(101);
    assert!(!child_line_ok(&bad, Selector::Hosted));
    let mut bad = line();
    bad["tests"].as_array_mut().unwrap().reverse();
    assert!(!child_line_ok(&bad, Selector::Hosted));
}
#[test]
fn single_test_footer_is_exact() {
    let good = format!(
        "running 1 test\ntest {KESTREL} ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 1.00s\n"
    );
    assert!(single_test_passed(&good, KESTREL));
    assert!(!single_test_passed(&good, GENERIC));
    assert!(!single_test_passed(&format!("{good}{good}"), KESTREL));
    assert!(!single_test_passed(
        &good.replace("1 passed; 0 failed", "0 passed; 1 failed"),
        KESTREL
    ));
}

#[test]
fn recovery_cannot_precede_job_empty_or_delete_identity_before_cleanup() {
    let mut state = CleanupOrder::default();
    assert!(state.start(false).is_err());
    assert!(!state.started());
    state.start(true).unwrap();
    assert!(state.started());
    assert!(state.start_recovery().is_err());
    assert!(state.delete_account().is_err());
    state.main_empty().unwrap();
    state.start_recovery().unwrap();
    assert!(state.delete_account().is_err());
    state.private_removed().unwrap();
    assert!(state.profile_absent_after().is_err());
    state.cleanup_empty().unwrap();
    state.profile_absent_after().unwrap();
    state.delete_account().unwrap();
    state.delete_scratch().unwrap();
    assert!(!state.clean_pass(true));
    state.readers_joined = true;
    state.outcome_bound = true;
    assert!(state.clean_pass(true));
    assert!(!state.clean_pass(false));
}
#[test]
fn normal_cleanup_needs_profile_absence_and_all_terminal_facts() {
    let mut state = CleanupOrder::default();
    state.start(true).unwrap();
    state.private_removed().unwrap();
    state.main_empty().unwrap();
    assert!(state.delete_account().is_err());
    state.profile_absent_after().unwrap();
    state.delete_account().unwrap();
    state.delete_scratch().unwrap();
    state.readers_joined = true;
    state.outcome_bound = true;
    assert!(state.clean_pass(true));
    assert!(state.start_recovery().is_err());
    assert!(state.delete_scratch().is_err());
    state.profile_after_absent = false;
    assert!(!state.clean_pass(true));
}
