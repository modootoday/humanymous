//! `adminMetrics` against the Go capture
//! (`tests/fixtures/gateway/admin-metrics-go-golden.json`): a fresh settings
//! plane and the same plane after one applied overlay, as in Go's
//! `testSettingsServer` (routes `/` balanced and `/checkout` strict, the
//! legacy rate defaults, token key `test-token-key-32-bytes-long!!!!`).
use super::*;
use crate::audit::control::Scope;
use crate::audit::record::Record;
use crate::authority::decision::DurableAudit;
use crate::gateway::admin_approval::Role;
use crate::gateway::admin_plane::PREFIX;
use crate::gateway::admin_plane::policy::Policy;
use crate::gateway::admin_plane::settings::Settings;
use crate::gateway::ban::Store;
use crate::gateway::http::Request;
use crate::gateway::settings::configsign::BootPolicy;
use crate::gateway::settings::overlay::OverlayFileStore;
use crate::network::history::rate::{self, Limits};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::time::Duration;

static GOLDEN: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/gateway/admin-metrics-go-golden.json"
    ))
    .unwrap()
});
static NEXT: AtomicU64 = AtomicU64::new(0);
const KEY: &[u8] = b"test-token-key-32-bytes-long!!!!";
/// Families the port does not emit (module documentation).
const OMITTED: [&str; 11] = [
    "hmn_gate_audit_records_total",
    "hmn_gate_audit_checkpoints_total",
    "hmn_gate_audit_integrity_ok",
    "hmn_gate_audit_witnessed",
    "hmn_gate_operational_log_queue_depth",
    "hmn_gate_operational_log_dropped_total",
    "hmn_gate_operational_log_write_errors_total",
    "hmn_gate_operational_log_sink_healthy",
    "hmn_gate_goroutines",
    "hmn_gate_heap_alloc_bytes",
    "hmn_gate_sys_bytes",
];

#[derive(Clone, Default)]
struct Sink(Arc<Mutex<Vec<Record>>>);
impl DurableAudit for Sink {
    fn append(&mut self, record: Record) -> std::io::Result<()> {
        self.0.lock().unwrap().push(record);
        Ok(())
    }
}

