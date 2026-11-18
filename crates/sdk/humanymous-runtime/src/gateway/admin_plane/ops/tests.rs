//! Ports of legacy cmd/server/ops_test.go (baseline 981d25c) onto the admin
//! plane's ops routes, plus differential parity with a Go oracle capture
//! (`tests/fixtures/gateway/ops-go-golden.json`).
//!
//! Go asserted the routes on the public Core server (`a.routes()`). Here the
//! same assertions run against the admin plane's loopback listener, and
//! `public_listener_never_serves_ops_routes` pins that the public application
//! listener exposes none of them (sot:root/adapter-ecosystem, management and
//! origin isolation; plan B09).
use super::*;
use crate::audit::control::Scope;
use crate::audit::record::Record;
use crate::authority::decision::DurableAudit;
use crate::gateway::admin_approval::Role;
use crate::gateway::admin_plane::AdminPlane;
use crate::gateway::ban::Store;
use crate::gateway::service::ShutdownRequest;
use crate::network::history::rate::{Config, Limits};
use humanymous_core::model::correction::{CanaryBudget, Outcome};
use humanymous_core::signals::report::SessionReport;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io;
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

const TOKEN: &str = "s3cret";
const NOT_FOUND: &str = "404 page not found\n";
const TEXT: &str = "text/plain; charset=utf-8";

fn golden() -> &'static Value {
    static GOLDEN: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    GOLDEN.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../tests/fixtures/gateway/ops-go-golden.json"
        ))
        .unwrap()
    })
}

#[derive(Default)]
struct NullAudit;

impl DurableAudit for NullAudit {
    fn append(&mut self, _: Record) -> io::Result<()> {
        Ok(())
    }
}

/// Stored session reports (Go `store.Get` returns a copy).
#[derive(Default)]
struct Reports(Mutex<BTreeMap<String, SessionReport>>);

impl Sessions for Reports {
    fn report(&self, session: &str) -> Option<SessionReport> {
        self.0.lock().unwrap().get(session).cloned()
    }
}

/// The six frozen reports the oracle stored.
fn golden_reports() -> Arc<Reports> {
    let stored = golden()["stored_reports"].as_object().unwrap();
    assert_eq!(stored.len(), 6);
    let reports = stored
        .iter()
        .map(|(id, report)| (id.clone(), serde_json::from_value(report.clone()).unwrap()))
        .collect();
    Arc::new(Reports(Mutex::new(reports)))
}

struct NoModel;
impl CounterSource for NoModel {}

struct Model(Controller);
impl CounterSource for Model {
    fn controller(&self) -> Option<&Controller> {
        Some(&self.0)
    }
}

/// Go counters_test.go: gamma 0 freezes theta so the canary trips.
fn tripped_controller() -> Controller {
    let controller = Controller::new(0.005, 0.5, 0.0).unwrap();
    arm(&controller);
    for _ in 0..400 {
        controller
            .observe_outcome(Outcome::PassSolved, 0.9, false)
            .unwrap();
    }
    controller
}

fn armed_controller() -> Controller {
    let controller = Controller::new(0.005, 0.5, 0.05).unwrap();
    arm(&controller);
    for _ in 0..37 {
        controller
            .observe_outcome(Outcome::PassSolved, 0.3, false)
            .unwrap();
    }
    controller
}

fn arm(controller: &Controller) {
    controller
        .arm_canary(
            CanaryBudget {
                max_human_fp: 0.10,
                probation_humans: 100_000,
                ..CanaryBudget::default()
            },
            Some(Arc::new(|_: &str| {})),
        )
        .unwrap();
}

fn plane_with(token: Option<&str>, counters: Arc<dyn CounterSource>) -> AdminPlane {
    let limits = Limits::new(64, 64, 128, Duration::from_secs(60), 1 << 20, 64).unwrap();
    let rate = Config::new(Duration::from_secs(60), 998, 1000, limits).unwrap();
    let bans = Arc::new(Mutex::new(Store::new(rate, 64).unwrap()));
    let scope = Scope {
        installation: "installation".into(),
        node: "node".into(),
    };
    let plane = AdminPlane::new("test", scope, b"k", bans, Box::new(NullAudit));
    let token = resolve_token(token.unwrap_or(""), None);
    plane.with_ops(Ops::new(
        "dev",
        token,
        Engine::default(),
        golden_reports(),
        counters,
    ))
}

/// Go `configureOps(token)` then `a.routes()`, here the admin plane.
fn plane(token: Option<&str>) -> AdminPlane {
    plane_with(token, Arc::new(NoModel))
}

