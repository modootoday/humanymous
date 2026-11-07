use super::*;

fn ticks(value: u64) -> Duration {
    Duration::from_secs(value)
}
fn configured(window: usize, warmup: usize, floor: f64, keys: usize) -> Detector {
    Detector::new(
        Config::new(
            window,
            6.0,
            warmup,
            floor,
            Limits::new(keys, window, 64, 1_000_000).unwrap(),
        )
        .unwrap(),
    )
}
fn snapshot(detector: &Detector) -> Vec<(String, Vec<f64>, Duration)> {
    detector
        .series
        .iter()
        .map(|(key, series)| (key.clone(), series.values.clone(), series.last))
        .collect()
}

#[test]
fn oracle_stable_stream_never_flags() {
    let mut detector = configured(32, 8, 0.0, 1);
    let values = [
        100.0, 102.0, 98.0, 101.0, 99.0, 103.0, 97.0, 100.0, 101.0, 99.0, 100.0, 102.0, 98.0,
        101.0, 99.0, 100.0,
    ];
    for (i, value) in values.into_iter().enumerate() {
        assert!(
            !detector
                .observe("k", value, ticks(i as u64))
                .unwrap()
                .anomalous
        );
    }
}

#[test]
fn oracle_outlier_flags_against_prior_history() {
    let mut detector = configured(32, 8, 0.0, 1);
    for i in 0..20 {
        detector
            .observe("k", 100.0 + (i % 3) as f64, ticks(i))
            .unwrap();
    }
    let result = detector.observe("k", 5000.0, ticks(21)).unwrap();
    assert!(result.anomalous);
    assert_eq!(result.samples, 20);
    assert_eq!(result.median, 101.0);
    assert_eq!(result.mad, 1.0);
    assert_eq!(result.score, 4899.0);
}

#[test]
fn oracle_warmup_suppresses_early_flags() {
    let mut detector = configured(32, 10, 0.0, 1);
    for i in 0..9 {
        let result = detector.observe("k", 1e9, ticks(i)).unwrap();
        assert!(!result.anomalous);
        assert!(!result.warmed_up);
        assert_eq!(result.score, 0.0);
    }
}

#[test]
fn oracle_mad_floor_prevents_constant_stream_overflag() {
    let mut detector = configured(32, 8, 1.0, 1);
    for i in 0..20 {
        detector.observe("k", 50.0, ticks(i)).unwrap();
    }
    let result = detector.observe("k", 50.5, ticks(21)).unwrap();
    assert!(!result.anomalous);
    assert_eq!((result.median, result.mad, result.score), (50.0, 1.0, 0.5));
}

#[test]
fn oracle_cardinality_bound_and_eviction_are_explicit() {
    let mut detector = configured(8, 12, 0.0, 100);
    for i in 0..1000 {
        let result = detector
            .observe(&format!("{i}-key"), 1.0, ticks(i))
            .unwrap();
        assert_eq!(result.evicted_key.is_some(), i >= 100);
        assert!(detector.series.len() <= 100);
        assert!(!result.warmed_up);
    }
}

#[test]
fn median_even_odd_and_strict_k_boundary() {
    assert_eq!(median(&[9.0, 1.0, 5.0]).unwrap(), 5.0);
    assert_eq!(median(&[8.0, 2.0, 4.0, 6.0]).unwrap(), 5.0);
    let mut detector = configured(2, 2, 1.0, 1);
    detector.observe("k", 0.0, ticks(0)).unwrap();
    detector.observe("k", 0.0, ticks(1)).unwrap();
    let equal = detector.observe("k", 6.0, ticks(2)).unwrap();
    assert!(equal.warmed_up);
    assert_eq!(equal.score, 6.0);
    assert!(!equal.anomalous);
    assert_eq!(detector.series["k"].values, [0.0, 6.0]);
    // The new value is scored against [0,6], not against itself.
    let above = detector.observe("k", 22.0, ticks(3)).unwrap();
    assert_eq!((above.median, above.mad), (3.0, 3.0));
    assert!(above.anomalous);
}

#[test]
fn ring_eviction_ties_and_empty_keys_are_bounded() {
    let mut detector = configured(2, 3, 0.0, 2);
    detector.observe("b", 1.0, ticks(0)).unwrap();
    detector.observe("a", 2.0, ticks(0)).unwrap();
    let result = detector.observe("", 3.0, ticks(0)).unwrap();
    assert_eq!(result.evicted_key.as_deref(), Some("a"));
    let result = detector.observe("c", 4.0, ticks(0)).unwrap();
    assert_eq!(result.evicted_key.as_deref(), Some(""));
    detector.observe("c", 5.0, ticks(1)).unwrap();
    detector.observe("c", 6.0, ticks(2)).unwrap();
    assert_eq!(detector.series["c"].values, [5.0, 6.0]);
    assert_eq!(detector.series.len(), 2);
}

