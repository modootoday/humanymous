//! Ports of legacy internal/gate wargame R251-R260 and R279/R280 cohort locks
//! (baseline 981d25c) plus the Rust fail-closed bounds.
use super::*;
use humanymous_core::signals::report::{KeyFeatures, MouseFeatures};

const NOW: Duration = Duration::from_secs(2_000_000_000);

fn synth_motor(curvature: f64) -> BehaviorSummary {
    BehaviorSummary {
        mouse: MouseFeatures {
            samples: 20,
            mean_curvature: curvature,
            velocity_std_dev: 2.0,
            coalesced_ratio: 1.0,
            straight_line_frac: 0.5,
            ..MouseFeatures::default()
        },
        key: KeyFeatures {
            keystrokes: 5,
            mean_flight_ms: 40.0,
            flight_std_dev: 5.0,
            ..KeyFeatures::default()
        },
        ..BehaviorSummary::default()
    }
}

fn legacy() -> Observer {
    Observer::new(Config::new(LEGACY_MIN_COHORT, LEGACY_IDLE, LEGACY_MAX_COHORTS, 1024).unwrap())
}

fn farm(observer: &mut Observer, cohort: &str, b: &BehaviorSummary, n: usize) -> Vec<Cohort> {
    (0..n)
        .map(|i| {
            observer
                .observe(cohort, b, &format!("sid-{i}"), NOW)
                .unwrap()
        })
        .collect()
}

#[test]
fn r251_identical_farm_flags_once() {
    let mut o = legacy();
    let b = synth_motor(0.30);
    let outcomes = farm(&mut o, "fp|203.0.113.0", &b, LEGACY_MIN_COHORT);
    assert_eq!(outcomes.last(), Some(&Cohort::Flagged(LEGACY_MIN_COHORT)));
    assert_eq!(
        outcomes[..5],
        (1..=5).map(Cohort::Counted).collect::<Vec<_>>()[..]
    );
    assert!(o.is_flagged("fp|203.0.113.0", signature(&b).unwrap()));
}

#[test]
fn r252_solo_session_does_not_flag() {
    let mut o = legacy();
    let b = synth_motor(0.55);
    assert_eq!(
        o.observe("solo-cohort", &b, "only", NOW),
        Ok(Cohort::Counted(1))
    );
    assert_eq!(
        o.observe("solo-cohort", &b, "only", NOW),
        Ok(Cohort::Counted(1))
    );
    assert!(!o.is_flagged("solo-cohort", signature(&b).unwrap()));
}

#[test]
fn r253_empty_behavior_is_ignored_without_allocation() {
    let mut o = legacy();
    assert_eq!(
        o.observe("c", &BehaviorSummary::default(), "sid1", NOW),
        Ok(Cohort::Ignored)
    );
    assert_eq!(o.cohorts(), 0);
    assert_eq!(signature(&BehaviorSummary::default()), None);
}

#[test]
fn r254_empty_cohort_key_or_session_is_ignored() {
    let mut o = legacy();
    let b = synth_motor(0.2);
    for (cohort, sid) in [("", "sid"), ("|", "sid"), ("c", "")] {
        assert_eq!(o.observe(cohort, &b, sid, NOW), Ok(Cohort::Ignored));
    }
    assert_eq!(o.cohorts(), 0);
}

#[test]
fn r255_near_identical_vectors_collide_including_ieee_edges() {
    assert_eq!(signature(&synth_motor(0.30)), signature(&synth_motor(0.31)));
    // 0.30 / 0.05 is 5.999..., rounding (not truncation) keeps one bucket.
    assert_eq!(
        signature(&synth_motor(0.30)),
        signature(&synth_motor(0.299))
    );
}

#[test]
fn r256_human_spread_separates_buckets() {
    assert_ne!(signature(&synth_motor(0.10)), signature(&synth_motor(0.90)));
}

#[test]
fn r257_flag_releases_session_set() {
    let mut o = legacy();
    let b = synth_motor(0.40);
    farm(&mut o, "farm", &b, LEGACY_MIN_COHORT + 50);
    assert_eq!(o.retained_sessions("farm", signature(&b).unwrap()), 0);
}

