use super::*;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

fn counter() -> (Arc<AtomicU32>, RollbackCallback) {
    let count = Arc::new(AtomicU32::new(0));
    let captured = count.clone();
    let callback: RollbackCallback = Arc::new(move |_| {
        captured.fetch_add(1, Ordering::SeqCst);
    });
    (count, callback)
}

#[test]
fn canary_trips_once_on_human_fp_breach_after_min_observe() {
    let count = Arc::new(AtomicU32::new(0));
    let reason = Arc::new(Mutex::new(String::new()));
    let (captured_count, captured_reason) = (count.clone(), reason.clone());
    let guard = CanaryGuard::new(
        CanaryBudget {
            max_human_fp: 0.05,
            probation_humans: 1_000,
            ..CanaryBudget::default()
        },
        Some(Arc::new(move |text: &str| {
            captured_count.fetch_add(1, Ordering::SeqCst);
            *captured_reason.lock().unwrap() = text.to_owned();
        })),
    )
    .unwrap();
    guard.arm(0);
    guard
        .evaluate(MIN_CANARY_OBSERVE - 1, 0.9, false, false)
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 0);
    guard
        .evaluate(MIN_CANARY_OBSERVE + 5, 0.9, false, false)
        .unwrap();
    guard
        .evaluate(MIN_CANARY_OBSERVE + 6, 0.9, false, false)
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    let stats = guard.snapshot(MIN_CANARY_OBSERVE + 6);
    assert_eq!(stats.state, CanaryState::Tripped);
    assert_eq!(stats.state.as_str(), "tripped");
    assert!(!stats.reason.is_empty());
    assert_eq!(
        *reason.lock().unwrap(),
        "human-FP 0.9000 over canary budget 0.0500"
    );
}

#[test]
fn canary_graduates_after_breach_free_probation_and_then_is_inert() {
    let (count, callback) = counter();
    let guard = CanaryGuard::new(
        CanaryBudget {
            max_human_fp: 0.05,
            probation_humans: 100,
            ..CanaryBudget::default()
        },
        Some(callback),
    )
    .unwrap();
    guard.arm(0);
    for humans in 1..=100 {
        guard.evaluate(humans, 0.0, false, false).unwrap();
    }
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(guard.snapshot(100).state, CanaryState::Graduated);
    guard.evaluate(200, 0.9, true, true).unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[test]
fn canary_trips_on_drift_and_on_pass_poisoning() {
    let (drift_count, drift_callback) = counter();
    let drift = CanaryGuard::new(
        CanaryBudget {
            max_human_fp: 0.9,
            probation_humans: 1_000,
            rollback_on_drift: true,
            ..CanaryBudget::default()
        },
        Some(drift_callback),
    )
    .unwrap();
    drift.arm(0);
    drift.evaluate(5, 0.0, true, false).unwrap();
    assert_eq!(drift_count.load(Ordering::SeqCst), 1);

    let (pass_count, pass_callback) = counter();
    let pass = CanaryGuard::new(
        CanaryBudget {
            max_human_fp: 0.9,
            probation_humans: 1_000,
            rollback_on_pass_anomaly: true,
            ..CanaryBudget::default()
        },
        Some(pass_callback),
    )
    .unwrap();
    pass.arm(0);
    pass.evaluate(5, 0.0, false, true).unwrap();
    assert_eq!(pass_count.load(Ordering::SeqCst), 1);
}

#[test]
fn canary_callback_runs_outside_the_lock_and_may_reenter() {
    let slot: Arc<OnceLock<Arc<CanaryGuard>>> = Arc::new(OnceLock::new());
    let observed = Arc::new(Mutex::new(None));
    let (captured_slot, captured_observed) = (slot.clone(), observed.clone());
    let guard = Arc::new(
        CanaryGuard::new(
            CanaryBudget {
                rollback_on_drift: true,
                ..CanaryBudget::default()
            },
            Some(Arc::new(move |_: &str| {
                let guard = captured_slot.get().expect("guard installed");
                *captured_observed.lock().unwrap() = Some(guard.snapshot(3).state);
                guard.evaluate(4, 0.0, true, false).unwrap();
            })),
        )
        .unwrap(),
    );
    slot.set(guard.clone()).unwrap();
    guard.arm(0);
    guard.evaluate(3, 0.0, true, false).unwrap();
    assert_eq!(*observed.lock().unwrap(), Some(CanaryState::Tripped));
}

#[test]
fn canary_defaults_off_snapshot_and_rejects_invalid_inputs() {
    let guard = CanaryGuard::new(CanaryBudget::default(), None).unwrap();
    let stats = guard.snapshot(10);
    assert_eq!(stats.state, CanaryState::Off);
    assert_eq!(
        (stats.humans_into_probation, stats.probation_humans),
        (0, 200)
    );
    let json = serde_json::to_value(&stats).unwrap();
    assert_eq!(json["state"], "off");
    assert!(json.get("reason").is_none());
    guard.evaluate(10, 0.9, true, true).unwrap();
    assert_eq!(guard.snapshot(10).state, CanaryState::Off);
    assert_eq!(
        CanaryGuard::new(
            CanaryBudget {
                max_human_fp: f32::NAN,
                ..CanaryBudget::default()
            },
            None
        )
        .unwrap_err(),
        CorrectionError::Nonfinite
    );
    guard.arm(50);
    assert_eq!(
        guard.evaluate(49, 0.0, false, false).unwrap_err(),
        CorrectionError::CountRegressed
    );
    assert_eq!(
        guard.evaluate(60, f32::NAN, false, false).unwrap_err(),
        CorrectionError::Nonfinite
    );
    assert_eq!(
        guard.evaluate(60, 1.5, false, false).unwrap_err(),
        CorrectionError::Probability
    );
    assert_eq!(guard.snapshot(60).state, CanaryState::Armed);
    assert_eq!(guard.snapshot(60).humans_into_probation, 10);
}
