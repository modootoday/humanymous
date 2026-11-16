//! Classification cases from legacy internal/gate smuggle_test and the HR-23 /
//! HR-27b record shapes (baseline 981d25c).
use super::*;
use crate::gateway::guard::spoof_scan;

type Pair = (&'static str, &'static str);

fn h(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn scan_classifies_in_legacy_precedence() {
    let cases: [(&[Pair], Option<Reason>); 9] = [
        (&[("Content-Length", "5")], None),
        (&[("Transfer-Encoding", "chunked")], None),
        (&[("Transfer-Encoding", " Chunked ")], None),
        (
            &[("Content-Length", "5"), ("Transfer-Encoding", "chunked")],
            Some(Reason::TeClConflict),
        ),
        (
            &[("content-length", "5"), ("Content-Length", "5")],
            Some(Reason::AmbiguousLength),
        ),
        (
            &[("Content-Length", "5"), ("Content-Length", "6")],
            Some(Reason::AmbiguousLength),
        ),
        (&[("Transfer-Encoding", "gzip")], Some(Reason::TeNotChunked)),
        (
            &[
                ("Transfer-Encoding", "chunked"),
                ("Transfer-Encoding", "chunked"),
            ],
            Some(Reason::TeNotChunked),
        ),
        (&[("X-A", "a\r\nX-B: b")], Some(Reason::ObsFold)),
    ];
    for (headers, want) in cases {
        assert_eq!(scan(&h(headers)), want, "{headers:?}");
    }
}

#[test]
fn smuggle_record_matches_legacy_shape() {
    let r = smuggle_record("node-1", "psn", Reason::TeClConflict);
    assert_eq!(r.event_type, "net.smuggle");
    assert_eq!(r.tenant_id, "node-1");
    assert_eq!(r.session_pseudonym, "psn");
    assert_eq!(r.verdict, "DENY");
    assert_eq!(r.triggered_rules, Some(vec!["HR-23".to_owned()]));
    assert_eq!(
        (r.enforcement_action.as_str(), r.enforcement_mode.as_str()),
        ("block", "enforce")
    );
    assert_eq!(r.fail_reason, "request framing ambiguity: te_cl_conflict");
}

#[test]
fn spoof_record_lists_only_static_header_names() {
    let found = spoof_scan(&h(&[
        ("X-Forwarded-For", "10.0.0.1 attacker-text"),
        ("X-Hmn-Verdict", "ALLOW"),
    ]));
    let r = spoof_record("node-1", "psn", &found);
    assert_eq!(r.event_type, "hdr.internal_header_ingress");
    assert_eq!(r.route_class, "html");
    assert_eq!(r.triggered_rules, Some(vec!["HR-27b".to_owned()]));
    assert_eq!(
        r.fail_reason,
        "inbound internal headers: X-Hmn-Verdict,X-Forwarded-For"
    );
    assert!(!r.fail_reason.contains("attacker-text"));
}

mod wargame;
