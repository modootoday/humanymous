//! Ports of legacy internal/gate wargame R261-R266 (baseline 981d25c) plus the
//! Rust fail-closed bounds.
use super::*;

const NOW: Duration = Duration::from_secs(2_100_000_000);

fn ms(value: u64) -> Duration {
    NOW + Duration::from_millis(value)
}

fn legacy(max_fingerprints: usize) -> Observer {
    let limits = Limits::new(1024, 64, 256, 1 << 20).unwrap();
    Observer::new(
        legacy_detector(limits).unwrap(),
        LEGACY_IDLE,
        max_fingerprints,
    )
    .unwrap()
}

#[test]
fn r261_first_hit_only_seeds_last_seen() {
    let mut o = legacy(LEGACY_MAX_FINGERPRINTS);
    assert_eq!(o.observe("fp1", NOW), Ok(Arrival::First));
    assert_eq!(o.fingerprints(), 1);
}

#[test]
fn r262_r264_warmup_then_spike_flags_on_whole_millisecond_gaps() {
    let mut o = legacy(LEGACY_MAX_FINGERPRINTS);
    o.observe("fp", ms(0)).unwrap();
    let mut t = 0;
    for i in 0..20 {
        t += 100 + (i % 3);
        let Arrival::Measured {
            gap_ms,
            measurement,
        } = o.observe("fp", ms(t)).unwrap()
        else {
            panic!("second and later hits are measured");
        };
        assert_eq!(gap_ms, u128::from(100 + (i % 3)));
        assert!(!measurement.anomalous, "steady gaps never flag ({i})");
        assert_eq!(measurement.warmed_up, i >= 12);
    }
    t += 100_000;
    let Arrival::Measured { measurement, .. } = o.observe("fp", ms(t)).unwrap() else {
        panic!("spike is measured");
    };
    assert!(
        measurement.anomalous,
        "a huge inter-arrival jump is an outlier"
    );
    let sub = o
        .observe("fp", ms(t) + Duration::from_micros(1999))
        .unwrap();
    assert!(
        matches!(sub, Arrival::Measured { gap_ms: 1, .. }),
        "gap truncates to ms"
    );
}

#[test]
fn r265_empty_fingerprint_is_ignored() {
    let mut o = legacy(LEGACY_MAX_FINGERPRINTS);
    assert_eq!(o.observe("", NOW), Ok(Arrival::Ignored));
    assert_eq!(o.fingerprints(), 0);
}

#[test]
fn r266_gc_drops_last_seen_strictly_after_idle() {
    let mut o = legacy(LEGACY_MAX_FINGERPRINTS);
    o.observe("fp-gc", NOW).unwrap();
    o.observe("fp-gc", NOW + Duration::from_secs(1)).unwrap();
    assert_eq!(o.gc(NOW + Duration::from_secs(1) + LEGACY_IDLE), Ok(0));
    assert_eq!(o.gc(NOW + Duration::from_secs(11 * 60)), Ok(1));
    assert_eq!(o.fingerprints(), 0);
    assert_eq!(
        o.observe("fp-gc", NOW + Duration::from_secs(12 * 60)),
        Ok(Arrival::First)
    );
}

#[test]
fn last_seen_bound_refuses_new_fingerprints_after_sweep() {
    let mut o = legacy(2);
    assert_eq!(o.observe("a", NOW), Ok(Arrival::First));
    assert_eq!(o.observe("b", NOW), Ok(Arrival::First));
    assert_eq!(o.observe("c", NOW), Ok(Arrival::Saturated));
    assert!(
        matches!(o.observe("a", ms(5)), Ok(Arrival::Measured { .. })),
        "known keys still measure"
    );
    let later = NOW + Duration::from_secs(11 * 60);
    assert_eq!(
        o.observe("c", later),
        Ok(Arrival::First),
        "idle entries are swept first"
    );
}

#[test]
fn time_regression_and_bad_configuration_are_refused() {
    let mut o = legacy(4);
    o.observe("fp", ms(10)).unwrap();
    assert_eq!(o.observe("fp", ms(9)), Err(Unavailable::TimeRegression));
    assert_eq!(o.gc(ms(9)), Err(Unavailable::TimeRegression));
    let limits = Limits::new(1, 64, 8, 1 << 20).unwrap();
    assert!(Observer::new(legacy_detector(limits).unwrap(), Duration::ZERO, 1).is_err());
    assert!(Observer::new(legacy_detector(limits).unwrap(), LEGACY_IDLE, 0).is_err());
}
