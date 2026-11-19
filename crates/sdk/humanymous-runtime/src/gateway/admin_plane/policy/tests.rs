//! Differential ports of the policy read route and the console page of legacy
//! internal/gate/admin.go (`adminPolicy`, `adminConsole`) and console.go at
//! baseline 981d25c against outputs recorded from the Go oracle
//! (`tests/fixtures/gateway/admin-policy-console-go-golden.json`). Requests go
//! through `AdminPlane::handle`, like Go `adminDo` through the admin handler.
use super::*;
use crate::audit::control::Scope;
use crate::audit::record::Record;
use crate::authority::decision::DurableAudit;
use crate::gateway::admin_approval::Role;
use crate::gateway::admin_plane::{Clock, PREFIX};
use crate::gateway::ban::Store;
use crate::gateway::http::Request;
use crate::network::history::rate::{Config, Limits};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

static GOLDEN: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/gateway/admin-policy-console-go-golden.json"
    ))
    .unwrap()
});

const TOKEN_KEY: &[u8] = b"fm11-r19-token-key-0123456789abcdef";

const CLOCK: Clock = Clock {
    monotonic: Duration::from_secs(5),
    unix: 1_800_000_000,
};

/// Records appended by the plane.
#[derive(Clone, Default)]
struct Journal(Arc<Mutex<Vec<Record>>>);

impl DurableAudit for Journal {
    fn append(&mut self, record: Record) -> io::Result<()> {
        self.0.lock().unwrap().push(record);
        Ok(())
    }
}

struct Stack {
    plane: AdminPlane,
    journal: Journal,
    kill: Arc<AtomicBool>,
    active: Arc<RwLock<Option<Overlay>>>,
}

fn token(role: &str) -> String {
    format!("{role}-token-0123456789")
}

/// The capture's `policy_routes` server: every preset kind plus an unknown
/// name, `RateWindow` 1m, `RateSoft` 998, `RateHard` 1000.
fn routes_boot() -> BootPolicy {
    BootPolicy {
        global_monitor: false,
        routes: BTreeMap::from(
            [
                ("/api", "strict"),
                ("/login", "attested"),
                ("/docs", "monitor"),
                ("/static", "off"),
                ("/x", "bogus"),
            ]
            .map(|(prefix, preset)| (prefix.to_owned(), preset.to_owned())),
        ),
        rate_window_nanos: 60_000_000_000,
        rate_soft: 998,
        rate_hard: 1000,
        hmac_key: TOKEN_KEY.to_vec(),
    }
}

/// The capture's `policy_defaults_monitor` server.
fn monitor_boot() -> BootPolicy {
    BootPolicy {
        global_monitor: true,
        hmac_key: TOKEN_KEY.to_vec(),
        ..BootPolicy::default()
    }
}

fn plane(journal: &Journal) -> AdminPlane {
    let limits = Limits::new(64, 64, 128, Duration::from_secs(60), 1 << 20, 64).unwrap();
    let rate = Config::new(Duration::from_secs(60), 998, 1000, limits).unwrap();
    let plane = AdminPlane::new(
        "test",
        Scope {
            installation: "installation".into(),
            node: "test".into(),
        },
        b"k",
        Arc::new(Mutex::new(Store::new(rate, 64).unwrap())),
        Box::new(journal.clone()),
    );
    for role in [Role::Auditor, Role::Operator, Role::Approver, Role::Dpo] {
        plane
            .auth()
            .add(&token(role.as_str()), &format!("{}-1", role.as_str()), role)
            .unwrap();
    }
    plane
}

fn stack(boot: BootPolicy) -> Stack {
    let journal = Journal::default();
    let kill = Arc::new(AtomicBool::new(false));
    let active = Arc::new(RwLock::new(None));
    let plane = plane(&journal).with_policy(Policy::new(boot, kill.clone(), active.clone()));
    Stack {
        plane,
        journal,
        kill,
        active,
    }
}

impl Stack {
    /// Go `adminDo`.
    fn call(&self, role: &str, method: &str, sub: &str) -> Response {
        call(&self.plane, role, method, sub)
    }
}

fn call(plane: &AdminPlane, role: &str, method: &str, sub: &str) -> Response {
    let mut headers = vec![("Host".to_owned(), "admin".to_owned())];
    if !role.is_empty() {
        headers.push(("Authorization".into(), format!("Bearer {}", token(role))));
    }
    let request = Request {
        method: method.into(),
        target: format!("{PREFIX}{sub}"),
        headers,
        body: Vec::new(),
    };
    plane.handle(&request, CLOCK)
}

fn body(response: &Response) -> &str {
    std::str::from_utf8(&response.body).unwrap()
}

