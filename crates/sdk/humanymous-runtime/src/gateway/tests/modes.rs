//! Per-gate module modes from the settings overlay on the served path
//! (`application::modes`, owner decision #3), replayed against the Go capture
//! `tests/fixtures/gateway/gate-modes-go-golden.json` (legacy gate.go
//! `gateModuleMode` and settings_runtime_test.go at baseline 981d25c). Fixture
//! controls through `Application::decide`; no browser or product e2e ran.
use super::edge::{ban_store, edge_config, plain};
use super::gates::{NOW_MS, app_with, assert_denied, records, signed_request};
use super::*;
use crate::gateway::application::{ActiveOverlay, Gate, overlay_mode};
use crate::gateway::ban::gate::Mode;
use crate::gateway::settings::overlay::{
    BootInput, KNOWN_GATES, Mode as OverlayMode, Overlay, resolve,
};
use crate::network::history::sweep::{Config as SweepConfig, Detector};
use std::sync::RwLock;
use std::time::{Duration, Instant};

pub(super) fn golden() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/gateway/gate-modes-go-golden.json"
    ))
    .unwrap()
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Off => "off",
        Mode::Monitor => "monitor",
        Mode::Shadow => "shadow",
        Mode::Enforce => "enforce",
    }
}

pub(super) fn overlay(gates: &[(&str, &str)]) -> Overlay {
    Overlay {
        schema_version: "1.0.0".into(),
        overlay_id: "ovl_test".into(),
        status: "active".into(),
        gates: gates
            .iter()
            .map(|(id, mode)| ((*id).to_owned(), OverlayMode::parse(mode)))
            .collect(),
        ..Overlay::default()
    }
}

pub(super) fn slot(overlay: Option<Overlay>) -> ActiveOverlay {
    Arc::new(RwLock::new(overlay))
}

const GATES: [Gate; 6] = [
    Gate::Ban,
    Gate::SpoofHeader,
    Gate::VerdictToken,
    Gate::ReconSweep,
    Gate::Inject,
    Gate::AttestFloor,
];

fn modes(app: &Application) -> Value {
    Value::Object(
        GATES
            .iter()
            .map(|gate| (gate.id().to_owned(), json!(mode_name(app.gate_mode(*gate)))))
            .collect(),
    )
}

fn expected(go: &Value) -> Value {
    Value::Object(
        GATES
            .iter()
            .map(|gate| (gate.id().to_owned(), go[gate.id()].clone()))
            .collect(),
    )
}

