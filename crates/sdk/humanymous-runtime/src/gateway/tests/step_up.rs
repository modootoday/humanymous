//! Owner decision #4 step-up proof on the served path (ceiling-guard #1):
//! the gateway Pass (`application::pass`) on RIT-authenticated sessions, the
//! server-side step-up proof a solve records, and the attestation floor on an
//! `attested` route (legacy internal/gate gates.go `applyVerdict` and
//! control.go `handleStepUp` at baseline 981d25c). Fixture controls only: no
//! ordinary browser, human, assistive technology or product e2e ran.
use super::*;
use humanymous_core::challenge::proof_of_work;

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

fn attested_config(pass: bool) -> Configuration {
    let mut cfg = config();
    cfg.routes[0].preset = Some("attested".into());
    if pass {
        cfg.pass = Some(serde_json::from_value(json!({})).unwrap());
    }
    cfg
}

/// One session with its signing state; `next` signs with the next counter.
struct Client {
    id: String,
    key: String,
    counter: u64,
}

impl Client {
    fn start(app: &mut Application, cfg: &Configuration) -> Self {
        let issued = issue(app, NOW_MS);
        collected(app, cfg, &issued, NOW_MS);
        Self {
            id: issued["session"].as_str().unwrap().to_owned(),
            key: issued["key"].as_str().unwrap().to_owned(),
            counter: 1,
        }
    }

    fn send(
        &mut self,
        app: &mut Application,
        cfg: &Configuration,
        method: &str,
        target: &str,
        body: Vec<u8>,
        edit: impl FnOnce(&mut Request),
    ) -> Decision {
        self.counter += 1;
        let mut req = request(method, target, body);
        edit(&mut req);
        sign(cfg, &mut req, &self.id, &self.key, self.counter, NOW_MS);
        app.decide(&req, || Ok(NOW_MS)).unwrap()
    }

    fn effect(&mut self, app: &mut Application, cfg: &Configuration) -> Decision {
        self.send(
            app,
            cfg,
            "POST",
            "/effect?exact=1",
            b"original bytes".to_vec(),
            |_| {},
        )
    }

    fn pass(
        &mut self,
        app: &mut Application,
        cfg: &Configuration,
        step: &str,
        body: Value,
    ) -> Value {
        let method = if step == "new" { "GET" } else { "POST" };
        let bytes = if step == "new" {
            Vec::new()
        } else {
            body.to_string().into_bytes()
        };
        match self.send(
            app,
            cfg,
            method,
            &format!("/_humanymous/pass/{step}"),
            bytes,
            |_| {},
        ) {
            Decision::Local(200, bytes) => serde_json::from_slice(&bytes).unwrap(),
            Decision::Local(status, bytes) => {
                panic!("{step}: {status} {}", String::from_utf8_lossy(&bytes))
            }
            _ => panic!("{step}: not a local answer"),
        }
    }

