//! Cross-file fixture consistency, not evidence of an actual browser or network.
#![cfg(not(target_arch = "wasm32"))]

use humanymous_collector::session::Scope;
use serde_json::Value;
use std::{collections::BTreeSet, net::SocketAddr};

#[test]
fn owned_runtime_fixture_and_webdriver_proxy_cannot_drift_independently() {
    let fixture: Value = serde_json::from_str(include_str!("../pilot/fixture.json")).unwrap();
    let driver: Value = serde_json::from_str(include_str!("../pilot/webdriver.json")).unwrap();
    let scope: Scope = serde_json::from_value(fixture["scope"].clone()).unwrap();
    let runner: SocketAddr = fixture["runner_listen"].as_str().unwrap().parse().unwrap();
    let proxy: SocketAddr = fixture["browser_proxy_listen"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(runner.ip().is_loopback() && proxy.ip().is_loopback());
    assert_ne!(runner.port(), 0);
    assert_ne!(proxy.port(), 0);
    assert_ne!(runner, proxy);
    assert_eq!(scope.authority, runner.to_string());
    assert_eq!(fixture["browser_origin"], format!("http://{runner}"));
    assert_eq!(driver["proxy"]["httpProxy"], proxy.to_string());
    assert_eq!(driver["proxy"]["proxyType"], "manual");
    assert_eq!(driver["proxy"]["noProxy"], serde_json::json!([]));

    // Keep this typed configuration narrow: new browser arguments, alternate
    // proxy modes or unrelated preferences need explicit source review.
    let keys = |value: &Value| -> BTreeSet<String> {
        value.as_object().unwrap().keys().cloned().collect()
    };
    assert_eq!(
        keys(&driver),
        BTreeSet::from(["proxy".into(), "moz:firefoxOptions".into()])
    );
    assert_eq!(
        keys(&driver["proxy"]),
        BTreeSet::from(["proxyType".into(), "httpProxy".into(), "noProxy".into()])
    );
    assert_eq!(
        keys(&driver["moz:firefoxOptions"]),
        BTreeSet::from(["prefs".into()])
    );
    let preferences = &driver["moz:firefoxOptions"]["prefs"];
    assert_eq!(
        keys(preferences),
        BTreeSet::from(["network.proxy.allow_hijacking_localhost".into()])
    );
    assert_eq!(preferences["network.proxy.allow_hijacking_localhost"], true);

    let routes = fixture["routes"].as_object().unwrap();
    assert_eq!(routes.len(), 4);
    let paths: BTreeSet<_> = routes
        .values()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(paths.len(), routes.len());
    assert_eq!(routes["session"], "/_humanymous/browser/session");
    assert_eq!(routes["collect"], "/_humanymous/browser/collect");
    assert!(paths.iter().all(|path| path.starts_with('/')
        && !path.starts_with("//")
        && !path.contains(['?', '#', '%', '\\'])));
    assert_eq!(fixture["expected"]["required_collector_status"], 422);
    assert_eq!(fixture["expected"]["required_collector_ack"], false);
    // The complete (hooks opted in) epoch is admitted with a consumed, non-ALLOW
    // decision, so the whole two-epoch pilot still has zero origin effects.
    assert_eq!(fixture["expected"]["complete_required_collector_ack"], true);
    assert_ne!(
        fixture["expected"]["complete_required_collector_status"],
        fixture["expected"]["required_collector_status"]
    );
    assert!(matches!(
        fixture["expected"]["complete_required_collector_verdict"].as_str(),
        Some("DENY" | "CHALLENGE")
    ));
    assert_eq!(fixture["expected"]["origin_effects"], 0);
    assert_eq!(fixture["expected"]["effect_requests"], 2);
    // The T4 ceiling epoch declares only its pacing. Its decision is observed
    // by the lab, so the fixture carries no ceiling verdict, status or effect.
    assert_eq!(
        keys(&fixture["ceiling"]),
        BTreeSet::from(["dwell_ms".into(), "think_ms".into()])
    );
    for pace in ["dwell_ms", "think_ms"] {
        assert!(
            fixture["ceiling"][pace]
                .as_u64()
                .is_some_and(|ms| (1000..=30_000).contains(&ms))
        );
    }
    assert_eq!(fixture["test_selection"]["selected"], 1);
    assert_eq!(fixture["test_selection"]["component_tests_filtered"], 66);
    assert_eq!(
        fixture["test_selection"]["exact_name"],
        "browser::transport::runtime_pilot::actual_runtime_session_report_and_required_capability_fail_closed"
    );
}