fn request(method: &str, target: &str, authorization: &str) -> Request {
    let mut headers = vec![("Host".into(), "admin".into())];
    if !authorization.is_empty() {
        headers.push(("Authorization".into(), authorization.into()));
    }
    Request {
        method: method.into(),
        target: target.into(),
        headers,
        body: Vec::new(),
    }
}

fn get(plane: &AdminPlane, target: &str, authorization: &str) -> Response {
    let clock = Clock {
        monotonic: Duration::ZERO,
        unix: 1_800_000_000,
    };
    plane.handle(&request("GET", target, authorization), clock)
}

fn body(response: &Response) -> &str {
    std::str::from_utf8(&response.body).unwrap()
}

/// The Go public path for an ops route, mapped onto [`OPS_PREFIX`].
fn ops_target(go_path: &str) -> String {
    let sub = go_path
        .strip_prefix("/api/")
        .or_else(|| go_path.strip_prefix('/'))
        .unwrap();
    format!("{OPS_PREFIX}{sub}")
}

/// Go `handleCounters` also writes the Go-runtime `goroutines`,
/// `heapAllocBytes` and `numGC` values, which the port does not emit.
fn without_go_runtime_fields(go_body: &str) -> String {
    let mut text = go_body.to_owned();
    for key in ["\"goroutines\":", "\"heapAllocBytes\":", "\"numGC\":"] {
        let start = text.find(key).unwrap();
        let end = start + text[start..].find(',').unwrap();
        text.replace_range(start..=end, "");
    }
    text
}

/// Go TestHealthzAlwaysOn: liveness answers 200 without any token configured
/// or presented.
#[test]
fn healthz_always_on() {
    let response = get(&plane(None), &format!("{OPS_PREFIX}healthz"), "");
    assert_eq!(response.status, 200);
    assert_eq!(response.content_type, super::super::JSON);
    assert_eq!(
        body(&response),
        "{\"status\":\"ok\",\"uptimeSec\":0,\"version\":\"dev\"}\n"
    );
}

/// Go TestOpsEndpointsAbsentWithoutToken: explain and counters are 404 when no
/// operator token is configured, whatever bearer is presented. A plane
/// without the ops routes attached serves none of them, healthz included.
#[test]
fn ops_endpoints_absent_without_token() {
    let plane = plane(None);
    for path in ["explain/abc", "counters"] {
        for authorization in ["", "Bearer ", "Bearer s3cret", "s3cret"] {
            let response = get(&plane, &format!("{OPS_PREFIX}{path}"), authorization);
            assert_eq!(
                (response.status, body(&response)),
                (404, NOT_FOUND),
                "{path}"
            );
            assert_eq!(response.content_type, TEXT);
        }
    }
    let bare = plane_without_ops();
    for path in ["healthz", "explain/abc", "counters"] {
        let response = get(&bare, &format!("{OPS_PREFIX}{path}"), "Bearer s3cret");
        assert_eq!(
            (response.status, body(&response)),
            (404, NOT_FOUND),
            "{path}"
        );
    }
}

fn plane_without_ops() -> AdminPlane {
    let mut plane = plane(None);
    plane.ops = None;
    plane
}

/// Go TestOpsEndpointsGated: with a token configured, a missing or wrong
/// bearer is 404 and the exact bearer is 200.
#[test]
fn ops_endpoints_gated() {
    let plane = plane(Some(TOKEN));
    let target = format!("{OPS_PREFIX}counters");
    for authorization in ["", "Bearer wrong", "bearer s3cret", "Bearer  s3cret"] {
        let response = get(&plane, &target, authorization);
        assert_eq!((response.status, body(&response)), (404, NOT_FOUND));
    }
    let response = get(&plane, &target, "Bearer s3cret");
    assert_eq!(response.status, 200, "{}", body(&response));
    assert_eq!(response.content_type, super::super::JSON);
}