#[test]
fn gc_uses_strict_ttl_and_reset_retains_clock() {
    let mut detector = configured(2, 2, 0.0, 2);
    detector.observe("a", 1.0, ticks(1)).unwrap();
    detector.observe("b", 2.0, ticks(2)).unwrap();
    assert_eq!(detector.gc(ticks(3), ticks(2)).unwrap(), 0);
    assert_eq!(detector.gc(ticks(4), ticks(2)).unwrap(), 1);
    assert!(detector.series.contains_key("b"));
    assert_eq!(detector.reserved_bytes, 17);
    detector.reset(ticks(4)).unwrap();
    assert_eq!(detector.reserved_bytes, 0);
    assert!(detector.series.is_empty());
    assert_eq!(
        detector.observe("k", 1.0, ticks(3)),
        Err(Unavailable::TimeRegression)
    );
    assert!(!detector.observe("k", 1.0, ticks(4)).unwrap().warmed_up);
}

#[test]
fn regression_is_rejected_before_observe_gc_or_reset_mutates() {
    let mut detector = configured(2, 2, 0.0, 2);
    detector.observe("k", 1.0, ticks(10)).unwrap();
    let before = snapshot(&detector);
    let bytes = detector.reserved_bytes;
    assert_eq!(
        detector.observe("other", 3.0, ticks(9)),
        Err(Unavailable::TimeRegression)
    );
    assert_eq!(
        detector.gc(ticks(9), ticks(0)),
        Err(Unavailable::TimeRegression)
    );
    assert_eq!(detector.reset(ticks(9)), Err(Unavailable::TimeRegression));
    assert_eq!(snapshot(&detector), before);
    assert_eq!(detector.reserved_bytes, bytes);
    assert_eq!(detector.clock.last, Some(ticks(10)));
}

#[test]
fn budget_failure_does_not_evict_or_advance_clock() {
    let config = Config::new(2, 6.0, 2, 0.0, Limits::new(1, 2, 4, 17).unwrap()).unwrap();
    let mut detector = Detector::new(config);
    detector.observe("a", 1.0, ticks(1)).unwrap();
    let before = snapshot(&detector);
    assert_eq!(
        detector.observe("bb", 2.0, ticks(2)),
        Err(Unavailable::Budget)
    );
    assert_eq!(
        detector.observe("oversize", 2.0, ticks(2)),
        Err(Unavailable::Budget)
    );
    assert_eq!(snapshot(&detector), before);
    assert_eq!(detector.reserved_bytes, 17);
    assert_eq!(detector.clock.last, Some(ticks(1)));
}

#[test]
fn invalid_numeric_input_and_overflow_preserve_history() {
    let mut detector = configured(2, 2, 0.0, 1);
    detector.observe("k", f64::MAX, ticks(0)).unwrap();
    detector.observe("k", f64::MAX, ticks(1)).unwrap();
    let before = snapshot(&detector);
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0] {
        assert_eq!(
            detector.observe("k", value, ticks(2)),
            Err(Unavailable::InvalidMeasurement)
        );
        assert_eq!(snapshot(&detector), before);
        assert_eq!(detector.clock.last, Some(ticks(1)));
    }
}

#[test]
fn invalid_configuration_has_no_implicit_defaults() {
    assert!(Limits::new(0, 1, 1, 8).is_err());
    assert!(Limits::new(1, usize::MAX, 1, usize::MAX).is_err());
    let limits = Limits::new(1, 2, 8, 32).unwrap();
    for (window, k, warmup, floor) in [
        (0, 6.0, 1, 0.0),
        (3, 6.0, 1, 0.0),
        (2, 0.0, 1, 0.0),
        (2, f64::NAN, 1, 0.0),
        (2, 6.0, 0, 0.0),
        (2, 6.0, 1, -1.0),
        (2, 6.0, 1, f64::INFINITY),
    ] {
        assert!(Config::new(window, k, warmup, floor, limits).is_err());
    }
    assert!(Config::new(2, 6.0, 3, 0.0, limits).is_ok());
}

#[test]
fn constructor_rejects_vec_layout_overflow_without_allocating_samples() {
    let maximum = (isize::MAX as usize) / size_of::<f64>();
    let too_large = maximum + 1;
    assert!(too_large.checked_mul(size_of::<f64>()).is_some());
    assert!(Limits::new(1, too_large, 1, usize::MAX).is_err());
    let limits = Limits::new(1, maximum, 1, usize::MAX).unwrap();
    assert!(Config::new(maximum, 6.0, 1, 0.0, limits).is_ok());
    assert!(Config::new(too_large, 6.0, 1, 0.0, limits).is_err());
    // Constructors only: never construct a Detector or allocate a sample vector.
}
