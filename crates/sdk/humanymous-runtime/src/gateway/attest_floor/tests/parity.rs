//! Differential ports of legacy internal/gate/attestfloor_test.go at
//! baseline 981d25c against outputs recorded from the Go oracle
//! (`tests/fixtures/gateway/attest-floor-and-admin-chain-go-golden.json`).
//! Go statuses map onto the Rust decisions as: forward is 200 with the origin
//! contacted, a step-up challenge is 401 without it.
use super::super::exchange::{self, ReceiptReason, StepUpRequest};
use super::super::fanout::{self, CredentialFanout};
use super::super::*;
use crate::gateway::route_policy::refuse_attested_catch_all;
use crate::gateway::token::{bind_key, client_subnet, token_bind};
use crate::network::correlation::{self, Registry};
use crate::network::history::anomaly;
use serde_json::Value;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::LazyLock;
use std::time::Duration;

const NOW: u64 = 1_800_000_000;
/// Go `attestTokenKey`.
const GO_KEY: &[u8] = b"ceiling-guard-test-token-key-32b!";

static GOLDEN: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/gateway/attest-floor-and-admin-chain-go-golden.json"
    ))
    .unwrap()
});

/// (case, sid, request addr, request ua, attested, verdict token, step-up proof)
type FloorCase<'a> = (
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    bool,
    Option<&'a str>,
    Option<&'a str>,
);

fn key() -> TokenKey {
    TokenKey::new(GO_KEY.to_vec()).unwrap()
}

/// Go `tokenBind(aReq(method, path, addr, ua))`: UA fingerprint plus subnet.
fn bind(addr: &str, ua: &str) -> String {
    let ip = addr.parse::<SocketAddr>().unwrap().ip();
    token_bind(bind_key(ua, "", "").as_deref(), &client_subnet(ip))
}

fn rows(name: &str) -> &'static [Value] {
    GOLDEN[name].as_array().unwrap()
}

fn row<'a>(rows: &'a [Value], case: &str) -> &'a Value {
    rows.iter().find(|row| row["case"] == case).unwrap()
}

/// Go TestAttestFloorDemotesUnattestedAllow, TestAttestFloorStepUpProofPasses,
/// TestAttestFloorStickyAllowLaunderingClosed, TestStepUpProofNotLiftable and
/// TestStepUpDomainSeparation: every request of the Go floor scenarios, through
/// real tokens and `decide`, gives the Go status and origin contact.
#[test]
fn go_attest_floor_http_cases_match_capture() {
    let (key, epochs) = (key(), Epochs::default());
    let epoch = epochs.current(NOW);
    let exp = NOW + 1800;
    let human = bind("198.51.100.7:5555", "Chrome/126-real");
    let allow = key
        .issue_verdict("human-allow", &human, &epoch, exp)
        .unwrap();
    let stepped = bind("198.51.100.9:5555", "Chrome/126-real");
    let proof = key
        .issue_step_up("human-stepped", &stepped, &epoch, exp)
        .unwrap();
    let victim = bind("203.0.113.5:1", "Chrome/126-victim");
    let lifted = key.issue_step_up("victim", &victim, &epoch, exp).unwrap();
    let confused = bind("198.51.100.21:1", "Chrome/126");
    let verdict_as_proof = key
        .issue_verdict("confuse", &confused, &epoch, exp)
        .unwrap();
    let cases: [FloorCase<'_>; 6] = [
        (
            "allow_token_public_route",
            "human-allow",
            "198.51.100.7:5555",
            "Chrome/126-real",
            false,
            Some(&allow),
            None,
        ),
        (
            "allow_token_attested_route",
            "human-allow",
            "198.51.100.7:5555",
            "Chrome/126-real",
            true,
            Some(&allow),
            None,
        ),
        (
            "step_up_attested_route",
            "human-stepped",
            "198.51.100.9:5555",
            "Chrome/126-real",
            true,
            None,
            Some(&proof),
        ),
        (
            "sticky_allow_attested_route",
            "sticky-allow",
            "198.51.100.11:1",
            "Chrome/126",
            true,
            None,
            None,
        ),
        (
            "lifted_step_up_attested_route",
            "victim",
            "45.66.77.88:1",
            "HeadlessChrome/126",
            true,
            None,
            Some(&lifted),
        ),
        (
            "verdict_token_in_step_up_slot",
            "confuse",
            "198.51.100.21:1",
            "Chrome/126",
            true,
            None,
            Some(&verdict_as_proof),
        ),
    ];
    let golden = rows("attest_floor_http");
    assert_eq!(golden.len(), cases.len());
    for (case, sid, addr, ua, attested, verdict, step_up) in cases {
        let bound = bind(addr, ua);
        let credentials = Credentials {
            key: &key,
            epochs: &epochs,
            bind: &bound,
            sid,
            now: NOW,
        };
        let outcome = decide(
            Request {
                attest_floor: attested,
                enforcing: true,
                verdict_token: credentials.verdict(verdict),
                sticky_allow: true,
                step_up: credentials.step_up(step_up),
            },
            FloorMode::Enforce,
        );
        let (status, contacted) = match outcome.decision {
            Decision::Forward => (200, true),
            Decision::StepUpChallenge => (401, false),
            Decision::Pipeline => panic!("{case}: an ALLOW session never reaches the pipeline"),
        };
        let go = row(golden, case);
        assert_eq!(status, go["status"], "{case}");
        assert_eq!(contacted, go["origin_contacted"], "{case}");
    }
}