/// Every oracle exchange, replayed against the admin plane's ops routes:
/// status, Content-Type and body bytes match Go (counters without the
/// Go-runtime fields).
#[test]
fn ops_exchanges_match_go_golden() {
    let exchanges = golden()["exchanges"].as_array().unwrap();
    assert_eq!(exchanges.len(), 25);
    assert_eq!(golden()["version"], "dev");
    for exchange in exchanges {
        let name = exchange["name"].as_str().unwrap();
        let configured = exchange["ops_token_configured"].as_bool().unwrap();
        let counters: Arc<dyn CounterSource> = match name {
            "counters_tripped_canary" => Arc::new(Model(tripped_controller())),
            "counters_armed_canary" => Arc::new(Model(armed_controller())),
            _ => Arc::new(NoModel),
        };
        let plane = plane_with(configured.then_some(TOKEN), counters);
        let go_path = exchange["path"].as_str().unwrap();
        let response = plane.handle(
            &request(
                exchange["method"].as_str().unwrap(),
                &ops_target(go_path),
                exchange["authorization"].as_str().unwrap_or(""),
            ),
            Clock {
                monotonic: Duration::ZERO,
                unix: 1_800_000_000,
            },
        );
        let go_body = exchange["body"].as_str().unwrap();
        let expected = if go_path == "/api/counters" && exchange["status"] == 200 {
            without_go_runtime_fields(go_body)
        } else {
            go_body.to_owned()
        };
        assert_eq!(u64::from(response.status), exchange["status"], "{name}");
        assert_eq!(response.content_type, exchange["content_type"], "{name}");
        assert_eq!(body(&response), expected, "{name}");
    }
}

/// The explanation scores a copy: the stored report, including its stored
/// scoring result, is unchanged, and a second explanation is identical.
#[test]
fn explain_scores_a_copy() {
    let reports = golden_reports();
    let before = reports.report("selenium-hr1").unwrap();
    let plane = plane(None).with_ops(Ops::new(
        "dev",
        resolve_token(TOKEN, None),
        Engine::default(),
        reports.clone(),
        Arc::new(NoModel),
    ));
    let target = format!("{OPS_PREFIX}explain/selenium-hr1");
    let first = get(&plane, &target, "Bearer s3cret");
    let second = get(&plane, &target, "Bearer s3cret");
    assert_eq!(first.status, 200);
    assert_eq!(first, second);
    let after = reports.report("selenium-hr1").unwrap();
    assert_eq!(
        serde_json::to_value(&before).unwrap(),
        serde_json::to_value(&after).unwrap()
    );
}

/// A non-finite policy cannot become a trace (Go could not produce one).
#[test]
fn explain_fails_closed_on_non_finite_policy() {
    let mut engine = Engine::default();
    engine.policy.deny_at = f64::NAN;
    let plane = plane(None).with_ops(Ops::new(
        "dev",
        resolve_token(TOKEN, None),
        engine,
        golden_reports(),
        Arc::new(NoModel),
    ));
    let response = get(
        &plane,
        &format!("{OPS_PREFIX}explain/selenium-hr1"),
        "Bearer s3cret",
    );
    assert_eq!(
        (response.status, body(&response)),
        (500, "score explanation unavailable\n")
    );
}

