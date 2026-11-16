use super::*;
use crate::model::correction::test_support::{float32, go_rng};

#[test]
fn conformal_threshold_holds_budget_on_uniform_humans() {
    let controller = ConformalThreshold::new(0.05, 0.5, 0.02).unwrap();
    let mut rng = go_rng(7);
    for _ in 0..40_000 {
        controller.observe(true, float32(&mut rng)).unwrap();
    }
    let theta = controller.threshold();
    assert!(
        (0.90..=0.985).contains(&theta),
        "theta did not converge near 1-budget: {theta}"
    );
    let fp = controller.snapshot().human_fp_est;
    assert!((0.02..=0.10).contains(&fp), "human-FP {fp} not near 0.05");
}

#[test]
fn conformal_threshold_rises_when_humans_are_over_flagged() {
    let controller = ConformalThreshold::new(0.01, 0.5, 0.02).unwrap();
    for _ in 0..5_000 {
        controller.observe(true, 0.9).unwrap();
    }
    assert!(controller.threshold() > 0.9);
}

#[test]
fn conformal_threshold_ignores_bot_observations() {
    let controller = ConformalThreshold::new(0.01, 0.7, 0.02).unwrap();
    let before = controller.threshold();
    for _ in 0..10_000 {
        controller.observe(false, 0.99).unwrap();
    }
    assert_eq!(controller.threshold(), before);
    assert_eq!(controller.snapshot().bots_seen, 10_000);
}

#[test]
fn conformal_threshold_floor_holds_when_humans_are_never_flagged() {
    let controller = ConformalThreshold::new(0.05, 0.6, 0.05).unwrap();
    for _ in 0..5_000 {
        controller.observe(true, 0.0).unwrap();
    }
    assert!(controller.threshold() >= 0.5);
}

#[test]
fn conformal_threshold_applies_legacy_defaults_and_rejects_nonfinite_configuration() {
    let snapshot = ConformalThreshold::new(0.0, 1.0, 0.0).unwrap().snapshot();
    assert_eq!(snapshot.budget, 0.005);
    assert_eq!(snapshot.theta, 0.5);
    assert_eq!(snapshot.human_fp_est, 0.005);
    for (budget, theta, gamma) in [
        (f32::NAN, 0.5, 0.01),
        (0.01, f32::INFINITY, 0.01),
        (0.01, 0.5, f32::NEG_INFINITY),
    ] {
        assert_eq!(
            ConformalThreshold::new(budget, theta, gamma).unwrap_err(),
            CorrectionError::Nonfinite
        );
    }
}

#[test]
fn conformal_threshold_rejects_invalid_probabilities_without_state_change() {
    let controller = ConformalThreshold::new(0.01, 0.7, 0.05).unwrap();
    let before = controller.snapshot();
    for (value, error) in [
        (f32::NAN, CorrectionError::Nonfinite),
        (f32::INFINITY, CorrectionError::Nonfinite),
        (-0.01, CorrectionError::Probability),
        (1.01, CorrectionError::Probability),
    ] {
        assert_eq!(controller.observe(true, value).unwrap_err(), error);
        assert_eq!(controller.observe(false, value).unwrap_err(), error);
        assert_eq!(controller.fires(value).unwrap_err(), error);
    }
    assert_eq!(controller.snapshot(), before);
    assert!(controller.fires(0.7).unwrap());
    assert!(!controller.fires(0.69).unwrap());
}
