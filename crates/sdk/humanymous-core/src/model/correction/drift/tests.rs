use super::*;

#[test]
fn page_hinkley_detects_a_sustained_rise_without_false_alarms() {
    let mut detector = PageHinkley::new(0.01, 3.0).unwrap();
    for index in 0..500 {
        assert!(
            !detector.update(0.02).unwrap(),
            "false alarm on stable stream at {index}"
        );
    }
    let mut fired = false;
    for _ in 0..500 {
        if detector.update(0.5).unwrap() {
            fired = true;
            break;
        }
    }
    assert!(fired, "Page-Hinkley failed to detect a sustained rise");
}

#[test]
fn psi_is_zero_for_identical_and_large_for_shifted_distributions() {
    let reference = [0.25, 0.25, 0.25, 0.25];
    assert!(psi(&reference, &reference).unwrap() <= 1e-9);
    let shifted = [0.7, 0.2, 0.07, 0.03];
    assert!(psi(&reference, &shifted).unwrap() >= 0.25);
}

#[test]
fn drift_gate_requires_two_of_three_agreeing_arms() {
    let monitor = DriftMonitor::new();
    monitor.update_psi(0.9).unwrap();
    assert!(!monitor.gate().fired, "covariate alone must not fire");
    for _ in 0..1_000 {
        monitor.observe_mimic(0.02).unwrap();
    }
    for _ in 0..1_000 {
        monitor.observe_mimic(0.8).unwrap();
    }
    let event = monitor.gate();
    assert!(event.mimic_alarm);
    assert!(event.fired, "mimic + covariate must fire: {event:?}");
}

#[test]
fn page_hinkley_defaults_reset_and_rejects_nonfinite() {
    let mut detector = PageHinkley::new(0.0, -1.0).unwrap();
    assert_eq!((detector.delta(), detector.lambda()), (0.005, 5.0));
    assert_eq!(
        PageHinkley::new(f64::NAN, 1.0).unwrap_err(),
        CorrectionError::Nonfinite
    );
    assert_eq!(
        PageHinkley::new(0.1, f64::INFINITY).unwrap_err(),
        CorrectionError::Nonfinite
    );
    for _ in 0..100 {
        detector.update(1.0).unwrap();
    }
    for _ in 0..100 {
        detector.update(0.0).unwrap();
    }
    for _ in 0..100 {
        detector.update(1.0).unwrap();
    }
    let before = detector.clone();
    assert_eq!(
        detector.update(f64::NAN).unwrap_err(),
        CorrectionError::Nonfinite
    );
    assert_eq!(detector, before);
    detector.reset();
    assert_eq!(detector, PageHinkley::new(0.005, 5.0).unwrap());
}

#[test]
fn psi_rejects_mismatched_empty_and_invalid_fractions() {
    assert_eq!(psi(&[], &[]).unwrap_err(), CorrectionError::Shape);
    assert_eq!(
        psi(&[1.0], &[0.5, 0.5]).unwrap_err(),
        CorrectionError::Shape
    );
    assert_eq!(
        psi(&[f64::NAN], &[1.0]).unwrap_err(),
        CorrectionError::Nonfinite
    );
    assert_eq!(
        psi(&[1.0], &[1.5]).unwrap_err(),
        CorrectionError::Probability
    );
    assert!(psi(&[1.0, 0.0], &[0.0, 1.0]).unwrap().is_finite());
}

#[test]
fn drift_monitor_rejects_invalid_inputs_and_peek_does_not_count_events() {
    let monitor = DriftMonitor::new();
    assert_eq!(
        monitor.observe_mimic(1.5).unwrap_err(),
        CorrectionError::Probability
    );
    assert_eq!(
        monitor.observe_oracle_error(f64::NAN).unwrap_err(),
        CorrectionError::Nonfinite
    );
    assert_eq!(
        monitor.update_psi(-0.1).unwrap_err(),
        CorrectionError::Probability
    );
    assert_eq!(
        monitor.update_psi(f64::INFINITY).unwrap_err(),
        CorrectionError::Nonfinite
    );
    monitor.update_psi(0.9).unwrap();
    for _ in 0..1_000 {
        monitor.observe_oracle_error(0.0).unwrap();
    }
    for _ in 0..1_000 {
        monitor.observe_oracle_error(1.0).unwrap();
    }
    for _ in 0..5 {
        assert!(monitor.peek().fired);
    }
    assert_eq!(monitor.events(), 0);
    assert!(monitor.gate().oracle_alarm);
    assert_eq!(monitor.events(), 1);
}
