//! Differential replay of the Go `ServeHTTP` gate chain at baseline 981d25c
//! (`tests/fixtures/gateway/gates-go-golden.json`). Each captured request is
//! replayed through these gate functions in Go order, with the real Rust
//! credential, fan-out and sweep kernels, and the records each gate appends
//! are compared with the records the oracle appended before its verdict
//! gate. Go effects map as: a verdict record follows means continue, 403 with
//! the incident pointer means deny, 200 with the origin contacted and no
//! verdict record means forward.
use super::*;
use crate::gateway::attest_floor::fanout::CredentialFanout;
use crate::gateway::credential::privacy_pass::tests as pat_fixture;
use crate::gateway::credential::web_bot_auth::tests as wba_fixture;
use crate::gateway::credential::webauthn::tests as webauthn_fixture;
use crate::gateway::token::{MAX_NONCES, Nonces, client_subnet};
use crate::network::correlation::{self, Registry};
use crate::network::history::anomaly;
use crate::network::history::sweep::{Config as SweepConfig, Detector, Sweep};
use serde_json::{Value, json};
use std::net::IpAddr;
use std::sync::LazyLock;
use std::time::Duration;

static GOLDEN: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/gateway/gates-go-golden.json"
    ))
    .unwrap()
});

const PSN: &str = "psn";
const AGENT_KID: &str = "lane-d-agent-key";
const CRED: &str = "lane-d-credential-0123456789";
const HOST: &str = "proxy";

fn case(name: &str) -> &'static Value {
    GOLDEN["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["case"] == name)
        .unwrap_or_else(|| panic!("no golden case {name}"))
}

/// The oracle's verdict-gate record (`applyVerdict`), which ends the gates.
fn is_verdict_record(record: &Value) -> bool {
    record["host"] == HOST
        && ["pass", "block", "challenge_pow"].contains(&record["action"].as_str().unwrap())
}

/// Records the Go gates appended, and the Go effect.
fn go_gates(case: &Value) -> (Vec<Value>, Effect, bool) {
    let records = case["records"].as_array().unwrap();
    let split = records.iter().position(is_verdict_record);
    let effect = if split.is_some() {
        Effect::Continue
    } else if case["status"] == 403 && case["incident"] == "see-audit-log" {
        assert_eq!(case["body"], format!("{DENY_BODY}\n"));
        assert_eq!(case["origin_hits"], 0);
        Effect::Deny
    } else {
        assert_eq!(
            (&case["status"], &case["origin_hits"]),
            (&json!(200), &json!(1))
        );
        Effect::Forward
    };
    let clear = case["set_cookie"] == json!([CLEAR_VERDICT_COOKIE]);
    if !clear {
        assert!(case["set_cookie"].is_null(), "{}", case["case"]);
    }
    (
        records[..split.unwrap_or(records.len())].to_vec(),
        effect,
        clear,
    )
}

/// A Rust record in the capture's shape. The verdict vocabulary is the
/// Runtime's uppercase one (documented difference); it is lowered here after
/// asserting it is uppercase.
fn view(record: &Record) -> Value {
    assert_eq!(record.verdict, record.verdict.to_ascii_uppercase());
    json!({
        "event_type": record.event_type,
        "actor_kind": record.actor.kind,
        "actor_psn_set": !record.actor.id_pseudonym.is_empty(),
        "tenant_id": record.tenant_id,
        "session_psn_set": !record.session_pseudonym.is_empty(),
        "host": record.host,
        "route_class": record.route_class,
        "verdict": record.verdict.to_ascii_lowercase(),
        "risk_score": record.risk_score,
        "rules": record.triggered_rules.clone().unwrap_or_default(),
        "action": record.enforcement_action,
        "mode": record.enforcement_mode,
        "fail_reason": record.fail_reason,
        "key_id": record.key_id,
        "config_ver": record.config_version,
    })
}

/// The gate capabilities the oracle stack installed.
struct Gates {
    directory: web_bot_auth::KeyDirectory,
    nonces: Nonces,
    pat: privacy_pass::Verifier,
    registry: webauthn::Registry,
    fanout: CredentialFanout,
    sweep: Detector,
    verdict_mode: Mode,
    floor_mode: FloorMode,
    sweep_mode: Mode,
}

