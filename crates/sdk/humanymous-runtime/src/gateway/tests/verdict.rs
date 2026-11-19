//! Key-gated `hmn_vt` verdict tokens and route presets with the attestation
//! floor on the served path (`application::verdict`, owner decision #4),
//! replayed against the Go capture `tests/fixtures/gateway/gate-modes-go-golden.json`
//! (legacy control-plane issuance, `verdictTokenGate`, `upgradeGate` and
//! `applyVerdict` at baseline 981d25c). Fixture controls through
//! `Application::decide`; no browser or product e2e ran.
use super::edge::edge_config;
use super::gates::{NOW_MS, app_with, assert_denied, records};
use super::modes::{golden, overlay, slot};
use super::*;
use crate::gateway::application::verdict_cookie;
use crate::gateway::route_policy::RouteTable;
use crate::gateway::token::{self, Epochs, TokenKey};

const CLIENT: &str = "203.0.113.7";

fn key() -> TokenKey {
    TokenKey::new(
        golden()["token_issue"]["key_ascii"]
            .as_str()
            .unwrap()
            .into(),
    )
    .unwrap()
}

fn keyed(audit: &Audit) -> Application {
    let mut app = app_with(edge_config("127.0.0.1:32123".parse().unwrap()), audit);
    app.set_client(Some(CLIENT.parse().unwrap()));
    app.install_verdict_token_key(key()).unwrap();
    app
}

fn presets(app: &mut Application, routes: &[(&str, &str)]) {
    app.install_route_presets(RouteTable {
        routes: routes
            .iter()
            .map(|(prefix, preset)| ((*prefix).to_owned(), (*preset).to_owned()))
            .collect(),
        global_monitor: false,
    })
    .unwrap();
}

/// A collected session whose protected requests score ALLOW.
fn session(app: &mut Application) -> Value {
    let issued = issue(app, NOW_MS);
    collected(app, &config(), &issued, NOW_MS);
    issued
}

fn protected_with(issued: &Value, n: u64, extra: &[(&str, String)]) -> Request {
    let mut req = request("POST", "/effect?exact=1", b"original bytes".to_vec());
    req.headers.extend(
        extra
            .iter()
            .map(|(name, value)| ((*name).to_owned(), value.clone())),
    );
    sign(
        &config(),
        &mut req,
        issued["session"].as_str().unwrap(),
        issued["key"].as_str().unwrap(),
        n,
        NOW_MS,
    );
    req
}

fn minted(decision: Decision) -> String {
    let Decision::Origin { upstream, .. } = decision else {
        panic!("expected origin");
    };
    let [cookie] = upstream.cookies() else {
        panic!("expected one Set-Cookie: {:?}", upstream.cookies());
    };
    let value = cookie.strip_prefix("hmn_vt=").unwrap();
    value.split(';').next().unwrap().to_owned()
}

fn cookie(token: &str) -> (&'static str, String) {
    ("Cookie", format!("theme=dark; hmn_vt={token}"))
}

/// The Go token bytes and cookie attributes for a fixed key, session,
/// binding, expiry and epoch, and the binding inputs.
#[test]
fn token_bytes_and_cookie_match_go() {
    let go = &golden()["token_issue"];
    let subnet = token::client_subnet(CLIENT.parse().unwrap());
    assert_eq!(subnet, go["client_subnet"]);
    let bind = token::token_bind(go["bind_key"].as_str(), &subnet);
    assert_eq!(bind, go["token_bind"]);
    let minted = key()
        .issue_verdict(
            go["sid"].as_str().unwrap(),
            &bind,
            go["epoch"].as_str().unwrap(),
            go["exp_unix"].as_u64().unwrap(),
        )
        .unwrap();
    assert_eq!(minted, go["token_fixed_epoch"]);
    assert_eq!(json!([verdict_cookie(&minted)]), go["allow_set_cookie"]);
    assert_eq!(
        go["exp_unix"].as_u64().unwrap() - go["now_unix"].as_u64().unwrap(),
        crate::gateway::application::VERDICT_TTL_SECS
    );
    // Go without a token key set no cookie.
    assert!(go["no_key_set_cookie"].is_null());
}