#[test]
fn r258_post_flag_sessions_are_not_retained() {
    let mut o = legacy();
    let b = synth_motor(0.42);
    farm(&mut o, "farm2", &b, LEGACY_MIN_COHORT);
    for i in 0..100 {
        assert_eq!(
            o.observe("farm2", &b, &format!("post-{i}"), NOW),
            Ok(Cohort::AlreadyFlagged)
        );
    }
    assert_eq!(o.retained_sessions("farm2", signature(&b).unwrap()), 0);
}

#[test]
fn r259_idle_cohort_is_evicted_strictly_after_ten_minutes() {
    let mut o = legacy();
    o.observe("idle", &synth_motor(0.2), "s1", NOW).unwrap();
    assert_eq!(o.gc(NOW + LEGACY_IDLE), Ok(0), "equality survives");
    assert_eq!(o.gc(NOW + Duration::from_secs(11 * 60)), Ok(1));
    assert_eq!(o.cohorts(), 0);
}

#[test]
fn r260_r279_r280_shadow_outcome_is_measurement_only() {
    // The observer owns no ban store, verdict or enforcement handle; its only
    // output is this enum. Pin the R280 farm-flag lock on its result.
    let mut o = legacy();
    let b = synth_motor(0.35);
    let outcomes = farm(&mut o, "close", &b, LEGACY_MIN_COHORT);
    assert!(matches!(outcomes.last(), Some(Cohort::Flagged(_))));
    assert!(o.is_flagged("close", signature(&b).unwrap()));
}

#[test]
fn signature_keeps_low_sixteen_bits_like_go() {
    let mut a = synth_motor(0.30);
    let mut b = synth_motor(0.30);
    a.key.mean_flight_ms = 10.0;
    b.key.mean_flight_ms = 10.0 * f64::from(1 + (1 << 16));
    assert_eq!(signature(&a), signature(&b), "Go byte(p), byte(p>>8) wraps");
}

#[test]
fn non_finite_moment_has_no_signature() {
    let mut b = synth_motor(0.30);
    b.mouse.velocity_std_dev = f64::NAN;
    assert_eq!(signature(&b), None);
    b.mouse.velocity_std_dev = f64::INFINITY;
    assert_eq!(signature(&b), None);
}

#[test]
fn cohort_and_signature_bounds_refuse_instead_of_growing() {
    let mut o = Observer::new(Config::new(6, LEGACY_IDLE, 2, 2).unwrap());
    let b = synth_motor(0.2);
    assert_eq!(o.observe("a", &b, "s", NOW), Ok(Cohort::Counted(1)));
    assert_eq!(o.observe("b", &b, "s", NOW), Ok(Cohort::Counted(1)));
    assert_eq!(o.observe("c", &b, "s", NOW), Ok(Cohort::Saturated));
    let later = NOW + Duration::from_secs(11 * 60);
    assert_eq!(
        o.observe("c", &b, "s", later),
        Ok(Cohort::Counted(1)),
        "idle cohorts are swept first"
    );
    assert_eq!(
        o.observe("c", &synth_motor(0.5), "s", later),
        Ok(Cohort::Counted(1))
    );
    assert_eq!(
        o.observe("c", &synth_motor(0.9), "s", later),
        Ok(Cohort::Saturated)
    );
}

#[test]
fn time_regression_and_bad_configuration_are_refused() {
    let mut o = legacy();
    o.observe("a", &synth_motor(0.2), "s", NOW).unwrap();
    assert_eq!(
        o.observe("a", &synth_motor(0.2), "t", NOW - Duration::from_secs(1)),
        Err(Unavailable::TimeRegression)
    );
    assert_eq!(
        o.retained_sessions("a", signature(&synth_motor(0.2)).unwrap()),
        1
    );
    assert!(Config::new(0, LEGACY_IDLE, 1, 1).is_err());
    assert!(Config::new(1, Duration::ZERO, 1, 1).is_err());
    assert!(Config::new(1, LEGACY_IDLE, 0, 1).is_err());
    assert!(Config::new(1, LEGACY_IDLE, 1, 0).is_err());
}
