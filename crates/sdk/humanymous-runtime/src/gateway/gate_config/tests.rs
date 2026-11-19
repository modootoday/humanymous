//! Zero-means-default resolution of the gate configuration. Go parity of the
//! default values is pinned in `control_plane::tests::golden`.
use super::*;
use crate::gateway::control_flood::default_limits;
use crate::gateway::route_policy::{PRESET_BALANCED, PRESET_STRICT};
use std::collections::BTreeMap;

#[test]
fn empty_fields_resolve_to_the_go_defaults() {
    let zero = GateConfig::default();
    assert_eq!(zero.control_path(), "/__hmn/");
    assert_eq!(zero.erasure_hold(), Duration::from_secs(300));
    assert_eq!(zero.cred_fanout_cap(), 8);
    assert_eq!(zero.max_body(), None);
    assert!(!zero.hsts && !zero.anomaly_shadow && zero.virtual_usb_results_dir.is_empty());
    assert_eq!(zero.resolve("/anything"), PRESET_BALANCED);
}

#[test]
fn set_fields_are_kept() {
    let config = GateConfig {
        control_path: "/__ctl/".into(),
        erasure_hold: Duration::from_millis(1),
        cred_fanout_cap: 1,
        max_body_bytes: 1 << 20,
        routes: RouteTable {
            routes: BTreeMap::from([("/admin".into(), "strict".into())]),
            global_monitor: false,
        },
        ..GateConfig::default()
    };
    assert_eq!(config.control_path(), "/__ctl/");
    assert_eq!(config.erasure_hold(), Duration::from_millis(1));
    assert_eq!(config.cred_fanout_cap(), 1);
    assert_eq!(config.max_body().map(NonZeroU64::get), Some(1 << 20));
    assert_eq!(config.resolve("//admin/./x"), PRESET_STRICT);
    let monitored = GateConfig {
        routes: RouteTable {
            global_monitor: true,
            ..config.routes.clone()
        },
        ..config
    };
    let policy = monitored.resolve("/admin");
    assert_eq!((policy.name, policy.enforce), ("strict", false));
}

#[test]
fn rate_settings_have_no_defaults() {
    let limits = || default_limits().unwrap();
    let valid = GateConfig {
        rate_window: Duration::from_secs(10),
        rate_soft: 60,
        rate_hard: 120,
        ..GateConfig::default()
    };
    assert_eq!(
        valid.rate(limits()).unwrap().window(),
        Duration::from_secs(10)
    );
    for broken in [
        GateConfig {
            rate_window: Duration::ZERO,
            ..valid.clone()
        },
        GateConfig {
            rate_soft: 0,
            ..valid.clone()
        },
        GateConfig {
            rate_hard: 0,
            ..valid.clone()
        },
        GateConfig {
            rate_soft: 121,
            ..valid.clone()
        },
    ] {
        assert!(broken.rate(limits()).is_err());
    }
}
