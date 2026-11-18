//! Ports of legacy `internal/collector/collector_test.go` (baseline 981d25c)
//! plus session.go/merge.go behaviors the Go file left implicit.
use super::*;
use humanymous_core::signals::report::ScoreResult;

const HOUR: Duration = Duration::from_secs(3600);

fn at(seconds: u64) -> Timestamp {
    Timestamp(Timestamp::default().0 + Duration::from_secs(1_000_000 + seconds))
}

fn signal(id: &str) -> Signal {
    Signal {
        id: id.to_owned(),
        ..Signal::default()
    }
}

fn network(ja4: &str, ids: &[&str]) -> NetworkReport {
    NetworkReport {
        ja4: ja4.to_owned(),
        signals: (!ids.is_empty()).then(|| ids.iter().map(|id| signal(id)).collect()),
        ..NetworkReport::default()
    }
}

fn network_ids(store: &CollectorStore, id: &str) -> Vec<String> {
    store
        .get(id)
        .expect("session")
        .network
        .signals
        .unwrap_or_default()
        .into_iter()
        .map(|signal| signal.id)
        .collect()
}

/// Go TestStoreLifecycle.
#[test]
fn store_lifecycle() {
    let mut store = CollectorStore::new(HOUR);
    let now = at(0);
    store.ensure("sid", now);
    let report = store.get("sid").expect("ensure creates the session");
    assert_eq!(report.session_id, "sid");
    assert_eq!(report.timestamp, now);
    assert!(store.get("missing").is_none());
    assert_eq!(store.rit_counter("sid"), 0);
    assert_eq!(store.advance_rit("sid", now), 1);
    assert_eq!(store.rit_counter("sid"), 1);
    assert!(!store.pow_solved("sid"));
    store.set_pow_solved("sid", now);
    assert!(store.pow_solved("sid"));
}

/// Go TestStoreGCExpiry.
#[test]
fn store_gc_expiry() {
    let mut store = CollectorStore::new(Duration::from_secs(60));
    store.ensure("old", at(0));
    store.ensure("fresh", at(90));
    store.gc(at(120));
    assert!(
        store.get("old").is_none(),
        "expired session was not collected"
    );
    assert!(store.get("fresh").is_some(), "fresh session was collected");
    // Exactly the TTL is still live; only strictly older entries expire.
    store.gc(at(150));
    assert!(store.get("fresh").is_some());
    store.gc(at(151));
    assert!(store.get("fresh").is_none());
}

/// Go TestMergePinsFirstNetwork.
#[test]
fn merge_pins_first_network() {
    let mut store = CollectorStore::new(HOUR);
    let now = at(0);
    store.merge_network("sid", network("first", &[]), now);
    store.merge_network("sid", network("second", &[]), now);
    assert_eq!(store.get("sid").unwrap().network.ja4, "first");

    store.append_network_signals("sid", vec![signal("l5.tls.ja3")], now);
    store.append_network_signals("sid", vec![signal("l5.rit.absent")], now);
    assert_eq!(
        network_ids(&store, "sid"),
        ["l5.tls.ja3", "l5.rit.absent"],
        "appended signals"
    );
    assert_eq!(store.get("sid").unwrap().network.ja4, "first");
}

/// Go TestMergeNetwork_RequestScopedHopResiduals.
#[test]
fn merge_network_request_scoped_hop_residuals() {
    let mut store = CollectorStore::new(HOUR);
    let now = at(0);
    store.merge_network("sid", network("pinned", &["l5.tls.grease_absent"]), now);
    store.merge_network(
        "sid",
        network(
            "ignored",
            &[
                "l5.header.xff_multi_hop",
                "l5.header.proxy_hop",
                "l5.tls.grease_absent",
            ],
        ),
        now,
    );
    assert_eq!(store.get("sid").unwrap().network.ja4, "pinned");
    assert_eq!(
        network_ids(&store, "sid"),
        [
            "l5.tls.grease_absent",
            "l5.header.xff_multi_hop",
            "l5.header.proxy_hop"
        ]
    );
}

/// Every request-scoped residual ID refreshes after the pin; others never do.
#[test]
fn request_scoped_residual_set_is_exact() {
    let scoped = [
        "l5.header.proxy_hop",
        "l5.header.client_ip_spoof",
        "l5.header.xff_multi_hop",
        "l5.header.forwarded_private",
        "l5.proxy.tor_circuit",
        "l5.proxy.anon_chain",
        "l5.ip.proxy_vpn_tor",
        "l5.ip.tor_exit",
        "l5.ip.datacenter_asn",
    ];
    for id in scoped {
        assert!(request_scoped_network(id), "{id}");
    }
    for id in [
        "l5.tls.ja3",
        "l5.h2dos.rapid_reset",
        "l5.rit.absent",
        "l5.header",
    ] {
        assert!(!request_scoped_network(id), "{id}");
    }
    let mut store = CollectorStore::new(HOUR);
    store.merge_network("sid", network("pinned", &[]), at(0));
    store.merge_network("sid", network("later", &["l5.tls.ja3"]), at(1));
    assert!(store.get("sid").unwrap().network.signals.is_none());
}