fn exchange_case(key: &TokenKey, case: &str) -> exchange::StepUpResponse {
    let other = TokenKey::new(b"a-totally-different-core-key!!!!".to_vec()).unwrap();
    let fresh = |key: &TokenKey, sid: &str| key.issue_receipt(sid, NOW + 120).unwrap();
    let (method, sid, receipt) = match case {
        "get_method" => ("GET", Some("s1"), Some(fresh(key, "s1"))),
        "no_session" => ("POST", None, Some(fresh(key, "s1"))),
        "missing_receipt" => ("POST", Some("s1"), None),
        "garbage_receipt" => ("POST", Some("s1"), Some("not-a-receipt".to_owned())),
        "key_mismatch" => ("POST", Some("s1"), Some(fresh(&other, "s1"))),
        "sid_divergence" => (
            "POST",
            Some("gate-side-sid"),
            Some(fresh(key, "core-side-sid")),
        ),
        "expired" => (
            "POST",
            Some("s1"),
            Some(key.issue_receipt("s1", NOW - 600).unwrap()),
        ),
        "verdict_token_as_receipt" => (
            "POST",
            Some("s1"),
            Some(key.issue_verdict("s1", "b", "e", NOW + 60).unwrap()),
        ),
        "valid" => ("POST", Some("s1"), Some(fresh(key, "s1"))),
        other => panic!("unknown case {other}"),
    };
    let bound = bind("198.51.100.31:1", "Chrome/126");
    let epoch = Epochs::default().current(NOW);
    exchange::redeem(
        Some(key),
        StepUpRequest {
            method,
            sid,
            receipt: receipt.as_deref(),
            bind: &bound,
            epoch: &epoch,
            now: NOW,
        },
    )
}

