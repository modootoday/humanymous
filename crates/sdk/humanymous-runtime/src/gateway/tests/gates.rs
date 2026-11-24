//! Served-path composition of the credential, upgrade and sweep gates
//! (`application::gates`, legacy internal/gate gate.go `ServeHTTP` at
//! baseline 981d25c). The gate decisions themselves are replayed against the
//! Go golden in `gateway::gates::tests`; these tests pin where the served path
//! runs them, what it appends and the deliberate differences named in the
//! `application::gates` module documentation. Fixture controls only: no
//! ordinary browser, human, assistive technology or product e2e ran.
use super::browser::{browser_request, configured, scope, signed};
use super::edge::edge_config;
use super::*;
use crate::gateway::Rejection;
use crate::gateway::attest_floor::fanout::CredentialFanout;
use crate::gateway::ban::gate::Mode;
use crate::gateway::credential::privacy_pass::tests as pat_fixture;
use crate::gateway::credential::web_bot_auth::tests as wba_fixture;
use crate::gateway::credential::webauthn::tests as webauthn_fixture;
use crate::gateway::gates::tests::fanout_registry;
use crate::gateway::gates::{DENY_BODY, DENY_STATUS, INCIDENT_HEADER};
use crate::network::history::sweep::{Config as SweepConfig, Detector};
use humanymous_collector::session::Session as ClientSession;
use std::time::Duration;

/// Wall clock of every served request here (ms); Web Bot Auth sees 10 s.
const NOW_MS: u64 = 10_000;

fn records(audit: &Audit, event: &str) -> Vec<Record> {
    audit
        .records
        .lock()
        .unwrap()
        .iter()
        .filter(|r| r.event_type == event)
        .cloned()
        .collect()
}

fn app_with(cfg: Configuration, audit: &Audit) -> Application {
    Application::new(cfg, Default::default(), Box::new(audit.clone())).unwrap()
}

/// A signed protected request of a freshly issued session, with `extra`
/// fields added before signing (integrity covers them).
fn signed_request(app: &mut Application, extra: &[(String, String)]) -> Request {
    let cfg = config();
    let issued = issue(app, NOW_MS);
    let mut req = request("POST", "/effect?exact=1", b"original bytes".to_vec());
    req.headers.extend(extra.iter().cloned());
    sign(
        &cfg,
        &mut req,
        issued["session"].as_str().unwrap(),
        issued["key"].as_str().unwrap(),
        1,
        NOW_MS,
    );
    req
}

/// Web Bot Auth fields over the served request line, signed by `key` for
/// `kid` at the served wall clock.
fn wba_fields(kid: &str, key: &ed25519_dalek::SigningKey, nonce: &str) -> Vec<(String, String)> {
    let target = wba_fixture::request("owned.test", "POST", "/effect");
    let (input, signature) = wba_fixture::sign_wba(&target, kid, key, NOW_MS / 1000, nonce);
    vec![
        ("Signature-Input".into(), input),
        ("Signature".into(), signature),
    ]
}

fn assert_denied(decision: Decision) {
    let Decision::Gate(gate) = decision else {
        panic!("expected a gate deny");
    };
    assert_eq!(
        (gate.status, gate.body, gate.fields),
        (
            DENY_STATUS,
            DENY_BODY,
            vec![(INCIDENT_HEADER.0, INCIDENT_HEADER.1.to_owned())]
        )
    );
}

/// Go ag_trusted: a verified agent signature forwards without behavioural
/// scoring; its record carries the linkage of the ALLOW it authorizes.
#[test]
fn verified_agent_signature_forwards_without_scoring() {
    let audit = Audit::default();
    let mut app = app_with(config(), &audit);
    let key = wba_fixture::signer(7);
    app.install_agent_keys(wba_fixture::directory(wba_fixture::KID, &key));
    let req = signed_request(&mut app, &wba_fields(wba_fixture::KID, &key, "n-1"));
    let decision = app.decide(&req, || Ok(NOW_MS)).unwrap();
    assert!(matches!(decision, Decision::Origin { .. }));
    assert!(records(&audit, "runtime.decision").is_empty());
    let verified = records(&audit, "agent.signature.verified");
    assert_eq!(verified.len(), 1);
    let rec = &verified[0];
    assert_eq!(
        (
            rec.verdict.as_str(),
            rec.enforcement_action.as_str(),
            rec.triggered_rules.clone(),
            rec.fail_reason.as_str(),
        ),
        (
            "ALLOW",
            "pass",
            Some(vec!["web-bot-auth".to_owned()]),
            "verified Web Bot Auth agent test-agent-key-1",
        )
    );
    assert!(!rec.request_id.is_empty() && !rec.correlation_id.is_empty());
    // The same nonce again is a replay: no fast path, scored as usual.
    let replay = signed_request(&mut app, &wba_fields(wba_fixture::KID, &key, "n-1"));
    app.decide(&replay, || Ok(NOW_MS)).unwrap();
    assert_eq!(records(&audit, "agent.signature.verified").len(), 1);
    assert_eq!(records(&audit, "runtime.decision").len(), 1);
}

