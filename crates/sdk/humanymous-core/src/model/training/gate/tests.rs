//! Ports of legacy `cmd/mlcorrect-train/gate_test.go`, plus the fail-closed
//! deltas documented on [`run_gate`].
use super::*;
use crate::behavior::features::FEATURE_DIM;
use crate::model::training::evaluate::SplitError;
use crate::model::training::record::{Admission, Record};
use crate::signals::report::BehaviorSummary;

/// Legacy `mkSample`: feature 0 is the discriminant (bots high, humans low);
/// the rest is deterministic jitter so standardization has nonzero variance.
fn sample(label: u8, cohort: &str, f0: f32, i: usize) -> Sample {
    let mut features = [0.0_f32; FEATURE_DIM];
    features[0] = f0 + (i % 3) as f32 * 0.02;
    for (j, value) in features.iter_mut().enumerate().skip(1) {
        *value = ((i + j) % 5) as f32 * 0.01;
    }
    Sample {
        features,
        record: Record {
            label,
            behavior: BehaviorSummary::default(),
            ts: Some(i as i64),
            cohort: (!cohort.is_empty()).then(|| cohort.to_owned()),
            source: None,
        },
        admission: Admission::ReviewRequired,
    }
}

/// Legacy `mkSamples`: 3 catalog bots : 2 typical humans : 1 edge human per
/// 6 records, so every time window carries all three. `poison_edge` makes
/// the accessibility-minority humans bot-like.
fn samples(poison_edge: bool) -> Vec<Sample> {
    (0..1200)
        .map(|i| match i % 6 {
            0 | 2 | 4 => sample(1, "", 3.0, i),
            1 | 5 => sample(0, "typical", 0.0, i),
            _ => sample(0, "edge", if poison_edge { 3.0 } else { 0.0 }, i),
        })
        .collect()
}

/// Legacy `testGateConfig`.
fn config() -> GateConfig<'static> {
    GateConfig {
        version: "test",
        hidden: 8,
        epochs: 40,
        learning_rate: 0.1,
        seed: 2,
        test_fraction: 0.25,
        max_human_fpr: 0.005,
        min_bot_tpr: 0.80,
        min_cohort_humans: 20,
        required_cohorts: &[],
    }
}

/// TestRunGate_CleanDataEmitsCandidate.
#[test]
fn clean_data_emits_candidate() {
    let report = run_gate(&samples(false), None, &config()).unwrap();
    assert!(
        report.candidate.is_some() && report.failures.is_empty(),
        "clean data must pass every gate and emit a candidate; failures={:?}",
        report.failures
    );
    assert!(report.cohorts["edge"].humans >= 20 && report.cohorts["typical"].humans >= 20);
    assert!(report.bot_tpr.unwrap() >= 0.80);
    assert_eq!((report.n_train, report.n_test), (900, 300));
    let candidate = report.candidate.unwrap();
    assert_eq!(candidate.version, "test");
    assert_eq!(candidate.hidden, 8);
    assert!(candidate.validate().is_ok());
    assert!(report.aut.is_some_and(|value| value > 0.9));
}

/// TestRunGate_PoisonedCohortBlocksCandidate_FailClosed.
#[test]
fn poisoned_cohort_blocks_candidate_fail_closed() {
    let report = run_gate(&samples(true), None, &config()).unwrap();
    assert!(
        report.candidate.is_none(),
        "a poisoned accessibility cohort must block the candidate (aggregate FPR was {:?})",
        report.aggregate_human_fpr
    );
    let joined = report.failures.join(" | ");
    assert!(
        joined.contains(r#"cohort "edge""#),
        "the failure must name the regressed edge cohort, got: {joined}"
    );
    assert!(
        report.cohorts["edge"].fpr().unwrap() > report.cohorts["typical"].fpr().unwrap(),
        "the edge cohort must show the higher FPR: {:?}",
        report.cohorts
    );
}

/// TestRunGate_TooFewSamplesIsAPreconditionError.
#[test]
fn too_few_samples_is_a_precondition_error() {
    assert_eq!(
        run_gate(&samples(false)[..50], None, &config()).unwrap_err(),
        GateError::TooFewSamples(50)
    );
    assert_eq!(
        GateError::TooFewSamples(50).to_string(),
        "need >=100 samples, got 50"
    );
}

#[test]
fn window_without_humans_or_bots_fails_closed() {
    // All-bot data: legacy iterated only over observed human cohorts and passed.
    let bots: Vec<_> = (0..200).map(|i| sample(1, "", 3.0, i)).collect();
    let report = run_gate(&bots, None, &config()).unwrap();
    assert!(report.candidate.is_none() && report.cohorts.is_empty());
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.starts_with("no human support"))
    );

    // All-human data with a zero floor: legacy reported bot TPR 0 and passed.
    let humans: Vec<_> = (0..200).map(|i| sample(0, "typical", 0.0, i)).collect();
    let lenient = GateConfig {
        min_bot_tpr: 0.0,
        ..config()
    };
    let report = run_gate(&humans, None, &lenient).unwrap();
    assert!(report.candidate.is_none() && report.bot_tpr.is_none());
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.starts_with("no bot support"))
    );
}

