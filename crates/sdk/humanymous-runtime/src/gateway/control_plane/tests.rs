//! Control-plane kernel tests: the deliberate differences from Go and the
//! collaborator contract. Byte parity with the oracle is in [`golden`].
use super::*;
use crate::gateway::verdict_store::Verdict;
use humanymous_core::signals::report::Contributor;

mod golden;

pub(super) const KEY: &[u8; 32] = b"0123456789abcdef0123456789abcdef";
/// Unix 1800000000, the oracle capture clock.
pub(super) const UNIX: u64 = 1_800_000_000;
/// Seconds from the Go zero time (year 1) to the Unix epoch.
const ZERO_TO_UNIX: u64 = 62_135_596_800;

pub(super) fn now(mono_ms: u64) -> Now {
    now_at(UNIX, mono_ms)
}

pub(super) fn now_at(unix: u64, mono_ms: u64) -> Now {
    Now {
        wall: Timestamp(Timestamp::default().0 + Duration::from_secs(ZERO_TO_UNIX + unix)),
        mono: Duration::from_millis(mono_ms),
    }
}

/// Deterministic collaborators.
#[derive(Default)]
pub(super) struct Fake {
    pub sids: Vec<String>,
    pub network: NetworkReport,
    pub result: Option<ScoreResult>,
    pub scored: usize,
    pub links: Vec<(String, String)>,
}

impl Backend for Fake {
    fn new_session_id(&mut self) -> Result<String, Unavailable> {
        self.sids.pop().ok_or(Unavailable::Budget)
    }
    fn network(&mut self, _request: &Request<'_>) -> NetworkReport {
        self.network.clone()
    }
    fn score(&mut self, _report: &mut SessionReport) -> Option<ScoreResult> {
        self.scored += 1;
        self.result.clone()
    }
    fn pseudonymize(&mut self, sid: &str, value: &str) -> String {
        format!("psn:{sid}:{value}")
    }
    fn link(&mut self, handle: &str, sid: &str) {
        self.links.push((handle.into(), sid.into()));
    }
}

pub(super) fn plane(fake: Fake) -> ControlPlane<Fake> {
    ControlPlane::new(
        fake,
        CollectorStore::new(Duration::from_secs(3600)),
        VerdictStore::new(Duration::from_secs(3600)),
    )
    .unwrap()
}

pub(super) fn keyed(fake: Fake) -> ControlPlane<Fake> {
    plane(fake).with_token_key(TokenKey::new(KEY.to_vec()).unwrap())
}

pub(super) fn request<'a>(method: &'a str, path: &'a str) -> Request<'a> {
    Request {
        method,
        path,
        label: None,
        peer: "127.0.0.1".parse().unwrap(),
        session: None,
        user_agent: "Mozilla/5.0 capture",
        accept_language: "en-US",
        sec_ch_ua: "",
        nonce: "",
        receipt: "",
        body: b"",
    }
}

fn result(verdict: &str, risk: f64) -> ScoreResult {
    ScoreResult {
        verdict: verdict.into(),
        risk_score: risk,
        top_contributors: Some(vec![Contributor {
            id: "l1.navigator.webdriver".into(),
            ..Contributor::default()
        }]),
        ..ScoreResult::default()
    }
}

fn collect<'a>(sid: &'a str, body: &'a [u8]) -> Request<'a> {
    Request {
        session: Some(sid),
        body,
        ..request("POST", "/collect")
    }
}

const REPORT: &[u8] = br#"{"userAgent":"Mozilla/5.0 capture","signals":[],"behavior":{}}"#;