/// Go ag_forged: a signature for an allowlisted keyid that does not verify
/// is denied 403 with the incident pointer before scoring.
#[test]
fn forged_agent_signature_is_denied_before_scoring() {
    let audit = Audit::default();
    let mut app = app_with(config(), &audit);
    let key = wba_fixture::signer(7);
    app.install_agent_keys(wba_fixture::directory(wba_fixture::KID, &key));
    let impostor = wba_fixture::signer(9);
    let req = signed_request(&mut app, &wba_fields(wba_fixture::KID, &impostor, "n-2"));
    assert_denied(app.decide(&req, || Ok(NOW_MS)).unwrap());
    let forged = records(&audit, "agent.signature.forged");
    assert_eq!(forged.len(), 1);
    assert_eq!(
        (
            forged[0].verdict.as_str(),
            forged[0].triggered_rules.clone()
        ),
        ("DENY", Some(vec!["web-bot-auth".to_owned()]))
    );
    assert!(records(&audit, "runtime.decision").is_empty());
}

/// Go ag_unknown_key: an unknown keyid is recorded and the request is scored.
#[test]
fn unknown_agent_key_is_recorded_and_scored() {
    let audit = Audit::default();
    let mut app = app_with(config(), &audit);
    let key = wba_fixture::signer(7);
    app.install_agent_keys(wba_fixture::directory(wba_fixture::KID, &key));
    let req = signed_request(&mut app, &wba_fields("not-allowlisted", &key, "n-3"));
    app.decide(&req, || Ok(NOW_MS)).unwrap();
    assert_eq!(records(&audit, "agent.signature.unknown").len(), 1);
    assert_eq!(records(&audit, "runtime.decision").len(), 1);
}

/// Without installed capabilities the gates append nothing and the served
/// path scores exactly as before (the service binary installs none).
#[test]
fn uninstalled_gates_leave_the_served_path_unchanged() {
    let audit = Audit::default();
    let mut app = app_with(config(), &audit);
    let key = wba_fixture::signer(7);
    let req = signed_request(&mut app, &wba_fields(wba_fixture::KID, &key, "n-4"));
    app.decide(&req, || Ok(NOW_MS)).unwrap();
    let events: Vec<String> = audit
        .records
        .lock()
        .unwrap()
        .iter()
        .map(|r| r.event_type.clone())
        .collect();
    assert_eq!(events, ["runtime.decision"]);
}

/// Go upgradeGate: a WebSocket upgrade with no valid verdict token is denied
/// (HR-26). Nothing on the served path mints `hmn_vt`, so every upgrade is.
#[test]
fn upgrade_without_verdict_token_is_denied() {
    let audit = Audit::default();
    let mut app = app_with(config(), &audit);
    let req = signed_request(
        &mut app,
        &[(
            "Sec-WebSocket-Key".into(),
            "dGhlIHNhbXBsZSBub25jZQ==".into(),
        )],
    );
    assert_denied(app.decide(&req, || Ok(NOW_MS)).unwrap());
    let denied = records(&audit, "upgrade.no_prior_verdict");
    assert_eq!(denied.len(), 1);
    assert_eq!(
        (
            denied[0].route_class.as_str(),
            denied[0].triggered_rules.clone()
        ),
        ("upgrade", Some(vec!["HR-26".to_owned()]))
    );
    assert!(records(&audit, "runtime.decision").is_empty());
}

fn sweeping(mode: Mode) -> Audit {
    sweeping_with(mode, None)
}

