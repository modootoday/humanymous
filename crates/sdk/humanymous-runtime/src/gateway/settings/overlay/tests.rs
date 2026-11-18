//! One-to-one ports of the oracle `internal/gate/settings` tests (and the
//! `TestConfigureEngineFromEffectiveNetPolicy` case of `admin_settings_test.go`),
//! plus the differential golden parity in `parity`.
use super::*;
use humanymous_core::scoring::engine::Engine;
use std::path::PathBuf;

mod parity;

fn boot(key: &[u8]) -> BootInput {
    BootInput {
        hmac_key: key.to_vec(),
        rate_window_sec: 10,
        rate_soft: 60,
        rate_hard: 120,
        ..BootInput::default()
    }
}

fn modes(entries: &[(&str, Mode)]) -> BTreeMap<String, Mode> {
    entries
        .iter()
        .map(|(id, mode)| ((*id).to_owned(), mode.clone()))
        .collect()
}

fn class_of(overlay: Overlay) -> MutationClass {
    classify(&resolve(&boot(b"k"), None), Some(&overlay))
}

pub(super) struct TempDir(PathBuf);
impl TempDir {
    pub(super) fn new(tag: &str) -> Self {
        let mut nonce = [0_u8; 8];
        getrandom::fill(&mut nonce).unwrap();
        let path = std::env::temp_dir().join(format!("hmn-overlay-{tag}-{}", hex::encode(nonce)));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub(super) fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// classify_test.go

#[test]
fn classify_hr12_monitor_is_b() {
    let overlay = Overlay {
        hard_rules: modes(&[("HR-12", Mode::Monitor)]),
        ..Overlay::default()
    };
    assert_eq!(class_of(overlay), MutationClass::B);
}

#[test]
fn classify_hr18_monitor_is_c() {
    let overlay = Overlay {
        hard_rules: modes(&[("HR-18", Mode::Monitor)]),
        ..Overlay::default()
    };
    assert_eq!(class_of(overlay), MutationClass::C);
}

#[test]
fn classify_challenge_at_big_raise_is_c() {
    let overlay = Overlay {
        scoring: Some(ScoringPatch {
            challenge_at: Some(50.0),
            ..ScoringPatch::default()
        }),
        ..Overlay::default()
    };
    assert_eq!(class_of(overlay), MutationClass::C);
}

#[test]
fn classify_lower_challenge_is_a() {
    let overlay = Overlay {
        scoring: Some(ScoringPatch {
            challenge_at: Some(20.0),
            ..ScoringPatch::default()
        }),
        ..Overlay::default()
    };
    assert_eq!(class_of(overlay), MutationClass::A);
}

#[test]
fn classify_net_correlation_monitor_is_c() {
    let overlay = Overlay {
        net_policy: modes(&[("net.correlation", Mode::Monitor)]),
        ..Overlay::default()
    };
    assert_eq!(class_of(overlay), MutationClass::C);
}

#[test]
fn classify_route_attested_to_monitor_is_d() {
    let mut input = boot(b"k");
    input.routes.insert("/checkout".into(), "attested".into());
    let overlay = Overlay {
        routes: [("/checkout".to_owned(), "monitor".to_owned())].into(),
        ..Overlay::default()
    };
    assert_eq!(
        classify(&resolve(&input, None), Some(&overlay)),
        MutationClass::D
    );
}

#[test]
fn classify_rate_hard_raise_is_b() {
    let overlay = Overlay {
        rate_limit: Some(RateLimitPatch {
            hard: Some(200),
            ..RateLimitPatch::default()
        }),
        ..Overlay::default()
    };
    assert_eq!(class_of(overlay), MutationClass::B);
}

#[test]
fn classify_rate_window_shorter_is_b() {
    let overlay = Overlay {
        rate_limit: Some(RateLimitPatch {
            window_sec: Some(5),
            ..RateLimitPatch::default()
        }),
        ..Overlay::default()
    };
    assert_eq!(class_of(overlay), MutationClass::B);
}

// settings_test.go

#[test]
fn validate_refuses_integrity_off() {
    let overlay = Overlay {
        schema_version: "1.0.0".into(),
        gates: modes(&[("gate.smuggle", Mode::Off)]),
        ..Overlay::default()
    };
    assert!(validate(Some(&overlay)).is_err());
}

#[test]
fn validate_refuses_hr_off() {
    let overlay = Overlay {
        hard_rules: modes(&[("HR-18", Mode::Off)]),
        ..Overlay::default()
    };
    assert!(validate(Some(&overlay)).is_err());
}

#[test]
fn validate_refuses_net_tcp_enforce() {
    let overlay = Overlay {
        net_policy: modes(&[("net.tcp", Mode::Enforce)]),
        ..Overlay::default()
    };
    assert!(validate(Some(&overlay)).is_err());
}

#[test]
fn validate_scoring_bounds() {
    let overlay = Overlay {
        scoring: Some(ScoringPatch {
            challenge_at: Some(5.0),
            ..ScoringPatch::default()
        }),
        ..Overlay::default()
    };
    assert!(validate(Some(&overlay)).is_err());
}

#[test]
fn validate_refuses_non_positive_rate_hard() {
    let overlay = Overlay {
        rate_limit: Some(RateLimitPatch {
            hard: Some(0),
            ..RateLimitPatch::default()
        }),
        ..Overlay::default()
    };
    assert!(validate(Some(&overlay)).is_err());
}

#[test]
fn resolve_empty_overlay_identity() {
    let mut input = boot(b"test-key-32-bytes-long!!!!!!!!");
    input.routes.insert("/".into(), "balanced".into());
    let first = resolve(&input, None);
    let second = resolve(&input, None);
    assert!(first.empty_overlay);
    assert_eq!(first.config_version, second.config_version);
    assert_eq!(
        (first.challenge_at, first.deny_at, first.layer_cap),
        (30.0, 70.0, 60.0)
    );
    assert_eq!(first.hr_mode("HR-12"), Mode::Enforce);
}

#[test]
fn resolve_overlay_scoring_and_kill() {
    let overlay = Overlay {
        schema_version: "1.0.0".into(),
        overlay_id: "ovl_test".into(),
        status: "active".into(),
        scoring: Some(ScoringPatch {
            challenge_at: Some(40.0),
            ..ScoringPatch::default()
        }),
        hard_rules: modes(&[("HR-12", Mode::Monitor)]),
        ..Overlay::default()
    };
    validate(Some(&overlay)).unwrap();
    let mut input = boot(b"k");
    let effective = resolve(&input, Some(&overlay));
    assert_eq!(effective.challenge_at, 40.0);
    assert_eq!(effective.hr_mode("HR-12"), Mode::Monitor);
    input.kill_switch = true;
    assert!(resolve(&input, Some(&overlay)).global_monitor);
}

#[test]
fn file_store_round_trip() {
    let directory = TempDir::new("roundtrip");
    let store = OverlayFileStore::open(directory.path()).unwrap();
    assert!(store.active().is_none());
    let mut overlay = Overlay {
        schema_version: "1.0.0".into(),
        overlay_id: "ovl1".into(),
        status: "active".into(),
        hard_rules: modes(&[("HR-12", Mode::Monitor)]),
        ..Overlay::default()
    };
    store.set_active(Some(&mut overlay)).unwrap();
    let reloaded = OverlayFileStore::open(directory.path()).unwrap();
    let active = reloaded.active().expect("reload");
    assert_eq!(active.overlay_id, "ovl1");
    assert_eq!(active.hard_rules["HR-12"], Mode::Monitor);
    std::fs::write(
        directory.path().join("settings.overlay.v1.json"),
        b"{not-json",
    )
    .unwrap();
    let recovered = OverlayFileStore::open(directory.path()).unwrap();
    assert_eq!(
        recovered.active().map(|overlay| overlay.overlay_id),
        Some("ovl1".into()),
        "expected LKG recovery, load error {:?}",
        recovered.load_error()
    );
}

// admin_settings_test.go: TestConfigureEngineFromEffectiveNetPolicy

#[test]
fn configure_engine_from_effective_net_policy() {
    let overlay = Overlay {
        schema_version: "1.0.0".into(),
        overlay_id: "ovl_n".into(),
        status: "active".into(),
        net_policy: modes(&[("net.proxy.hop", Mode::Monitor)]),
        hard_rules: modes(&[("HR-12", Mode::Monitor)]),
        ..Overlay::default()
    };
    let mut engine = Engine::default();
    configure_engine(&mut engine, &resolve(&boot(b"k"), Some(&overlay)));
    assert_eq!(engine.rule_modes["HR-12"], "monitor");
    assert_eq!(
        engine.net_policy.as_ref().unwrap()["net.proxy.hop"],
        "monitor"
    );
    assert_eq!(engine.policy.version, "1.0.0+overlay");
}

// Port-specific negative boundaries.

#[test]
fn empty_overlay_keeps_frozen_engine_defaults() {
    let mut engine = Engine::default();
    configure_engine(&mut engine, &resolve(&boot(b"k"), None));
    let frozen = Engine::default();
    assert_eq!(engine.policy, frozen.policy);
    assert!(engine.rule_modes.is_empty() && engine.weight_multipliers.is_empty());
    assert!(engine.net_policy.is_none());
}

/// Go's range checks are false for NaN, so the oracle validator accepts a NaN
/// challengeAt or weight multiplier; the port keeps that, and the frozen
/// engine then refuses to score (fail-closed) instead of deciding on NaN.
#[test]
fn nan_scoring_overlay_validates_like_go_but_scoring_fails_closed() {
    use humanymous_core::scoring::engine::ScoringError;
    use humanymous_core::signals::report::SessionReport;
    let challenge = Overlay {
        scoring: Some(ScoringPatch {
            challenge_at: Some(f64::NAN),
            ..ScoringPatch::default()
        }),
        ..Overlay::default()
    };
    let weight = Overlay {
        weight_multipliers: BTreeMap::from([("sig".to_owned(), f64::NAN)]),
        ..Overlay::default()
    };
    for overlay in [challenge, weight] {
        assert_eq!(validate(Some(&overlay)), Ok(()), "{overlay:?}");
        let mut engine = Engine::default();
        configure_engine(&mut engine, &resolve(&boot(b"k"), Some(&overlay)));
        assert_eq!(
            engine.score(&mut SessionReport::default()),
            Err(ScoringError::NonFinitePolicy),
            "{overlay:?}"
        );
    }
}

#[test]
fn store_refuses_empty_directory_and_invalid_overlay_without_writing() {
    assert_eq!(
        OverlayFileStore::open("")
            .err()
            .map(|error| error.to_string()),
        Some("settings: empty store dir".into())
    );
    let directory = TempDir::new("invalid");
    let store = OverlayFileStore::open(directory.path()).unwrap();
    let mut overlay = Overlay {
        gates: modes(&[("gate.verdict_token", Mode::Off)]),
        ..Overlay::default()
    };
    assert!(matches!(
        store.set_active(Some(&mut overlay)),
        Err(OverlayStoreError::Invalid(_))
    ));
    assert_eq!(overlay.status, "");
    assert!(!directory.path().join("settings.overlay.v1.json").exists());
}

#[test]
fn decoder_rejects_malformed_documents_and_depth_beyond_go_limit() {
    for document in [
        &b""[..],
        b"{",
        b"{} x",
        b"[]",
        b"\"x\"",
        b"{\"a\":01}",
        b"{\"a\":\"\\x\"}",
    ] {
        assert!(Overlay::from_json(document).is_err(), "{document:?}");
    }
    let deep_ok = format!("{{\"x\":{}{}}}", "[".repeat(9_999), "]".repeat(9_999));
    assert!(Overlay::from_json(deep_ok.as_bytes()).is_ok());
    let too_deep = format!("{{\"x\":{}{}}}", "[".repeat(10_000), "]".repeat(10_000));
    assert!(Overlay::from_json(too_deep.as_bytes()).is_err());
}
