//! Ports TestServerGCRunsWithoutPanic (legacy internal/gate/gc_test.go,
//! baseline 981d25c) plus the per-kernel fan-out and refusal isolation.
use super::*;
use crate::gateway::ban::{Key, Store};
use crate::gateway::verdict_store::{Sticky, Verdict, VerdictStore};
use crate::network::history::rate;

const T0: Duration = Duration::from_secs(1_700_000_000);

fn bans() -> Store {
    let limits = rate::Limits::new(64, 64, 128, Duration::from_secs(60), 1 << 20, 64).unwrap();
    let rate = rate::Config::new(Duration::from_secs(10), 1, 1, limits).unwrap();
    Store::new(rate, 16).unwrap()
}
fn detector() -> sweep::Detector {
    sweep::Detector::new(sweep::Config::new(Duration::from_secs(30), 8, 8).unwrap())
}
fn anomaly_detector() -> anomaly::Detector {
    anomaly::Detector::new(
        anomaly::Config::new(
            4,
            6.0,
            2,
            0.0,
            anomaly::Limits::new(8, 4, 64, 1_000_000).unwrap(),
        )
        .unwrap(),
    )
}
fn credentials() -> correlation::Registry {
    correlation::Registry::new(
        correlation::Config::new(
            Duration::from_secs(3600),
            correlation::Limits::new(1000, 1000, 1000, 256, 1_000_000).unwrap(),
            anomaly::Limits::new(16, 256, 256, 1_000_000).unwrap(),
        )
        .unwrap(),
    )
}
fn deny(at: Duration) -> Sticky {
    Sticky {
        verdict: Verdict::Deny,
        updated: at,
        ..Sticky::default()
    }
}

#[test]
fn server_gc_runs_without_panic_with_default_kernels() {
    // Every optional kernel off: a no-op that is still complete.
    let report = sweep(Kernels::default(), T0);
    assert_eq!(report, Report::default());
    assert!(report.complete());

    // The default in-memory ledgers present: swept without refusal.
    let verdicts = VerdictStore::new(Duration::from_secs(60));
    let mut store = bans();
    let report = sweep(
        Kernels {
            verdicts: Some(&verdicts),
            bans: Some(&mut store),
            ..Kernels::default()
        },
        T0,
    );
    assert!(report.verdicts);
    assert_eq!(report.bans, Some(Ok(())));
    assert!(report.complete());
}

#[test]
fn sweeps_every_present_kernel() {
    let verdicts = VerdictStore::new(Duration::from_secs(60));
    assert!(verdicts.set("sid-1", deny(T0)));
    let mut store = bans();
    let ip = Key::parse("ip:1.2.3.4").unwrap();
    for _ in 0..8 {
        store.observe(&ip, T0).unwrap();
    }
    assert!(store.check(&ip, T0).is_some(), "hard breach must auto-ban");
    let mut recon = detector();
    recon.observe("fp-1", "sid-1", T0).unwrap();
    let mut series = anomaly_detector();
    series.observe("fp-1", 1.0, T0).unwrap();
    let mut creds = credentials();
    creds
        .observe(
            "cred-1",
            "203.0.113.0/24",
            "sid-1",
            T0,
            correlation::FleetInput::LocalOnly,
        )
        .unwrap();

    let later = T0 + ban::STRIKE_DECAY + Duration::from_secs(2 * 3600);
    let report = sweep(
        Kernels {
            verdicts: Some(&verdicts),
            bans: Some(&mut store),
            sweep: Some(&mut recon),
            anomaly: Some(&mut series),
            credentials: Some(&mut creds),
        },
        later,
    );
    assert!(report.complete());
    assert!(verdicts.is_empty(), "expired sticky verdict must be swept");
    assert!(store.list(later).is_empty(), "expired ban must be swept");
    assert_eq!(report.sweep, Some(Ok(1)), "aged-out binding must be swept");
    assert_eq!(report.anomaly, Some(Ok(1)), "idle series must be swept");
    assert_eq!(report.credentials, Some(Ok(())));
    assert!(matches!(creds.stats("cred-1"), correlation::Stats::Absent));
}

#[test]
fn a_refusing_kernel_does_not_stop_the_others() {
    let verdicts = VerdictStore::new(Duration::from_secs(60));
    assert!(verdicts.set("sid-1", deny(T0)));
    let mut recon = detector();
    let ahead = T0 + Duration::from_secs(600);
    recon.observe("fp-1", "sid-1", ahead).unwrap();
    let mut series = anomaly_detector();
    series.observe("fp-1", 1.0, T0).unwrap();

    let later = T0 + Duration::from_secs(300);
    let report = sweep(
        Kernels {
            verdicts: Some(&verdicts),
            sweep: Some(&mut recon),
            anomaly: Some(&mut series),
            ..Kernels::default()
        },
        later,
    );
    assert_eq!(report.sweep, Some(Err(Unavailable::TimeRegression)));
    assert!(
        !report.complete(),
        "a refused kernel makes the sweep incomplete"
    );
    assert!(report.verdicts && verdicts.is_empty());
    assert_eq!(
        report.anomaly,
        Some(Ok(0)),
        "series idle exactly 300s survives"
    );
}

#[test]
fn anomaly_ttl_matches_the_legacy_observer() {
    assert_eq!(ANOMALY_IDLE_TTL, Duration::from_secs(600));
}