#[test]
fn undersampled_and_missing_required_cohorts_block_the_candidate() {
    let required = ["edge".to_owned(), "screen-reader".to_owned()];
    let strict = GateConfig {
        min_cohort_humans: 60,
        required_cohorts: &required,
        ..config()
    };
    let report = run_gate(&samples(false), None, &strict).unwrap();
    assert!(report.candidate.is_none());
    assert_eq!(
        report.failures,
        [
            r#"required cohort "screen-reader" is absent from the test window: cannot confirm human-FP≈0"#,
            r#"cohort "edge" has only 50 humans (<60): cannot confirm human-FP≈0"#,
        ]
    );
}

#[test]
fn catalog_regression_below_the_bot_floor_blocks_the_candidate() {
    // Bots in the newest (test) window evade: they look like humans.
    let drifted: Vec<_> = samples(false)
        .into_iter()
        .map(|s| {
            let i = s.record.ts.unwrap() as usize;
            if s.record.label == 1 && i >= 900 {
                sample(1, "", 0.0, i)
            } else {
                s
            }
        })
        .collect();
    let report = run_gate(&drifted, None, &config()).unwrap();
    let tpr = report.bot_tpr.unwrap();
    assert!(tpr < 0.80, "evading bots must regress TPR, got {tpr}");
    assert!(report.candidate.is_none());
    assert!(
        report.failures.contains(&format!(
            "bot TPR {tpr:.3} < floor 0.800 (catalog regression)"
        )),
        "{:?}",
        report.failures
    );
}

#[test]
fn invalid_configuration_and_inputs_are_typed_preconditions() {
    for broken in [
        GateConfig {
            max_human_fpr: f64::NAN,
            ..config()
        },
        GateConfig {
            min_bot_tpr: 1.5,
            ..config()
        },
        GateConfig {
            learning_rate: f64::INFINITY,
            ..config()
        },
    ] {
        assert_eq!(
            run_gate(&samples(false), None, &broken).unwrap_err(),
            GateError::Config
        );
    }
    let fraction = GateConfig {
        test_fraction: 0.0,
        ..config()
    };
    assert_eq!(
        run_gate(&samples(false), None, &fraction).unwrap_err(),
        GateError::Split(SplitError::Fraction)
    );
    let mut untimed = samples(false);
    untimed[7].record.ts = None;
    assert_eq!(
        run_gate(&untimed, None, &config()).unwrap_err(),
        GateError::Split(SplitError::MissingTimestamp)
    );
}

#[test]
fn warm_start_requires_a_matching_base_and_keeps_shape() {
    let base = run_gate(&samples(false), None, &config())
        .unwrap()
        .candidate
        .unwrap();
    let warm = run_gate(&samples(false), Some(&base), &config()).unwrap();
    let candidate = warm.candidate.expect("warm start on clean data passes");
    assert_eq!(candidate.hidden, base.hidden);
    let wider = GateConfig {
        hidden: 16,
        ..config()
    };
    assert_eq!(
        run_gate(&samples(false), Some(&base), &wider).unwrap_err(),
        GateError::Training("WarmBase".to_owned())
    );
}

#[test]
fn gate_is_deterministic_for_a_seed() {
    let first = run_gate(&samples(false), None, &config()).unwrap();
    let second = run_gate(&samples(false), None, &config()).unwrap();
    assert_eq!(
        serde_json::to_vec(&first.candidate).unwrap(),
        serde_json::to_vec(&second.candidate).unwrap()
    );
}