pub(in crate::gateway) fn fanout_registry() -> Registry {
    Registry::new(
        correlation::Config::new(
            fanout::WINDOW,
            correlation::Limits::new(1024, 1, 1024, 256, 1 << 20).unwrap(),
            anomaly::Limits::new(16, 256, 256, 1 << 20).unwrap(),
        )
        .unwrap(),
    )
}

impl Gates {
    fn new(agent: &ed25519_dalek::SigningKey, credential: &rcgen::KeyPair, cap: i64) -> Self {
        Self {
            directory: wba_fixture::directory(AGENT_KID, agent),
            nonces: Nonces::new(Duration::from_secs(3600), MAX_NONCES),
            pat: pat_fixture::verifier(pat_fixture::ISSUER_A_SPKI),
            registry: webauthn_fixture::registry(CRED, credential),
            fanout: CredentialFanout::new(fanout_registry(), cap),
            // Go `NewSweepDetector(30*time.Second, 8)`.
            sweep: Detector::new(SweepConfig::new(Duration::from_secs(30), 8, 1024).unwrap()),
            verdict_mode: Mode::Enforce,
            floor_mode: FloorMode::Enforce,
            sweep_mode: Mode::Enforce,
        }
    }
}

/// One replayed request.
#[derive(Default)]
struct Req {
    path: &'static str,
    sid: &'static str,
    addr: &'static str,
    upgrade: bool,
    signature: Option<(String, String)>,
    pat: Option<String>,
    assertion: Option<String>,
    token: Option<Token>,
    step_up: bool,
    /// Go `hasValidToken`, independent of the verdict-token gate's mode.
    valid_token: bool,
    config_version: &'static str,
}

fn route(path: &str) -> Route<'static> {
    match path {
        "/transfer" => Route {
            enforcing: true,
            attest_floor: true,
            name: "attested",
        },
        "/mon" => Route {
            enforcing: false,
            attest_floor: false,
            name: "monitor",
        },
        _ => Route {
            enforcing: true,
            attest_floor: false,
            name: "balanced",
        },
    }
}

/// Accumulates gate outcomes until one ends the chain.
#[derive(Default)]
struct Acc {
    records: Vec<Record>,
    clear: bool,
    effect: Option<Effect>,
}

impl Acc {
    /// Take one gate's outcome; true when it ended the chain.
    fn ended(&mut self, outcome: Outcome) -> bool {
        self.records.extend(outcome.records);
        self.clear |= outcome.clear_verdict_cookie;
        if outcome.effect != Effect::Continue {
            self.effect = Some(outcome.effect);
        }
        self.effect.is_some()
    }

    fn done(self) -> (Vec<Record>, Effect, bool) {
        (
            self.records,
            self.effect.unwrap_or(Effect::Continue),
            self.clear,
        )
    }
}

/// The gates of Go `ServeHTTP` from route resolution to the verdict gate.
fn chain(gates: &mut Gates, req: &Req, now: Duration) -> (Vec<Record>, Effect, bool) {
    let mut headers = vec![("Accept".to_owned(), "*/*".to_owned())];
    if req.upgrade {
        headers.push((
            "Sec-WebSocket-Key".into(),
            "dGhlIHNhbXBsZSBub25jZQ==".into(),
        ));
    }
    let class = crate::gateway::request::route_class(&headers).as_str();
    let subject = Subject {
        tenant: "test",
        pseudonym: PSN,
        route_class: class,
    };
    let route = route(req.path);
    let mut acc = Acc::default();
    let wba = web_bot_auth::Request {
        authority: HOST,
        method: "GET",
        path: req.path,
        signature_input: req.signature.as_ref().map(|s| s.0.as_str()),
        signature: req
            .signature
            .as_ref()
            .map(|s| s.1.as_str())
            .filter(|s| !s.is_empty()),
    };
    let verdict = web_bot_auth::verify(&wba, Some(&gates.directory), &mut gates.nonces, now);
    if acc.ended(agent_auth(&subject, &route, &verdict)) {
        return acc.done();
    }
    let outcome = pat(&subject, &route, || {
        gates.pat.verify(req.pat.as_deref(), now)
    });
    if acc.ended(outcome) {
        return acc.done();
    }
    let subnet = client_subnet(req.addr.parse::<IpAddr>().unwrap());
    let outcome = webauthn(
        &subject,
        &route,
        || gates.registry.verify(req.assertion.as_deref()),
        |credential| {
            gates
                .fanout
                .observe(credential, &subnet, req.sid, route.attest_floor, now)
        },
    );
    if acc.ended(outcome) {
        return acc.done();
    }
    let floor = Floor {
        mode: gates.floor_mode,
        step_up: req.step_up,
        audit: FloorAudit {
            actor_pseudonym: PSN,
            session_pseudonym: PSN,
            host: HOST,
            config_version: req.config_version,
        },
    };
    let token = req.token.unwrap_or(Token::Absent);
    let outcome = verdict_token(&subject, &route, gates.verdict_mode, || token, &floor);
    if acc.ended(outcome) || acc.ended(upgrade(&subject, &route, req.valid_token)) {
        return acc.done();
    }
    let outcome = sweep(&subject, &route, gates.sweep_mode, || {
        gates
            .sweep
            .observe("fp", req.sid, now)
            .map(|counted| matches!(counted, Sweep::Counted { sweeping: true, .. }))
    })
    .unwrap();
    acc.ended(outcome);
    acc.done()
}

