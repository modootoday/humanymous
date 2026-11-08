//! Analytic and Rust-derived checks, not observations of a Go training execution.
use super::*;
use crate::model::inference::{Mlp, Predictor};
use crate::model::training::{
    evaluate::{LabeledPrediction, evaluate},
    standardize::standardize,
};

pub(super) fn config() -> TrainConfig<'static> {
    TrainConfig {
        hidden: 1,
        epochs: 1,
        learning_rate: 0.25,
        seed: 2,
        version: "offline-test",
    }
}

pub(super) fn stats() -> Standardization {
    Standardization {
        mean: [0.0; FEATURE_DIM],
        std: [1.0; FEATURE_DIM],
    }
}

pub(super) fn base() -> MlpBundle {
    MlpBundle {
        version: "base".into(),
        schema_hash: schema_hash(),
        feature_dim: FEATURE_DIM as i64,
        hidden: 1,
        mean: vec![0.0; FEATURE_DIM],
        std: vec![1.0; FEATURE_DIM],
        w1: vec![vec![0.0; FEATURE_DIM]],
        b1: vec![1.0],
        w2: vec![0.5],
        b2: -0.5,
        cal_a: 1.0,
        cal_b: 0.0,
    }
}

fn bits(bundle: &MlpBundle) -> Vec<u32> {
    bundle
        .w1
        .iter()
        .flatten()
        .chain(&bundle.b1)
        .chain(&bundle.w2)
        .chain([&bundle.b2, &bundle.cal_a, &bundle.cal_b])
        .chain(&bundle.mean)
        .chain(&bundle.std)
        .map(|v| v.to_bits())
        .collect()
}