/// A scored ALLOW mints `hmn_vt` for the RIT session and client binding with
/// a `token.issued` record; the next request presenting it takes the trusted
/// fast path without scoring; a forged token is cleared and re-scored.
#[test]
fn scored_allow_mints_hmn_vt_and_the_token_fast_paths() {
    let audit = Audit::default();
    let mut app = keyed(&audit);
    let issued = session(&mut app);
    let sid = issued["session"].as_str().unwrap();
    let token = minted(
        app.decide(&protected_with(&issued, 2, &[]), || Ok(NOW_MS))
            .unwrap(),
    );
    let now = NOW_MS / 1000;
    let bind = token::token_bind(
        token::bind_key("Mozilla/5.0 Chrome/126.0 Safari/537.36", "", "Chromium").as_deref(),
        "203.0.113.0/24",
    );
    let expected = key()
        .issue_verdict(sid, &bind, &Epochs::default().current(now), now + 1800)
        .unwrap();
    assert_eq!(token, expected);
    let issued_records = records(&audit, "token.issued");
    assert_eq!(issued_records.len(), 1);
    assert_eq!(
        (
            issued_records[0].verdict.as_str(),
            issued_records[0].enforcement_mode.as_str(),
            issued_records[0].key_id.as_str()
        ),
        ("ALLOW", "enforce", "k1")
    );
    assert_eq!(records(&audit, "runtime.decision").len(), 1);

    let decision = app
        .decide(&protected_with(&issued, 3, &[cookie(&token)]), || {
            Ok(NOW_MS)
        })
        .unwrap();
    let Decision::Origin { upstream, .. } = decision else {
        panic!("fast path");
    };
    assert!(upstream.cookies().is_empty());
    assert_eq!(records(&audit, "runtime.decision").len(), 1);
    let fast = records(&audit, "enforcement.allow");
    assert_eq!(fast.len(), 1);
    assert_eq!(
        (
            fast[0].enforcement_action.as_str(),
            fast[0].verdict.as_str()
        ),
        ("pass", "ALLOW")
    );

    let forged = format!("{}x", &token[..token.len() - 1]);
    let token = minted(
        app.decide(&protected_with(&issued, 4, &[cookie(&forged)]), || {
            Ok(NOW_MS)
        })
        .unwrap(),
    );
    assert_eq!(token, expected);
    assert_eq!(records(&audit, "token.binding_mismatch").len(), 1);
    assert_eq!(records(&audit, "runtime.decision").len(), 2);
}

/// Without a key nothing is issued and an upgrade on an enforcing route stays
/// denied (HR-26). With a key, Go `hasValidToken` ignores the gate's mode: a
/// valid token lets the upgrade through while `gate.verdict_token` monitor
/// only records the would-fast-path.
#[test]
fn upgrade_needs_a_minted_token_and_no_key_issues_none() {
    let audit = Audit::default();
    let mut app = app_with(edge_config("127.0.0.1:32123".parse().unwrap()), &audit);
    app.set_client(Some(CLIENT.parse().unwrap()));
    let issued = session(&mut app);
    let Decision::Origin { upstream, .. } = app
        .decide(&protected_with(&issued, 2, &[]), || Ok(NOW_MS))
        .unwrap()
    else {
        panic!("origin");
    };
    assert!(upstream.cookies().is_empty());
    assert!(records(&audit, "token.issued").is_empty());
    let ws = ("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==".to_owned());
    assert_denied(
        app.decide(
            &protected_with(&issued, 3, std::slice::from_ref(&ws)),
            || Ok(NOW_MS),
        )
        .unwrap(),
    );

    let audit = Audit::default();
    let mut app = keyed(&audit);
    app.install_gate_modes(slot(Some(overlay(&[("gate.verdict_token", "monitor")]))));
    let issued = session(&mut app);
    let token = minted(
        app.decide(&protected_with(&issued, 2, &[]), || Ok(NOW_MS))
            .unwrap(),
    );
    let decision = app
        .decide(
            &protected_with(&issued, 3, &[ws.clone(), cookie(&token)]),
            || Ok(NOW_MS),
        )
        .unwrap();
    assert!(matches!(decision, Decision::Origin { .. }));
    assert!(records(&audit, "upgrade.no_prior_verdict").is_empty());
    let would = records(&audit, "enforcement.allow");
    let go = &golden()["monitor_cases"]["verdict_token_monitor"]["records"][0];
    assert_eq!(would.len(), 1);
    assert_eq!(would[0].enforcement_action, go["action"]);
    assert_eq!(would[0].enforcement_mode, go["mode"]);
    // Scored, since monitor never takes the fast path.
    assert_eq!(records(&audit, "runtime.decision").len(), 2);
}

fn floor_fields(record: &crate::audit::record::Record) -> Value {
    json!([
        record.event_type,
        record.enforcement_action,
        record.enforcement_mode,
        record.fail_reason,
        record.verdict.to_ascii_lowercase()
    ])
}

