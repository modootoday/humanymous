//! Ports of legacy cmd/server/counters_test.go (baseline 981d25c) over the
//! real lab correction controller.
use super::*;
use humanymous_core::model::correction::{CanaryBudget, Outcome};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

fn inputs(controller: Option<&Controller>) -> Inputs<'_> {
    Inputs {
        version: "test",
        uptime_sec: 7,
        controller,
        active_bundle: None,
        oracle_traces: 0,
        fleet_fallbacks: 0,
    }
}

#[test]
fn handle_counters_shows_tripped_canary() {
    // gamma 0 freezes theta so the model keeps over-flagging confirmed humans.
    let controller = Controller::new(0.005, 0.5, 0.0).unwrap();
    let tripped = Arc::new(AtomicBool::new(false));
    let flag = tripped.clone();
    controller
        .arm_canary(
            CanaryBudget {
                max_human_fp: 0.10,
                probation_humans: 100_000,
                ..CanaryBudget::default()
            },
            Some(Arc::new(move |_: &str| flag.store(true, Ordering::SeqCst))),
        )
        .unwrap();
    for _ in 0..400 {
        controller
            .observe_outcome(Outcome::PassSolved, 0.9, false)
            .unwrap();
    }
    assert!(
        tripped.load(Ordering::SeqCst),
        "precondition: sustained over-flagging should trip the canary"
    );
    let body = rollup(&inputs(Some(&controller)));
    assert_eq!(body["ml"]["enabled"], true);
    assert_eq!(body["ml"]["canaryState"], "tripped");
    assert!(body["ml"]["canaryReason"].is_string());
    assert!(body["ml"]["fireThreshold"].is_number());
    assert!(body["fleet"].is_object());
}

#[test]
fn handle_counters_nil_safe_without_model() {
    let body = rollup(&inputs(None));
    assert_eq!(body["ml"]["enabled"], false);
    assert!(body["ml"].get("canaryState").is_none());
    assert_eq!(body["ml"]["oracleTraces"], 0);
    assert_eq!(body["version"], "test");
    assert_eq!(body["uptimeSec"], 7);
    assert_eq!(body["fleet"]["redisFallbackTotal"], 0);
}

#[test]
fn armed_canary_bundle_and_fleet_fallbacks_are_reported() {
    let controller = Controller::new(0.005, 0.5, 0.01).unwrap();
    controller
        .arm_canary(CanaryBudget::default(), None)
        .unwrap();
    let body = rollup(&Inputs {
        active_bundle: Some("bundle-7"),
        oracle_traces: 3,
        fleet_fallbacks: 20,
        ..inputs(Some(&controller))
    });
    assert_eq!(body["ml"]["canaryState"], "armed");
    assert!(body["ml"].get("canaryReason").is_none());
    assert_eq!(body["ml"]["activeBundle"], "bundle-7");
    assert_eq!(body["ml"]["oracleTraces"], 3);
    assert_eq!(body["fleet"]["redisFallbackTotal"], 20);
}