/// Replay `name` and compare with the oracle; `go_fix` applies a documented
/// difference to the Go records before comparison.
fn replay_with(gates: &mut Gates, name: &str, req: &Req, go_fix: impl FnOnce(&mut Vec<Value>)) {
    let golden = case(name);
    let (mut expected, effect, clear) = go_gates(golden);
    go_fix(&mut expected);
    let now = Duration::from_secs(wba_fixture::NOW);
    let (records, got_effect, got_clear) = chain(gates, req, now);
    let got: Vec<Value> = records.iter().map(view).collect();
    assert_eq!(got, expected, "{name}: records");
    assert_eq!((got_effect, got_clear), (effect, clear), "{name}: effect");
}

fn replay(gates: &mut Gates, name: &str, req: &Req) {
    replay_with(gates, name, req, |_| {});
}

fn stack(cap: i64) -> (Gates, ed25519_dalek::SigningKey, rcgen::KeyPair) {
    let agent = wba_fixture::signer(7);
    let credential = webauthn_fixture::key();
    let gates = Gates::new(&agent, &credential, cap);
    (gates, agent, credential)
}

const ADDR: &str = "198.51.100.7";

fn base(path: &'static str, sid: &'static str) -> Req {
    Req {
        path,
        sid,
        addr: ADDR,
        ..Default::default()
    }
}

fn token(meta: &str) -> Token {
    match meta {
        "bad_sig" => Token::Invalid(Reason::BadSignature),
        "binding_mismatch" => Token::Invalid(Reason::BindingMismatch),
        "expired" => Token::Invalid(Reason::Expired),
        "malformed" => Token::Invalid(Reason::Malformed),
        "absent" => Token::Absent,
        other => panic!("{other}"),
    }
}

fn mode(name: &str) -> (Mode, FloorMode) {
    match name {
        "monitor" => (Mode::Monitor, FloorMode::Monitor),
        "shadow" => (Mode::Shadow, FloorMode::Shadow),
        _ => (Mode::Enforce, FloorMode::Enforce),
    }
}

/// The floor audit records the oracle's `configVersion()`; it is a caller
/// input here, taken from the captured record.
fn config_version(name: &str) -> &'static str {
    case(name)["records"][0]["config_ver"].as_str().unwrap()
}