/// Nine sessions on one fingerprint with boot mode `boot` and, when given,
/// an active settings overlay naming `gate.recon_sweep`.
fn sweeping_with(boot: Mode, overlay: Option<crate::gateway::settings::overlay::Mode>) -> Audit {
    use crate::gateway::settings::overlay::{Mode as Named, Overlay};
    let audit = Audit::default();
    let mut app = app_with(config(), &audit);
    app.install_sweep(
        Detector::new(SweepConfig::new(Duration::from_secs(30), 8, 1024).unwrap()),
        boot,
    );
    let effective = match &overlay {
        None => boot,
        Some(Named::Off) => Mode::Off,
        Some(Named::Monitor) => Mode::Monitor,
        Some(Named::Shadow) => Mode::Shadow,
        Some(_) => Mode::Enforce,
    };
    if let Some(mode) = overlay {
        let active = Overlay {
            status: "active".into(),
            gates: [("gate.recon_sweep".to_owned(), mode)].into(),
            ..Default::default()
        };
        app.install_gate_overlay(std::sync::Arc::new(std::sync::RwLock::new(Some(active))));
    }
    for n in 1..=9 {
        let req = signed_request(&mut app, &[]);
        let decision = app.decide(&req, || Ok(NOW_MS)).unwrap();
        if n == 9 && effective == Mode::Enforce {
            assert_denied(decision);
        } else {
            assert!(!matches!(decision, Decision::Gate(_)), "request {n}");
        }
    }
    audit
}

/// Go sw_enforce_ninth / sw_monitor_ninth: the ninth session on one header
/// fingerprint within the window is a decision-probing sweep (HR-30).
#[test]
fn sweep_gate_denies_the_ninth_session_and_monitor_only_records() {
    let enforced = sweeping(Mode::Enforce);
    let probing = records(&enforced, "recon.decision_probing");
    assert_eq!(probing.len(), 1);
    assert_eq!(
        (
            probing[0].enforcement_action.as_str(),
            probing[0].triggered_rules.clone()
        ),
        ("block", Some(vec!["HR-30".to_owned()]))
    );
    assert_eq!(records(&enforced, "runtime.decision").len(), 8);
    let monitored = sweeping(Mode::Monitor);
    let probing = records(&monitored, "recon.decision_probing");
    assert_eq!(probing.len(), 1);
    assert_eq!(
        (
            probing[0].enforcement_mode.as_str(),
            probing[0].enforcement_action.as_str()
        ),
        ("monitor", "would_block")
    );
    assert_eq!(records(&monitored, "runtime.decision").len(), 9);
}

/// Go TestSettingsReconMonitorAuditsWithoutBlocking at 981d25c: an overlay
/// naming `gate.recon_sweep: monitor` over an enforcing boot mode blocks
/// nothing and records `would_block` in mode `monitor`; an overlay naming
/// `enforce` over a monitoring boot mode denies the ninth session (owner
/// decision #3).
#[test]
fn settings_overlay_sets_the_sweep_mode() {
    use crate::gateway::settings::overlay::Mode as Named;
    let monitored = sweeping_with(Mode::Enforce, Some(Named::Monitor));
    let probing = records(&monitored, "recon.decision_probing");
    assert_eq!(probing.len(), 1);
    assert_eq!(
        (
            probing[0].enforcement_mode.as_str(),
            probing[0].enforcement_action.as_str()
        ),
        ("monitor", "would_block")
    );
    assert_eq!(records(&monitored, "runtime.decision").len(), 9);
    let enforced = sweeping_with(Mode::Monitor, Some(Named::Enforce));
    let probing = records(&enforced, "recon.decision_probing");
    assert_eq!(probing.len(), 1);
    assert_eq!(probing[0].enforcement_action, "block");
    assert_eq!(records(&enforced, "runtime.decision").len(), 8);
    let off = sweeping_with(Mode::Enforce, Some(Named::Off));
    assert!(records(&off, "recon.decision_probing").is_empty());
}

/// Owner decision #4, route presets: on a `monitor` route the gates see a
/// non-enforcing route (Go `enforcing(route)`), so an enforcing sweep gate
/// neither counts nor denies, and every session is scored.
#[test]
fn monitor_route_preset_stands_down_the_enforcing_sweep_gate() {
    let audit = Audit::default();
    let mut cfg = config();
    cfg.routes[0].preset = Some("monitor".into());
    let mut app = app_with(cfg, &audit);
    app.install_sweep(
        Detector::new(SweepConfig::new(Duration::from_secs(30), 8, 1024).unwrap()),
        Mode::Enforce,
    );
    for n in 1..=9 {
        let req = signed_request(&mut app, &[]);
        let decision = app.decide(&req, || Ok(NOW_MS)).unwrap();
        assert!(!matches!(decision, Decision::Gate(_)), "request {n}");
    }
    assert!(records(&audit, "recon.decision_probing").is_empty());
    assert_eq!(records(&audit, "runtime.decision").len(), 9);
}

