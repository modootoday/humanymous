use super::*;
use crate::history::anomaly;
use std::time::Duration;
fn ticks(value: u64) -> Duration {
    Duration::from_secs(value)
}
fn registry() -> Registry {
    Registry::new(
        Config::new(
            ticks(3600),
            Limits::new(1000, 1000, 1000, 256, 1_000_000).unwrap(),
            anomaly::Limits::new(16, 256, 256, 1_000_000).unwrap(),
        )
        .unwrap(),
    )
}
fn has(out: &Outcome, id: &str) -> bool {
    out.signals.iter().any(|s| s.id == id)
}
#[test]
fn oracle_proxy_rotation_at_three_not_two_subnets() {
    let mut r = registry();
    for i in 0..3 {
        let out = r
            .observe(
                "fp|ja4",
                &format!("sub{i}"),
                "s",
                ticks(0),
                FleetInput::LocalOnly,
            )
            .unwrap();
        assert_eq!(has(&out, "l5.correlation.proxy_rotation"), i == 2);
    }
}
#[test]
fn oracle_shared_fingerprint_at_five_and_proxy_takes_precedence() {
    let mut r = registry();
    for i in 0..5 {
        let out = r
            .observe(
                "fp|ja4",
                "sub0",
                &format!("s{i}"),
                ticks(0),
                FleetInput::LocalOnly,
            )
            .unwrap();
        assert_eq!(has(&out, "l5.correlation.shared_fingerprint"), i == 4);
    }
    r.observe("fp|ja4", "sub1", "s", ticks(0), FleetInput::LocalOnly)
        .unwrap();
    let out = r
        .observe("fp|ja4", "sub2", "s", ticks(0), FleetInput::LocalOnly)
        .unwrap();
    assert!(has(&out, "l5.correlation.proxy_rotation"));
    assert!(!has(&out, "l5.correlation.shared_fingerprint"));
}
#[test]
fn oracle_single_session_has_no_signal() {
    assert!(
        registry()
            .observe("fp|ja4", "sub", "s", ticks(0), FleetInput::LocalOnly)
            .unwrap()
            .signals
            .is_empty()
    );
}
#[test]
fn oracle_empty_keys_are_ignored_with_explicit_missing_scope() {
    let mut r = registry();
    for key in ["", "|"] {
        let out = r
            .observe(key, "sub", "s", ticks(0), FleetInput::LocalOnly)
            .unwrap();
        assert!(out.signals.is_empty());
        assert_eq!(out.scope, Scope::MissingKey);
        assert_eq!(r.stats(key), Stats::Absent);
    }
}
#[test]
fn oracle_fanout_needs_three_fingerprints_and_three_subnets() {
    let mut r = registry();
    for i in 0..3 {
        let out = r
            .observe_ja4_fanout("ja4", &format!("fp{i}"), &format!("sub{i}"), ticks(0))
            .unwrap();
        assert_eq!(has(&out, "l5.correlation.fp_churn_proxy"), i == 2);
    }
    let mut half = registry();
    for i in 0..3 {
        assert!(!has(
            &half
                .observe_ja4_fanout("ja4", &format!("fp{i}"), "same", ticks(0))
                .unwrap(),
            "l5.correlation.fp_churn_proxy"
        ));
    }
}
#[test]
fn globally_repeated_subnet_does_not_feed_a_velocity_outlier() {
    let mut r = registry();
    for i in 0..40 {
        r.observe(
            &format!("org{i}"),
            "sub",
            "s",
            ticks(0),
            FleetInput::LocalOnly,
        )
        .unwrap();
    }
    let out = r
        .observe(
            "reported",
            "sub",
            "s",
            ticks(0),
            FleetInput::Reported(vec![0, 50]),
        )
        .unwrap();
    assert!(!has(&out, "l5.correlation.ip_velocity"));
    assert_eq!(out.scope, Scope::CoordinatorReportedUnqualified);
    assert_eq!(
        r.stats("reported"),
        Stats::Observed {
            subnets: 1,
            sessions: 1
        }
    );
}
#[test]
fn oracle_reported_fleet_velocity_affects_only_weight_zero_residual() {
    let mut r = registry();
    for i in 0..40 {
        r.observe(
            &format!("org{i}|ja4"),
            "sub",
            "s",
            ticks(0),
            FleetInput::Reported(vec![1, 1]),
        )
        .unwrap();
    }
    let out = r
        .observe(
            "bot|ja4",
            "sub",
            "s",
            ticks(0),
            FleetInput::Reported(vec![1, 50]),
        )
        .unwrap();
    assert!(has(&out, "l5.correlation.ip_velocity"));
    assert!(!has(&out, "l5.correlation.proxy_rotation"));
    assert_eq!(out.scope, Scope::CoordinatorReportedUnqualified);
    assert_eq!(
        r.stats("bot|ja4"),
        Stats::Observed {
            subnets: 1,
            sessions: 1
        }
    );
}
#[test]
fn oracle_coordinator_outage_keeps_local_burst_observation() {
    let mut r = registry();
    for i in 0..40 {
        r.observe(
            &format!("org{i}"),
            "sub",
            "s",
            ticks(0),
            FleetInput::Unavailable,
        )
        .unwrap();
    }
    let mut fired = false;
    for i in 0..30 {
        let out = r
            .observe(
                "bot",
                &format!("sub{i}"),
                "s",
                ticks(i),
                FleetInput::Unavailable,
            )
            .unwrap();
        assert_eq!(out.scope, Scope::LocalDegraded);
        fired |= has(&out, "l5.correlation.ip_velocity");
    }
    assert!(fired);
}
#[test]
fn oracle_fallback_counters_distinguish_absent_reported_and_malformed() {
    for mode in 0..4 {
        let mut r = registry();
        for i in 0..20 {
            let fleet = match mode {
                0 => FleetInput::LocalOnly,
                1 => FleetInput::Reported(vec![1, 1]),
                2 => FleetInput::Unavailable,
                _ => FleetInput::Reported(vec![1]),
            };
            r.observe(&format!("fp{i}"), "sub", "s", ticks(0), fleet)
                .unwrap();
        }
        assert_eq!(r.fleet_fallbacks(), if mode < 2 { 0 } else { 20 });
    }
}
#[test]
fn oracle_concurrent_fallbacks_are_serialized_by_the_owning_runtime() {
    let registry = std::sync::Arc::new(std::sync::Mutex::new(registry()));
    let mut workers = Vec::new();
    for thread in 0..8 {
        let registry = registry.clone();
        workers.push(std::thread::spawn(move || {
            for i in 0..50 {
                registry
                    .lock()
                    .unwrap()
                    .observe(
                        &format!("fp{thread}-{i}"),
                        "sub",
                        "s",
                        ticks(0),
                        FleetInput::Unavailable,
                    )
                    .unwrap();
            }
        }));
    }
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(registry.lock().unwrap().fleet_fallbacks(), 400);
}
#[test]
fn oracle_digest_is_stable_keyed_and_known_answer_compatible() {
    assert_eq!(
        key_digest(&[], "a"),
        "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
    );
    assert_ne!(key_digest(&[], "a"), key_digest(&[], "b"));
    assert_ne!(key_digest(b"fleetkey", "a"), key_digest(&[], "a"));
    assert_eq!(
        key_digest(&[0x0b; 20], "Hi There"),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
    assert_eq!(
        key_digest(
            &[0xaa; 131],
            "Test Using Larger Than Block-Size Key - Hash Key First"
        ),
        "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
    );
}
#[test]
fn oracle_velocity_remains_weight_zero() {
    let definition =
        humanymous_core::signals::observation::lookup("l5.correlation.ip_velocity").unwrap();
    assert_eq!(definition.weight, 0.0);
}
#[test]
fn oracle_local_burst_against_population_emits_weight_zero_velocity() {
    let mut r = registry();
    for i in 0..40 {
        r.observe(
            &format!("org{i}"),
            "sub",
            "s",
            ticks(0),
            FleetInput::LocalOnly,
        )
        .unwrap();
    }
    let mut fired = false;
    for i in 0..30 {
        for signal in r
            .observe(
                "bot",
                &format!("sub{i}"),
                "s",
                ticks(i),
                FleetInput::LocalOnly,
            )
            .unwrap()
            .signals
        {
            if signal.id == "l5.correlation.ip_velocity" {
                assert_eq!(signal.weight, 0.0);
                fired = true;
            }
        }
    }
    assert!(fired);
}
#[test]
fn oracle_slow_organic_accumulation_is_not_velocity() {
    let mut r = registry();
    for i in 0..100 {
        assert!(!has(
            &r.observe(
                "slow",
                &format!("sub{i}"),
                "s",
                ticks(i * 30),
                FleetInput::LocalOnly
            )
            .unwrap(),
            "l5.correlation.ip_velocity"
        ));
    }
}
#[test]
fn ttl_equality_stats_gc_reset_and_fallback_retention() {
    let mut r = registry();
    r.observe("key", "sub", "s", ticks(0), FleetInput::Unavailable)
        .unwrap();
    r.gc(ticks(3600)).unwrap();
    assert!(matches!(r.stats("key"), Stats::Observed { .. }));
    r.gc(ticks(3601)).unwrap();
    assert_eq!(r.stats("key"), Stats::Absent);
    r.reset(ticks(3601)).unwrap();
    assert_eq!(r.fleet_fallbacks(), 1);
    assert!(
        r.observe("key", "sub", "s", ticks(1), FleetInput::LocalOnly)
            .is_err()
    );
}
#[test]
fn set_budget_and_time_failure_preserve_counts() {
    let mut r = Registry::new(
        Config::new(
            ticks(1),
            Limits::new(1, 1, 1, 64, 10000).unwrap(),
            anomaly::Limits::new(16, 256, 64, 10000).unwrap(),
        )
        .unwrap(),
    );
    r.observe("key", "sub", "s", ticks(1), FleetInput::LocalOnly)
        .unwrap();
    assert!(
        r.observe("key", "other", "s", ticks(1), FleetInput::LocalOnly)
            .is_err()
    );
    assert!(
        r.observe("key", "sub", "s", ticks(0), FleetInput::LocalOnly)
            .is_err()
    );
    assert_eq!(
        r.stats("key"),
        Stats::Observed {
            subnets: 1,
            sessions: 1
        }
    );
}
#[test]
fn reported_count_clamps_and_added_requires_exact_one() {
    assert_eq!(FleetInput::Reported(vec![1, -1]).counts(false, 2).1, 0);
    assert_eq!(
        FleetInput::Reported(vec![1, i64::MAX]).counts(false, 2).1,
        i32::MAX as usize
    );
    assert!(!FleetInput::Reported(vec![2, 9]).counts(true, 2).0);
}