#[test]
fn verdict_token_gate_matches_go() {
    let (mut gates, _, _) = stack(0);
    let valid = |path, sid| Req {
        token: Some(Token::Valid),
        valid_token: true,
        ..base(path, sid)
    };
    replay(&mut gates, "vt_valid_balanced", &valid("/", "vt-a"));
    for name in [
        "vt_bad_sig",
        "vt_binding_mismatch",
        "vt_wrong_session",
        "vt_expired",
        "vt_malformed",
        "vt_no_cookie",
    ] {
        let meta = case(name)["meta"]["token"].as_str().unwrap();
        let req = Req {
            token: Some(token(meta)),
            ..base("/", "vt-a")
        };
        replay(&mut gates, name, &req);
    }
    replay(&mut gates, "vt_monitor_route", &valid("/mon", "vt-m"));
    replay(
        &mut gates,
        "vt_attest_floor_enforce",
        &valid("/transfer", "vt-f"),
    );
    let stepped = Req {
        step_up: true,
        ..valid("/transfer", "vt-f")
    };
    replay(&mut gates, "vt_attest_floor_step_up", &stepped);
    for name in ["monitor", "shadow"] {
        let (mut gates, _, _) = stack(0);
        (gates.verdict_mode, _) = mode(name);
        replay(&mut gates, &format!("vt_{name}_valid"), &valid("/", "vt-a"));
        let invalid = Req {
            token: Some(Token::Invalid(Reason::Malformed)),
            ..base("/", "vt-a")
        };
        replay(&mut gates, &format!("vt_{name}_invalid"), &invalid);
    }
}

#[test]
fn attestation_floor_audit_matches_go() {
    for name in ["monitor", "shadow"] {
        let (mut gates, _, _) = stack(0);
        (_, gates.floor_mode) = mode(name);
        let case_name = format!("vt_attest_floor_{name}");
        let req = Req {
            token: Some(Token::Valid),
            valid_token: true,
            config_version: config_version(&case_name),
            ..base("/transfer", "vt-f")
        };
        replay(&mut gates, &case_name, &req);

        // The applyVerdict half: a sticky ALLOW without a token.
        let case_name = format!("af_scoring_{name}");
        let decided = attest_floor::decide(
            attest_floor::Request {
                attest_floor: true,
                enforcing: true,
                verdict_token: false,
                sticky_allow: true,
                step_up: false,
            },
            gates.floor_mode,
        );
        assert_eq!(decided.decision, attest_floor::Decision::Forward);
        let audit = FloorAudit {
            actor_pseudonym: PSN,
            session_pseudonym: PSN,
            host: HOST,
            config_version: config_version(&case_name),
        };
        let record = attestation_floor_audit(
            "test",
            "static",
            gates.floor_mode,
            decided.would_require_step_up.unwrap(),
            &audit,
        );
        assert_eq!(view(&record), case(&case_name)["records"][0]);
        assert!(is_verdict_record(&case(&case_name)["records"][1]));
    }
}

#[test]
fn upgrade_gate_matches_go() {
    let (mut gates, _, _) = stack(0);
    let ws = |path, token| Req {
        upgrade: true,
        token,
        ..base(path, "up-a")
    };
    replay(&mut gates, "up_no_token", &ws("/", None));
    let bad = ws("/", Some(Token::Invalid(Reason::Malformed)));
    replay(&mut gates, "up_bad_token", &bad);
    replay(&mut gates, "up_monitor_route", &ws("/mon", None));
    replay(&mut gates, "up_key_only", &ws("/", None));
    // A valid token admits the upgrade even when the verdict-token gate only
    // monitors: Go `hasValidToken` ignores that gate's mode.
    let (mut gates, _, _) = stack(0);
    gates.verdict_mode = Mode::Monitor;
    let valid = Req {
        valid_token: true,
        ..ws("/", Some(Token::Valid))
    };
    replay(&mut gates, "up_valid_token_monitor_vt", &valid);
}

