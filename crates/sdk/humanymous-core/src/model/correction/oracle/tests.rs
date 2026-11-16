use super::*;

#[test]
fn confirm_maps_outcomes_to_labels_and_auto_admission() {
    let cases = [
        (Outcome::PassSolved, Label::Human, true),
        (Outcome::CatalogBot, Label::Bot, true),
        (Outcome::ChallengeFailed, Label::Bot, true),
        (Outcome::ChallengeAbandoned, Label::Bot, true),
        (Outcome::Unknown, Label::Ambiguous, false),
    ];
    for (outcome, label, auto) in cases {
        assert_eq!(confirm(outcome), (label, auto), "{outcome:?}");
    }
}

#[test]
fn pass_guard_caps_the_pass_fraction_of_a_batch() {
    let guard = PassGuard::new(0.4, 0.6).unwrap();
    let admitted = guard.admit_pass_fraction(600).unwrap();
    assert!((380..=400).contains(&admitted), "admitted {admitted}");
    let fraction = admitted as f64 / (admitted + 600) as f64;
    assert!(fraction <= 0.41, "fraction {fraction}");
}

#[test]
fn pass_guard_anomaly_stops_pass_ingestion() {
    let guard = PassGuard::new(0.4, 0.6).unwrap();
    assert!(!guard.solve_rate_anomalous());
    for _ in 0..5_000 {
        guard.observe_pass_attempt(true);
    }
    assert!(
        guard.solve_rate_anomalous(),
        "solve rate {}",
        guard.solve_rate()
    );
    assert_eq!(guard.admit_pass_fraction(600).unwrap(), 0);
}

#[test]
fn pass_guard_defaults_full_fraction_and_rejects_invalid_inputs() {
    let guard = PassGuard::new(0.0, 1.0).unwrap();
    assert_eq!(guard.solve_rate(), 0.6);
    // Legacy float32 order: 600 * 0.4 / (1 - 0.4) rounds to 399.99998, truncated.
    assert_eq!(guard.admit_pass_fraction(600).unwrap(), 399);
    let full = PassGuard::new(1.0, 0.5).unwrap();
    assert_eq!(full.admit_pass_fraction(600).unwrap(), 600);
    assert_eq!(full.admit_pass_fraction(0).unwrap(), 0);
    assert_eq!(
        full.admit_pass_fraction(MAX_BATCH_LABELS + 1).unwrap_err(),
        CorrectionError::Limit
    );
    assert_eq!(
        PassGuard::new(f32::NAN, 0.6).unwrap_err(),
        CorrectionError::Nonfinite
    );
    assert_eq!(
        PassGuard::new(0.4, f32::INFINITY).unwrap_err(),
        CorrectionError::Nonfinite
    );
}