/// A gate record that cannot be appended fails closed (503 durable_audit)
/// and poisons the epoch instead of forwarding unaudited.
#[test]
fn unaudited_trust_upgrade_fails_closed() {
    let audit = Audit::default();
    let mut app = app_with(config(), &audit);
    let key = wba_fixture::signer(7);
    app.install_agent_keys(wba_fixture::directory(wba_fixture::KID, &key));
    let req = signed_request(&mut app, &wba_fields(wba_fixture::KID, &key, "n-5"));
    audit.fail.store(true, Ordering::SeqCst);
    assert_eq!(
        app.decide(&req, || Ok(NOW_MS)).err(),
        Some(Rejection::new(503, "durable_audit"))
    );
    assert!(!app.epoch_health().serving());
}

fn webauthn_parts() -> (rcgen::KeyPair, CredentialFanout) {
    (
        webauthn_fixture::key(),
        CredentialFanout::new(fanout_registry(), 1),
    )
}

/// WebAuthn counts fan-out per client subnet, which needs the trusted edge
/// address: installing it without PROXY v2 is refused.
#[test]
fn webauthn_needs_the_trusted_client_address() {
    let (key, fanout) = webauthn_parts();
    let mut app = app_with(config(), &Audit::default());
    assert_eq!(
        app.install_webauthn(
            webauthn_fixture::registry(webauthn_fixture::CRED, &key),
            fanout
        ),
        Err(Rejection::new(503, "configuration"))
    );
}

/// Go wa_verified and wa_second_subnet_public: a verified assertion forwards;
/// on a route without the attestation floor a second subnet over the cap
/// still upgrades. With no attributed client the fan-out cannot count and
/// the fast path is withheld (scored, no record).
#[test]
fn webauthn_assertion_upgrades_only_with_an_attributed_client() {
    let audit = Audit::default();
    let mut app = app_with(edge_config("127.0.0.1:32123".parse().unwrap()), &audit);
    let (key, fanout) = webauthn_parts();
    app.install_webauthn(
        webauthn_fixture::registry(webauthn_fixture::CRED, &key),
        fanout,
    )
    .unwrap();
    let assertion = |counter| {
        vec![(
            "WebAuthn-Assertion".to_owned(),
            webauthn_fixture::sign_assertion(&key, webauthn_fixture::CRED, counter),
        )]
    };
    let req = signed_request(&mut app, &assertion(1));
    app.decide(&req, || Ok(NOW_MS)).unwrap();
    assert!(records(&audit, "webauthn.assertion.verified").is_empty());
    assert_eq!(records(&audit, "runtime.decision").len(), 1);
    for (counter, client) in [(2, [198, 51, 100, 7]), (3, [203, 0, 113, 9])] {
        app.set_client(Some(client.into()));
        let req = signed_request(&mut app, &assertion(counter));
        assert!(matches!(
            app.decide(&req, || Ok(NOW_MS)).unwrap(),
            Decision::Origin { .. }
        ));
    }
    assert_eq!(records(&audit, "webauthn.assertion.verified").len(), 2);
    assert_eq!(records(&audit, "runtime.decision").len(), 1);
}

/// The native RIT profile carries the session in `Authorization: Bearer`, so
/// a Private Access Token cannot be presented there: installed issuers never
/// upgrade it and the request is scored.
#[test]
fn pat_is_not_presentable_under_the_native_profile() {
    let audit = Audit::default();
    let mut app = app_with(config(), &audit);
    app.install_pat_issuers(pat_fixture::verifier(pat_fixture::ISSUER_A_SPKI));
    let req = signed_request(&mut app, &[]);
    app.decide(&req, || Ok(NOW_MS)).unwrap();
    assert!(records(&audit, "privacypass.token.verified").is_empty());
    assert_eq!(records(&audit, "runtime.decision").len(), 1);
}