/// Status, content type and body of a captured response.
fn assert_matches(response: &Response, golden: &Value, name: &str) {
    assert_eq!(response.status, golden["status"], "{name} status");
    assert_eq!(
        response.content_type, golden["content_type"],
        "{name} content type"
    );
    assert_eq!(body(response), golden["body"], "{name} body");
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Go TestAdminConsoleAndFeatures (policy half) and R141: two policy reads
/// are byte-identical to the capture (routes in the capture's prefix order),
/// meta-audited like Go, follow the live kill switch, and have no write verb.
#[test]
fn go_policy_view_matches_capture() {
    let golden = &GOLDEN["policy_routes"];
    let s = stack(routes_boot());
    let first = s.call("auditor", "GET", "policy");
    let second = s.call("auditor", "GET", "policy");
    assert_matches(&first, &golden["first"], "first");
    assert_matches(&second, &golden["second"], "second");
    let records: Vec<Value> = s
        .journal
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|record| {
            serde_json::json!({
                "event_type": record.event_type,
                "actor_kind": record.actor.kind,
                "tenant_id": record.tenant_id,
                "route_class": record.route_class,
                "action": record.enforcement_action,
                "fail_reason": record.fail_reason,
                "key_id": record.key_id,
            })
        })
        .collect();
    assert_eq!(Value::Array(records), golden["records"]);

    s.kill.store(true, Ordering::SeqCst);
    assert_matches(
        &s.call("operator", "GET", "policy"),
        &GOLDEN["policy_routes_kill"],
        "kill",
    );
    s.kill.store(false, Ordering::SeqCst);
    assert_matches(
        &s.call("operator", "POST", "policy"),
        &GOLDEN["policy_post_operator"],
        "post",
    );
    assert_matches(
        &s.call("", "GET", "policy"),
        &GOLDEN["policy_unauth"],
        "unauth",
    );
    assert_matches(
        &s.call("dpo", "GET", "policy?x=1"),
        &GOLDEN["policy_query"],
        "query",
    );
}

/// Defaults (no routes, zero rate configuration) under the boot global
/// monitor: every row stops enforcing and the default thresholds apply.
#[test]
fn go_policy_defaults_under_global_monitor_match_capture() {
    let s = stack(monitor_boot());
    assert_matches(
        &s.call("auditor", "GET", "policy"),
        &GOLDEN["policy_defaults_monitor"],
        "defaults",
    );
}

/// The version follows the active overlay like Go `SettingsEffective`, while
/// the view reports the boot routes and posture and changes nothing.
#[test]
fn policy_view_keys_the_version_to_the_active_overlay_and_only_reads() {
    let s = stack(routes_boot());
    let plain: Value = serde_json::from_slice(&s.call("auditor", "GET", "policy").body).unwrap();
    let overlay = Overlay {
        overlay_id: "ovl_r19".into(),
        status: "active".into(),
        routes: BTreeMap::from([("/api".to_owned(), "monitor".to_owned())]),
        ..Overlay::default()
    };
    *s.active.write().unwrap() = Some(overlay.clone());
    let with: Value = serde_json::from_slice(&s.call("auditor", "GET", "policy").body).unwrap();
    let want = routes_boot().config_version(false, Some(&overlay));
    assert_eq!(with["configVersion"], want.as_str());
    assert_ne!(with["configVersion"], plain["configVersion"]);
    assert_eq!(with["routes"], plain["routes"], "boot routes are reported");
    assert_eq!(*s.active.read().unwrap(), Some(overlay));
    assert!(!s.kill.load(Ordering::SeqCst));
}

/// Without an attached policy the route is the plain 404, as before.
#[test]
fn policy_route_is_not_served_without_a_policy() {
    let plane = plane(&Journal::default());
    let w = call(&plane, "auditor", "GET", "policy");
    assert_eq!((w.status, body(&w)), (404, "404 page not found\n"));
}

/// The console page is Go `styledConsoleHTML` byte for byte, served as
/// `text/html` at the bare prefix and at `console` to any authenticated role.
/// Deliberate difference: an unauthenticated request (Go: 200) and a POST
/// (Go: 200) keep the 404, and no bearer token is ever injected.
#[test]
fn go_console_page_matches_capture() {
    let golden = &GOLDEN["console"];
    assert_eq!(console::page().len() as u64, golden["styled_len"]);
    assert_eq!(sha256(console::page().as_bytes()), golden["styled_sha256"]);
    let s = stack(routes_boot());
    for (role, sub, name) in [
        ("auditor", "console", "auditor_console"),
        ("auditor", "", "unauth_root"),
        ("operator", "console", "unauth_console"),
        ("dpo", "", "unauth_root"),
    ] {
        let w = s.call(role, "GET", sub);
        let want = &golden[name];
        assert_eq!(w.status, want["status"], "{role} {sub}");
        assert_eq!(w.content_type, want["content_type"], "{role} {sub}");
        assert_eq!(w.body.len() as u64, want["body_len"], "{role} {sub}");
        assert_eq!(sha256(&w.body), want["body_sha256"], "{role} {sub}");
        assert!(body(&w).contains("Ledger"));
        assert!(!body(&w).contains("window.__HMN_TOKEN="));
    }
    assert_eq!(golden["dev_token_injected"], true, "Go injected a token");
    let tail = s.call("auditor", "GET", "console/");
    assert_eq!(tail.status, golden["unauth_console_tail"]["status"]);
    assert_eq!(body(&tail), golden["unauth_console_tail"]["body"]);

    // Go served these without authentication; the plane keeps the 404.
    assert_eq!(golden["unauth_console"]["status"], 200);
    assert_eq!(golden["unauth_post_status"], 200);
    for (method, sub) in [("GET", "console"), ("GET", ""), ("POST", "console")] {
        let w = s.call("", method, sub);
        assert_eq!((w.status, body(&w)), (404, "404 page not found\n"));
    }
    let w = s.call("operator", "POST", "console");
    assert_eq!(w.status, 404, "console has no write verb");
    assert!(
        s.journal
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|record| record.event_type == "admin.auth_fail")
    );
}
