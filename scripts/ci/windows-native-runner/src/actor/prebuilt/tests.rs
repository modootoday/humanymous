use super::*;
use serde_json::json;
fn app() -> Value {
    json!({"schema":"humanymous.dotnet-prebuilt-files/1","files":APP_NAMES.map(|path| json!({"path":path,"bytes":1,"sha256":"a".repeat(64)}))})
}
fn parse(value: &Value, kind: ClosureKind) -> io::Result<Manifest> {
    let bytes = serde_json::to_vec(value).unwrap();
    parse_manifest(&bytes, &format!("{:x}", Sha256::digest(&bytes)), kind)
}
fn provenance() -> Value {
    json!({"managed":{"source_sha":"a".repeat(40),"required_source_sha":"b".repeat(40),"receipt_sha256":"c".repeat(64)},"native":{"source_sha":"d".repeat(40),"cargo_lock_sha256":"e".repeat(64),"receipt_sha256":"f".repeat(64)},"harness":{"source_sha":"1".repeat(40),"cargo_lock_sha256":"2".repeat(64),"receipt_sha256":"3".repeat(64)}})
}
fn context() -> Value {
    json!({"schema":"humanymous.dotnet-prebuilt-launch/1","apphost":"C:/artifacts/app/Humanymous.HostPilot.exe","app_root":"C:/artifacts/app","system_root":"C:/Windows","runtime_root":"C:/artifacts/runtime","private_work":"C:/actor/work","audit_root":"C:/actor/work/audit","handoff_root":"C:/actor/handoff","actor_sid":"S-1-5-21-1-2-3-1001","provenance":provenance(),"budget":{"issued_utc_ms":1000,"run_expires_utc_ms":181000,"cleanup_expires_utc_ms":241000,"parent_elapsed_ms_at_dispatch":100}})
}
fn parsed_launch(value: &Value) -> io::Result<LaunchContext> {
    LaunchContext::parse(
        &serde_json::to_vec(value).unwrap(),
        1200,
        std::time::Instant::now(),
    )
}
#[test]
fn app_manifest_is_closed_bounded_and_hash_bound() {
    let good = app();
    let parsed = parse(&good, ClosureKind::App).unwrap();
    assert_eq!(parsed.total_bytes, 6);
    assert_eq!(parsed.files.len(), 6);
    assert_eq!(parsed.files[0].path, APP_NAMES[0]);
    assert_eq!(parsed.files[0].bytes, 1);
    assert_eq!(parsed.files[0].sha256, "a".repeat(64));
    assert!(
        parse_manifest(
            &serde_json::to_vec(&good).unwrap(),
            &"0".repeat(64),
            ClosureKind::App
        )
        .is_err()
    );
    for path in [
        "../escape.dll",
        "Humanymous.HostPilot.exe:stream",
        "CON.dll",
        "C:/absolute.dll",
        "x\\file.dll",
        "Humanymous.HostPilot.dll.",
        "humanymous_native_abi_export.dll",
        "dotnet.exe",
    ] {
        let mut bad = good.clone();
        bad["files"][0]["path"] = json!(path);
        assert!(parse(&bad, ClosureKind::App).is_err());
    }
    for bytes in [json!(-1), json!(1.0), json!(0), json!(u64::MAX)] {
        let mut bad = good.clone();
        bad["files"][0]["bytes"] = bytes;
        assert!(parse(&bad, ClosureKind::App).is_err());
    }
    let mut bad = good.clone();
    bad["files"][1] = bad["files"][0].clone();
    assert!(parse(&bad, ClosureKind::App).is_err());
    let mut bad = good;
    bad["unexpected"] = json!(true);
    assert!(parse(&bad, ClosureKind::App).is_err());
    let raw = b"{\"schema\":1,\"schema\":2}";
    assert!(parse_manifest(raw, &format!("{:x}", Sha256::digest(raw)), ClosureKind::App).is_err());
}
#[test]
fn runtime_requires_exact_count_three_roles_and_resource_limits() {
    let mut rows = vec![
        json!({"path":"host/fxr/10.0.14/hostfxr.dll","bytes":1,"sha256":"b".repeat(64)}),
        json!({"path":"shared/Microsoft.NETCore.App/10.0.14/coreclr.dll","bytes":1,"sha256":"b".repeat(64)}),
    ];
    for index in 0..334 {
        rows.push(json!({"path":format!("shared/Microsoft.AspNetCore.App/10.0.14/file{index}.dll"),"bytes":1,"sha256":"b".repeat(64)}));
    }
    let good = json!({"schema":"humanymous.dotnet-prebuilt-files/1","files":rows});
    assert!(parse(&good, ClosureKind::Runtime).is_ok());
    let mut bad = good.clone();
    bad["files"].as_array_mut().unwrap().pop();
    assert!(parse(&bad, ClosureKind::Runtime).is_err());
    let mut bad = good.clone();
    bad["files"][0]["path"] = json!("sdk/10.0.403/dotnet.dll");
    assert!(parse(&bad, ClosureKind::Runtime).is_err());
    let mut bad = good.clone();
    bad["files"][0]["bytes"] = json!(64 * 1024 * 1024 + 1);
    assert!(parse(&bad, ClosureKind::Runtime).is_err());
    let mut bad = good;
    for row in bad["files"].as_array_mut().unwrap() {
        row["bytes"] = json!(1024 * 1024);
    }
    assert!(parse(&bad, ClosureKind::Runtime).is_err());
}
#[test]
fn launch_context_has_fixed_arguments_and_no_ambient_environment() {
    let good = context();
    let launch = parsed_launch(&good).unwrap();
    assert_eq!(
        launch.harness_arguments(),
        [
            "--ignored",
            "--nocapture",
            "--exact",
            SELECTOR,
            "--test-threads=1"
        ]
    );
    assert_eq!(launch.launch.actor_sid, "S-1-5-21-1-2-3-1001");
    assert_eq!(
        launch.launch.provenance.harness.receipt_sha256,
        "3".repeat(64)
    );
    assert!(launch.launch.audit_root.ends_with("/audit"));
    let env = launch.environment();
    assert_eq!(env.len(), 11);
    assert!(
        env.iter()
            .all(|(key, _)| !["GITHUB_TOKEN", "DOTNET_STARTUP_HOOKS", "COMSPEC"].contains(key))
    );
    for (key, value) in [
        ("apphost", json!("C:/a/dotnet.exe")),
        ("private_work", json!("C:/artifacts/runtime")),
        ("actor_sid", json!("S-1-5-021-1")),
        ("run_seconds", json!(181)),
        ("cleanup_seconds", json!(61)),
        ("environment", json!({"GITHUB_TOKEN":"private"})),
    ] {
        let mut bad = good.clone();
        bad[key] = value;
        assert!(parsed_launch(&bad).is_err());
    }
}
#[test]
fn shared_parent_context_is_unverified_and_one_mapping_is_retained() {
    let now = std::time::Instant::now();
    let good = context();
    let launch = LaunchContext::parse(&serde_json::to_vec(&good).unwrap(), 1200, now).unwrap();
    let original = launch.budget.run_deadline();
    assert!(original.is_some());
    assert!(launch.budget.cleanup_deadline() > original.unwrap());
    assert_eq!(launch.budget.run_deadline(), original);
    assert_eq!(launch.launch.provenance.managed.source_sha, "a".repeat(40));
    assert_eq!(launch.launch.provenance.native.source_sha, "d".repeat(40));
    let mut cleanup = good;
    cleanup["schema"] = json!("humanymous.dotnet-prebuilt-cleanup/1");
    for key in ["apphost", "app_root", "runtime_root", "audit_root"] {
        cleanup.as_object_mut().unwrap().remove(key);
    }
    assert!(parent_context::decode(&serde_json::to_vec(&cleanup).unwrap()).is_ok());
    assert!(LaunchContext::parse(&serde_json::to_vec(&cleanup).unwrap(), 1200, now).is_err());
}
#[test]
fn joint_input_has_named_lineage_fixed_roles_and_distinct_original_manifest() {
    let app_rows:Vec<_>=APP_NAMES.iter().zip(APP_ROLES).map(|(name,role)|json!({"role":role,"relativepath":name,"bytes":1,"sha256":"a".repeat(64)})).collect();
    let mut runtime_rows = vec![
        json!({"relativepath":"host/fxr/10.0.14/hostfxr.dll","bytes":1,"sha256":"b".repeat(64)}),
        json!({"relativepath":"shared/Microsoft.NETCore.App/10.0.14/coreclr.dll","bytes":1,"sha256":"b".repeat(64)}),
    ];
    for index in 0..334 {
        runtime_rows.push(json!({"relativepath":format!("shared/Microsoft.AspNetCore.App/10.0.14/file{index}.dll"),"bytes":1,"sha256":"b".repeat(64)}));
    }
    let app_rows = json!(app_rows);
    let runtime_rows = json!(runtime_rows);
    let app_digest = format!(
        "{:x}",
        Sha256::digest(normalized_manifest(&app_rows, true).unwrap())
    );
    let runtime_bytes = normalized_manifest(&runtime_rows, false).unwrap();
    let runtime_digest = format!("{:x}", Sha256::digest(&runtime_bytes));
    let provenance = provenance();
    let provenance_digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&provenance).unwrap())
    );
    let harness_digest = "c".repeat(64);
    let pins = InputPins {
        app_manifest: &app_digest,
        runtime_manifest: &runtime_digest,
        harness: &harness_digest,
        provenance: &provenance_digest,
    };
    let good = json!({"schema":"humanymous.dotnet-prebuilt-actor-input/1","profile":"standard-user-dotnet-prebuilt","nonce":"a".repeat(32),"harness":{"path":"C:/artifacts/hosted.exe","bytes":100,"sha256":harness_digest},"app":{"root":"C:/artifacts/app","files":app_rows},"runtime":{"root":"C:/artifacts/runtime","manifest":{"path":"C:/manifests/runtime.json","bytes":runtime_bytes.len(),"sha256":runtime_digest},"files":runtime_rows,"parent_observation_sha256":OBSERVATION_SHA256},"provenance":provenance});
    let decoded = parse_input(&serde_json::to_vec(&good).unwrap(), &pins).unwrap();
    assert_eq!(decoded.nonce, "a".repeat(32));
    assert_eq!(decoded.harness.bytes, 100);
    assert_eq!(decoded.app.files.len(), 6);
    assert_eq!(decoded.runtime.files.len(), 336);
    assert_eq!(decoded.runtime.total_bytes, 336);
    assert_ne!(decoded.app_root, decoded.runtime_root);
    assert_eq!(decoded.runtime_manifest.bytes, runtime_bytes.len() as u64);
    assert_eq!(decoded.provenance.harness.source_sha, "1".repeat(40));
    assert_ne!(runtime_digest, OBSERVATION_SHA256);
    for key in ["command", "environment", "run_seconds"] {
        let mut bad = good.clone();
        bad[key] = json!("untrusted");
        assert!(parse_input(&serde_json::to_vec(&bad).unwrap(), &pins).is_err());
    }
    let mut bad = good.clone();
    bad["runtime"]["parent_observation_sha256"] = json!(runtime_digest);
    assert!(parse_input(&serde_json::to_vec(&bad).unwrap(), &pins).is_err());
    let mut bad = good.clone();
    bad["app"]["files"][0]["role"] = json!("generic-abi");
    assert!(parse_input(&serde_json::to_vec(&bad).unwrap(), &pins).is_err());
    let mut bad = good;
    bad["provenance"]["harness"]["receipt_sha256"] = json!("0".repeat(64));
    assert!(parse_input(&serde_json::to_vec(&bad).unwrap(), &pins).is_err());
}
#[test]
fn recovery_cannot_precede_job_empty_or_delete_identity_before_cleanup() {
    let mut state = CleanupOrder::default();
    assert!(state.start(false).is_err());
    state.start(true).unwrap();
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