#[test]
fn agent_auth_gate_matches_go_with_named_differences() {
    let (mut gates, agent, _) = stack(0);
    let now = wba_fixture::NOW;
    let signed =
        |path: &'static str, kid: &str, key: &ed25519_dalek::SigningKey, created, nonce| {
            let target = wba_fixture::request(HOST, "GET", path);
            Req {
                signature: Some(wba_fixture::sign_wba(&target, kid, key, created, nonce)),
                ..base(path, "ag-a")
            }
        };
    replay(
        &mut gates,
        "ag_trusted",
        &signed("/", AGENT_KID, &agent, now, "n1"),
    );
    replay(
        &mut gates,
        "ag_replay",
        &signed("/", AGENT_KID, &agent, now, "n1"),
    );
    let other = wba_fixture::signer(9);
    replay(
        &mut gates,
        "ag_forged",
        &signed("/", AGENT_KID, &other, now, "n2"),
    );
    let req = signed("/mon", AGENT_KID, &other, now, "n3");
    replay(&mut gates, "ag_forged_monitor_route", &req);
    let req = signed("/mon", AGENT_KID, &agent, now, "n4");
    replay(&mut gates, "ag_trusted_monitor_route", &req);
    // Difference: the credential kernel names a keyid only when it is on the
    // allowlist, so an unknown or stale signature records an empty keyid.
    let req = signed("/", "not-allowlisted", &other, now, "n5");
    replay_with(&mut gates, "ag_unknown_key", &req, |go| {
        assert_eq!(
            go[0]["fail_reason"],
            "unknown/unsupported agent signature keyid not-allowlisted"
        );
        go[0]["fail_reason"] = json!("unknown/unsupported agent signature keyid ");
    });
    let req = signed("/", AGENT_KID, &agent, now - 7200, "n6");
    replay_with(&mut gates, "ag_expired", &req, |go| {
        assert_eq!(
            go[0]["fail_reason"],
            format!("unknown/unsupported agent signature keyid {AGENT_KID}")
        );
        go[0]["fail_reason"] = json!("unknown/unsupported agent signature keyid ");
    });
    // Difference: a half signature is neutral and recorded; Go ignored it.
    let half = Req {
        signature: Some(("sig1=(\"@authority\")".into(), String::new())),
        ..base("/", "ag-a")
    };
    replay_with(&mut gates, "ag_half_present", &half, |go| {
        assert!(go.is_empty());
        go.push(json!({
            "event_type": "agent.signature.unknown", "actor_kind": "subject",
            "actor_psn_set": true, "tenant_id": "test", "session_psn_set": false,
            "host": "", "route_class": "static", "verdict": "", "risk_score": 0,
            "rules": [], "action": "", "mode": "enforce",
            "fail_reason": "unknown/unsupported agent signature keyid ",
            "key_id": "k1", "config_ver": "",
        }));
    });
}

#[test]
fn pat_gate_matches_go() {
    let (mut gates, _, _) = stack(0);
    let der = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        pat_fixture::ISSUER_A_SPKI,
    )
    .unwrap();
    let issuer = hex::encode(<sha2::Sha256 as sha2::Digest>::digest(der));
    let redact = |go: &mut Vec<Value>| {
        for record in go.iter_mut() {
            let reason = record["fail_reason"].as_str().unwrap().to_owned();
            record["fail_reason"] = json!(reason.replace("<redacted>", &issuer[..12]));
        }
    };
    let presented = |path, header: String| Req {
        pat: Some(header),
        ..base(path, "pat-a")
    };
    let token = pat_fixture::mint();
    replay_with(
        &mut gates,
        "pat_verified",
        &presented("/", token.clone()),
        redact,
    );
    replay(
        &mut gates,
        "pat_double_spend",
        &presented("/", token.clone()),
    );
    // Not verified (and so not spent) on a non-enforcing route.
    let mut fresh = Gates::new(&wba_fixture::signer(7), &webauthn_fixture::key(), 0);
    replay(
        &mut fresh,
        "pat_monitor_route",
        &presented("/mon", token.clone()),
    );
    replay_with(&mut fresh, "pat_verified", &presented("/", token), redact);
    let garbage = presented("/", "PrivateToken token=\"AAAA\"".into());
    replay(&mut gates, "pat_garbage", &garbage);
}

#[test]
fn webauthn_gate_and_fanout_cap_match_go() {
    let (mut gates, _, credential) = stack(1);
    let presented = |path, addr, counter| Req {
        addr,
        assertion: Some(webauthn_fixture::sign_assertion(&credential, CRED, counter)),
        ..base(path, "wa-a")
    };
    replay(&mut gates, "wa_verified", &presented("/", ADDR, 1));
    replay(&mut gates, "wa_replay", &presented("/", ADDR, 1));
    let second = presented("/", "203.0.113.9", 2);
    replay(&mut gates, "wa_second_subnet_public", &second);
    let third = presented("/transfer", "192.0.2.9", 3);
    replay(&mut gates, "wa_fanout_withheld_attested", &third);
    replay(&mut gates, "wa_monitor_route", &presented("/mon", ADDR, 4));
}