/// Go configureOps: trimmed flag, else trimmed HMN_OPS_TOKEN; blank is off.
#[test]
fn configure_ops_matches_go_golden() {
    let cases = golden()["configure_ops"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    for case in cases {
        let flag = case["flag"].as_str().unwrap();
        let environment = case["env"].as_str().unwrap();
        let resolved = resolve_token(flag, Some(environment));
        let expected = case["token"].as_str().unwrap();
        assert_eq!(
            resolved.as_deref().map(String::as_str).unwrap_or(""),
            expected,
            "{case}"
        );
        assert_eq!(resolved.is_some(), !expected.is_empty(), "{case}");
    }
    assert!(resolve_token("", None).is_none());
}

/// Go canReadSession: the owner cookie or the operator bearer, else 404.
#[test]
fn can_read_session_matches_go_golden() {
    let cases = golden()["can_read_session"].as_array().unwrap();
    assert_eq!(cases.len(), 11);
    for case in cases {
        let token = case["ops_token"].as_str().unwrap();
        let allowed = can_read_session(
            Some(token).filter(|token| !token.is_empty()),
            case["hsid_cookie"].as_str(),
            case["session"].as_str().unwrap(),
            case["authorization"].as_str(),
        );
        assert_eq!(allowed, case["allowed"].as_bool().unwrap(), "{case}");
        let refused = (404, NOT_FOUND);
        if !allowed {
            assert_eq!(
                (
                    case["status"].as_u64().unwrap() as u16,
                    case["body"].as_str().unwrap()
                ),
                refused
            );
        }
    }
    assert!(!operator_authorized(None, Some("Bearer ")));
    assert!(!operator_authorized(Some(""), Some("")));
}

/// The ops token and the admin bearer table are independent credentials.
#[test]
fn ops_token_and_admin_bearers_are_separate() {
    let plane = plane(Some(TOKEN));
    plane
        .auth()
        .add("operator-token-0123456789", "operator-1", Role::Operator)
        .unwrap();
    let admin = get(&plane, "/_humanymous/admin/bans", "Bearer s3cret");
    assert_eq!((admin.status, body(&admin)), (404, NOT_FOUND));
    let ops = get(
        &plane,
        &format!("{OPS_PREFIX}counters"),
        "Bearer operator-token-0123456789",
    );
    assert_eq!((ops.status, body(&ops)), (404, NOT_FOUND));
    let admin = get(
        &plane,
        "/_humanymous/admin/bans",
        "Bearer operator-token-0123456789",
    );
    assert_eq!(admin.status, 200);
}

/// The public application listener has no ops handler: reserved control
/// paths are refused and the Go public paths are not in the route inventory,
/// even with the operator bearer.
#[test]
fn public_listener_never_serves_ops_routes() {
    use crate::gateway::application::Application;
    let mut app = Application::new(
        crate::gateway::tests::config(),
        Default::default(),
        Box::new(NullAudit),
    )
    .unwrap();
    for (target, refusal) in [
        ("/_humanymous/ops/healthz", (403, "reserved_route")),
        ("/_humanymous/ops/counters", (403, "reserved_route")),
        (
            "/_humanymous/ops/explain/selenium-hr1",
            (403, "reserved_route"),
        ),
        ("/healthz", (409, "route_inventory")),
        ("/api/counters", (409, "route_inventory")),
        ("/api/explain/selenium-hr1", (409, "route_inventory")),
    ] {
        let request = Request {
            method: "GET".into(),
            target: target.into(),
            headers: vec![
                ("Host".into(), "owned.test".into()),
                ("Content-Length".into(), "0".into()),
                ("sec-ch-ua".into(), "Chromium".into()),
                (
                    "User-Agent".into(),
                    "Mozilla/5.0 Chrome/126.0 Safari/537.36".into(),
                ),
                ("Accept".into(), "*/*".into()),
                ("Sec-Fetch-Site".into(), "same-origin".into()),
                ("Accept-Encoding".into(), "gzip, deflate, br, zstd".into()),
                ("Authorization".into(), "Bearer s3cret".into()),
            ],
            body: Vec::new(),
        };
        let Err(rejection) = app.decide(&request, || Ok(10_000)) else {
            panic!("{target} on the public listener must be refused");
        };
        assert_eq!((rejection.status, rejection.code), refusal, "{target}");
    }
}

/// The ops routes are served by the admin plane's real loopback listener.
#[tokio::test]
async fn ops_routes_served_on_admin_loopback_listener() {
    let plane = Arc::new(plane(Some(TOKEN)));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (_stop, shutdown) = watch::channel::<Option<ShutdownRequest>>(None);
    let task = tokio::spawn(plane.serve_until(listener, shutdown));
    let exchange = |target: &'static str, authorization: &'static str| async move {
        let mut text = format!("GET {target} HTTP/1.1\r\nHost: admin\r\nContent-Length: 0\r\n");
        if !authorization.is_empty() {
            text.push_str(&format!("Authorization: {authorization}\r\n"));
        }
        text.push_str("\r\n");
        let mut stream = TcpStream::connect(address).await.unwrap();
        stream.write_all(text.as_bytes()).await.unwrap();
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).await.unwrap();
        let reply = String::from_utf8(bytes).unwrap();
        let (head, body) = reply.split_once("\r\n\r\n").unwrap();
        let status: u16 = head.split(' ').nth(1).unwrap().parse().unwrap();
        (status, body.to_owned())
    };
    let (status, body) = exchange("/_humanymous/ops/healthz", "").await;
    assert_eq!(status, 200);
    assert!(
        body.starts_with("{\"status\":\"ok\",\"uptimeSec\":"),
        "{body}"
    );
    let (status, body) = exchange("/_humanymous/ops/counters", "").await;
    assert_eq!((status, body.as_str()), (404, NOT_FOUND));
    let (status, body) = exchange("/_humanymous/ops/counters", "Bearer s3cret").await;
    assert_eq!(status, 200);
    let counters: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(counters["ml"]["enabled"], false);
    assert_eq!(counters["fleet"]["redisFallbackTotal"], 0);
    let (status, body) = exchange("/_humanymous/ops/explain/selenium-hr1", "Bearer s3cret").await;
    assert_eq!(status, 200);
    let trace: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(trace["verdict"], "DENY");
    assert_eq!(trace["hardRuleFired"], "HR-1");
    task.abort();
}
