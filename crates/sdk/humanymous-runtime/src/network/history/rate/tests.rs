use super::*;

fn ticks(value: u64) -> Duration {
    Duration::from_secs(value)
}
fn limiter(
    window: u64,
    soft: usize,
    hard: usize,
    keys: usize,
    hits: usize,
    scan: usize,
) -> Limiter {
    Limiter::new(
        Config::new(
            ticks(window),
            soft,
            hard,
            Limits::new(keys, hits, 32, ticks(100), 1_000_000, scan).unwrap(),
        )
        .unwrap(),
    )
}
fn counted(result: Observation) -> (usize, Level, Coverage) {
    match result {
        Observation::Counted {
            count,
            level,
            coverage,
        } => (count, level, coverage),
        Observation::IgnoredEmptyKey => panic!("expected an observed key"),
    }
}

#[test]
fn oracle_sliding_window_classifies_twelve_fast_hits_as_hard() {
    let mut rate = limiter(1, 5, 10, 1, 20, 1);
    for i in 0..12 {
        let (count, level, _) =
            counted(rate.observe("fpA", Duration::from_millis(i * 10)).unwrap());
        assert_eq!(count, i as usize + 1);
        assert_eq!(level, rate.level(count));
        if i == 11 {
            assert_eq!(level, Level::Hard);
        }
    }
}

#[test]
fn oracle_window_expiry_does_not_accumulate_spaced_requests() {
    let mut rate = limiter(1, 5, 10, 1, 20, 1);
    for i in 0..12 {
        let (count, level, _) = counted(rate.observe("fpB", ticks(i * 2)).unwrap());
        assert_eq!((count, level), (1, Level::Normal));
    }
}

#[test]
fn oracle_per_key_isolation_uses_ordered_caller_time() {
    let mut rate = limiter(1, 5, 10, 2, 20, 1);
    for i in 0..11 {
        rate.observe("flooder", Duration::from_millis(i * 10))
            .unwrap();
    }
    let (count, level, _) = counted(
        rate.observe("innocent", Duration::from_millis(100))
            .unwrap(),
    );
    assert_eq!((count, level), (1, Level::Normal));
}

#[test]
fn oracle_empty_key_is_ignored_without_normal_count_or_clock_advance() {
    let mut rate = limiter(1, 5, 10, 1, 20, 1);
    assert_eq!(
        rate.observe("", ticks(1)).unwrap(),
        Observation::IgnoredEmptyKey
    );
    assert!(rate.hits.is_empty());
    assert_eq!(rate.clock.last, None);
    rate.observe("a", ticks(0)).unwrap();
    assert_eq!(
        rate.observe("", ticks(0)).unwrap(),
        Observation::IgnoredEmptyKey
    );
}

#[test]
fn oracle_configure_preserves_rolling_hits_and_applies_cutpoints() {
    let mut rate = limiter(10, 3, 5, 1, 20, 1);
    rate.observe("fp:x", ticks(0)).unwrap();
    rate.observe("fp:x", ticks(1)).unwrap();
    let effect = rate.configure(ticks(20), 2, 3, ticks(1)).unwrap();
    assert!(effect.window_expanded && effect.history_incomplete);
    assert_eq!(rate.level(2), Level::Soft);
    let (count, level, coverage) = counted(rate.observe("fp:x", ticks(2)).unwrap());
    assert_eq!((count, level), (3, Level::Hard));
    assert!(matches!(coverage, Coverage::WindowExpanded { .. }));
}

#[test]
fn exact_window_and_cutpoint_boundaries_are_preserved() {
    let mut rate = limiter(1, 2, 3, 1, 4, 1);
    assert_eq!(
        counted(rate.observe("a", ticks(0)).unwrap()).1,
        Level::Normal
    );
    assert_eq!(counted(rate.observe("a", ticks(0)).unwrap()).1, Level::Soft);
    assert_eq!(counted(rate.observe("a", ticks(0)).unwrap()).1, Level::Hard);
    assert_eq!(counted(rate.observe("a", ticks(1)).unwrap()).0, 1);
    rate.configure(ticks(1), 2, 2, ticks(1)).unwrap();
    assert_eq!(rate.level(2), Level::Hard);
}

#[test]
fn bounded_gc_cursor_progresses_and_ttl_equality_survives() {
    let mut rate = limiter(1, 2, 3, 3, 4, 1);
    for key in ["a", "b", "c"] {
        rate.observe(key, ticks(0)).unwrap();
    }
    assert_eq!(
        rate.gc(ticks(6)).unwrap(),
        GcResult {
            scanned: 1,
            removed: 0
        }
    );
    for _ in 0..3 {
        assert_eq!(
            rate.gc(ticks(7)).unwrap(),
            GcResult {
                scanned: 1,
                removed: 1
            }
        );
    }
    assert!(rate.hits.is_empty());
    assert_eq!(rate.reserved_bytes, 0);
    assert_eq!(
        rate.gc(ticks(7)).unwrap(),
        GcResult {
            scanned: 0,
            removed: 0
        }
    );
}

