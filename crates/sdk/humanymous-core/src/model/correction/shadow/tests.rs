use super::*;

fn scored(p_bot: f32) -> Prediction {
    Prediction {
        p_bot,
        abstain: false,
    }
}

#[test]
fn shadow_comparator_tracks_directional_divergence() {
    let comparator = ShadowComparator::default();
    for _ in 0..100 {
        comparator.observe(scored(0.4), scored(0.8), 0.5).unwrap();
    }
    let stats = comparator.snapshot();
    assert_eq!((stats.n, stats.both_scored), (100, 100));
    assert_eq!((stats.shadow_fires_more, stats.active_fires_more), (100, 0));
    assert!(
        (0.39..=0.41).contains(&stats.mean_abs_delta),
        "{}",
        stats.mean_abs_delta
    );
    assert_eq!(stats.fire_agreement, 0.0);
}

#[test]
fn shadow_comparator_counts_agreement_and_candidate_abstention() {
    let comparator = ShadowComparator::default();
    for _ in 0..60 {
        comparator.observe(scored(0.9), scored(0.85), 0.5).unwrap();
    }
    for _ in 0..40 {
        comparator
            .observe(scored(0.9), Prediction::abstained(), 0.5)
            .unwrap();
    }
    let stats = comparator.snapshot();
    assert_eq!(stats.both_scored, 60);
    assert_eq!(stats.shadow_abstained, 40);
    assert_eq!(stats.fire_agreement, 1.0);
}

#[test]
fn shadow_comparator_counts_lenient_candidates_and_active_abstention() {
    let comparator = ShadowComparator::default();
    comparator.observe(scored(0.7), scored(0.2), 0.5).unwrap();
    comparator
        .observe(Prediction::abstained(), scored(0.2), 0.5)
        .unwrap();
    comparator
        .observe(Prediction::abstained(), Prediction::abstained(), 0.5)
        .unwrap();
    let stats = comparator.snapshot();
    assert_eq!(stats.n, 3);
    assert_eq!(stats.active_fires_more, 1);
    assert_eq!(stats.active_abstained, 1);
    assert_eq!(stats.both_scored, 1);
}

#[test]
fn shadow_comparator_rejects_invalid_pairs_without_counting_them() {
    let comparator = ShadowComparator::default();
    assert_eq!(
        comparator
            .observe(scored(f32::NAN), scored(0.2), 0.5)
            .unwrap_err(),
        CorrectionError::Nonfinite
    );
    assert_eq!(
        comparator
            .observe(scored(0.2), scored(1.2), 0.5)
            .unwrap_err(),
        CorrectionError::Probability
    );
    assert_eq!(
        comparator
            .observe(scored(0.2), scored(0.2), f32::INFINITY)
            .unwrap_err(),
        CorrectionError::Nonfinite
    );
    let abstained_nan = Prediction {
        p_bot: f32::NAN,
        abstain: true,
    };
    comparator.observe(abstained_nan, scored(0.2), 0.5).unwrap();
    let stats = comparator.snapshot();
    assert_eq!((stats.n, stats.rejected, stats.both_scored), (1, 3, 0));
    let json = serde_json::to_value(stats).unwrap();
    assert_eq!(json["shadowAbstained"], 0);
    assert_eq!(json["bothScored"], 0);
}
