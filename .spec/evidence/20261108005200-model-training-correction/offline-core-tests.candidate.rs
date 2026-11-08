//! REVIEW CANDIDATE: intended for Core model/training/offline/tests.rs.
use super::*;
use crate::signals::report::BehaviorSummary;

fn samples(first: i64) -> Vec<Sample> {
    (0..4).map(|i| {
        let label = u8::from(i % 2 == 0);
        let mut behavior = BehaviorSummary::default();
        behavior.mouse.mean_curvature = if label == 1 { 0.75 } else { 0.25 };
        super::super::record::Record { label, behavior, ts: Some(first + i),
            cohort: Some(if i == 1 { "a" } else { "b" }.into()),
            source: Some(if label == 1 { "catalog" } else { "pass" }.into()) }
            .sample(&schema_hash(), RecordLimits::default()).unwrap()
    }).collect()
}
fn config() -> OfflineConfig<'static> {
    // Schema is supplied by each call; this sentinel is never used for training.
    OfflineConfig { schema: "", version: "offline", hidden: 1, epochs: 0,
        learning_rate: 0.03125, seed: 2 }
}
fn fit(train: &[Sample], holdout: &[Sample]) -> Result<OfflineReport, OfflineError> {
    let schema = schema_hash();
    fit_offline(OfflineInput { train, holdout, base: None },
        OfflineConfig { schema: &schema, ..config() },
        OfflineLimits { min_holdout_humans_per_cohort: 1, ..OfflineLimits::default() })
}

#[test]
fn train_only_statistics_and_unsigned_data_are_explicit() {
    let train = samples(0); let mut holdout = samples(100);
    // A different holdout distribution must not influence the training mean.
    for s in &mut holdout {
        s.record.behavior.mouse.mean_curvature += 1.0;
        *s = s.record.clone().sample(&schema_hash(), RecordLimits::default()).unwrap();
    }
    let report = fit(&train, &holdout).unwrap();
    assert_eq!(report.model.mean[1].to_bits(), 0.5_f32.to_bits());
    assert_eq!(report.metrics.total, 4);
    assert_eq!(report.cohorts.len(), 2);
    assert!(!report.activation); assert!(!report.signed);
    assert_eq!(report.summary.updates, 0);
    assert_eq!(report.model.cal_a.to_bits(), 1.0_f32.to_bits());
    assert_eq!(report.model.cal_b.to_bits(), 0);
}

#[test]
fn strict_time_missing_metadata_cohort_and_admission_refuse() {
    let train = samples(0); let mut holdout = samples(100);
    holdout[0].record.ts = Some(3);
    assert_eq!(fit(&train, &holdout).unwrap_err(), OfflineError::Time);
    holdout = samples(100); holdout[0].record.ts = None;
    assert_eq!(fit(&train, &holdout).unwrap_err(), OfflineError::Time);
    holdout = samples(100); holdout[0].record.cohort = None;
    assert_eq!(fit(&train, &holdout).unwrap_err(), OfflineError::Cohort);
    holdout = samples(100); holdout[1].record.source = None;
    assert_eq!(fit(&train, &holdout).unwrap_err(), OfflineError::Admission);
    holdout = samples(100); holdout[1].features[1] += 1.0;
    assert_eq!(fit(&train, &holdout).unwrap_err(), OfflineError::FeatureMismatch);
}

#[test]
fn cohort_support_is_not_satisfied_by_aggregate_or_forged_features() {
    let train = samples(0); let mut holdout = samples(100);
    holdout[3].record.cohort = Some("a".into());
    assert_eq!(fit(&train, &holdout).unwrap_err(), OfflineError::Support);
    holdout = samples(100); holdout[0].features[26] = f32::NAN;
    assert_eq!(fit(&train, &holdout).unwrap_err(), OfflineError::FeatureMismatch);
    let schema = schema_hash();
    assert_eq!(fit_offline(OfflineInput { train: &train, holdout: &samples(100), base: None },
        OfflineConfig { schema: &schema, ..config() },
        OfflineLimits { total_samples: 7, ..OfflineLimits::default() }).unwrap_err(), OfflineError::Limits);
}

#[test]
fn warm_candidate_preserves_weights_and_resets_calibration_with_new_train_stats() {
    let train = samples(0); let holdout = samples(100); let schema = schema_hash();
    let mut w1 = vec![vec![0.0; 27]]; w1[0][1] = 8.0;
    let base = MlpBundle { version: "old".into(), schema_hash: schema.clone(),
        feature_dim: 27, hidden: 1, mean: vec![0.0; 27], std: vec![1.0; 27],
        w1, b1: vec![8.0], w2: vec![1.0], b2: -8.0, cal_a: 2.0, cal_b: -0.5 };
    let report = fit_offline(OfflineInput { train: &train, holdout: &holdout, base: Some(&base) },
        OfflineConfig { schema: &schema, ..config() },
        OfflineLimits { min_holdout_humans_per_cohort: 1, ..OfflineLimits::default() }).unwrap();
    assert_eq!(report.model.w1, base.w1); assert_eq!(report.model.b1, base.b1);
    assert_eq!(report.model.w2, base.w2); assert_eq!(report.model.b2, base.b2);
    assert_eq!(report.model.mean[1], 0.5); assert_ne!(report.model.std, base.std);
    assert_eq!(report.model.cal_a, 1.0); assert_eq!(report.model.cal_b.to_bits(), 0);
    assert_eq!(report.summary.rng_draws, 0); assert_eq!(report.summary.updates, 0);
    assert!(!report.activation); assert!(!report.signed);
}

#[test]
fn config_shape_capacity_and_nonfinite_guards_return_data_errors() {
    let train = samples(0); let holdout = samples(100); let schema = schema_hash();
    let limits = OfflineLimits { min_holdout_humans_per_cohort: 1, ..OfflineLimits::default() };
    let run = |c| fit_offline(OfflineInput { train: &train, holdout: &holdout, base: None }, c, limits);
    assert_eq!(run(OfflineConfig { schema: &schema, hidden: 257, ..config() }).unwrap_err(), OfflineError::Limits);
    assert_eq!(run(OfflineConfig { schema: &schema, learning_rate: f32::NAN, ..config() }).unwrap_err(), OfflineError::Config);
    let version = "v".repeat(129);
    assert_eq!(run(OfflineConfig { schema: &schema, version: &version, ..config() }).unwrap_err(), OfflineError::Config);
    assert_eq!(run(config()).unwrap_err(), OfflineError::Schema);
    let malformed = MlpBundle::default();
    assert_eq!(fit_offline(OfflineInput { train: &train, holdout: &holdout, base: Some(&malformed) },
        OfflineConfig { schema: &schema, ..config() }, limits).unwrap_err(), OfflineError::WarmBase);
    let mut nonfinite = samples(100);
    nonfinite[3].record.behavior.mouse.mean_curvature = f64::NAN;
    assert_eq!(fit(&train, &nonfinite).unwrap_err(), OfflineError::Input);
}
