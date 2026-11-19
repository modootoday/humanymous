//! Ports of legacy internal/gate/configsign_test.go and differential parity
//! against `configVersion` outputs captured from the frozen Go oracle at
//! 981d25c (fixture `provenance`). Expectations are never regenerated.
use super::*;
use crate::gateway::settings::overlay::OverlayFileStore;
use serde_json::Value;
use std::path::PathBuf;

fn golden() -> Value {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/gateway/configsign-guard-go-golden.json"
    ))
    .expect("golden fixture")
}

struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "hmn-configsign-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn policy(case: &Value) -> BootPolicy {
    BootPolicy {
        global_monitor: case["global_monitor"].as_bool().unwrap(),
        routes: case["routes"]
            .as_object()
            .map(|routes| {
                routes
                    .iter()
                    .map(|(prefix, preset)| (prefix.clone(), preset.as_str().unwrap().to_owned()))
                    .collect()
            })
            .unwrap_or_default(),
        rate_window_nanos: case["rate_window_ns"].as_i64().unwrap(),
        rate_soft: case["rate_soft"].as_i64().unwrap(),
        rate_hard: case["rate_hard"].as_i64().unwrap(),
        hmac_key: case["mac_material"].as_str().unwrap().as_bytes().to_vec(),
    }
}

/// Every captured server posture, kill switch and boot-loaded overlay yields
/// the exact Go `cfg-…` version, including the rate defaults, sub-second
/// window truncation, bytewise route order, the overlay digest of a pending
/// overlay and a discarded invalid overlay.
#[test]
fn config_version_matches_go_capture() {
    let golden = golden();
    let cases = golden["outputs"]["config_version"].as_array().unwrap();
    assert_eq!(cases.len(), 19);
    let mut compared = 0;
    for case in cases {
        let name = case["name"].as_str().unwrap();
        if !case["server_error"].as_str().unwrap().is_empty() {
            // Go refused to construct the server (attested catch-all); the
            // refusal belongs to the route-policy row, not to this hash.
            assert_eq!(name, "attested_catch_all_refused");
            continue;
        }
        let policy = policy(case);
        let kill = case["kill_switch"].as_bool().unwrap();
        let scratch = Scratch::new(name);
        let active = match case["overlay"].as_str() {
            Some(body) => {
                std::fs::write(scratch.0.join("settings.overlay.v1.json"), body).unwrap();
                let store = OverlayFileStore::open(&scratch.0).unwrap();
                let active = store.active();
                assert_eq!(
                    active.is_some(),
                    case["store_loaded"].as_bool().unwrap(),
                    "{name}: store load"
                );
                active
            }
            None => None,
        };
        let version = policy.config_version(kill, active.as_ref());
        assert_eq!(version, case["config_version"].as_str().unwrap(), "{name}");
        assert!(case["stable"].as_bool().unwrap());
        assert_eq!(policy.config_version(kill, active.as_ref()), version);
        compared += 1;
    }
    assert_eq!(compared, 18);
}

/// Go TestConfigVersionSignedAndStable: a stable `cfg-` hash that changes when
/// the effective policy (the kill switch) changes.
#[test]
fn config_version_signed_and_stable() {
    let policy = BootPolicy {
        rate_window_nanos: 60_000_000_000,
        rate_soft: 998,
        rate_hard: 1000,
        hmac_key: b"k".to_vec(),
        ..BootPolicy::default()
    };
    let v1 = policy.config_version(false, None);
    assert!(v1.starts_with("cfg-") && v1.len() == 16, "{v1}");
    assert_eq!(policy.config_version(false, None), v1);
    assert_ne!(policy.config_version(true, None), v1);
}

#[test]
fn rate_defaults_follow_legacy_helpers() {
    let policy = BootPolicy {
        rate_window_nanos: 2_999_999_999,
        rate_soft: 0,
        rate_hard: -3,
        ..BootPolicy::default()
    };
    assert_eq!(policy.rate_window_sec(), 2);
    assert_eq!((policy.rate_soft(), policy.rate_hard()), (60, 120));
    let defaults = BootPolicy::default();
    assert_eq!(defaults.rate_window_sec(), 10);
    assert_eq!(
        BootPolicy {
            rate_window_nanos: i64::MAX,
            ..BootPolicy::default()
        }
        .rate_window_sec(),
        9_223_372_036
    );
}