/// Go TestStepUpEndpointMintsProof, TestStepUpCrossPlaneKeyMismatch and
/// TestStepUpCrossPlaneSidDivergence: every `/stepup` exchange gives the Go
/// status, body, cookie and audit record; the minted proof then passes the
/// floor from the same binding only.
#[test]
fn go_step_up_exchange_matches_capture() {
    let key = key();
    for go in rows("step_up_exchange") {
        let case = go["case"].as_str().unwrap();
        let response = exchange_case(&key, case);
        assert_eq!(response.status, go["status"], "{case}");
        assert_eq!(response.body, go["body"], "{case}");
        match &go["step_up_cookie"] {
            Value::Null => assert!(response.proof.is_none(), "{case}: no cookie"),
            cookie => {
                let proof = response.proof.as_deref().unwrap();
                assert_eq!(
                    exchange::set_cookie(proof).replace(proof, "<value>"),
                    cookie["header"],
                    "{case}"
                );
            }
        }
        let records = go["records"].as_array().unwrap();
        assert_eq!(
            records.len(),
            usize::from(response.audit.is_some()),
            "{case}"
        );
        if let (Some(audit), Some(record)) = (response.audit, records.first()) {
            assert_eq!(audit.event_type, record["event_type"], "{case}");
            assert_eq!(audit.verdict, record["verdict"], "{case}");
            assert_eq!(audit.fail_reason, record["fail_reason"], "{case}");
            // Constant fields of both legacy records.
            assert_eq!(record["actor_kind"], "subject");
            assert_eq!(record["tenant_id"], "control");
            assert_eq!(record["route_class"], "control");
            assert_eq!(record["mode"], "enforce");
            assert_eq!(record["key_id"], "k1");
        }
    }
    // The minted proof passes the floor from the same binding, not another.
    let proof = exchange_case(&key, "valid").proof.unwrap();
    let epochs = Epochs::default();
    for (addr, ua, passes) in [
        ("198.51.100.31:1", "Chrome/126", true),
        ("45.66.77.88:1", "Chrome/126", false),
    ] {
        let bound = bind(addr, ua);
        let credentials = Credentials {
            key: &key,
            epochs: &epochs,
            bind: &bound,
            sid: "s1",
            now: NOW,
        };
        assert_eq!(credentials.step_up(Some(&proof)), passes, "{addr}");
    }
}

/// Go TestStepUpTokenVerify (receipt half) and the typed reason assertion of
/// TestStepUpCrossPlaneSidDivergence.
#[test]
fn go_receipt_reasons_match_capture() {
    let key = key();
    let receipt = key.issue_receipt("sidX", NOW + 60).unwrap();
    let golden = &GOLDEN["receipt_reasons"];
    let cases = [
        ("ok", Some(receipt.as_str()), "sidX", NOW),
        ("sid_mismatch", Some(&receipt), "sidOther", NOW),
        ("within_skew", Some(&receipt), "sidX", NOW + 90),
        ("past_skew", Some(&receipt), "sidX", NOW + 600),
        ("garbage", Some("garbage"), "sidX", NOW),
        ("empty_session", Some(&receipt), "", NOW),
    ];
    for (case, token, sid, now) in cases {
        assert_eq!(
            exchange::receipt_reason(&key, token, sid, now).as_str(),
            golden[case],
            "{case}"
        );
    }
    assert_eq!(
        exchange::receipt_reason(&key, None, "sidX", NOW),
        ReceiptReason::Bad
    );
}

/// Rust refuses to mint a proof it cannot encode (here an empty binding)
/// with 500 and no cookie; Go minted one anyway.
#[test]
fn unencodable_proof_is_refused_without_a_cookie() {
    let key = key();
    let receipt = key.issue_receipt("s1", NOW + 60).unwrap();
    let response = exchange::redeem(
        Some(&key),
        StepUpRequest {
            method: "POST",
            sid: Some("s1"),
            receipt: Some(&receipt),
            bind: "",
            epoch: "e1",
            now: NOW,
        },
    );
    assert_eq!(
        (response.status, response.proof, response.audit),
        (500, None, None)
    );
    let unconfigured = exchange::redeem(
        None,
        StepUpRequest {
            method: "POST",
            sid: Some("s1"),
            receipt: Some(&receipt),
            bind: "b",
            epoch: "e1",
            now: NOW,
        },
    );
    assert_eq!(
        (unconfigured.status, unconfigured.body),
        (501, "step-up not configured\n")
    );
}