    /// Issue a Pass instance, pay its proof of work and solve it.
    fn solve_pass(&mut self, app: &mut Application, cfg: &Configuration) -> Value {
        let issued = self.pass(app, cfg, "new", Value::Null);
        let pow = &issued["preflight"]["pow"];
        let nonce = proof_of_work::solve(
            pow["seed"].as_str().unwrap(),
            i32::try_from(pow["difficulty"].as_i64().unwrap()).unwrap(),
            1 << 26,
        )
        .unwrap();
        let paid = self.pass(
            app,
            cfg,
            "pow",
            json!({"bucket": pow["bucket"], "nonce": nonce, "challengeNonce": issued["challengeNonce"]}),
        );
        assert_eq!(paid, json!({"ok": true}));
        let challenge = &issued["challenge"];
        let n = challenge["n"].as_i64().unwrap();
        let center = challenge["center"].as_i64().unwrap();
        let offsets: Vec<i64> = challenge["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| (center - row["keyIndex"].as_i64().unwrap()).rem_euclid(n))
            .collect();
        self.pass(
            app,
            cfg,
            "solve",
            json!({"challengeNonce": issued["challengeNonce"], "offsets": offsets,
                "trusted": true, "keys": 3, "keyDurs": [120, 140, 130, 150]}),
        )
    }
}

/// Ceiling-guard #1 end to end: on an `attested` route a scored ALLOW is held
/// with 401 (`step_up_challenge`, verdict and score recorded unchanged); the
/// session solves the gateway Pass over RIT-signed requests, which records
/// `token.issued` and a server-side step-up proof; the same request then
/// reaches the origin. Nothing is sent to the client but the Pass answers.
#[test]
fn attested_route_holds_until_the_session_solves_the_gateway_pass() {
    let audit = Audit::default();
    let cfg = attested_config(true);
    let mut app = Application::new(
        attested_config(true),
        Default::default(),
        Box::new(audit.clone()),
    )
    .unwrap();
    let mut client = Client::start(&mut app, &cfg);
    let held = client.effect(&mut app, &cfg);
    let Decision::Local(401, body) = held else {
        panic!("not held for step-up");
    };
    assert_eq!(
        serde_json::from_slice::<Value>(&body).unwrap(),
        json!({"verdict":"ALLOW","step_up":"required"})
    );
    let decision = records(&audit, "runtime.decision").pop().unwrap();
    assert_eq!(
        (
            decision.verdict.as_str(),
            decision.enforcement_action.as_str()
        ),
        ("ALLOW", "step_up_challenge")
    );

    let solved = client.solve_pass(&mut app, &cfg);
    assert_eq!(solved["ok"], json!(true), "{solved}");
    assert!(
        solved.get("stepUpReceipt").is_none(),
        "no receipt on the wire"
    );
    let minted = records(&audit, "token.issued");
    assert_eq!(minted.len(), 1);
    assert_eq!(minted[0].fail_reason, "step-up proof minted (Pass solved)");

    assert!(matches!(
        client.effect(&mut app, &cfg),
        Decision::Origin { .. }
    ));
    assert_eq!(
        records(&audit, "runtime.decision")
            .pop()
            .unwrap()
            .enforcement_action,
        "ALLOW"
    );
}

/// The proof is bound to the client (Go `tokenBind`): another fingerprint on
/// the same session is held again. With `gate.attest_floor: monitor` the
/// floor records `would_challenge` and forwards (Go
/// TestSettingsAttestationFloorMonitorAuditsWithoutChallenging).
#[test]
fn step_up_is_bound_to_the_client_and_monitor_floor_only_records() {
    let audit = Audit::default();
    let cfg = attested_config(true);
    let mut app = Application::new(
        attested_config(true),
        Default::default(),
        Box::new(audit.clone()),
    )
    .unwrap();
    let mut client = Client::start(&mut app, &cfg);
    assert_eq!(client.solve_pass(&mut app, &cfg)["ok"], json!(true));
    let elsewhere = client.send(
        &mut app,
        &cfg,
        "POST",
        "/effect?exact=1",
        b"original bytes".to_vec(),
        |req| {
            for (name, value) in &mut req.headers {
                if name.eq_ignore_ascii_case("user-agent") {
                    *value = "Mozilla/5.0 Firefox/130.0".into();
                }
            }
        },
    );
    assert!(matches!(elsewhere, Decision::Local(401, _)));

    use crate::gateway::settings::overlay::{Mode as Named, Overlay};
    let audit = Audit::default();
    let mut app = Application::new(
        attested_config(true),
        Default::default(),
        Box::new(audit.clone()),
    )
    .unwrap();
    let overlay = Overlay {
        status: "active".into(),
        gates: [("gate.attest_floor".to_owned(), Named::Monitor)].into(),
        ..Default::default()
    };
    app.install_gate_overlay(std::sync::Arc::new(std::sync::RwLock::new(Some(overlay))));
    let mut client = Client::start(&mut app, &cfg);
    assert!(matches!(
        client.effect(&mut app, &cfg),
        Decision::Origin { .. }
    ));
    let would = records(&audit, "enforcement.challenge.issued");
    assert_eq!(
        (
            would.len(),
            would[0].enforcement_action.as_str(),
            would[0].enforcement_mode.as_str()
        ),
        (1, "would_challenge", "monitor")
    );
    assert_eq!(
        would[0].fail_reason,
        "scoring allow would require step-up (audit only)"
    );
}

/// Without the `pass` object the `attested` preset is refused (no step-up
/// source) and the Pass API answers as any reserved route (403).
#[test]
fn attested_needs_the_pass_object_and_pass_routes_are_reserved_without_it() {
    assert!(attested_config(false).validate().is_err());
    assert!(attested_config(true).validate().is_ok());
    let mut bad = config();
    bad.pass = Some(serde_json::from_value(json!({"step_up_ttl_ms":0})).unwrap());
    assert!(bad.validate().is_err());

    let cfg = config();
    let mut app =
        Application::new(config(), Default::default(), Box::new(Audit::default())).unwrap();
    let issued = issue(&mut app, NOW_MS);
    let mut req = request("GET", "/_humanymous/pass/new", Vec::new());
    sign(
        &cfg,
        &mut req,
        issued["session"].as_str().unwrap(),
        issued["key"].as_str().unwrap(),
        1,
        NOW_MS,
    );
    assert_eq!(
        app.decide(&req, || Ok(NOW_MS)).err(),
        Some(crate::gateway::Rejection::new(403, "reserved_route"))
    );
}
