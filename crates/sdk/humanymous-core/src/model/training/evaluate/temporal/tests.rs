//! Ports of legacy `internal/mltrain/train_test.go` split, cohort and AUT tests.
use super::*;
use crate::behavior::features::schema_hash;
use crate::model::inference::Prediction;
use crate::model::training::record::{Limits, Record};
use crate::signals::report::BehaviorSummary;

fn sample(ts: Option<i64>) -> Sample {
    Record {
        label: 0,
        behavior: BehaviorSummary::default(),
        ts,
        cohort: None,
        source: Some("pass".into()),
    }
    .sample(&schema_hash(), Limits::default())
    .unwrap()
}

fn pair(label: u8, p_bot: f32) -> LabeledPrediction {
    LabeledPrediction {
        label,
        prediction: Prediction {
            p_bot,
            abstain: false,
        },
    }
}

/// TestTesseractSplit_TrainPastTestFuture: scrambled input still yields a
/// strictly older training window and a newer test window.
#[test]
fn split_trains_on_the_past_and_tests_on_the_future() {
    let samples: Vec<Sample> = (0..100).map(|i| sample(Some((i * 37) % 100))).collect();
    let (train, test) = time_split(&samples, 0.2).unwrap();
    assert_eq!((train.len(), test.len()), (80, 20));
    let newest_train = train.iter().filter_map(|s| s.record.ts).max().unwrap();
    let oldest_test = test.iter().filter_map(|s| s.record.ts).min().unwrap();
    assert!(newest_train < oldest_test);
    // Go's cut arithmetic: n - trunc(n * fraction), at least one.
    let (train, test) = time_split(&samples[..3], 0.99).unwrap();
    assert_eq!((train.len(), test.len()), (1, 2));
}

#[test]
fn split_ties_keep_input_order() {
    let mut samples: Vec<Sample> = (0..4).map(|_| sample(Some(7))).collect();
    for (index, sample) in samples.iter_mut().enumerate() {
        sample.record.cohort = Some(index.to_string());
    }
    let (train, test) = time_split(&samples, 0.5).unwrap();
    let order: Vec<_> = train
        .iter()
        .chain(&test)
        .map(|s| s.record.cohort.clone().unwrap())
        .collect();
    assert_eq!(order, ["0", "1", "2", "3"]);
}

#[test]
fn split_refuses_what_go_left_to_its_caller() {
    let samples: Vec<Sample> = (0..10).map(|i| sample(Some(i))).collect();
    for fraction in [0.0, 1.0, -0.5, f64::NAN] {
        assert_eq!(
            time_split(&samples, fraction).unwrap_err(),
            SplitError::Fraction
        );
    }
    assert_eq!(
        time_split(&samples[..1], 0.5).unwrap_err(),
        SplitError::TooFew
    );
    assert_eq!(
        time_split(&samples, 0.05).unwrap_err(),
        SplitError::EmptyWindow
    );
    let mut missing = samples.clone();
    missing[4].record.ts = None;
    assert_eq!(
        time_split(&missing, 0.5).unwrap_err(),
        SplitError::MissingTimestamp
    );
}

/// TestCohortFPR_IsPerCohort: a harder cohort shows a higher rate, and the
/// empty or absent cohort is `default`.
#[test]
fn cohort_fpr_is_per_cohort() {
    let row = |cohort, label, p_bot| CohortPrediction {
        cohort,
        pair: pair(label, p_bot),
    };
    let rows = [
        row(Some("typical"), 0, 0.1),
        row(Some("typical"), 0, 0.1),
        row(Some("typical"), 0, 0.1),
        row(Some("typical"), 0, 0.5),
        row(Some("edge"), 0, 0.9),
        row(Some("edge"), 0, 0.2),
        row(Some(""), 0, 0.1),
        row(None, 0, 0.7),
        row(Some("typical"), 1, 0.9),
    ];
    let stats = cohort_fpr(&rows, rows.len()).unwrap();
    assert_eq!(
        stats.keys().collect::<Vec<_>>(),
        ["default", "edge", "typical"]
    );
    assert_eq!(
        stats["typical"],
        CohortStat {
            humans: 4,
            human_fp: 1
        }
    );
    assert_eq!(stats["edge"].fpr(), Some(0.5));
    assert_eq!(stats["default"].fpr(), Some(0.5));
    assert!(stats["edge"].fpr() > stats["typical"].fpr());
    assert_eq!(CohortStat::default().fpr(), None);
    let invalid = [row(Some("typical"), 0, 0.1), row(None, 1, f32::NAN)];
    assert_eq!(
        cohort_fpr(&invalid, 2),
        Err(EvaluationError::UnusablePrediction)
    );
}

/// TestAUT_BoundedAndFull: bounded mean of time-bucket accuracies; empty
/// input is unavailable rather than Go's zero.
#[test]
fn aut_averages_time_ordered_bucket_accuracy() {
    let timed = |ts, label, p_bot| TimedPrediction {
        ts,
        pair: pair(label, p_bot),
    };
    // Scrambled timestamps: the five oldest are all correct, the five newest
    // have three correct, so two buckets give (1.0 + 0.6) / 2.
    let rows = [
        timed(9, 0, 0.9),
        timed(0, 0, 0.1),
        timed(8, 1, 0.1),
        timed(1, 1, 0.9),
        timed(7, 0, 0.1),
        timed(2, 0, 0.1),
        timed(6, 1, 0.9),
        timed(3, 1, 0.9),
        timed(5, 0, 0.1),
        timed(4, 0, 0.1),
    ];
    assert_eq!(aut(&rows, 2, 10).unwrap(), Some(0.8));
    // Zero buckets is one bucket: overall accuracy.
    assert_eq!(aut(&rows, 0, 10).unwrap(), Some(0.8));
    // More buckets than rows skips empty slices.
    assert_eq!(aut(&rows[..3], 5, 3).unwrap(), Some(1.0 / 3.0));
    assert_eq!(aut(&[], 5, 1).unwrap(), None);
    assert_eq!(
        aut(&[timed(0, 2, 0.1)], 1, 1),
        Err(EvaluationError::InvalidLabel)
    );
}