fn go_floor_fields(record: &Value) -> Value {
    json!([
        record["event_type"],
        record["action"],
        record["mode"],
        record["fail_reason"],
        record["verdict"]
    ])
}

/// The `attested` preset: an enforced floor demotes an unproven scored ALLOW
/// to the 401 step-up challenge without contacting the origin or minting a
/// token; under `gate.attest_floor` monitor it forwards with the
/// would-challenge audit, and the token fast path records the would-step-up
/// note before its pass.
#[test]
fn attested_preset_floor_challenges_or_audits() {
    let go = golden();
    let audit = Audit::default();
    let mut app = keyed(&audit);
    presets(&mut app, &[("/effect", "attested")]);
    let issued = session(&mut app);
    let decision = app
        .decide(&protected_with(&issued, 2, &[]), || Ok(NOW_MS))
        .unwrap();
    let gold = &go["monitor_cases"]["attest_floor_enforce_scoring"];
    let Decision::Local(status, body) = decision else {
        panic!("step-up");
    };
    assert_eq!(json!(status), gold["response"]["status"]);
    assert_eq!(
        serde_json::from_slice::<Value>(&body).unwrap(),
        json!({"verdict":"CHALLENGE"})
    );
    let challenged = records(&audit, "enforcement.challenge.issued");
    assert_eq!(challenged.len(), 1);
    assert_eq!(
        floor_fields(&challenged[0]),
        go_floor_fields(&gold["records"][0])
    );
    assert!(records(&audit, "token.issued").is_empty());

    let audit = Audit::default();
    let mut app = keyed(&audit);
    presets(&mut app, &[("/effect", "attested")]);
    app.install_gate_modes(slot(Some(overlay(&[("gate.attest_floor", "monitor")]))));
    let issued = session(&mut app);
    let token = minted(
        app.decide(&protected_with(&issued, 2, &[]), || Ok(NOW_MS))
            .unwrap(),
    );
    let gold = &go["monitor_cases"]["attest_floor_monitor_scoring"];
    let would = records(&audit, "enforcement.challenge.issued");
    assert_eq!(would.len(), 1);
    assert_eq!(
        floor_fields(&would[0]),
        go_floor_fields(&gold["records"][0])
    );

    let decision = app
        .decide(&protected_with(&issued, 3, &[cookie(&token)]), || {
            Ok(NOW_MS)
        })
        .unwrap();
    assert!(matches!(decision, Decision::Origin { .. }));
    let gold = &go["monitor_cases"]["attest_floor_monitor_token"]["records"];
    let would = records(&audit, "enforcement.challenge.issued");
    assert_eq!(would.len(), 2);
    assert_eq!(floor_fields(&would[1]), go_floor_fields(&gold[0]));
    let pass = records(&audit, "enforcement.allow");
    assert_eq!(pass.len(), 1);
    assert_eq!(pass[0].config_version, gold[1]["config_version"]);
    assert_eq!(pass[0].enforcement_action, gold[1]["action"]);

    // Enforced again: the token no longer fast-paths and scoring is demoted.
    let active = slot(None);
    app.install_gate_modes(active);
    let decision = app
        .decide(&protected_with(&issued, 4, &[cookie(&token)]), || {
            Ok(NOW_MS)
        })
        .unwrap();
    assert!(matches!(decision, Decision::Local(401, _)));
}

/// Preset refusals: the floor on the catch-all prefix, and a key that needs a
/// trusted client address. A `monitor` preset makes the gates non-enforcing
/// but never passes a scored non-ALLOW (named difference).
#[test]
fn route_preset_and_key_refusals() {
    let mut app = keyed(&Audit::default());
    for root in ["/", ""] {
        assert!(
            app.install_route_presets(RouteTable {
                routes: [(root.to_owned(), "attested".to_owned())].into(),
                global_monitor: false,
            })
            .is_err()
        );
    }
    let mut plain = app_with(config(), &Audit::default());
    assert!(plain.install_verdict_token_key(key()).is_err());

    // Monitor preset: an upgrade without a token is not denied by HR-26, but
    // an unscored session (no collected report) is still not forwarded.
    let audit = Audit::default();
    let mut app = keyed(&audit);
    presets(&mut app, &[("/effect", "monitor")]);
    let issued = issue(&mut app, NOW_MS);
    let ws = ("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==".to_owned());
    let decision = app
        .decide(&protected_with(&issued, 1, &[ws]), || Ok(NOW_MS))
        .unwrap();
    assert!(records(&audit, "upgrade.no_prior_verdict").is_empty());
    assert!(matches!(decision, Decision::Local(403, _)));
}