fn run(rows: &[TrainingRow<'_>], base: Option<&MlpBundle>) -> Result<TrainingResult, TrainError> {
    train(
        config(),
        rows,
        &stats(),
        &schema_hash(),
        base,
        TrainLimits::default(),
    )
}

#[test]
fn analytic_active_step_uses_old_output_weight_and_exact_bias_order() {
    let mut x = [0.0; FEATURE_DIM];
    x[0] = 1.0;
    let before = base();
    let snapshot = bits(&before);
    let got = run(
        &[TrainingRow {
            features: &x,
            label: 1,
        }],
        Some(&before),
    )
    .unwrap();
    assert_eq!(got.candidate.w1[0][0].to_bits(), 0.0625_f32.to_bits());
    assert_eq!(got.candidate.b1[0].to_bits(), 1.0625_f32.to_bits());
    assert_eq!(got.candidate.w2[0].to_bits(), 0.625_f32.to_bits());
    assert_eq!(got.candidate.b2.to_bits(), (-0.375_f32).to_bits());
    assert!(got.candidate.w1[0][1..].iter().all(|v| v.to_bits() == 0));
    assert_eq!(bits(&before), snapshot);
    assert_eq!(got.summary.updates, 1);
    assert_eq!(got.summary.rng_draws, 0);
}

#[test]
fn analytic_inactive_and_signed_zero_relu_only_update_output_bias() {
    let mut x = [0.0; FEATURE_DIM];
    x[0] = 1.0;
    for bias in [-1.0, 0.0, -0.0] {
        let mut b = base();
        b.b1[0] = bias;
        b.b2 = 0.0;
        let got = run(
            &[TrainingRow {
                features: &x,
                label: 1,
            }],
            Some(&b),
        )
        .unwrap();
        assert_eq!(got.candidate.b2.to_bits(), 0.125_f32.to_bits());
        assert_eq!(got.candidate.w2, b.w2);
        assert_eq!(got.candidate.w1, b.w1);
        // Subtraction itself may change signed zero; compare the source operation.
        assert_eq!(got.candidate.b1[0].to_bits(), (bias - 0.0_f32).to_bits());
    }
}

#[test]
fn rounded_product_is_not_fused_with_accumulator() {
    // 4097^2 is 16785409, which rounds to 16785408 in f32. A fused
    // multiply-add would leave 1 instead of 0 and activate this hidden unit.
    let mut x = [0.0; FEATURE_DIM];
    x[0] = 4097.0;
    let mut b = base();
    b.w1[0][0] = 4097.0;
    b.b1[0] = -16_785_408.0;
    b.b2 = 0.0;
    let got = run(
        &[TrainingRow {
            features: &x,
            label: 1,
        }],
        Some(&b),
    )
    .unwrap();
    assert_eq!(got.candidate.b2.to_bits(), 0.125_f32.to_bits());
    assert_eq!(got.candidate.w2, b.w2);
    assert_eq!(got.candidate.w1, b.w1);
}

/// Separate gradient formulation: snapshot all output weights, accumulate a
/// gradient matrix, then apply updates. Shared exp and RNG are correlated limits.
fn reference_step(b: &mut MlpBundle, x: &FeatureVector, label: u8, lr: f32) {
    let z: Vec<f32> = (0..FEATURE_DIM)
        .map(|i| (x[i] - b.mean[i]) / b.std[i])
        .collect();
    let old = b.clone();
    let mut pre = vec![0.0; b.hidden as usize];
    let mut logit = old.b2;
    for h in 0..pre.len() {
        let mut value = old.b1[h];
        for (i, input) in z.iter().enumerate() {
            value += old.w1[h][i] * input;
        }
        pre[h] = value;
        logit += old.w2[h] * if value > 0.0 { value } else { 0.0 };
    }
    let d = (1.0_f64 / (1.0 + f64::from(-logit).exp())) as f32 - f32::from(label);
    let mut gw1 = vec![vec![0.0; FEATURE_DIM]; pre.len()];
    let mut gb1 = vec![0.0; pre.len()];
    let mut gw2 = vec![0.0; pre.len()];
    for h in 0..pre.len() {
        gw2[h] = d * if pre[h] > 0.0 { pre[h] } else { 0.0 };
        gb1[h] = if pre[h] > 0.0 { d * old.w2[h] } else { 0.0 };
        for (i, input) in z.iter().enumerate() {
            gw1[h][i] = (lr * gb1[h]) * input;
        }
    }
    for h in 0..pre.len() {
        for i in 0..FEATURE_DIM {
            b.w1[h][i] = old.w1[h][i] - gw1[h][i];
        }
        b.b1[h] = old.b1[h] - lr * gb1[h];
        b.w2[h] = old.w2[h] - lr * gw2[h];
    }
    b.b2 = old.b2 - lr * d;
}

#[test]
fn rust_derived_reference_matches_each_epoch_all_weight_bits_and_stream_counts() {
    let vectors: Vec<_> = (0..5)
        .map(|j| std::array::from_fn::<_, FEATURE_DIM, _>(|i| ((j * 7 + i) as f32 - 10.0) / 32.0))
        .collect();
    let rows: Vec<_> = vectors
        .iter()
        .enumerate()
        .map(|(i, x)| TrainingRow {
            features: x,
            label: (i % 2) as u8,
        })
        .collect();
    for seed in [0, -1, 2_147_483_647, i64::MIN, i64::MAX] {
        for warm in [false, true] {
            let mut c = config();
            c.hidden = 3;
            c.learning_rate = 0.05;
            c.seed = seed;
            let mut r = GoRng::new(seed, 1_000_000).unwrap();
            let initial = r.initialize(3, None).unwrap();
            let initial = MlpBundle {
                hidden: 3,
                w1: initial.w1,
                b1: initial.b1,
                w2: initial.w2,
                b2: initial.b2,
                ..base()
            };
            let mut reference = initial.clone();
            let mut kernel_weights = InitialWeights {
                w1: initial.w1.clone(),
                b1: initial.b1.clone(),
                w2: initial.w2.clone(),
                b2: initial.b2,
            };
            let mut pre = [0.0; 3];
            let mut act = [0.0; 3];
            if warm {
                r = GoRng::new(seed, 1_000_000).unwrap();
            }
            let mut ids: Vec<_> = (0..rows.len()).collect();
            for epochs in 1..=3 {
                r.shuffle(&mut ids).unwrap();
                for &id in &ids {
                    reference_step(
                        &mut reference,
                        &vectors[id],
                        rows[id].label,
                        c.learning_rate,
                    );
                    update(
                        &mut kernel_weights,
                        rows[id],
                        &stats(),
                        c.learning_rate,
                        &mut pre,
                        &mut act,
                    )
                    .unwrap();
                    let kernel_bits: Vec<_> = kernel_weights
                        .w1
                        .iter()
                        .flatten()
                        .chain(&kernel_weights.b1)
                        .chain(&kernel_weights.w2)
                        .chain([&kernel_weights.b2])
                        .map(|v| v.to_bits())
                        .collect();
                    let reference_bits: Vec<_> = reference
                        .w1
                        .iter()
                        .flatten()
                        .chain(&reference.b1)
                        .chain(&reference.w2)
                        .chain([&reference.b2])
                        .map(|v| v.to_bits())
                        .collect();
                    assert_eq!(
                        kernel_bits, reference_bits,
                        "Rust-derived per-sample weights"
                    );
                }
                c.epochs = epochs;
                let got = train(
                    c,
                    &rows,
                    &stats(),
                    &schema_hash(),
                    warm.then_some(&initial),
                    TrainLimits::default(),
                )
                .unwrap();
                assert_eq!(bits(&got.candidate), bits(&reference));
                assert_eq!(got.summary.final_order, ids);
                assert_eq!(got.summary.rng_draws, r.snapshot().draws);
            }
        }
    }
}

#[test]
fn warm_zero_epochs_copies_every_weight_but_new_stats_and_calibration_change_function() {
    let mut b = base();
    b.w1[0][0] = 1.0;
    b.cal_a = 2.0;
    b.cal_b = 3.0;
    let original = bits(&b);
    let mut s = stats();
    s.mean[0] = 2.0;
    let mut c = config();
    c.epochs = 0;
    let got = train(c, &[], &s, &schema_hash(), Some(&b), TrainLimits::default()).unwrap();
    assert_eq!(got.candidate.w1, b.w1);
    assert_eq!(got.candidate.b1, b.b1);
    assert_eq!(got.candidate.w2, b.w2);
    assert_eq!(got.candidate.b2, b.b2);
    assert_eq!(got.candidate.cal_a, 1.0);
    assert_eq!(got.candidate.cal_b.to_bits(), 0);
    assert_eq!(got.summary.rng_draws, 0);
    assert_eq!(bits(&b), original);
    let x = [0.0; FEATURE_DIM];
    assert_ne!(
        Mlp::from_bundle(b).unwrap().predict(&x).p_bot.to_bits(),
        Mlp::from_bundle(got.candidate)
            .unwrap()
            .predict(&x)
            .p_bot
            .to_bits()
    );
}

#[test]
fn defaults_empty_input_and_version_changes_preserve_weight_determinism() {
    let mut c = config();
    c.hidden = -1;
    c.epochs = -9;
    c.learning_rate = -1.0;
    let first = train(
        c,
        &[],
        &stats(),
        &schema_hash(),
        None,
        TrainLimits::default(),
    )
    .unwrap();
    assert_eq!(first.summary.hidden, 16);
    assert_eq!(first.summary.epochs, 0);
    assert_eq!(first.summary.learning_rate, 0.05);
    assert_eq!(first.summary.updates, 0);
    c.version = "different";
    let second = train(
        c,
        &[],
        &stats(),
        &schema_hash(),
        None,
        TrainLimits::default(),
    )
    .unwrap();
    assert_eq!(bits(&first.candidate), bits(&second.candidate));
    assert_eq!(first.summary, second.summary);
    c.epochs = 1000;
    let empty = train(
        c,
        &[],
        &stats(),
        &schema_hash(),
        None,
        TrainLimits::default(),
    )
    .unwrap();
    assert_eq!(first.summary.rng_draws, empty.summary.rng_draws);
}

#[test]
fn limits_schema_labels_statistics_and_warm_shape_fail_before_work() {
    let x = [0.0; FEATURE_DIM];
    let b = base();
    let before = bits(&b);
    let valid = TrainingRow {
        features: &x,
        label: 1,
    };
    let invalid = TrainingRow {
        features: &x,
        label: 2,
    };
    assert_eq!(
        run(&[valid, invalid], Some(&b)).unwrap_err(),
        TrainError::Label
    );
    assert_eq!(
        train(
            config(),
            &[valid],
            &stats(),
            "wrong",
            Some(&b),
            TrainLimits::default()
        )
        .unwrap_err(),
        TrainError::Schema
    );
    for value in [0.0, -1.0, f32::INFINITY, f32::NAN] {
        let mut s = stats();
        s.std[26] = value;
        assert_eq!(
            train(
                config(),
                &[valid],
                &s,
                &schema_hash(),
                Some(&b),
                TrainLimits::default()
            )
            .unwrap_err(),
            TrainError::Statistics
        );
    }
    let mut bad = b.clone();
    bad.w1[0].pop();
    assert_eq!(run(&[valid], Some(&bad)).unwrap_err(), TrainError::WarmBase);
    for field in 0..6 {
        let mut limits = TrainLimits::default();
        match field {
            0 => limits.samples = 0,
            1 => limits.hidden = 257,
            2 => limits.epochs = 1001,
            3 => limits.updates = 1_000_001,
            4 => limits.work_units = 100_000_001,
            _ => limits.rng_draws = 0,
        }
        assert_eq!(
            train(
                config(),
                &[valid],
                &stats(),
                &schema_hash(),
                Some(&b),
                limits
            )
            .unwrap_err(),
            TrainError::Limit
        );
    }
    assert_eq!(bits(&b), before);
    assert_eq!(x, [0.0; FEATURE_DIM]);
}

#[test]
fn exact_work_limits_and_rejection_draw_budget_discard_entire_candidate() {
    let x = [0.0; FEATURE_DIM];
    let row = TrainingRow {
        features: &x,
        label: 1,
    };
    let b = base();
    let mut limits = TrainLimits {
        samples: 1,
        hidden: 1,
        epochs: 1,
        updates: 1,
        work_units: 27,
        rng_draws: 1,
    };
    assert!(train(config(), &[row], &stats(), &schema_hash(), Some(&b), limits).is_ok());
    limits.work_units = 26;
    assert_eq!(
        train(config(), &[row], &stats(), &schema_hash(), Some(&b), limits).unwrap_err(),
        TrainError::Limit
    );
    limits = TrainLimits {
        rng_draws: 1,
        ..TrainLimits::default()
    };
    assert_eq!(
        train(config(), &[row], &stats(), &schema_hash(), None, limits).unwrap_err(),
        TrainError::RngBudget
    );
    assert_eq!(
        train(
            config(),
            &[row; 3],
            &stats(),
            &schema_hash(),
            Some(&b),
            limits
        )
        .unwrap_err(),
        TrainError::RngBudget
    );
    let mut c = config();
    c.epochs = i64::MAX;
    assert_eq!(
        train(c, &[row], &stats(), &schema_hash(), Some(&b), limits).unwrap_err(),
        TrainError::Limit
    );
}

#[test]
fn finite_input_overflows_and_invalid_config_never_mutate_inputs() {
    let mut x = [0.0; FEATURE_DIM];
    x[0] = f32::MAX;
    let row = TrainingRow {
        features: &x,
        label: 1,
    };
    let b = base();
    let before = bits(&b);
    let mut s = stats();
    s.mean[0] = -f32::MAX;
    assert_eq!(
        train(
            config(),
            &[row],
            &s,
            &schema_hash(),
            Some(&b),
            TrainLimits::default()
        )
        .unwrap_err(),
        TrainError::Nonfinite
    );
    let mut overflowing = b.clone();
    overflowing.w1[0][0] = 2.0;
    assert_eq!(
        run(&[row], Some(&overflowing)).unwrap_err(),
        TrainError::Nonfinite
    );
    for lr in [f32::NAN, f32::INFINITY] {
        let mut c = config();
        c.learning_rate = lr;
        assert_eq!(
            train(
                c,
                &[row],
                &stats(),
                &schema_hash(),
                Some(&b),
                TrainLimits::default()
            )
            .unwrap_err(),
            TrainError::Config
        );
    }
    assert_eq!(bits(&b), before);
    assert_eq!(x[0].to_bits(), f32::MAX.to_bits());
    let mut bad = x;
    bad[26] = f32::NAN;
    assert_eq!(
        run(
            &[TrainingRow {
                features: &bad,
                label: 0
            }],
            Some(&b)
        )
        .unwrap_err(),
        TrainError::Nonfinite
    );
    let zeros = [0.0; FEATURE_DIM];
    let mut exploding = b.clone();
    exploding.b1[0] = f32::MAX;
    exploding.w2[0] = 0.0;
    exploding.b2 = 0.0;
    let mut c = config();
    c.learning_rate = 4.0;
    assert_eq!(
        train(
            c,
            &[TrainingRow {
                features: &zeros,
                label: 1
            }],
            &stats(),
            &schema_hash(),
            Some(&exploding),
            TrainLimits::default()
        )
        .unwrap_err(),
        TrainError::Nonfinite
    );
}

#[test]
fn finite_negative_logit_allows_exp_overflow_sigmoid_zero() {
    let x = [0.0; FEATURE_DIM];
    let mut b = base();
    b.b1[0] = -1.0;
    b.b2 = -1000.0;
    let got = run(
        &[TrainingRow {
            features: &x,
            label: 0,
        }],
        Some(&b),
    )
    .unwrap();
    assert_eq!(got.candidate.b2.to_bits(), (-1000.0_f32).to_bits());
    assert_eq!(got.candidate.w2, b.w2);
    let prediction = Mlp::from_bundle(got.candidate).unwrap().predict(&x);
    assert!(!prediction.abstain);
    assert_eq!(prediction.p_bot.to_bits(), 0.0_f32.to_bits());
}

#[test]
fn version_utf8_byte_capacity_accepts_128_and_rejects_129() {
    let accepted = "v".repeat(128);
    let rejected = "v".repeat(129);
    let b = base();
    let before = bits(&b);
    let mut c = config();
    c.epochs = 0;
    c.version = &accepted;
    let got = train(
        c,
        &[],
        &stats(),
        &schema_hash(),
        Some(&b),
        TrainLimits::default(),
    )
    .unwrap();
    assert_eq!(got.candidate.version.len(), 128);
    c.version = &rejected;
    assert_eq!(
        train(
            c,
            &[],
            &stats(),
            &schema_hash(),
            Some(&b),
            TrainLimits::default()
        )
        .unwrap_err(),
        TrainError::Config
    );
    assert_eq!(bits(&b), before);
}

#[test]
fn rust_derived_legacy_separable_dataset_learns_and_reuses_existing_evaluator() {
    // Dataset stream is separate from training's seed2 stream, as in the Go test.
    let mut data_rng = GoRng::new(1, 1_000_000).unwrap();
    let data: Vec<FeatureVector> = (0..1000)
        .map(|i| {
            let mut x = [0.0; FEATURE_DIM];
            for v in &mut x {
                *v = data_rng.norm_float64().unwrap() as f32 * 0.1;
            }
            let noise = data_rng.norm_float64().unwrap() as f32 * 0.2;
            x[0] = if i % 2 == 0 { 3.0 + noise } else { 0.0 + noise };
            x
        })
        .collect();
    let s = standardize(&data[..800], &schema_hash(), 100_000).unwrap();
    let rows: Vec<_> = data[..800]
        .iter()
        .enumerate()
        .map(|(i, x)| TrainingRow {
            features: x,
            label: u8::from(i % 2 == 0),
        })
        .collect();
    let c = TrainConfig {
        hidden: 8,
        epochs: 40,
        learning_rate: 0.1,
        ..config()
    };
    let first = train(c, &rows, &s, &schema_hash(), None, TrainLimits::default()).unwrap();
    let second = train(c, &rows, &s, &schema_hash(), None, TrainLimits::default()).unwrap();
    assert_eq!(bits(&first.candidate), bits(&second.candidate));
    assert_eq!(first.summary, second.summary);
    let model = Mlp::from_bundle(first.candidate).unwrap();
    let pairs: Vec<_> = data[800..]
        .iter()
        .enumerate()
        .map(|(i, x)| LabeledPrediction {
            label: u8::from(i % 2 == 0),
            prediction: model.predict(x),
        })
        .collect();
    let counts = evaluate(&pairs, 100_000).unwrap();
    assert!(counts.accuracy().unwrap() >= 0.95);
    assert!(counts.human_fpr().unwrap() <= 0.05);
    assert!(counts.bot_tpr().unwrap() >= 0.95);
}