fn registry(distinct: usize) -> Registry {
    Registry::new(
        correlation::Config::new(
            fanout::WINDOW,
            correlation::Limits::new(1024, 1, distinct, 256, 1 << 20).unwrap(),
            anomaly::Limits::new(16, 256, 256, 1 << 20).unwrap(),
        )
        .unwrap(),
    )
}

fn fanout_sequence(name: &str) {
    let mut fanout = CredentialFanout::new(registry(1024), 2);
    for (index, go) in rows(name).iter().enumerate() {
        let addr: SocketAddr = go["addr"].as_str().unwrap().parse().unwrap();
        let attested = go["path"] == "/transfer";
        let decision = fanout.observe(
            "cred-ceiling",
            &client_subnet(addr.ip()),
            "sid",
            attested,
            Duration::from_secs(index as u64),
        );
        let first = &go["records"][0];
        match go["status"].as_u64().unwrap() {
            200 => assert_eq!(decision, fanout::Decision::Upgrade, "{name}[{index}]"),
            401 => match decision {
                fanout::Decision::Withheld { note, .. } => {
                    assert_eq!(note, first["fail_reason"], "{name}[{index}]");
                    assert_eq!(first["event_type"], "webauthn.assertion.verified");
                }
                other => panic!("{name}[{index}]: expected withheld, got {other:?}"),
            },
            other => panic!("unexpected Go status {other}"),
        }
    }
}

/// Go TestWebAuthnFanoutCap: with cap 2, the third distinct /24 on the
/// attested route loses the fast path, with the Go audit note.
#[test]
fn go_fanout_cap_withholds_past_cap_on_attested_route() {
    fanout_sequence("fanout_cap_attested");
}

/// Go TestWebAuthnFanoutCapUnmarkedRoutesUntouched: past the cap a balanced
/// route keeps the fast path, and fan-out counted there withholds the next
/// attested request.
#[test]
fn go_fanout_cap_leaves_unmarked_routes_untouched() {
    fanout_sequence("fanout_cap_unmarked");
}

/// Go `DefaultCredFanoutCap` for a non-positive setting.
#[test]
fn non_positive_cap_selects_the_go_default() {
    let default = GOLDEN["default_cred_fanout_cap"].as_u64().unwrap() as usize;
    assert_eq!(fanout::DEFAULT_CAP, default);
    for configured in [0, -1, i64::MIN] {
        assert_eq!(
            CredentialFanout::new(registry(8), configured).cap(),
            default
        );
    }
    assert_eq!(CredentialFanout::new(registry(8), 3).cap(), 3);
}

/// Rust fails closed where Go's map never refused: an observation the
/// registry cannot admit withholds the fast path on every route.
#[test]
fn unadmitted_observation_withholds_the_upgrade_everywhere() {
    let mut fanout = CredentialFanout::new(registry(1), 8);
    let at = Duration::from_secs(1);
    assert_eq!(
        fanout.observe("c", "203.0.113.0/24", "s", false, at),
        fanout::Decision::Upgrade
    );
    assert_eq!(
        fanout.observe("c", "203.0.114.0/24", "s", false, at),
        fanout::Decision::Unavailable
    );
    // A clock regression is refused the same way.
    assert_eq!(
        fanout.observe("d", "203.0.113.0/24", "s", false, Duration::ZERO),
        fanout::Decision::Unavailable
    );
}

/// Go TestNewServerRefusesAttestedCatchAll, widened to the prefixes the
/// oracle normalizes to the root: only a floored preset on a root or empty
/// prefix is refused, with the Go error text.
#[test]
fn go_attested_catch_all_refusal_matches_capture() {
    let golden = GOLDEN["catch_all"].as_object().unwrap();
    assert!(!golden.is_empty());
    for (entry, message) in golden {
        let (preset, prefix) = entry.split_once(' ').unwrap();
        let routes = BTreeMap::from([(prefix.to_owned(), preset.to_owned())]);
        let got = refuse_attested_catch_all(&routes)
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
        assert_eq!(got, message.as_str().unwrap(), "{entry:?}");
    }
}