/// Go TestMergeClientAndLabel.
#[test]
fn merge_client_and_label() {
    let mut store = CollectorStore::new(HOUR);
    let now = at(0);
    let client = ClientReport {
        user_agent: "Chrome".into(),
        ..ClientReport::default()
    };
    store.merge_client("sid", client, now);
    store.set_label("sid", "bot:selenium", now);
    let report = store.get("sid").unwrap();
    assert_eq!(report.client.user_agent, "Chrome");
    assert_eq!(report.label, "bot:selenium");
}

/// merge.go sanitizeClientSignals: forged server provenance is dropped.
#[test]
fn merge_client_drops_server_provenance_claims() {
    let mut store = CollectorStore::new(HOUR);
    let mut forged = signal("l2.adv.no_voices");
    forged.collected = Source::Server;
    let mut honest = signal("l2.adv.no_voices");
    honest.collected = Source::Js;
    let client = ClientReport {
        signals: Some(vec![
            forged,
            signal("l5.rit.ok"),
            signal("l6.ua_tls_mismatch"),
            signal("l7.pow.solved"),
            signal("x.cross"),
            honest.clone(),
        ]),
        ..ClientReport::default()
    };
    store.merge_client("sid", client, at(0));
    assert_eq!(store.get("sid").unwrap().client.signals, Some(vec![honest]));

    let all_forged = ClientReport {
        signals: Some(vec![signal("l7.pow.solved")]),
        ..ClientReport::default()
    };
    store.merge_client("sid", all_forged, at(1));
    assert_eq!(store.get("sid").unwrap().client.signals, Some(Vec::new()));
    store.merge_client("sid", ClientReport::default(), at(2));
    assert_eq!(store.get("sid").unwrap().client.signals, None);
}

/// StoreScored replaces the whole report; later merges keep the network pin.
#[test]
fn store_scored_replaces_report() {
    let mut store = CollectorStore::new(HOUR);
    store.merge_network("sid", network("pinned", &[]), at(0));
    let scored = SessionReport {
        session_id: "sid".into(),
        timestamp: at(5),
        label: "human".into(),
        scoring: ScoreResult {
            risk_score: 0.25,
            ..ScoreResult::default()
        },
        ..SessionReport::default()
    };
    store.store_scored("sid", scored.clone(), at(6));
    assert_eq!(store.get("sid").unwrap(), scored);
    store.merge_network("sid", network("again", &[]), at(7));
    assert_eq!(store.get("sid").unwrap().network.ja4, "");
}

/// RIT advance/bootstrap create bare sessions with a zero report timestamp.
#[test]
fn rit_bootstrap_is_one_shot() {
    let mut store = CollectorStore::new(HOUR);
    assert!(store.claim_rit_bootstrap("sid", at(0)));
    assert!(!store.claim_rit_bootstrap("sid", at(1)));
    assert!(!store.claim_rit_bootstrap("sid", at(2)));
    let report = store.get("sid").unwrap();
    assert_eq!(report.session_id, "sid");
    assert_eq!(report.timestamp, Timestamp::default());

    assert_eq!(store.advance_rit("new", at(3)), 1);
    assert_eq!(store.advance_rit("new", at(3)), 2);
    assert_eq!(store.get("new").unwrap().timestamp, Timestamp::default());
    assert!(store.claim_rit_bootstrap("new", at(4)));
    // Bootstrap and advance refresh activity for GC.
    store.gc(Timestamp(at(4).0 + HOUR));
    assert!(store.get("new").is_some());
    assert!(store.get("sid").is_none());
}

/// PoW issue is first-write-wins and cleared on solve; too-fast is permanent.
#[test]
fn pow_issue_solve_and_too_fast() {
    let mut store = CollectorStore::new(HOUR);
    store.set_pow_issued("ghost", at(0), 20);
    store.set_pow_too_fast("ghost");
    store.set_pow_solved("ghost", at(0));
    assert!(store.get("ghost").is_none(), "unknown sessions are ignored");
    assert_eq!(store.pow_difficulty("ghost"), 0);
    assert!(!store.pow_too_fast("ghost"));

    store.ensure("sid", at(0));
    assert_eq!(store.pow_issued_at("sid"), Timestamp::default());
    store.set_pow_issued("sid", at(10), 18);
    store.set_pow_issued("sid", at(20), 22);
    assert_eq!(store.pow_issued_at("sid"), at(10));
    assert_eq!(store.pow_difficulty("sid"), 18);

    store.set_pow_too_fast("sid");
    store.set_pow_solved("sid", at(30));
    assert!(store.pow_solved("sid"));
    assert!(store.pow_too_fast("sid"), "too-fast survives a solve");
    assert_eq!(store.pow_issued_at("sid"), Timestamp::default());
    assert_eq!(store.pow_difficulty("sid"), 0);

    store.set_pow_issued("sid", at(40), 21);
    assert_eq!(store.pow_issued_at("sid"), at(40));
    assert_eq!(store.pow_difficulty("sid"), 21);
}

/// List is bounded, and like Go returns one report for a zero limit.
#[test]
fn list_is_bounded() {
    let mut store = CollectorStore::new(HOUR);
    assert!(store.list(0).is_empty());
    assert!(store.list(5).is_empty());
    for id in ["a", "b", "c"] {
        store.ensure(id, at(0));
    }
    assert_eq!(store.list(2).len(), 2);
    assert_eq!(store.list(0).len(), 1);
    let mut ids: Vec<_> = store
        .list(10)
        .into_iter()
        .map(|report| report.session_id)
        .collect();
    ids.sort();
    assert_eq!(ids, ["a", "b", "c"]);
}