#[test]
fn loader_serves_only_the_supplied_collector_bundle() {
    let mut cp = plane(Fake::default());
    let absent = cp.handle(&request("GET", "/loader.js"), now(0));
    assert_eq!(
        (absent.status, absent.body.as_slice()),
        (404, &b"404 page not found\n"[..])
    );
    let mut cp = cp.with_loader(b"wasm-bundle-bytes".to_vec());
    let served = cp.handle(&request("GET", "/loader.js"), now(1));
    assert_eq!(served.status, 200);
    assert_eq!(served.body, b"wasm-bundle-bytes");
    assert_eq!(
        served.header("Content-Type"),
        Some("application/javascript; charset=utf-8")
    );
    assert_eq!(served.header("Cache-Control"), Some("no-store"));
    assert!(served.records.is_empty());
}

#[test]
fn non_canonical_paths_are_not_redirected() {
    let mut cp = plane(Fake::default());
    for path in [
        "/./session",
        "//collect",
        "/session/",
        "/collect/..",
        "",
        "session",
    ] {
        let response = cp.handle(&request("GET", path), now(0));
        assert_eq!(response.status, 404, "{path:?}");
        assert!(response.header("Location").is_none());
    }
    assert!(cp.store().get("anything").is_none());
}

#[test]
fn scoring_failure_fails_closed_without_sticky_or_token() {
    let mut cp = keyed(Fake::default());
    let response = cp.handle(&collect("sid-a", REPORT), now(0));
    assert_eq!(response.status, 503);
    assert_eq!(response.body, b"scoring unavailable\n");
    assert!(response.set_cookies().is_empty());
    assert!(response.records.is_empty());
    assert_eq!(cp.backend().scored, 1);
    assert_eq!(
        cp.verdicts().get("sid-a", Duration::ZERO).verdict,
        Verdict::Unknown
    );
}

