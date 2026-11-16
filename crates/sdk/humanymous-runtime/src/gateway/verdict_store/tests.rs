//! Ports TestVerdictStoreGC (legacy internal/gate/gc_test.go, baseline
//! 981d25c) plus TTL, capacity and mapping boundaries.
use super::*;

const T0: Duration = Duration::from_secs(1_000);

fn deny(at: Duration) -> Sticky {
    Sticky {
        verdict: Verdict::Deny,
        updated: at,
        ..Sticky::default()
    }
}

#[test]
fn gc_sweeps_expired_sticky_verdicts() {
    let store = VerdictStore::new(Duration::from_secs(60));
    assert!(store.set("sid-1", deny(T0)));
    store.gc(T0 + Duration::from_secs(90));
    assert!(store.is_empty(), "expired sticky verdict must be swept");
}

#[test]
fn get_honours_ttl_inclusively_and_absent_is_unknown() {
    let store = VerdictStore::new(Duration::from_secs(60));
    assert_eq!(store.get("none", T0).verdict, Verdict::Unknown);
    store.set("s", deny(T0));
    assert_eq!(
        store.get("s", T0 + Duration::from_secs(60)).verdict,
        Verdict::Deny
    );
    assert_eq!(
        store.get("s", T0 + Duration::from_secs(61)).verdict,
        Verdict::Unknown,
        "stale reads as unknown"
    );
    store.gc(T0 + Duration::from_secs(60));
    assert_eq!(store.len(), 1, "TTL equality survives GC like Go");
}

#[test]
fn saturation_stops_tracking_new_sessions_but_updates_existing() {
    let store = VerdictStore::with_capacity(Duration::from_secs(60), 2);
    assert!(store.set("a", deny(T0)));
    assert!(store.set("b", deny(T0)));
    assert!(!store.set("c", deny(T0)), "new session not tracked at cap");
    assert_eq!(store.get("c", T0).verdict, Verdict::Unknown);
    let allow = Sticky {
        verdict: Verdict::Allow,
        risk: 12.5,
        rule: String::new(),
        top: vec!["l1.navigator.webdriver".into()],
        updated: T0,
    };
    assert!(
        store.set("a", allow.clone()),
        "existing session still updates"
    );
    assert_eq!(store.get("a", T0), allow);
    store.gc(T0 + Duration::from_secs(61));
    assert!(
        store.set("c", deny(T0 + Duration::from_secs(61))),
        "GC frees capacity"
    );
}

#[test]
fn engine_verdicts_map_like_go() {
    assert_eq!(Verdict::from_engine("DENY"), Verdict::Deny);
    assert_eq!(Verdict::from_engine("CHALLENGE"), Verdict::Challenge);
    assert_eq!(Verdict::from_engine("ALLOW"), Verdict::Allow);
    for other in ["", "allow", "BLOCK"] {
        assert_eq!(Verdict::from_engine(other), Verdict::Unknown);
    }
    assert_eq!(
        [
            Verdict::Allow,
            Verdict::Challenge,
            Verdict::Deny,
            Verdict::Unknown
        ]
        .map(Verdict::as_str),
        ["allow", "challenge", "deny", "none"]
    );
}

#[test]
fn clock_regression_does_not_expire_entries() {
    let store = VerdictStore::new(Duration::from_secs(60));
    store.set("s", deny(T0));
    assert_eq!(
        store.get("s", T0 - Duration::from_secs(5)).verdict,
        Verdict::Deny
    );
}
