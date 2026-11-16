use super::*;
use crate::behavior::features::{FEATURE_DIM, schema_hash};
use crate::model::correction::DEFAULT_WINDOW;
use crate::model::correction::test_support::{feature_vector, go_rng};
use crate::model::inference::{Mlp, MlpBundle, Runtime};

fn scored(p_bot: f32) -> Prediction {
    Prediction {
        p_bot,
        abstain: false,
    }
}

#[test]
fn controller_proposal_starts_at_theta0() {
    let controller = Controller::new(0.01, 0.5, 0.02).unwrap();
    assert_eq!(controller.proposed_fire_threshold(), 0.5);
}

#[test]
fn controller_calibration_raises_proposal_on_over_flagged_humans() {
    let controller = Controller::new(0.01, 0.5, 0.05).unwrap();
    let start = controller.proposed_fire_threshold();
    for _ in 0..300 {
        controller
            .observe_outcome(Outcome::PassSolved, 0.95, false)
            .unwrap();
    }
    assert!(controller.proposed_fire_threshold() > start);
    assert_eq!(controller.snapshot().calibration.humans_seen, 300);
}

#[test]
fn controller_ambiguous_outcomes_do_not_calibrate_or_count_labels() {
    let controller = Controller::new(0.01, 0.5, 0.05).unwrap();
    for _ in 0..100 {
        controller
            .observe_outcome(Outcome::Unknown, 0.99, true)
            .unwrap();
    }
    assert_eq!(controller.proposed_fire_threshold(), 0.5);
    let calibration = controller.snapshot().calibration;
    assert_eq!((calibration.humans_seen, calibration.bots_seen), (0, 0));
}

#[test]
fn controller_drift_fires_on_studd_plus_covariate() {
    let controller = Controller::new(0.01, 0.5, 0.02).unwrap();
    for _ in 0..400 {
        controller
            .observe_outcome(Outcome::CatalogBot, 0.95, true)
            .unwrap();
    }
    for _ in 0..400 {
        controller
            .observe_outcome(Outcome::CatalogBot, 0.02, true)
            .unwrap();
    }
    controller.update_covariate(0.9).unwrap();
    let event = controller.snapshot().drift;
    assert!(event.mimic_alarm);
    assert!(event.covariate_shift);
    assert!(event.fired, "{event:?}");
}

#[test]
fn controller_surfaces_pass_solve_anomaly() {
    let controller = Controller::new(0.01, 0.5, 0.02).unwrap();
    assert!(!controller.snapshot().pass_anomalous);
    for _ in 0..5_000 {
        controller.observe_pass(true);
    }
    assert!(controller.snapshot().pass_anomalous);
}

#[test]
fn controller_shadow_observer_feeds_snapshot() {
    let controller = Controller::new(0.01, 0.5, 0.02).unwrap();
    for _ in 0..50 {
        controller.observe_shadow(scored(0.3), scored(0.9));
    }
    let shadow = controller.snapshot().shadow;
    assert_eq!((shadow.n, shadow.shadow_fires_more), (50, 50));
    controller.observe_shadow(scored(f32::NAN), scored(0.9));
    assert_eq!(controller.snapshot().shadow.rejected, 1);
}

#[test]
fn controller_feature_observer_drives_covariate_arm() {
    let mut rng = go_rng(4);
    let controller = Controller::new(0.01, 0.5, 0.02).unwrap();
    for _ in 0..DEFAULT_WINDOW {
        controller.observe_features(&feature_vector(&mut rng, 0.0));
    }
    for _ in 0..DEFAULT_WINDOW {
        controller.observe_features(&feature_vector(&mut rng, 5.0));
    }
    let event = controller.snapshot().drift;
    assert!(event.covariate_shift, "maxPSI={}", event.max_psi);
    controller.observe_features(&[0.0; 2]);
    assert_eq!(controller.snapshot().rejected_features, 1);
}

#[test]
fn controller_armed_canary_trips_on_sustained_over_flagging() {
    let count = Arc::new(AtomicU64::new(0));
    let captured = count.clone();
    let controller = Controller::new(0.005, 0.5, 0.0).unwrap();
    controller
        .arm_canary(
            CanaryBudget {
                max_human_fp: 0.10,
                probation_humans: 100_000,
                ..CanaryBudget::default()
            },
            Some(Arc::new(move |_: &str| {
                captured.fetch_add(1, Ordering::SeqCst);
            })),
        )
        .unwrap();
    for _ in 0..400 {
        controller
            .observe_outcome(Outcome::PassSolved, 0.9, false)
            .unwrap();
    }
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(
        controller.snapshot().canary.state,
        crate::model::correction::CanaryState::Tripped
    );
}

#[test]
fn controller_rejects_invalid_probability_before_any_monitor_changes() {
    let controller = Controller::new(0.01, 0.5, 0.05).unwrap();
    let before = controller.snapshot();
    for value in [f32::NAN, -0.5, 1.5] {
        assert!(
            controller
                .observe_outcome(Outcome::PassSolved, value, false)
                .is_err()
        );
    }
    assert_eq!(controller.snapshot(), before);
    assert_eq!(
        Controller::new(f32::NAN, 0.5, 0.01).unwrap_err(),
        CorrectionError::Nonfinite
    );
}

#[test]
fn controller_snapshot_does_not_count_drift_events() {
    let controller = Controller::new(0.01, 0.5, 0.02).unwrap();
    controller.update_covariate(0.9).unwrap();
    for _ in 0..400 {
        controller
            .observe_outcome(Outcome::Unknown, 0.0, false)
            .unwrap();
    }
    for _ in 0..400 {
        controller
            .observe_outcome(Outcome::Unknown, 1.0, false)
            .unwrap();
    }
    let counted = controller.snapshot().drift_events;
    assert!(counted > 0);
    for _ in 0..10 {
        assert!(controller.snapshot().drift.fired);
    }
    assert_eq!(controller.snapshot().drift_events, counted);
}

#[test]
fn controller_wires_into_lab_runtime_without_moving_the_served_threshold() {
    let row = vec![0.0_f32; FEATURE_DIM];
    let bundle = MlpBundle {
        version: "lab".into(),
        schema_hash: schema_hash(),
        feature_dim: FEATURE_DIM as i64,
        hidden: 2,
        mean: vec![0.0; FEATURE_DIM],
        std: vec![1.0; FEATURE_DIM],
        w1: vec![row.clone(), row],
        b1: vec![0.0; 2],
        w2: vec![0.0; 2],
        b2: 0.0,
        cal_a: 1.0,
        cal_b: 0.0,
    };
    let model = Arc::new(Mlp::from_bundle(bundle).unwrap());
    let controller = Arc::new(Controller::new(0.01, 0.5, 0.05).unwrap());
    let runtime = Runtime::default();
    runtime.set(Some(model.clone()));
    runtime.set_shadow(Some(model));
    runtime.set_feature_observer(Some(controller.clone()));
    runtime.set_shadow_observer(Some(controller.clone()));
    for _ in 0..300 {
        controller
            .observe_outcome(Outcome::PassSolved, 0.95, false)
            .unwrap();
    }
    assert!(controller.proposed_fire_threshold() > 0.5);
    let served = runtime.score(&[0.0; FEATURE_DIM]);
    assert_eq!(served.p_bot, 0.5);
    assert_eq!(runtime.fire_threshold(), 0.5);
    let snapshot = controller.snapshot();
    assert_eq!(
        (snapshot.shadow.n, snapshot.shadow.fire_agreement),
        (1, 1.0)
    );
    assert!(!controller.features.ready());
}
