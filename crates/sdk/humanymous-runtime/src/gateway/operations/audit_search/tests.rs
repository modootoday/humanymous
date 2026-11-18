//! Port of legacy internal/gate/ws9_test.go TestAuditSearchAndPagination
//! and internal/gate/prodhardening_test.go TestAuditSubjectFilter (baseline
//! 981d25c) at the search kernel, plus filter and cursor edges.
use super::*;
use crate::audit::record::Actor;

fn record(seq: u64, verdict: &str, host: &str, risk: i64, rules: &[&str]) -> Record {
    Record {
        seq,
        verdict: verdict.into(),
        host: host.into(),
        risk_score: risk,
        triggered_rules: (!rules.is_empty()).then(|| rules.iter().map(|r| (*r).into()).collect()),
        ..Record::default()
    }
}

fn seeded() -> Vec<Record> {
    let mut out = Vec::new();
    for i in 0..6 {
        out.push(record(2 * i + 1, "deny", "checkout.acme", 80, &["HR-7"]));
        out.push(record(2 * i + 2, "allow", "blog.acme", 5, &[]));
    }
    out
}

fn handle(seq: u64) -> String {
    format!("INC-{seq}")
}

#[test]
fn audit_search_and_pagination() {
    let records = seeded();
    let filtered = Query::from_pairs([
        ("verdict", "deny"),
        ("host", "checkout"),
        ("minRisk", "50"),
        ("rule", "HR-7"),
    ]);
    let page = search(&records, &filtered, handle);
    assert!(page.count > 0, "filtered query returned nothing");
    for hit in &page.records {
        assert_eq!(hit.record.verdict, "deny");
        assert!(hit.record.host.contains("checkout"));
    }

    let first = search(&records, &Query::from_pairs([("limit", "3")]), handle);
    assert_eq!(first.count, 3);
    assert_ne!(first.next_before, 0);
    let before = first.next_before.to_string();
    let second = search(
        &records,
        &Query::from_pairs([("limit", "3"), ("before", before.as_str())]),
        handle,
    );
    assert_eq!(second.count, 3);
    assert!(
        second
            .records
            .iter()
            .all(|hit| hit.record.seq < first.next_before),
        "the next page is older"
    );
}

#[test]
fn newest_first_cursor_and_incident_handles() {
    let records = seeded();
    let page = search(&records, &Query::from_pairs([("limit", "2")]), handle);
    let seqs: Vec<u64> = page.records.iter().map(|hit| hit.record.seq).collect();
    assert_eq!(seqs, [12, 11]);
    assert_eq!(page.next_before, 11);
    assert_eq!(page.records[0].incident, "INC-12");
    let empty = search(
        &records,
        &Query::from_pairs([("verdict", "challenge")]),
        handle,
    );
    assert_eq!((empty.count, empty.next_before), (0, 0));
}

#[test]
fn limit_and_numeric_fallbacks_match_legacy() {
    for raw in ["0", "-1", "501", "x", ""] {
        assert_eq!(
            Query::from_pairs([("limit", raw)]).limit,
            DEFAULT_LIMIT,
            "{raw}"
        );
    }
    assert_eq!(Query::from_pairs([("limit", "500")]).limit, 500);
    let query = Query::from_pairs([("minRisk", "high"), ("before", "-3")]);
    assert_eq!((query.min_risk, query.before), (0, 0));
}

#[test]
fn host_is_case_insensitive_and_route_rule_subject_are_exact() {
    let mut a = record(1, "deny", "Checkout.ACME", 90, &["HR-7", "HR-2"]);
    a.route_class = "checkout".into();
    a.session_pseudonym = "psn-s".into();
    let mut b = record(2, "deny", "checkout.acme", 90, &["HR-70"]);
    b.route_class = "checkout-v2".into();
    b.actor = Actor {
        id_pseudonym: "psn-a".into(),
        ..Actor::default()
    };
    let records = [a, b];
    let seqs = |pairs: &[(&str, &str)]| -> Vec<u64> {
        search(&records, &Query::from_pairs(pairs.iter().copied()), handle)
            .records
            .iter()
            .map(|hit| hit.record.seq)
            .collect()
    };
    assert_eq!(seqs(&[("host", "CHECKOUT")]), [2, 1]);
    assert_eq!(seqs(&[("route", "checkout")]), [1]);
    assert_eq!(seqs(&[("rule", "HR-7")]), [1]);
    assert_eq!(seqs(&[("subject", "psn-s")]), [1]);
    assert_eq!(seqs(&[("subject", "psn-a")]), [2]);
    assert_eq!(seqs(&[("minRisk", "91")]), Vec::<u64>::new());
}

/// Go TestAuditSubjectFilter (prodhardening_test.go): a GDPR Art. 15 access
/// query scoped to one pseudonymous subject returns its session-pseudonym and
/// actor-pseudonym records and excludes every other subject.
#[test]
fn audit_subject_filter() {
    let decision = |seq: u64, verdict: &str| Record {
        seq,
        event_type: "gate.decision".into(),
        verdict: verdict.into(),
        ..Record::default()
    };
    let mut by_session = decision(1, "allow");
    by_session.session_pseudonym = "subjA".into();
    let mut other = decision(2, "deny");
    other.session_pseudonym = "subjB".into();
    let mut by_actor = decision(3, "challenge");
    by_actor.actor = Actor {
        kind: "operator".into(),
        id_pseudonym: "subjA".into(),
    };
    let records = [by_session, other, by_actor];
    let page = search(&records, &Query::from_pairs([("subject", "subjA")]), handle);
    let body = serde_json::to_string(
        &page
            .records
            .iter()
            .map(|hit| &hit.record)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert!(body.contains("subjA"), "{body}");
    assert!(!body.contains("subjB"), "{body}");
    assert!(body.matches("subjA").count() >= 2, "{body}");
    assert_eq!(page.count, 2);
    let seqs: Vec<u64> = page.records.iter().map(|hit| hit.record.seq).collect();
    assert_eq!(seqs, [3, 1]);
}