/// Go defaults: every known gate enforces with no settings store, with an
/// empty store and for an unknown id; an active overlay overrides only the
/// gates it names (Go `settings.Resolve`).
#[test]
fn gate_modes_follow_go_defaults_and_the_active_overlay() {
    let go = golden();
    let known: Vec<&str> = go["known_gates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap())
        .collect();
    assert_eq!(known, KNOWN_GATES);
    let boot = BootInput {
        rate_window_sec: 10,
        rate_soft: 60,
        rate_hard: 120,
        ..BootInput::default()
    };
    let effective = resolve(&boot, None);
    for id in KNOWN_GATES {
        assert_eq!(
            effective.gate_mode(id).as_str(),
            go["resolve_nil_gates"][id]
        );
    }
    assert_eq!(
        effective.gate_mode("gate.unknown").as_str(),
        go["gate_mode_unknown_id"]
    );

    let mut app = app_with(config(), &Audit::default());
    assert_eq!(modes(&app), expected(&go["no_store_modes"]));
    let active = slot(None);
    app.install_gate_modes(active.clone());
    assert_eq!(modes(&app), expected(&go["empty_store_modes"]));
    let partial = overlay(&[
        ("gate.ban", "monitor"),
        ("gate.recon_sweep", "shadow"),
        ("gate.inject", "off"),
    ]);
    *active.write().unwrap() = Some(partial.clone());
    assert_eq!(modes(&app), expected(&go["partial_overlay_modes"]));
    // The kernel equals the frozen resolver for every known and unknown id.
    for id in KNOWN_GATES.iter().copied().chain(["gate.unknown"]) {
        let resolved = resolve(&boot, Some(&partial)).gate_mode(id);
        let ours = overlay_mode(Some(&partial), id).map_or("enforce", mode_name);
        assert_eq!(ours, resolved.as_str(), "{id}");
        assert_eq!(go["partial_overlay_modes"][id], ours, "{id}");
    }
    // An overlay that is not active contributes nothing, as in Resolve.
    let mut expired = partial;
    expired.status = "expired".into();
    assert_eq!(overlay_mode(Some(&expired), "gate.ban"), None);
    assert_eq!(
        resolve(&boot, Some(&expired)).gate_mode("gate.ban"),
        OverlayMode::Enforce
    );
    // Rolled back: the boot modes return.
    *active.write().unwrap() = None;
    assert_eq!(modes(&app), expected(&go["no_store_modes"]));
}

/// The startup edge modes are the boot modes an overlay entry overrides.
#[test]
fn startup_edge_modes_are_boot_modes_under_the_overlay() {
    let mut cfg = config();
    cfg.edge.spoof_header = Mode::Shadow;
    let mut app = app_with(cfg, &Audit::default());
    let active = slot(Some(overlay(&[("gate.ban", "off")])));
    app.install_gate_modes(active.clone());
    assert_eq!(app.gate_mode(Gate::SpoofHeader), Mode::Shadow);
    assert_eq!(app.gate_mode(Gate::Ban), Mode::Off);
    *active.write().unwrap() = Some(overlay(&[("gate.spoof_header", "enforce")]));
    assert_eq!(app.gate_mode(Gate::SpoofHeader), Mode::Enforce);
    assert_eq!(app.gate_mode(Gate::Ban), Mode::Enforce);
}

fn ratelimit_records(audit: &Audit) -> Vec<Value> {
    audit
        .records
        .lock()
        .unwrap()
        .iter()
        .filter(|r| r.event_type.starts_with("ratelimit."))
        .map(|r| {
            json!({"event_type":r.event_type,"action":r.enforcement_action,"mode":r.enforcement_mode,
                "route_class":r.route_class})
        })
        .collect()
}

fn go_ratelimit(go: &Value) -> Vec<Value> {
    go["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["event_type"].as_str().unwrap().starts_with("ratelimit."))
        .map(|r| {
            json!({"event_type":r["event_type"],"action":r["action"],"mode":r["mode"],
                "route_class":r["route_class"]})
        })
        .collect()
}

/// Go TestSettingsGateBanMonitorIsAuditOnly (and its shadow variant): with
/// `gate.ban` monitor or shadow in the overlay, a flood past the hard cutpoint
/// is never blocked, no ban is created, and the would-rate-limit and
/// would-auto-ban records carry the overlay mode. Switching the overlay entry
/// back to enforce on the next request blocks it.
#[test]
fn ban_gate_overlay_monitor_is_audit_only() {
    let go = golden();
    for (mode, case) in [("monitor", "ban_monitor"), ("shadow", "ban_shadow")] {
        let audit = Audit::default();
        let mut app = app_with(edge_config("127.0.0.1:32123".parse().unwrap()), &audit);
        let store = ban_store(1, 2);
        app.install_bans(store.clone(), Instant::now()).unwrap();
        let active = slot(Some(overlay(&[("gate.ban", mode)])));
        app.install_gate_modes(active.clone());
        app.set_client(Some("198.51.100.44".parse().unwrap()));
        for n in 0..4 {
            let decision = app.decide(&plain("/data", &[]), || Ok(NOW_MS));
            assert!(
                !matches!(decision, Ok(Decision::Gate(_))),
                "{mode} request {n} blocked"
            );
        }
        let gold = &go["monitor_cases"][case];
        assert_eq!(gold["codes"], json!([200, 200, 200, 200]));
        assert_eq!(gold["bans"], 0);
        assert!(store.lock().unwrap().list(Duration::ZERO).is_empty());
        assert_eq!(ratelimit_records(&audit), go_ratelimit(gold), "{mode}");
        // The next request reads the overlay again: enforce bans the flood.
        *active.write().unwrap() = Some(overlay(&[("gate.ban", "enforce")]));
        let decision = app.decide(&plain("/data", &[]), || Ok(NOW_MS)).unwrap();
        assert!(matches!(decision, Decision::Gate(_)), "{mode}");
    }
}

/// Go TestSettingsReconMonitorAuditsWithoutBlocking: with `gate.recon_sweep`
/// monitor in the overlay, the ninth session on one fingerprint is recorded
/// `would_block` and not denied; with no store (Go default enforce) it is
/// denied.
#[test]
fn recon_sweep_overlay_monitor_audits_without_blocking() {
    let go = golden();
    let detector = || Detector::new(SweepConfig::new(Duration::from_secs(30), 8, 1024).unwrap());
    let audit = Audit::default();
    let mut app = app_with(config(), &audit);
    app.install_sweep(detector(), Mode::Enforce);
    app.install_gate_modes(slot(Some(overlay(&[("gate.recon_sweep", "monitor")]))));
    for n in 1..=10 {
        let req = signed_request(&mut app, &[]);
        let decision = app.decide(&req, || Ok(NOW_MS)).unwrap();
        assert!(!matches!(decision, Decision::Gate(_)), "request {n}");
    }
    let gold = &go["monitor_cases"]["recon_monitor"];
    assert!(gold["codes"].as_array().unwrap().iter().all(|c| c == 200));
    let would: Vec<Value> = gold["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["event_type"] == "recon.decision_probing")
        .map(|r| json!([r["action"], r["mode"], r["fail_reason"]]))
        .collect();
    let ours: Vec<Value> = records(&audit, "recon.decision_probing")
        .iter()
        .map(|r| json!([r.enforcement_action, r.enforcement_mode, r.fail_reason]))
        .collect();
    assert_eq!(ours, would);
    assert!(!ours.is_empty());

    // No overlay: the boot mode (enforce) denies the ninth session.
    let audit = Audit::default();
    let mut app = app_with(config(), &audit);
    app.install_sweep(detector(), Mode::Enforce);
    let gold = &go["monitor_cases"]["recon_no_store_enforce"]["codes"];
    for n in 1..=9 {
        let req = signed_request(&mut app, &[]);
        let decision = app.decide(&req, || Ok(NOW_MS)).unwrap();
        if gold[n - 1] == 403 {
            assert_denied(decision);
        } else {
            assert!(!matches!(decision, Decision::Gate(_)), "request {n}");
        }
    }
}

/// The overlay is read per request: a spoof-header demotion published after
/// start takes effect on the next request and its rollback restores the deny.
#[test]
fn spoof_gate_reads_the_overlay_per_request() {
    let audit = Audit::default();
    let mut app = app_with(config(), &audit);
    let active = slot(None);
    app.install_gate_modes(active.clone());
    let forged = || plain("/effect", &[("X-Real-Ip", "10.0.0.1")]);
    assert!(matches!(
        app.decide(&forged(), || Ok(NOW_MS)),
        Ok(Decision::Gate(_))
    ));
    *active.write().unwrap() = Some(overlay(&[("gate.spoof_header", "monitor")]));
    assert!(!matches!(
        app.decide(&forged(), || Ok(NOW_MS)),
        Ok(Decision::Gate(_))
    ));
    let spoof: Vec<String> = audit
        .records
        .lock()
        .unwrap()
        .iter()
        .filter(|r| r.triggered_rules == Some(vec!["HR-27b".to_owned()]))
        .map(|r| r.enforcement_mode.clone())
        .collect();
    assert_eq!(spoof, ["enforce", "monitor"]);
    *active.write().unwrap() = None;
    assert!(matches!(
        app.decide(&forged(), || Ok(NOW_MS)),
        Ok(Decision::Gate(_))
    ));
}
