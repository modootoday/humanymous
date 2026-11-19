//! Port of legacy internal/gate/ws7_test.go (TestControlPlaneFloodMetered) at
//! baseline 981d25c, plus the Go defaults and the bounded fail-closed paths.
use super::*;

fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}

/// Go `WithControlLimiter(time.Minute, 3, 5)`.
fn tiny() -> Guard {
    Guard::with_thresholds(Duration::from_secs(60), 3, 5, default_limits().unwrap()).unwrap()
}

/// Go TestControlPlaneFloodMetered: twelve collect requests from one source
/// are dropped near the hard threshold, the flood yields exactly one sampled
/// breach record, and an unrelated source is not collaterally limited.
#[test]
fn control_plane_flood_metered() {
    let mut guard = tiny();
    let flood = ip("203.0.113.50");
    let mut blocked_at = None;
    let mut breaches = Vec::new();
    for i in 0..12_u64 {
        let decision = guard.admit(flood, "collect", Duration::from_millis(i));
        if let Decision::Drop { breach } = decision {
            blocked_at.get_or_insert(i);
            breaches.extend(breach);
        }
    }
    let blocked_at = blocked_at.expect("control-plane flood was never rate-limited");
    assert!(
        blocked_at <= 6,
        "flood should be dropped near the hard threshold (5), got first drop at {blocked_at}"
    );
    // Exactly at the hard cutpoint, as in Go (count 5 is the fifth request).
    assert_eq!(blocked_at, 4);
    assert_eq!(breaches.len(), 1, "flood must be sampled, not chained");
    let record = &breaches[0];
    assert_eq!(record.event_type, EVENT);
    assert_eq!(record.actor.kind, "system");
    assert_eq!(record.tenant_id, "control");
    assert_eq!(record.route_class, "control");
    // Go `string(VerdictDeny)`.
    assert_eq!(record.verdict, "deny");
    assert_eq!(record.enforcement_action, "rate_limit");
    assert_eq!(record.enforcement_mode, "enforce");
    assert_eq!(record.fail_reason, "control-plane flood: collect");
    assert_eq!(record.key_id, "k1");

    assert_eq!(
        guard.admit(ip("198.51.100.9"), "collect", Duration::from_millis(12)),
        Decision::Pass,
        "an unrelated IP was collaterally rate-limited"
    );
}

/// Go `NewControlPlane` defaults: 150 soft and 300 hard per 10 s; the 300th
/// request is dropped and the 301st carries the one breach record.
#[test]
fn defaults_match_go_and_breach_is_sampled_at_hard_plus_one() {
    assert_eq!((WINDOW, SOFT, HARD), (Duration::from_secs(10), 150, 300));
    assert_eq!(
        (STATUS, RETRY_AFTER, BODY),
        (429, "10", "control plane rate limited\n")
    );
    let mut guard = Guard::new().unwrap();
    let source = ip("2001:db8::7");
    for n in 1..HARD {
        assert_eq!(
            guard.admit(source, "session", Duration::from_millis(n as u64)),
            Decision::Pass,
            "request {n}"
        );
    }
    let at = |n: usize| Duration::from_millis(n as u64);
    assert_eq!(
        guard.admit(source, "session", at(HARD)),
        Decision::Drop { breach: None }
    );
    let Decision::Drop {
        breach: Some(record),
    } = guard.admit(source, "session", at(HARD + 1))
    else {
        panic!("hard plus one must carry the sampled breach");
    };
    assert_eq!(record.fail_reason, "control-plane flood: session");
    assert_eq!(
        guard.admit(source, "session", at(HARD + 2)),
        Decision::Drop { breach: None }
    );
}

/// The window is rolling: once the dropped burst ages out the source is served
/// again, and a later flood is sampled again.
#[test]
fn source_reopens_after_the_window_and_resamples() {
    let mut guard = tiny();
    let source = ip("203.0.113.51");
    let mut records = 0;
    for i in 0..8_u64 {
        if let Decision::Drop { breach: Some(_) } =
            guard.admit(source, "collect", Duration::from_secs(i))
        {
            records += 1;
        }
    }
    assert_eq!(records, 1);
    let later = Duration::from_secs(200);
    assert_eq!(guard.admit(source, "collect", later), Decision::Pass);
    for i in 1..6_u64 {
        if let Decision::Drop { breach: Some(_) } =
            guard.admit(source, "collect", later + Duration::from_millis(i))
        {
            records += 1;
        }
    }
    assert_eq!(records, 2);
}

/// Deliberate difference: limiter refusals fail closed as drops without a
/// record (clock regression, a full source table, the per-source hit bound).
#[test]
fn bounded_refusals_fail_closed_without_records() {
    let mut guard = tiny();
    assert_eq!(
        guard.admit(ip("192.0.2.1"), "collect", Duration::from_secs(5)),
        Decision::Pass
    );
    assert_eq!(
        guard.admit(ip("192.0.2.2"), "collect", Duration::from_secs(4)),
        Decision::Drop { breach: None }
    );

    let limits = Limits::new(1, 8, 64, Duration::from_secs(60), 1 << 20, 16).unwrap();
    let mut guard = Guard::with_thresholds(Duration::from_secs(60), 3, 5, limits).unwrap();
    assert_eq!(
        guard.admit(ip("192.0.2.1"), "collect", Duration::ZERO),
        Decision::Pass
    );
    assert_eq!(
        guard.admit(ip("192.0.2.2"), "collect", Duration::ZERO),
        Decision::Drop { breach: None }
    );
    let mut sampled = 0;
    for i in 0..20_u64 {
        match guard.admit(ip("192.0.2.1"), "collect", Duration::from_millis(i)) {
            Decision::Drop { breach: Some(_) } => sampled += 1,
            Decision::Pass | Decision::Drop { breach: None } => {}
        }
    }
    // The eighth hit fills the per-source bound; the rest still drop.
    assert_eq!(sampled, 1);
    assert!(matches!(
        guard.admit(ip("192.0.2.1"), "collect", Duration::from_millis(30)),
        Decision::Drop { breach: None }
    ));
}

/// Invalid thresholds are refused at construction, and idle sources are
/// evicted by the bounded GC.
#[test]
fn invalid_thresholds_refused_and_idle_sources_evicted() {
    let limits = default_limits().unwrap();
    assert!(Guard::with_thresholds(Duration::ZERO, 3, 5, limits).is_err());
    assert!(Guard::with_thresholds(Duration::from_secs(60), 0, 5, limits).is_err());
    assert!(Guard::with_thresholds(Duration::from_secs(60), 6, 5, limits).is_err());
    let mut guard = tiny();
    assert_eq!(
        guard.admit(ip("192.0.2.9"), "collect", Duration::ZERO),
        Decision::Pass
    );
    let swept = guard.gc(Duration::from_secs(60 * 6 + 1)).unwrap();
    assert_eq!((swept.scanned, swept.removed), (1, 1));
}