struct Scratch(std::path::PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn clock(step: u64) -> Clock {
    Clock {
        monotonic: Duration::from_secs(step),
        unix: 1_800_000_000 + step,
    }
}

fn request(method: &str, sub: &str, role: Role, body: &str) -> Request {
    Request {
        method: method.into(),
        target: format!("{PREFIX}{sub}"),
        headers: vec![
            ("Host".to_owned(), "admin".to_owned()),
            (
                "Authorization".into(),
                format!("Bearer {}-token-0123456789", role.as_str()),
            ),
        ],
        body: body.as_bytes().to_vec(),
    }
}

/// Go `testSettingsServer` with the policy view and settings sharing one
/// kill flag and active overlay slot, as `gateway::service::admin` composes.
fn plane() -> (AdminPlane, Scratch, Arc<AtomicBool>) {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "humanymous-admin-metrics-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    std::fs::create_dir_all(&scratch.0).unwrap();
    let limits = Limits::new(64, 256, 128, Duration::from_secs(3600), 1 << 20, 64).unwrap();
    let config = rate::Config::new(Duration::from_secs(10), 60, 120, limits).unwrap();
    let bans = Arc::new(Mutex::new(Store::new(config, 64).unwrap()));
    let boot = BootPolicy {
        global_monitor: false,
        routes: BTreeMap::from([
            ("/".to_owned(), "balanced".to_owned()),
            ("/checkout".to_owned(), "strict".to_owned()),
        ]),
        rate_window_nanos: 0,
        rate_soft: 0,
        rate_hard: 0,
        hmac_key: KEY.to_vec(),
    };
    let kill = Arc::new(AtomicBool::new(false));
    let active = Arc::new(RwLock::new(None));
    let store = OverlayFileStore::open(&scratch.0).unwrap();
    let settings =
        Settings::new(boot.boot_input(false), kill.clone(), store).with_active_slot(active.clone());
    let plane = AdminPlane::new(
        "n1",
        Scope {
            installation: "installation".into(),
            node: "n1".into(),
        },
        KEY,
        bans,
        Box::new(Sink::default()),
    )
    .with_settings(settings, clock(0))
    .unwrap()
    .with_policy(Policy::new(boot, kill.clone(), active));
    for (role, id) in [(Role::Auditor, "aud-1"), (Role::Operator, "op-1")] {
        plane
            .auth()
            .add(&format!("{}-token-0123456789", role.as_str()), id, role)
            .unwrap();
    }
    (plane, scratch, kill)
}

fn scrape(plane: &AdminPlane, role: Role, step: u64) -> (Response, String) {
    let response = plane.handle(&request("GET", "metrics", role, ""), clock(step));
    let text = String::from_utf8(response.body.clone()).unwrap();
    (response, text)
}

/// The Go capture without the omitted families, with uptime and the
/// key-dependent config version masked.
fn expected(state: &str) -> String {
    let mut lines = Vec::new();
    for line in GOLDEN[state].as_str().unwrap().lines() {
        let name = line
            .trim_start_matches("# HELP ")
            .trim_start_matches("# TYPE ")
            .split([' ', '{'])
            .next()
            .unwrap();
        if !OMITTED.contains(&name) {
            lines.push(mask(line));
        }
    }
    lines.join("\n") + "\n"
}

fn mask(line: &str) -> String {
    if line.starts_with("hmn_gate_uptime_seconds ") {
        return "hmn_gate_uptime_seconds <uptime>".into();
    }
    if let Some(rest) = line.strip_prefix("hmn_gate_config_version{version=\"cfg-") {
        assert!(rest.ends_with("\"} 1"), "{line}");
        return "hmn_gate_config_version{version=\"cfg-<version>\"} 1".into();
    }
    line.to_owned()
}

fn masked(text: &str) -> String {
    text.lines().map(mask).collect::<Vec<_>>().join("\n") + "\n"
}

#[test]
fn golden_fresh_and_applied_expositions() {
    let (plane, _scratch, _) = plane();
    let (response, fresh) = scrape(&plane, Role::Auditor, 1);
    assert_eq!(response.status, 200);
    assert_eq!(
        response.content_type,
        GOLDEN["content_type"].as_str().unwrap()
    );
    assert_eq!(masked(&fresh), expected("fresh"), "{fresh}");
    let version = plane.settings().unwrap().effective().config_version;
    let body = format!(r#"{{"parentConfigVersion":"{version}","scoring":{{"challengeAt":20}}}}"#);
    let applied = plane.handle(
        &request("POST", "settings/overlays", Role::Operator, &body),
        clock(2),
    );
    assert_eq!(
        applied.status,
        200,
        "{}",
        String::from_utf8_lossy(&applied.body)
    );
    let (_, after) = scrape(&plane, Role::Auditor, 3);
    assert_eq!(masked(&after), expected("applied"), "{after}");
    // The config version label is the policy view's version, which the
    // overlay changed.
    let label = |text: &str| {
        text.lines()
            .find(|line| line.starts_with("hmn_gate_config_version{"))
            .unwrap()
            .to_owned()
    };
    assert_ne!(label(&fresh), label(&after));
    assert!(after.contains(&format!(
        "hmn_gate_config_version{{version=\"{}\"}} 1",
        plane.settings().unwrap().effective().config_version
    )));
}

#[test]
fn uptime_kill_switch_bans_and_label_escaping() {
    let (plane, _scratch, kill) = plane();
    let (_, text) = scrape(&plane, Role::Auditor, 42);
    assert!(
        text.contains("\nhmn_gate_uptime_seconds 42\n")
            || text.starts_with("# HELP hmn_gate_uptime_seconds")
    );
    assert!(text.contains("hmn_gate_uptime_seconds 42\n"), "{text}");
    kill.store(true, Ordering::SeqCst);
    let (_, text) = scrape(&plane, Role::Auditor, 43);
    assert!(text.contains("hmn_gate_killswitch 1\n"), "{text}");
    assert!(text.contains("hmn_gate_monitor 1\n"), "{text}");
    let escape = &GOLDEN["label_escape"];
    assert_eq!(
        label(escape["input"].as_str().unwrap()),
        escape["output"].as_str().unwrap()
    );
    for name in OMITTED {
        assert!(!text.contains(name), "{name} emitted");
    }
}

#[test]
fn any_role_reads_and_anonymous_is_refused() {
    let (plane, _scratch, _) = plane();
    assert_eq!(scrape(&plane, Role::Operator, 1).0.status, 200);
    let anonymous = Request {
        method: "GET".into(),
        target: format!("{PREFIX}metrics"),
        headers: vec![("Host".to_owned(), "admin".to_owned())],
        body: Vec::new(),
    };
    assert_eq!(plane.handle(&anonymous, clock(1)).status, 404);
    assert_eq!(
        plane
            .handle(&request("POST", "metrics", Role::Operator, ""), clock(1))
            .status,
        404
    );
}

#[test]
fn a_bare_plane_reports_zeros_and_no_config_version() {
    let limits = Limits::new(64, 256, 128, Duration::from_secs(3600), 1 << 20, 64).unwrap();
    let config = rate::Config::new(Duration::from_secs(10), 60, 120, limits).unwrap();
    let bans = Arc::new(Mutex::new(Store::new(config, 64).unwrap()));
    let plane = AdminPlane::new(
        "n1",
        Scope {
            installation: "installation".into(),
            node: "n1".into(),
        },
        KEY,
        bans,
        Box::new(Sink::default()),
    );
    plane
        .auth()
        .add("auditor-token-0123456789", "aud-1", Role::Auditor)
        .unwrap();
    let (response, text) = scrape(&plane, Role::Auditor, 0);
    assert_eq!(response.status, 200);
    assert!(
        text.contains("hmn_gate_killswitch 0\n")
            && text.contains("hmn_gate_settings_overlay_active 0\n")
    );
    assert!(text.contains("hmn_gate_settings_apply_total{result=\"applied\"} 0\n"));
    assert!(!text.contains("hmn_gate_config_version"), "{text}");
}
