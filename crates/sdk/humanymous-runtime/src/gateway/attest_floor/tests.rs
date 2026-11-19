//! Ports of legacy internal/gate/wargame_t4_promotion_test.go (R346-R348) at
//! baseline 981d25c, using real verdict and step-up tokens, plus the
//! monitor/shadow/off modes of `gate.attest_floor`.
use super::*;
use crate::gateway::token::{MIN_KEY, bind_key, token_bind};

const NOW: u64 = 1_000_000;

fn key() -> TokenKey {
    let mut bytes = b"t4-attest-floor-key".to_vec();
    bytes.resize(MIN_KEY, b'.');
    TokenKey::new(bytes).unwrap()
}

fn bind(ua: &str, subnet: &str) -> String {
    token_bind(bind_key(ua, "", "").as_deref(), subnet)
}

fn request(
    attest_floor: bool,
    credentials: &Credentials<'_>,
    verdict: Option<&str>,
    step_up: Option<&str>,
    sticky_allow: bool,
) -> Request {
    Request {
        attest_floor,
        enforcing: true,
        verdict_token: credentials.verdict(verdict),
        sticky_allow,
        step_up: credentials.step_up(step_up),
    }
}

#[test]
fn wargame_r346_t4_allow_token_priced_on_attested_route() {
    let (key, epochs) = (key(), Epochs::default());
    let bind = bind("Chrome/126-coherent", "198.51.100.0/24");
    let sid = "t4-coherent-allow";
    let token = key
        .issue_verdict(sid, &bind, &epochs.current(NOW), NOW + 1800)
        .unwrap();
    let credentials = Credentials {
        key: &key,
        epochs: &epochs,
        bind: &bind,
        sid,
        now: NOW,
    };
    let browse = decide(
        request(false, &credentials, Some(&token), None, true),
        FloorMode::Enforce,
    );
    assert_eq!(
        browse.decision,
        Decision::Forward,
        "browse route may fast-path ALLOW"
    );
    let transfer = decide(
        request(true, &credentials, Some(&token), None, true),
        FloorMode::Enforce,
    );
    assert_eq!(
        transfer.decision,
        Decision::StepUpChallenge,
        "attested route must price ALLOW to step-up (401)"
    );
}

#[test]
fn wargame_r347_t4_step_up_unlocks_attested_after_proof() {
    let (key, epochs) = (key(), Epochs::default());
    let bind = bind("Chrome/126-coherent", "198.51.100.0/24");
    let sid = "t4-stepped";
    let proof = key
        .issue_step_up(sid, &bind, &epochs.current(NOW), NOW + 1800)
        .unwrap();
    let credentials = Credentials {
        key: &key,
        epochs: &epochs,
        bind: &bind,
        sid,
        now: NOW,
    };
    let outcome = decide(
        request(true, &credentials, None, Some(&proof), true),
        FloorMode::Enforce,
    );
    assert_eq!(outcome.decision, Decision::Forward);
    assert_eq!(outcome.would_require_step_up, None);
}

#[test]
fn wargame_r348_t4_sticky_allow_cannot_skip_floor() {
    let (key, epochs) = (key(), Epochs::default());
    let bind = bind("Chrome/126", "198.51.100.0/24");
    let credentials = Credentials {
        key: &key,
        epochs: &epochs,
        bind: &bind,
        sid: "t4-sticky",
        now: NOW,
    };
    let outcome = decide(
        request(true, &credentials, None, None, true),
        FloorMode::Enforce,
    );
    assert_eq!(outcome.decision, Decision::StepUpChallenge);
}

#[test]
fn foreign_or_verdict_proofs_do_not_unlock_the_floor() {
    let (key, epochs) = (key(), Epochs::default());
    let own = bind("Chrome/126", "198.51.100.0/24");
    let foreign_bind = bind("Chrome/126", "203.0.113.0/24");
    let sid = "t4-lifted";
    let epoch = epochs.current(NOW);
    let verdict_as_step_up = key.issue_verdict(sid, &own, &epoch, NOW + 60).unwrap();
    let other_session = key
        .issue_step_up("someone-else", &own, &epoch, NOW + 60)
        .unwrap();
    let other_bind = key
        .issue_step_up(sid, &foreign_bind, &epoch, NOW + 60)
        .unwrap();
    let expired = key.issue_step_up(sid, &own, &epoch, NOW - 1).unwrap();
    let credentials = Credentials {
        key: &key,
        epochs: &epochs,
        bind: &own,
        sid,
        now: NOW,
    };
    for proof in [
        verdict_as_step_up.as_str(),
        &other_session,
        &other_bind,
        &expired,
        "",
    ] {
        let outcome = decide(
            request(true, &credentials, None, Some(proof), true),
            FloorMode::Enforce,
        );
        assert_eq!(outcome.decision, Decision::StepUpChallenge, "{proof:?}");
    }
}

#[test]
fn monitor_and_shadow_forward_and_report_off_forwards_silently() {
    let token = Request {
        attest_floor: true,
        enforcing: true,
        verdict_token: true,
        sticky_allow: true,
        step_up: false,
    };
    let sticky = Request {
        verdict_token: false,
        ..token
    };
    for mode in [FloorMode::Monitor, FloorMode::Shadow] {
        assert_eq!(
            decide(token, mode),
            Outcome {
                decision: Decision::Forward,
                would_require_step_up: Some("verdict-token fast path would require step-up"),
            }
        );
        assert_eq!(
            decide(sticky, mode),
            Outcome {
                decision: Decision::Forward,
                would_require_step_up: Some("scoring allow would require step-up"),
            }
        );
    }
    for request in [token, sticky] {
        assert_eq!(
            decide(request, FloorMode::Off),
            Outcome {
                decision: Decision::Forward,
                would_require_step_up: None
            }
        );
    }
}

#[test]
fn non_allow_and_non_enforcing_routes_are_left_to_the_pipeline() {
    let unscored = Request {
        attest_floor: true,
        enforcing: true,
        verdict_token: false,
        sticky_allow: false,
        step_up: false,
    };
    assert_eq!(
        decide(unscored, FloorMode::Enforce).decision,
        Decision::Pipeline
    );
    let monitoring = Request {
        enforcing: false,
        sticky_allow: true,
        ..unscored
    };
    assert_eq!(
        decide(monitoring, FloorMode::Enforce).decision,
        Decision::Forward
    );
}

mod parity;