/// Go pat_verified / pat_double_spend under the browser profile, where the
/// Authorization field is free and covered by the request signature.
#[test]
fn pat_upgrades_once_under_the_browser_profile() {
    let audit = Audit::default();
    let mut app = app_with(configured("127.0.0.1:32123".parse().unwrap()), &audit);
    app.install_pat_issuers(pat_fixture::verifier(pat_fixture::ISSUER_A_SPKI));
    let token = pat_fixture::mint();
    let mut forwarded = Vec::new();
    // One fresh browser session per request: the second presents the same
    // token from another session and is a double spend.
    for _ in 0..2 {
        let bootstrap = browser_request("POST", "/_humanymous/browser/session", b"{}".to_vec());
        let Decision::Local(200, body) = app.decide(&bootstrap, || Ok(NOW_MS)).unwrap() else {
            panic!("bootstrap");
        };
        let mut client = ClientSession::from_json(scope(), &body, NOW_MS, 1000).unwrap();
        let mut req = browser_request("POST", "/effect?exact=1", b"original bytes".to_vec());
        req.headers.push(("Authorization".into(), token.clone()));
        signed(&mut client, &mut req, NOW_MS);
        let decision = app.decide(&req, || Ok(NOW_MS)).unwrap();
        forwarded.push(matches!(decision, Decision::Origin { .. }));
    }
    let verified = records(&audit, "privacypass.token.verified");
    assert_eq!(verified.len(), 1);
    assert_eq!(
        verified[0].triggered_rules,
        Some(vec!["privacy-pass".to_owned()])
    );
    assert_eq!(records(&audit, "runtime.decision").len(), 1);
    assert!(forwarded[0]);
}

/// Legacy `Server.GC`: maintenance sweeps the installed sweep detector and
/// fan-out tracker and refuses nothing when none is installed.
#[test]
fn maintenance_sweeps_installed_gate_kernels() {
    let mut bare = app_with(config(), &Audit::default());
    assert_eq!(bare.gc(NOW_MS), Ok(()));
    let mut app = app_with(
        edge_config("127.0.0.1:32123".parse().unwrap()),
        &Audit::default(),
    );
    let (key, fanout) = webauthn_parts();
    app.install_webauthn(
        webauthn_fixture::registry(webauthn_fixture::CRED, &key),
        fanout,
    )
    .unwrap();
    app.install_sweep(
        Detector::new(SweepConfig::new(Duration::from_secs(30), 8, 1024).unwrap()),
        Mode::Enforce,
    );
    assert_eq!(app.gc(NOW_MS), Ok(()));
}

/// A deny after authorization writes the gate fields first, then the
/// counter acknowledgement every authorized response carries.
#[test]
fn gate_deny_fields_precede_the_acknowledgement() {
    let gate = crate::gateway::application::GateResponse {
        status: DENY_STATUS,
        fields: vec![(INCIDENT_HEADER.0, INCIDENT_HEADER.1.to_owned())],
        body: DENY_BODY,
    };
    let incident = (INCIDENT_HEADER.0.to_owned(), INCIDENT_HEADER.1.to_owned());
    assert_eq!(gate.with_ack(None).unwrap(), vec![incident.clone()]);
    let mut req = request("POST", "/effect", vec![]);
    req.headers
        .push(("X-Humanymous-Token".into(), "A".repeat(43)));
    let ack = crate::gateway::browser::Ack::new(&[7; 32], &req, 3, 1);
    let mut expected = vec![incident];
    expected.extend(ack.headers(DENY_STATUS).unwrap());
    assert_eq!(gate.with_ack(Some(&ack)).unwrap(), expected);
}

/// Two signed requests of one session; `second` edits the second request's
/// fields before it is signed. Returns the second decision.
fn same_session_twice(
    app: &mut Application,
    second: impl FnOnce(&mut Request),
    between: impl FnOnce(),
) -> Decision {
    let cfg = config();
    let issued = issue(app, NOW_MS);
    let (id, key) = (
        issued["session"].as_str().unwrap().to_owned(),
        issued["key"].as_str().unwrap().to_owned(),
    );
    collected(app, &cfg, &issued, NOW_MS);
    let mut first = request("POST", "/effect?exact=1", b"original bytes".to_vec());
    sign(&cfg, &mut first, &id, &key, 2, NOW_MS);
    assert!(matches!(
        app.decide(&first, || Ok(NOW_MS)).unwrap(),
        Decision::Origin { .. }
    ));
    between();
    let mut req = request("POST", "/effect?exact=1", b"original bytes".to_vec());
    second(&mut req);
    sign(&cfg, &mut req, &id, &key, 3, NOW_MS);
    app.decide(&req, || Ok(NOW_MS)).unwrap()
}

fn fast_app(audit: &Audit, ttl: Duration) -> Application {
    let mut app = app_with(config(), audit);
    app.install_verdict_fast_path(ttl, 64, Mode::Enforce);
    app
}