#[test]
fn failure_preserves_hits_policy_cursor_and_clock() {
    let mut rate = limiter(1, 2, 3, 1, 2, 1);
    rate.observe("a", ticks(1)).unwrap();
    rate.observe("a", ticks(1)).unwrap();
    let before = rate.hits.clone();
    assert_eq!(rate.observe("a", ticks(1)), Err(Unavailable::Budget));
    assert_eq!(rate.observe("other", ticks(1)), Err(Unavailable::Budget));
    assert_eq!(
        rate.observe("a", ticks(0)),
        Err(Unavailable::TimeRegression)
    );
    assert_eq!(rate.gc(ticks(0)), Err(Unavailable::TimeRegression));
    assert_eq!(rate.reset(ticks(0)), Err(Unavailable::TimeRegression));
    assert_eq!(
        rate.configure(ticks(2), 1, 2, ticks(0)),
        Err(Unavailable::TimeRegression)
    );
    assert_eq!(
        rate.configure(ticks(2), 0, 2, ticks(2)),
        Err(Unavailable::InvalidConfiguration)
    );
    assert_eq!(rate.hits, before);
    assert_eq!(rate.clock.last, Some(ticks(1)));
    assert_eq!(rate.config.window, ticks(1));
    assert_eq!(rate.gc_cursor, None);
    // Expired hits may be trimmed and replaced without losing recent hits.
    assert_eq!(counted(rate.observe("a", ticks(2)).unwrap()).0, 1);
}

#[test]
fn payload_and_key_budgets_fail_before_creating_history() {
    let limits = Limits::new(2, 1, 2, ticks(1), size_of::<Duration>() + 1, 1).unwrap();
    let mut rate = Limiter::new(Config::new(ticks(1), 1, 2, limits).unwrap());
    assert_eq!(rate.observe("aa", ticks(0)), Err(Unavailable::Budget));
    assert_eq!(rate.observe("long", ticks(0)), Err(Unavailable::Budget));
    assert!(rate.hits.is_empty());
    assert_eq!(rate.clock.last, None);
    rate.observe("a", ticks(0)).unwrap();
    assert_eq!(rate.observe("b", ticks(0)), Err(Unavailable::Budget));
    assert_eq!(rate.hits.len(), 1);
}

#[test]
fn expansion_gap_and_reset_do_not_fabricate_retrospective_hits() {
    let mut rate = limiter(1, 2, 3, 1, 4, 1);
    rate.observe("a", ticks(0)).unwrap();
    rate.observe("a", ticks(2)).unwrap(); // First hit expired permanently.
    rate.configure(ticks(10), 2, 3, ticks(2)).unwrap();
    assert_eq!(rate.hits["a"], [ticks(2)]);
    rate.reset(ticks(2)).unwrap();
    assert!(rate.hits.is_empty());
    assert_eq!(rate.reserved_bytes, 0);
    assert_eq!(rate.config.window, ticks(10));
    assert_eq!(
        rate.observe("a", ticks(1)),
        Err(Unavailable::TimeRegression)
    );
    assert!(
        matches!(counted(rate.observe("a", ticks(2)).unwrap()).2, Coverage::WindowExpanded { complete_after } if complete_after == ticks(12))
    );
    assert_eq!(
        counted(rate.observe("a", ticks(12)).unwrap()).2,
        Coverage::Retained
    );
}

#[test]
fn invalid_config_has_no_defaults_or_overflowing_ttl() {
    assert!(Limits::new(0, 1, 1, ticks(1), 100, 1).is_err());
    assert!(Limits::new(1, 1, 1, ticks(1), 100, 4097).is_err());
    assert!(Limits::new(1, usize::MAX, 1, ticks(1), usize::MAX, 1).is_err());
    let limits = Limits::new(1, 1, 1, Duration::MAX, 100, 1).unwrap();
    assert!(Config::new(Duration::ZERO, 1, 2, limits).is_err());
    assert!(Config::new(Duration::MAX, 1, 2, limits).is_err());
    assert!(Config::new(ticks(1), 0, 2, limits).is_err());
    assert!(Config::new(ticks(1), 3, 2, limits).is_err());
}

#[test]
fn tightening_defers_key_trim_and_expansion_overflow_preserves_config() {
    let mut rate = limiter(10, 3, 5, 2, 4, 2);
    rate.observe("a", ticks(0)).unwrap();
    rate.observe("b", ticks(0)).unwrap();
    rate.observe("a", ticks(4)).unwrap();
    rate.configure(ticks(2), 1, 2, ticks(4)).unwrap();
    assert_eq!(rate.hits["a"], [ticks(0), ticks(4)]);
    assert_eq!(rate.hits["b"], [ticks(0)]);
    assert_eq!(counted(rate.observe("a", ticks(4)).unwrap()).0, 2);
    assert_eq!(rate.hits["b"], [ticks(0)]);
    let before = rate.hits.clone();
    assert_eq!(
        rate.configure(ticks(3), 1, 2, Duration::MAX),
        Err(Unavailable::InvalidConfiguration)
    );
    assert_eq!(rate.hits, before);
    assert_eq!(rate.clock.last, Some(ticks(4)));
    assert_eq!(rate.config.window, ticks(2));
}

#[test]
fn constructor_rejects_vec_layout_overflow_without_allocating_hits() {
    let maximum = (isize::MAX as usize) / size_of::<Duration>();
    let too_large = maximum + 1;
    assert!(too_large.checked_mul(size_of::<Duration>()).is_some());
    assert!(Limits::new(1, too_large, 1, ticks(1), usize::MAX, 1).is_err());
    let limits = Limits::new(1, maximum, 1, ticks(1), usize::MAX, 1).unwrap();
    assert!(Config::new(ticks(1), 1, 2, limits).is_ok());
    // Constructors only: never construct a Limiter or allocate timestamp storage.
}
