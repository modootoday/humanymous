//! The local stores satisfy the legacy seams (baseline 981d25c seams.go
//! compile-time assertions) and behave the same through a trait object.
use super::*;
use crate::network::history::rate::{Config, Limits};

const T0: Duration = Duration::from_secs(1_000_000);

fn limits() -> Limits {
    Limits::new(64, 64, 128, Duration::from_secs(60), 1 << 20, 64).unwrap()
}

fn ledger() -> Box<dyn BanLedger> {
    let rate = Config::new(Duration::from_secs(10), 3, 5, limits()).unwrap();
    Box::new(Store::new(rate, 16).unwrap())
}

#[test]
fn ban_store_enforces_through_the_seam() {
    let mut l = ledger();
    let key = Key::parse("ip:203.0.113.7").unwrap();
    assert_eq!(l.check(&key, T0), None);
    let mut banned = None;
    for i in 0..10 {
        if let Outcome::AutoBanned { entry, .. } =
            l.observe(&key, T0 + Duration::from_millis(i)).unwrap()
        {
            banned = Some(entry);
            break;
        }
    }
    let entry = banned.expect("a flood past hard auto-bans through the seam");
    assert_eq!(
        l.check(&key, T0 + Duration::from_millis(20)),
        Some(entry.clone())
    );
    assert_eq!(l.list(T0 + Duration::from_millis(20)), vec![entry]);
    assert_eq!(l.lift(&key), Ok(true));
    assert_eq!(l.check(&key, T0 + Duration::from_millis(30)), None);
}

#[test]
fn monitor_mode_never_bans_through_the_seam() {
    let mut l = ledger();
    let key = Key::parse("ip:203.0.113.8").unwrap();
    let mut last = Level::Normal;
    for i in 0..10 {
        last = l.observe_rate(&key, T0 + Duration::from_millis(i)).unwrap();
    }
    assert_eq!(last, Level::Hard);
    assert_eq!(l.check(&key, T0 + Duration::from_millis(20)), None);
    assert!(l.list(T0 + Duration::from_millis(20)).is_empty());
}

#[test]
fn temporary_ban_and_broad_key_refusal_through_the_seam() {
    let mut l = ledger();
    let narrow = Key::parse("ip:198.51.100.1").unwrap();
    let entry = l
        .add_temporary(&narrow, "manual", None, Duration::from_secs(60), T0)
        .unwrap();
    assert_eq!(l.check(&narrow, T0), Some(entry));
    let broad = Key::parse("cidr:198.51.100.0/24").unwrap();
    assert_eq!(
        l.add_temporary(&broad, "manual", None, Duration::from_secs(60), T0),
        Err(Refusal::DualControlRequired)
    );
}

#[test]
fn limiter_counts_classifies_and_reconfigures_through_the_seam() {
    let mut r: Box<dyn RateLimiter> = Box::new(Limiter::new(
        Config::new(Duration::from_secs(1), 5, 10, limits()).unwrap(),
    ));
    assert_eq!(r.observe("", T0), Ok(Observation::IgnoredEmptyKey));
    assert!(matches!(
        r.observe("fp", T0),
        Ok(Observation::Counted { count: 1, .. })
    ));
    assert_eq!((r.level(5), r.level(10)), (Level::Soft, Level::Hard));
    r.configure(Duration::from_secs(1), 2, 3, T0).unwrap();
    assert_eq!((r.level(2), r.level(3)), (Level::Soft, Level::Hard));
}

#[test]
fn verdict_store_serves_through_the_seam() {
    use crate::gateway::verdict_store::{Sticky, Verdict, VerdictStore};
    let ledger: Box<dyn VerdictLedger> = Box::new(VerdictStore::new(Duration::from_secs(60)));
    let t0 = Duration::from_secs(100);
    let deny = Sticky {
        verdict: Verdict::Deny,
        updated: t0,
        ..Sticky::default()
    };
    assert!(ledger.set("s", deny.clone()));
    assert_eq!(ledger.get("s", t0), deny);
    ledger.gc(t0 + Duration::from_secs(61));
    assert_eq!(ledger.get("s", t0).verdict, Verdict::Unknown);
}