#[test]
fn body_decoding_is_strict_where_go_was_lenient() {
    let fake = Fake {
        result: Some(result("CHALLENGE", 50.0)),
        ..Fake::default()
    };
    let mut cp = plane(fake);
    // Go: last duplicate wins, invalid UTF-8 is replaced. Rust refuses both.
    for body in [
        &br#"{"userAgent":"a","userAgent":"b"}"#[..],
        b"{\"userAgent\":\"\xff\"}",
    ] {
        let response = cp.handle(&collect("sid-a", body), now(0));
        assert_eq!(
            (response.status, response.body.as_slice()),
            (400, &b"bad body\n"[..])
        );
    }
    // A JSON null is an empty report, as in Go.
    let response = cp.handle(&collect("sid-a", b" null "), now(1));
    assert_eq!(response.status, 200);
    let stored = cp.store().get("sid-a").unwrap();
    assert_eq!(stored.client.session_id, "sid-a");
    // A differently cased key is ignored, not matched.
    cp.handle(&collect("sid-b", br#"{"UserAgent":"x"}"#), now(2));
    assert_eq!(cp.store().get("sid-b").unwrap().client.user_agent, "");
    // Oversize is a read error even when the session cookie was minted.
    let mut cp = plane(Fake {
        sids: vec!["0123456789abcdef0123456789abcdef".into()],
        ..Fake::default()
    });
    let big = vec![b' '; MAX_BEACON + 1];
    let response = cp.handle(
        &Request {
            body: &big,
            ..request("POST", "/collect")
        },
        now(3),
    );
    assert_eq!(
        (response.status, response.body.as_slice()),
        (400, &b"read error\n"[..])
    );
    assert_eq!(
        response.set_cookies(),
        ["hsid=0123456789abcdef0123456789abcdef; Path=/; HttpOnly; Secure; SameSite=Lax"]
    );
    let exact = vec![b' '; MAX_BEACON];
    let response = cp.handle(&collect("sid-c", &exact), now(4));
    assert_eq!(
        (response.status, response.body.as_slice()),
        (400, &b"bad body\n"[..])
    );
}

#[test]
fn clock_and_session_source_failures_fail_closed() {
    let fake = Fake {
        result: Some(result("ALLOW", 1.0)),
        ..Fake::default()
    };
    let mut cp = keyed(fake);
    let before_epoch = Now {
        wall: Timestamp::default(),
        mono: Duration::ZERO,
    };
    let response = cp.handle(&collect("sid-a", REPORT), before_epoch);
    assert_eq!(response.status, 503);
    assert_eq!(cp.backend().scored, 0);
    let stepup = cp.handle(
        &Request {
            session: Some("sid-a"),
            ..request("POST", "/stepup")
        },
        before_epoch,
    );
    assert_eq!(stepup.status, 503);
    assert!(stepup.records.is_empty());

    // No randomness, or a malformed id: no session is invented.
    for sids in [
        vec![],
        vec!["not-hex".to_owned()],
        vec!["ABCDEF0123456789ABCDEF0123456789".to_owned()],
    ] {
        let mut cp = plane(Fake {
            sids,
            ..Fake::default()
        });
        let response = cp.handle(&request("GET", "/session"), now(0));
        assert_eq!(
            (response.status, response.body.as_slice()),
            (503, &b"session unavailable\n"[..])
        );
        assert!(response.set_cookies().is_empty());
    }
}

#[test]
fn verdict_token_round_trips_and_is_only_issued_on_allow() {
    for (verdict, issued) in [("ALLOW", true), ("CHALLENGE", false), ("DENY", false)] {
        let mut cp = keyed(Fake {
            result: Some(result(verdict, 10.0)),
            ..Fake::default()
        });
        let response = cp.handle(&collect("sid-a", REPORT), now(0));
        assert_eq!(response.status, 200);
        let cookies = response.set_cookies();
        assert_eq!(cookies.len(), usize::from(issued), "{verdict}");
        if issued {
            let value = cookies[0]
                .strip_prefix("hmn_vt=")
                .and_then(|rest| rest.split(';').next())
                .unwrap();
            let bind = token_bind(&collect("sid-a", REPORT));
            let key = TokenKey::new(KEY.to_vec()).unwrap();
            key.verify_verdict(value, &bind, "sid-a", UNIX, &["e1".into()])
                .unwrap();
            assert!(
                key.verify_verdict(value, &bind, "sid-b", UNIX, &["e1".into()])
                    .is_err()
            );
            assert_eq!(response.records[0].event_type, EVENT_TOKEN_ISSUED);
        }
    }
    // Without a key nothing is issued, even on ALLOW.
    let mut cp = plane(Fake {
        result: Some(result("ALLOW", 10.0)),
        ..Fake::default()
    });
    let response = cp.handle(&collect("sid-a", REPORT), now(0));
    assert!(response.set_cookies().is_empty());
    assert_eq!(response.records[0].event_type, "scoring.evaluated");
    // A session id that cannot be sealed gets no token; scoring still answers.
    let mut cp = keyed(Fake {
        result: Some(result("ALLOW", 10.0)),
        ..Fake::default()
    });
    let response = cp.handle(&collect("a|b", REPORT), now(0));
    assert_eq!(response.status, 200);
    assert!(response.set_cookies().is_empty());
    assert_eq!(
        cp.verdicts().get("a|b", Duration::ZERO).verdict,
        Verdict::Allow
    );
}

#[test]
fn shared_epochs_replace_the_fallback_epoch() {
    let mut cp = keyed(Fake {
        result: Some(result("ALLOW", 10.0)),
        ..Fake::default()
    })
    .with_epochs(Epochs::default());
    let response = cp.handle(&collect("sid-a", REPORT), now(0));
    let value = response.set_cookies()[0]
        .strip_prefix("hmn_vt=")
        .and_then(|rest| rest.split(';').next())
        .unwrap()
        .to_owned();
    let key = TokenKey::new(KEY.to_vec()).unwrap();
    let bind = token_bind(&collect("sid-a", REPORT));
    let epochs = Epochs::default().accepted(UNIX);
    key.verify_verdict(&value, &bind, "sid-a", UNIX, &epochs)
        .unwrap();
    assert!(
        key.verify_verdict(&value, &bind, "sid-a", UNIX, &["e1".into()])
            .is_err()
    );
}

#[test]
fn step_up_proof_binds_to_the_presenting_client() {
    let key = TokenKey::new(KEY.to_vec()).unwrap();
    let receipt = key.issue_receipt("sid-a", UNIX + 60).unwrap();
    let mut cp = keyed(Fake::default());
    let presented = Request {
        session: Some("sid-a"),
        receipt: &receipt,
        peer: "198.51.100.20".parse().unwrap(),
        ..request("POST", "/stepup")
    };
    let response = cp.handle(&presented, now(0));
    assert_eq!(response.status, 200);
    let proof = response.set_cookies()[0]
        .strip_prefix("hmn_su=")
        .and_then(|rest| rest.split(';').next())
        .unwrap()
        .to_owned();
    let bind = token_bind(&presented);
    key.verify_step_up(&proof, &bind, "sid-a", UNIX, &["e1".into()])
        .unwrap();
    let elsewhere = token_bind(&Request {
        peer: "203.0.113.20".parse().unwrap(),
        ..presented
    });
    assert!(
        key.verify_step_up(&proof, &elsewhere, "sid-a", UNIX, &["e1".into()])
            .is_err()
    );
    // The step-up proof never verifies as a verdict token.
    assert!(
        key.verify_verdict(&proof, &bind, "sid-a", UNIX, &["e1".into()])
            .is_err()
    );
    // A receipt is session-bound; the proof grants nothing for an empty session.
    let empty = cp.handle(
        &Request {
            session: Some(""),
            ..presented
        },
        now(1),
    );
    assert_eq!(
        (empty.status, empty.body.as_slice()),
        (400, &b"no session\n"[..])
    );
}

#[test]
fn cohort_shadow_is_measured_but_never_changes_the_answer() {
    let config =
        crate::network::history::cohort::Config::new(2, Duration::from_secs(60), 8, 8).unwrap();
    let body = br#"{"behavior":{"mouse":{"samples":10,"meanCurvature":0.2}}}"#;
    let fake = || Fake {
        result: Some(result("CHALLENGE", 40.0)),
        ..Fake::default()
    };
    let mut shadowed = plane(fake()).with_cohort(Observer::new(config));
    let mut plain = plane(fake());
    let first = shadowed.handle(&collect("sid-a", body), now(0));
    let second = shadowed.handle(&collect("sid-b", body), now(1));
    assert_eq!(first.cohort, Some(Cohort::Counted(1)));
    assert_eq!(second.cohort, Some(Cohort::Flagged(2)));
    let unshadowed = plain.handle(&collect("sid-b", body), now(1));
    assert_eq!(unshadowed.cohort, None);
    assert_eq!(second.body, unshadowed.body);
    assert_eq!(second.records, unshadowed.records);
    assert_eq!(shadowed.gc(Duration::from_secs(120)).unwrap(), 1);
}

#[test]
fn scoring_inputs_and_links_follow_go() {
    let mut cp = plane(Fake {
        result: Some(result("DENY", 99.9)),
        ..Fake::default()
    });
    let labelled = Request {
        label: Some("bot-lab"),
        ..collect("sid-a", REPORT)
    };
    let response = cp.handle(&labelled, now(0));
    assert_eq!(response.status, 200);
    let stored = cp.store().get("sid-a").unwrap();
    assert_eq!(stored.label, "bot-lab");
    assert_eq!(stored.client.user_agent, "Mozilla/5.0 capture");
    assert_eq!(
        cp.backend().links,
        [("psn:sid-a:sid-a".to_owned(), "sid-a".to_owned())]
    );
    let record = &response.records[0];
    assert_eq!(record.actor.id_pseudonym, "psn:sid-a:127.0.0.1");
    assert_eq!(record.session_pseudonym, "psn:sid-a:sid-a");
    assert_eq!(record.risk_score, 99);
    // An empty label leaves the stored label alone.
    cp.handle(
        &Request {
            label: Some(""),
            ..collect("sid-a", REPORT)
        },
        now(1),
    );
    assert_eq!(cp.store().get("sid-a").unwrap().label, "bot-lab");
}
