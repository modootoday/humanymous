use super::*;
use serde_json::{Value, json};
fn source() -> Value {
    json!({"managed":{"source_sha":"a".repeat(40),"required_source_sha":"b".repeat(40),"receipt_sha256":"c".repeat(64)},"native":{"source_sha":"d".repeat(40),"cargo_lock_sha256":"e".repeat(64),"receipt_sha256":"f".repeat(64)},"harness":{"source_sha":"1".repeat(40),"cargo_lock_sha256":"2".repeat(64),"receipt_sha256":"3".repeat(64)}})
}
fn wire(cleanup: bool, elapsed: u64) -> Value {
    let mut value = json!({"schema":if cleanup{"humanymous.dotnet-prebuilt-cleanup/1"}else{"humanymous.dotnet-prebuilt-launch/1"},"system_root":"C:/Windows","private_work":"C:/actor/work","handoff_root":"C:/actor/handoff","actor_sid":"S-1-5-21-1-2-3-1001","provenance":source(),"budget":{"issued_utc_ms":1000,"run_expires_utc_ms":181000,"cleanup_expires_utc_ms":241000,"parent_elapsed_ms_at_dispatch":elapsed}});
    if !cleanup {
        value["apphost"] = json!("C:/app/Humanymous.HostPilot.exe");
        value["app_root"] = json!("C:/app");
        value["runtime_root"] = json!("C:/runtime");
        value["audit_root"] = json!("C:/actor/work/audit");
    }
    value
}
fn parsed(value: &Value) -> Result<UnverifiedParentContext, WireError> {
    decode(&serde_json::to_vec(value).unwrap())
}
#[test]
fn golden_cleanup_wire_and_workload_round_trip_are_closed() {
    let context = parsed(&wire(true, 180000)).unwrap();
    let bytes = encode(&context).unwrap();
    let expected = format!(
        "{{\"schema\":\"humanymous.dotnet-prebuilt-cleanup/1\",\"system_root\":\"C:/Windows\",\"private_work\":\"C:/actor/work\",\"handoff_root\":\"C:/actor/handoff\",\"actor_sid\":\"S-1-5-21-1-2-3-1001\",\"provenance\":{{\"managed\":{{\"source_sha\":\"{}\",\"required_source_sha\":\"{}\",\"receipt_sha256\":\"{}\"}},\"native\":{{\"source_sha\":\"{}\",\"cargo_lock_sha256\":\"{}\",\"receipt_sha256\":\"{}\"}},\"harness\":{{\"source_sha\":\"{}\",\"cargo_lock_sha256\":\"{}\",\"receipt_sha256\":\"{}\"}}}},\"budget\":{{\"issued_utc_ms\":1000,\"run_expires_utc_ms\":181000,\"cleanup_expires_utc_ms\":241000,\"parent_elapsed_ms_at_dispatch\":180000}}}}\n",
        "a".repeat(40),
        "b".repeat(40),
        "c".repeat(64),
        "d".repeat(40),
        "e".repeat(64),
        "f".repeat(64),
        "1".repeat(40),
        "2".repeat(64),
        "3".repeat(64)
    );
    assert_eq!(bytes, expected.as_bytes());
    assert!(decode(&bytes).is_ok());
    let workload = parsed(&wire(false, 100)).unwrap();
    let encoded = encode(&workload).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&encoded).unwrap(),
        wire(false, 100)
    );
}
#[test]
fn unknown_duplicate_control_oversized_and_wrong_lineage_fail() {
    let oversized = vec![b' '; MAX_BYTES + 1];
    for bytes in [
        b"{\"schema\":1,\"schema\":2}".as_slice(),
        oversized.as_slice(),
    ] {
        assert!(decode(bytes).is_err());
    }
    for cleanup in [false, true] {
        let good = wire(cleanup, 100);
        let mut bad = good.clone();
        bad["command"] = json!("private");
        assert!(parsed(&bad).is_err());
        let mut bad = good.clone();
        bad["provenance"]["managed"]["extra"] = json!(1);
        assert!(parsed(&bad).is_err());
        let mut bad = good.clone();
        bad["private_work"] = json!("C:/bad\npath");
        assert!(parsed(&bad).is_err());
        let mut bad = good;
        bad["provenance"]["native"]["source_sha"] = json!("F".repeat(40));
        assert!(parsed(&bad).is_err());
    }
    let mut bad = wire(true, 180000);
    bad["apphost"] = json!("C:/workload.exe");
    assert!(parsed(&bad).is_err());
}
#[test]
fn one_original_budget_and_cleanup_only_boundaries() {
    let now = Instant::now();
    let context = parsed(&wire(false, 100)).unwrap();
    let budget = context.map_budget(1200, now).unwrap();
    assert_eq!(
        budget
            .run_deadline()
            .unwrap()
            .duration_since(now)
            .as_millis(),
        179800
    );
    assert_eq!(
        budget.cleanup_deadline().duration_since(now).as_millis(),
        239800
    );
    assert_eq!(
        budget
            .run_deadline()
            .unwrap()
            .duration_since(now + Duration::from_millis(800))
            .as_millis(),
        179000
    );
    for elapsed in [180000_u64, 180001, 239999] {
        let context = parsed(&wire(true, elapsed)).unwrap();
        let budget = context.map_budget(1000 + elapsed, now).unwrap();
        assert!(budget.run_deadline().is_none());
        assert_eq!(
            budget.cleanup_deadline().duration_since(now).as_millis(),
            u128::from(240000 - elapsed)
        );
        assert!(parsed(&wire(false, elapsed)).is_err());
    }
    assert!(parsed(&wire(true, 240000)).is_err());
    for wall in [0, 1000, 181000, 241000, u64::MAX] {
        assert!(context.map_budget(wall, now).is_err());
    }
}
#[test]
fn fixed_environment_has_no_ambient_keys_and_cleanup_has_no_runtime() {
    let workload = parsed(&wire(false, 100)).unwrap();
    let env = workload.fixed_environment();
    assert_eq!(env.len(), 11);
    assert!(env.contains(&("DOTNET_EnableDiagnostics", "0".into())));
    assert!(
        env.iter()
            .all(|(key, _)| !["GITHUB_TOKEN", "DOTNET_STARTUP_HOOKS", "COMSPEC"].contains(key))
    );
    let cleanup = parsed(&wire(true, 180000)).unwrap();
    let env = cleanup.fixed_environment();
    assert_eq!(env.len(), 6);
    assert!(env.iter().all(|(key, _)| !key.starts_with("DOTNET")));
}