#[test]
fn sweep_gate_matches_go() {
    const SESSIONS: [&str; 9] = [
        "sw-0", "sw-1", "sw-2", "sw-3", "sw-4", "sw-5", "sw-6", "sw-7", "sw-8",
    ];
    for name in ["enforce", "monitor", "shadow"] {
        let (mut gates, _, _) = stack(0);
        (gates.sweep_mode, _) = mode(name);
        let now = Duration::from_secs(wba_fixture::NOW);
        for sid in &SESSIONS[..8] {
            let (_, effect, _) = chain(&mut gates, &base("/", sid), now);
            assert_eq!(effect, Effect::Continue);
        }
        replay(&mut gates, &format!("sw_{name}_ninth"), &base("/", "sw-8"));
        let repeat = format!("sw_{name}_repeat_known_session");
        replay(&mut gates, &repeat, &base("/", "sw-8"));
    }
    // An enforcing sweep gate skips (and counts nothing on) a monitor route.
    let (mut gates, _, _) = stack(0);
    let now = Duration::from_secs(wba_fixture::NOW);
    const MONITORED: [&str; 10] = [
        "swm-0", "swm-1", "swm-2", "swm-3", "swm-4", "swm-5", "swm-6", "swm-7", "swm-8", "swm-9",
    ];
    for sid in &MONITORED[..9] {
        chain(&mut gates, &base("/mon", sid), now);
    }
    replay(
        &mut gates,
        "sw_enforce_monitor_route_tenth",
        &base("/mon", MONITORED[9]),
    );
    replay(
        &mut gates,
        "sw_enforce_after_monitor_route",
        &base("/", "swm-10"),
    );
}

#[test]
fn sweep_detector_refusal_fails_closed_only_when_enforcing() {
    let subject = Subject {
        tenant: "t",
        pseudonym: "",
        route_class: "static",
    };
    let route = route("/");
    let refused = || Err(Unavailable::Budget);
    assert_eq!(
        sweep(&subject, &route, Mode::Enforce, refused),
        Err(Unavailable::Budget)
    );
    for mode in [Mode::Monitor, Mode::Shadow] {
        let outcome = sweep(&subject, &route, mode, refused).unwrap();
        assert_eq!(outcome, Outcome::proceed());
    }
    let never = || -> Result<bool, Unavailable> { panic!("off counts nothing") };
    assert_eq!(
        sweep(&subject, &route, Mode::Off, never).unwrap(),
        Outcome::proceed()
    );
}

#[test]
fn gates_skip_their_kernels_where_go_did() {
    let subject = Subject {
        tenant: "t",
        pseudonym: "",
        route_class: "static",
    };
    let monitor = route("/mon");
    let floor = Floor {
        mode: FloorMode::Enforce,
        step_up: false,
        audit: FloorAudit {
            actor_pseudonym: "",
            session_pseudonym: "",
            host: "",
            config_version: "",
        },
    };
    let never_pat = || -> privacy_pass::Verdict { panic!("PAT spent on a monitor route") };
    assert_eq!(pat(&subject, &monitor, never_pat), Outcome::proceed());
    let never_assertion = || -> webauthn::Verdict { panic!("assertion on a monitor route") };
    let never_fanout = |_: &str| -> fanout::Decision { panic!("fan-out on a monitor route") };
    let outcome = webauthn(&subject, &monitor, never_assertion, never_fanout);
    assert_eq!(outcome, Outcome::proceed());
    let never_token = || -> Token { panic!("token checked while off") };
    let outcome = verdict_token(&subject, &route("/"), Mode::Off, never_token, &floor);
    assert_eq!(outcome, Outcome::proceed());
    // A registry that cannot admit the observation withholds the upgrade
    // without a record (attest_floor::fanout difference).
    let verified = || webauthn::Verdict::Verified {
        credential_id: CRED.into(),
    };
    let outcome = webauthn(&subject, &route("/"), verified, |_| {
        fanout::Decision::Unavailable
    });
    assert_eq!(outcome, Outcome::proceed());
    // Go `credID[:16]` on a multibyte id cuts back to a character boundary.
    assert_eq!(
        short("\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}"),
        "\u{e9}".repeat(8)
    );
}
