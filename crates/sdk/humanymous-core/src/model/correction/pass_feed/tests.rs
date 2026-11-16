//! Ports of legacy cmd/server/mlcorrect_ops_test.go (baseline 981d25c).
use super::*;
use crate::signals::report::{KeyFeatures, MouseFeatures};

fn scored(p_bot: f32) -> Prediction {
    Prediction {
        p_bot,
        abstain: false,
    }
}

fn behavior(samples: i64, keystrokes: i64) -> BehaviorSummary {
    BehaviorSummary {
        mouse: MouseFeatures {
            samples,
            ..MouseFeatures::default()
        },
        key: KeyFeatures {
            keystrokes,
            ..KeyFeatures::default()
        },
        ..BehaviorSummary::default()
    }
}

#[test]
fn solved_pass_raises_threshold_on_over_flagged_humans() {
    let controller = Controller::new(0.01, 0.5, 0.05).unwrap();
    let start = controller.proposed_fire_threshold();
    let b = BehaviorSummary::default();
    for _ in 0..300 {
        feed_pass_outcome(Some(&controller), &b, 0, true, scored(0.9), false).unwrap();
    }
    assert!(controller.proposed_fire_threshold() > start);
    assert_eq!(controller.snapshot().calibration.humans_seen, 300);
}

#[test]
fn failed_pass_is_not_a_bot_label() {
    let controller = Controller::new(0.01, 0.5, 0.05).unwrap();
    let b = BehaviorSummary::default();
    for _ in 0..100 {
        let fed = feed_pass_outcome(Some(&controller), &b, 0, false, scored(0.9), true).unwrap();
        assert_eq!(
            fed,
            Fed {
                trace: None,
                drift: None
            }
        );
    }
    let stats = controller.snapshot().calibration;
    assert_eq!((stats.humans_seen, stats.bots_seen), (0, 0));
}

#[test]
fn absent_controller_is_a_noop_except_for_the_trace() {
    let b = behavior(40, 0);
    let solved = feed_pass_outcome(None, &b, 7, true, scored(0.5), false).unwrap();
    assert!(solved.trace.is_some() && solved.drift.is_none());
    let failed = feed_pass_outcome(None, &b, 7, false, scored(0.5), false).unwrap();
    assert_eq!(
        failed,
        Fed {
            trace: None,
            drift: None
        }
    );
}

#[test]
fn solved_pass_yields_one_labeled_cohort_tagged_human_trace() {
    let mut b = behavior(40, 0);
    b.duration_s = 7.0;
    let fed = feed_pass_outcome(None, &b, 4242, true, scored(0.1), false).unwrap();
    let trace = fed.trace.unwrap();
    assert_eq!(
        (trace.label, trace.source, trace.ts, trace.cohort),
        (0, "pass", 4242, Cohort::Pointer)
    );
    assert_eq!(trace.behavior, b);
}

#[test]
fn abstaining_model_does_not_calibrate() {
    let controller = Controller::new(0.01, 0.5, 0.05).unwrap();
    let fed = feed_pass_outcome(
        Some(&controller),
        &BehaviorSummary::default(),
        0,
        true,
        Prediction::abstained(),
        false,
    )
    .unwrap();
    assert!(fed.trace.is_some() && fed.drift.is_none());
    assert_eq!(controller.snapshot().calibration.humans_seen, 0);
}

#[test]
fn invalid_prediction_fails_closed_without_calibrating() {
    let controller = Controller::new(0.01, 0.5, 0.05).unwrap();
    assert!(
        feed_pass_outcome(
            Some(&controller),
            &BehaviorSummary::default(),
            0,
            true,
            scored(f32::NAN),
            false
        )
        .is_err()
    );
    assert_eq!(controller.snapshot().calibration.humans_seen, 0);
}

#[test]
fn cohort_follows_input_modality() {
    assert_eq!(cohort_of(&behavior(30, 0)), Cohort::Pointer);
    assert_eq!(cohort_of(&behavior(0, 12)), Cohort::Keyboard);
    assert_eq!(cohort_of(&behavior(2, 5)), Cohort::Keyboard);
    assert_eq!(cohort_of(&BehaviorSummary::default()), Cohort::Default);
    assert_eq!(Cohort::Keyboard.as_str(), "keyboard");
}