/// Owner decision #4, option 1 (Go `verdictTokenGate` fast path, HR-28): a
/// scored ALLOW keeps the session's verdict on the server, so the session's
/// next request is forwarded without scoring, audited `enforcement.allow`
/// `pass` with the route preset as `config_version`. Without the fast path
/// installed every request is scored, as Go without a token key.
#[test]
fn sticky_allow_takes_the_fast_path_without_rescoring() {
    let audit = Audit::default();
    let mut app = fast_app(&audit, Duration::from_secs(60));
    let decision = same_session_twice(&mut app, |_| {}, || {});
    assert!(matches!(decision, Decision::Origin { .. }));
    assert_eq!(records(&audit, "runtime.decision").len(), 1);
    let fast = records(&audit, "enforcement.allow");
    assert_eq!(fast.len(), 1);
    assert_eq!(
        (
            fast[0].enforcement_action.as_str(),
            fast[0].enforcement_mode.as_str(),
            fast[0].config_version.as_str()
        ),
        ("pass", "enforce", "balanced")
    );
    assert!(!fast[0].request_id.is_empty());

    let audit = Audit::default();
    let mut app = app_with(config(), &audit);
    same_session_twice(&mut app, |_| {}, || {});
    assert_eq!(records(&audit, "runtime.decision").len(), 2);
    assert!(records(&audit, "enforcement.allow").is_empty());
}

/// A changed fingerprint or an expired verdict re-scores (Go clears the
/// cookie and falls through): the mismatch is recorded and the request gets
/// a full `runtime.decision`.
#[test]
fn sticky_allow_rescores_on_a_new_fingerprint_or_after_expiry() {
    let audit = Audit::default();
    let mut app = fast_app(&audit, Duration::from_secs(60));
    same_session_twice(
        &mut app,
        |req| {
            for (name, value) in &mut req.headers {
                if name.eq_ignore_ascii_case("user-agent") {
                    *value = "Mozilla/5.0 Firefox/130.0".into();
                }
            }
        },
        || {},
    );
    assert_eq!(records(&audit, "runtime.decision").len(), 2);
    let mismatch = records(&audit, "token.binding_mismatch");
    assert_eq!(mismatch.len(), 1);
    assert_eq!(
        mismatch[0].fail_reason,
        "verdict token binding_mismatch \u{2014} cleared, re-scoring"
    );

    let audit = Audit::default();
    let mut app = fast_app(&audit, Duration::from_millis(1));
    same_session_twice(
        &mut app,
        |_| {},
        || std::thread::sleep(Duration::from_millis(20)),
    );
    assert_eq!(records(&audit, "runtime.decision").len(), 2);
    assert_eq!(
        records(&audit, "token.binding_mismatch")[0].fail_reason,
        "verdict token expired \u{2014} cleared, re-scoring"
    );
}

/// Go TestSettingsVerdictTokenMonitorDoesNotFastPath at 981d25c: with the
/// overlay naming `gate.verdict_token: monitor` a valid verdict does not
/// fast-path; `would_fast_path` is recorded in mode `monitor` and the request
/// is scored. A non-enforcing route (`monitor` preset) keeps no verdict.
#[test]
fn verdict_fast_path_monitor_and_non_enforcing_routes_score() {
    use crate::gateway::settings::overlay::{Mode as Named, Overlay};
    let audit = Audit::default();
    let mut app = fast_app(&audit, Duration::from_secs(60));
    let overlay = Overlay {
        status: "active".into(),
        gates: [("gate.verdict_token".to_owned(), Named::Monitor)].into(),
        ..Default::default()
    };
    app.install_gate_overlay(std::sync::Arc::new(std::sync::RwLock::new(Some(overlay))));
    same_session_twice(&mut app, |_| {}, || {});
    assert_eq!(records(&audit, "runtime.decision").len(), 2);
    let would = records(&audit, "enforcement.allow");
    assert_eq!(
        (
            would.len(),
            would[0].enforcement_action.as_str(),
            would[0].enforcement_mode.as_str()
        ),
        (1, "would_fast_path", "monitor")
    );

    let audit = Audit::default();
    let mut cfg = config();
    cfg.routes[0].preset = Some("monitor".into());
    let mut app = app_with(cfg, &audit);
    app.install_verdict_fast_path(Duration::from_secs(60), 64, Mode::Enforce);
    same_session_twice(&mut app, |_| {}, || {});
    assert_eq!(records(&audit, "runtime.decision").len(), 2);
    assert!(records(&audit, "enforcement.allow").is_empty());
}
