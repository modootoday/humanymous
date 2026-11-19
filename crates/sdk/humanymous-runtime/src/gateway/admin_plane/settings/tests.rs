//! Differential port of legacy internal/gate/admin_settings_test.go and
//! settings_runtime_test.go (the admin-plane half) at baseline 981d25c
//! against outputs recorded from the Go oracle through its admin handler
//! (`tests/fixtures/gateway/settings-admin-go-golden.json`), plus pins of the
//! deliberate fail-closed differences.
//!
//! Requests go through `AdminPlane::handle` with a shared sealed log, like Go
//! `AdminHandler` over `sink.Log()`. Go's random overlay ids are injected,
//! its random approval ids are mapped and its wall-clock `createdAt` and
//! `expiresAt` are masked; lifetimes are pinned by the golden `expiry` table
//! and the unit tests below.
use super::*;
use crate::audit::control::Scope;
use crate::audit::log::{Config, Log};
use crate::audit::storage::{Wal, WalOptions};
use crate::authority::decision::DurableAudit;
use crate::gateway::admin_plane::PREFIX;
use crate::gateway::admin_plane::chain::Chain;
use crate::gateway::ban::{Key, Outcome};
use crate::gateway::http::Request;
use crate::gateway::token::TokenKey;
use crate::network::history::rate::{self, Level, Limits};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::{fs, io};

static GOLDEN: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/gateway/settings-admin-go-golden.json"
    ))
    .unwrap()
});

static NEXT: AtomicU64 = AtomicU64::new(0);

const KEY: &[u8] = b"test-token-key-32-bytes-long!!!!";
const UNIX: u64 = 1_800_000_000;

/// A unique scratch directory removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "humanymous-admin-settings-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("log")).unwrap();
        fs::create_dir_all(path.join("overlay")).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The plane's original sink, replaced by `with_chain` in the replay.
/// `fail` names an event type whose append fails.
#[derive(Clone, Default)]
struct Sink {
    records: Arc<Mutex<Vec<Record>>>,
    fail: Arc<Mutex<Option<&'static str>>>,
}

impl DurableAudit for Sink {
    fn append(&mut self, record: Record) -> io::Result<()> {
        if *self.fail.lock().unwrap() == Some(record.event_type.as_str()) {
            return Err(io::Error::other("sink down"));
        }
        self.records.lock().unwrap().push(record);
        Ok(())
    }
}

impl Sink {
    fn events(&self) -> Vec<(String, String)> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|record| keep(&record.event_type))
            .map(|record| (record.event_type.clone(), record.fail_reason.clone()))
            .collect()
    }
}

fn keep(event: &str) -> bool {
    event.starts_with("settings.") || event == "config.changed" || event == "approval.requested"
}

fn token(role: &str) -> String {
    format!("{role}-token-0123456789")
}

fn operator_id(role: &str) -> &'static str {
    match role {
        "operator" => "op-1",
        "approver" => "appr-1",
        "auditor" => "aud-1",
        _ => "dpo-1",
    }
}

fn boot(window: i64, soft: i64, hard: i64) -> BootInput {
    BootInput {
        global_monitor: false,
        kill_switch: false,
        routes: BTreeMap::from([
            ("/".to_owned(), "balanced".to_owned()),
            ("/checkout".to_owned(), "strict".to_owned()),
        ]),
        rate_window_sec: window,
        rate_soft: soft,
        rate_hard: hard,
        hmac_key: KEY.to_vec(),
    }
}

fn bans(window: u64, soft: usize, hard: usize) -> Arc<Mutex<Store>> {
    let limits = Limits::new(64, 256, 128, Duration::from_secs(3600), 1 << 20, 64).unwrap();
    let config = rate::Config::new(Duration::from_secs(window), soft, hard, limits).unwrap();
    Arc::new(Mutex::new(Store::new(config, 64).unwrap()))
}

fn plane(bans: Arc<Mutex<Store>>, sink: Box<dyn DurableAudit>) -> AdminPlane {
    let plane = AdminPlane::new(
        "n1",
        Scope {
            installation: "installation".into(),
            node: "n1".into(),
        },
        KEY,
        bans,
        sink,
    );
    for role in [Role::Auditor, Role::Operator, Role::Approver, Role::Dpo] {
        plane
            .auth()
            .add(&token(role.as_str()), operator_id(role.as_str()), role)
            .unwrap();
    }
    plane
}

fn clock(step: u64) -> Clock {
    Clock {
        monotonic: Duration::from_secs(5 + step),
        unix: UNIX + step,
    }
}

fn request(method: &str, sub: &str, role: &str, body: &str) -> Request {
    Request {
        method: method.into(),
        target: format!("{PREFIX}{sub}"),
        headers: vec![
            ("Host".to_owned(), "admin".to_owned()),
            ("Authorization".into(), format!("Bearer {}", token(role))),
        ],
        body: body.as_bytes().to_vec(),
    }
}

fn store_with(dir: &Path, setup: &Value) -> OverlayFileStore {
    match setup["store"].as_str().unwrap() {
        "raw" => {
            fs::write(
                dir.join("settings.overlay.v1.json"),
                setup["raw"].as_str().unwrap(),
            )
            .unwrap();
            OverlayFileStore::open(dir).unwrap()
        }
        "overlay" => {
            let store = OverlayFileStore::open(dir).unwrap();
            let mut overlay =
                Overlay::from_json(setup["overlay"].as_str().unwrap().as_bytes()).unwrap();
            store.set_active(Some(&mut overlay)).unwrap();
            store
        }
        _ => OverlayFileStore::open(dir).unwrap(),
    }
}

/// Replace every RFC 3339 instant with a marker (Go stamps the wall clock).
fn mask_times(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < bytes.len() {
        let date = bytes.get(index..index + 11).is_some_and(|head| {
            head.iter().enumerate().all(|(at, byte)| match at {
                4 | 7 => *byte == b'-',
                10 => *byte == b'T',
                _ => byte.is_ascii_digit(),
            })
        });
        if date {
            let mut end = index + 11;
            while end < bytes.len() && (bytes[end].is_ascii_digit() || b":.".contains(&bytes[end]))
            {
                end += 1;
            }
            if bytes.get(end) == Some(&b'Z') {
                out.push_str("<time>");
                index = end + 1;
                continue;
            }
        }
        let width = text[index..].chars().next().unwrap().len_utf8();
        out.push_str(&text[index..index + width]);
        index += width;
    }
    out
}

/// Order-insensitive views of the two responses whose order Go took from a
/// map or from random ids.
fn unordered(path: &str, body: &str) -> Option<Value> {
    let mut value: Value = serde_json::from_str(body).ok()?;
    match path {
        "approvals" => {
            if let Some(Value::Array(pending)) = value.get_mut("pending") {
                pending.sort_by_key(|action| action["id"].to_string());
            }
        }
        "settings/schema" => {
            if let Some(Value::Array(rules)) = value.get_mut("integrityCriticalHardRules") {
                rules.sort_by_key(ToString::to_string);
            }
        }
        _ => return None,
    }
    Some(value)
}

struct Replay {
    plane: AdminPlane,
    log: Arc<Mutex<Log>>,
    kill: Arc<AtomicBool>,
    rust_aid: String,
    ids: Vec<(String, String)>,
    step: u64,
    _scratch: Scratch,
}

impl Replay {
    fn records_since(&self, before: usize) -> Vec<Value> {
        self.log.lock().unwrap().records().unwrap()[before..]
            .iter()
            .filter(|record| keep(&record.event_type))
            .map(|record| {
                json!({
                    "actor_kind": record.actor.kind,
                    "event_type": record.event_type,
                    "fail_reason": self.to_go(&record.fail_reason),
                    "mode": record.enforcement_mode,
                })
            })
            .collect()
    }

    fn len(&self) -> usize {
        self.log.lock().unwrap().records().unwrap().len()
    }

    fn to_go(&self, text: &str) -> String {
        self.ids
            .iter()
            .fold(text.to_owned(), |text, (rust, go)| text.replace(rust, go))
    }

    fn settings(&self) -> &Settings {
        self.plane.settings.as_ref().unwrap()
    }

    fn request(&mut self, scenario: &str, step: &Value) {
        let name = format!("{scenario}/{}", step["name"].as_str().unwrap());
        let cv = self.settings().effective().config_version;
        assert_eq!(cv, step["cv"], "{name}: configuration version");
        let sent = step["body"].as_str().unwrap().replace("$CV", &cv);
        assert_eq!(sent, step["sentBody"], "{name}: sent body");
        for record in step["records"].as_array().unwrap() {
            if record["event_type"] == "settings.overlay.proposed" {
                let reason = record["fail_reason"].as_str().unwrap();
                let id = reason.split("overlayId=").nth(1).unwrap();
                self.settings().inject_overlay_id(id);
            }
        }
        let path = step["path"]
            .as_str()
            .unwrap()
            .replace("$AID", &self.rust_aid);
        let before = self.len();
        self.step += 1;
        let response = self.plane.handle(
            &request(
                step["method"].as_str().unwrap(),
                &path,
                step["role"].as_str().unwrap(),
                &sent,
            ),
            clock(self.step),
        );
        let body = String::from_utf8(response.body.clone()).unwrap();
        let golden = step["response"].as_str().unwrap();
        if let (Ok(ours), Ok(theirs)) = (
            serde_json::from_str::<Value>(&body),
            serde_json::from_str::<Value>(golden),
        ) && let (Some(Value::String(ours)), Some(Value::String(theirs))) =
            (ours.get("approvalId"), theirs.get("approvalId"))
        {
            self.rust_aid.clone_from(ours);
            self.ids.push((ours.clone(), theirs.clone()));
        }
        assert_eq!(response.status, step["status"], "{name}: status {body}");
        assert_eq!(
            response.content_type, step["contentType"],
            "{name}: content type"
        );
        let ours = mask_times(&self.to_go(&body));
        let theirs = mask_times(golden);
        match unordered(&path, &ours) {
            Some(value) => assert_eq!(Some(value), unordered(&path, &theirs), "{name}: body"),
            None => assert_eq!(ours, theirs, "{name}: body"),
        }
        assert_eq!(
            Value::Array(self.records_since(before)),
            step["records"],
            "{name}: records"
        );
    }
}

fn replay(scenario: &Value) {
    let name = scenario["name"].as_str().unwrap();
    let scratch = Scratch::new();
    let seed = [7; 32];
    let log = Arc::new(Mutex::new(
        Log::open(
            Wal::open(&scratch.0.join("log"), WalOptions::default()).unwrap(),
            Config {
                node_id: "n1".into(),
                hmac_key: KEY.to_vec(),
                signing_seed: seed,
                checkpoint_every: 32,
                witness: None,
            },
            None,
            true,
        )
        .unwrap(),
    ));
    let limits = Limits::new(64, 64, 128, Duration::from_secs(60), 1 << 20, 64).unwrap();
    let chain = Chain::new(log.clone(), TokenKey::new(KEY.to_vec()).unwrap(), limits).unwrap();
    let plane = plane(bans(10, 60, 120), Box::new(Sink::default())).with_chain(chain);
    let kill = Arc::new(AtomicBool::new(false));
    let store = store_with(&scratch.0.join("overlay"), &scenario["setup"]);
    let before = log.lock().unwrap().records().unwrap().len();
    let attached = plane.with_settings(
        Settings::new(boot(10, 60, 120), kill.clone(), store),
        clock(0),
    );
    let plane = match attached {
        Err(refusal) => {
            assert_eq!(
                refusal.to_string(),
                scenario["attachError"],
                "{name}: attach"
            );
            assert!(scenario["steps"].as_array().unwrap().is_empty());
            return;
        }
        Ok(plane) => plane,
    };
    assert_eq!(scenario["attachError"], "", "{name}: attach");
    let mut replay = Replay {
        plane,
        log,
        kill,
        rust_aid: String::new(),
        ids: Vec::new(),
        step: 0,
        _scratch: scratch,
    };
    assert_eq!(
        Value::Array(replay.records_since(before)),
        scenario["attachRecords"],
        "{name}: attach records"
    );
    for step in scenario["steps"].as_array().unwrap() {
        match step["kind"].as_str().unwrap() {
            "request" => replay.request(name, step),
            "kill" => replay
                .kill
                .store(step["on"].as_bool().unwrap(), Ordering::SeqCst),
            "emit" => {
                let mut log = replay.log.lock().unwrap();
                for record in step["records"].as_array().unwrap() {
                    log.append(Record {
                        event_type: record["event"].as_str().unwrap().into(),
                        risk_score: record["risk"].as_i64().unwrap(),
                        triggered_rules: record["rules"].as_array().map(|rules| {
                            rules
                                .iter()
                                .map(|rule| rule.as_str().unwrap().to_owned())
                                .collect()
                        }),
                        tenant_id: "n1".into(),
                        enforcement_mode: "enforce".into(),
                        key_id: "k1".into(),
                        ..Record::default()
                    })
                    .unwrap();
                }
            }
            "resolve" => {
                for result in step["results"].as_array().unwrap() {
                    let policy = replay
                        .settings()
                        .resolve_path(result["path"].as_str().unwrap());
                    assert_eq!(policy.name, result["name"], "{name}: resolve {result}");
                    assert_eq!(
                        policy.enforce, result["enforce"],
                        "{name}: resolve {result}"
                    );
                }
            }
            "effective" => assert_eq!(
                String::from_utf8(replay.settings().effective().to_json().unwrap()).unwrap(),
                step["json"],
                "{name}: effective"
            ),
            other => panic!("unknown step kind {other}"),
        }
    }
}

fn scenario(name: &str) -> &'static Value {
    GOLDEN["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .find(|scenario| scenario["name"] == name)
        .unwrap()
}

#[test]
fn golden_covers_every_scenario() {
    let names: Vec<&str> = GOLDEN["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .map(|scenario| scenario["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "class-a-apply",
            "class-b-dual-control",
            "class-c",
            "refusals",
            "decoding",
            "net-routes-rate",
            "rollback-dual-control",
            "kill-switch",
            "dry-run",
            "expiry",
            "future-expiry",
            "invalid-active-file",
            "runtime-refused-attach",
        ]
    );
}

// admin_settings_test.go: TestSettingsProposeClassAApplies, TestSettingsCASStale
#[test]
fn golden_class_a_apply() {
    replay(scenario("class-a-apply"));
}

// admin_settings_test.go: TestSettingsProposeClassBPending,
// TestSettingsRollbackDemotionIsImmediate
#[test]
fn golden_class_b_dual_control() {
    replay(scenario("class-b-dual-control"));
}

// admin_settings_test.go: TestSettingsClassCRequiresConfirm
#[test]
fn golden_class_c() {
    replay(scenario("class-c"));
}

#[test]
fn golden_refusals() {
    replay(scenario("refusals"));
}

#[test]
fn golden_decoding() {
    replay(scenario("decoding"));
}

// admin_settings_test.go: TestSettingsProposeNetPolicyAndRoutes,
// TestSettingsProposeRateLoosenIsB
#[test]
fn golden_net_routes_rate() {
    replay(scenario("net-routes-rate"));
}

// admin_settings_test.go: TestSettingsRollbackClearBClassOverlayIsA,
// TestSettingsCASStale (stale approval commits)
#[test]
fn golden_rollback_dual_control() {
    replay(scenario("rollback-dual-control"));
}

#[test]
fn golden_kill_switch() {
    replay(scenario("kill-switch"));
}

// admin_settings_test.go: TestSettingsDryRunLowNUnusable
#[test]
fn golden_dry_run() {
    replay(scenario("dry-run"));
}

#[test]
fn golden_expiry() {
    replay(scenario("expiry"));
}

#[test]
fn golden_future_expiry() {
    replay(scenario("future-expiry"));
}

#[test]
fn golden_invalid_active_file() {
    replay(scenario("invalid-active-file"));
}

#[test]
fn golden_runtime_refused_attach() {
    replay(scenario("runtime-refused-attach"));
}

#[test]
fn golden_expiry_for_class() {
    let mut checked = 0;
    for entry in GOLDEN["expiry"].as_array().unwrap() {
        let class = match entry["class"].as_str().unwrap() {
            "A" => MutationClass::A,
            "B" => MutationClass::B,
            "C" => MutationClass::C,
            "D" => MutationClass::D,
            // Go's zero class is unreachable: `classify` always names one.
            _ => continue,
        };
        let got = expiry_for_class(class, entry["wantSec"].as_i64().unwrap());
        match (&entry["error"], &entry["offsetSec"]) {
            (Value::String(error), _) => assert_eq!(got, Err(error.as_str()), "{entry}"),
            (_, Value::Number(offset)) => assert_eq!(got, Ok(offset.as_i64()), "{entry}"),
            _ => assert_eq!(got, Ok(None), "{entry}"),
        }
        checked += 1;
    }
    assert_eq!(checked, 40);
}

/// A plane with a recording sink and an empty overlay store, without a chain.
struct Unit {
    plane: AdminPlane,
    sink: Sink,
    bans: Arc<Mutex<Store>>,
    kill: Arc<AtomicBool>,
    _scratch: Scratch,
}

fn unit_with(rate: (i64, i64, i64), overlay: Option<&str>) -> Unit {
    let scratch = Scratch::new();
    let sink = Sink::default();
    let window = u64::try_from(rate.0).unwrap();
    let bans = bans(
        window,
        usize::try_from(rate.1).unwrap(),
        usize::try_from(rate.2).unwrap(),
    );
    let kill = Arc::new(AtomicBool::new(false));
    let setup = match overlay {
        Some(json) => json!({"store": "overlay", "overlay": json}),
        None => json!({"store": "empty"}),
    };
    let store = store_with(&scratch.0.join("overlay"), &setup);
    let plane = plane(bans.clone(), Box::new(sink.clone()))
        .with_settings(
            Settings::new(boot(rate.0, rate.1, rate.2), kill.clone(), store),
            clock(0),
        )
        .unwrap();
    Unit {
        plane,
        sink,
        bans,
        kill,
        _scratch: scratch,
    }
}

impl Unit {
    fn settings(&self) -> &Settings {
        self.plane.settings().unwrap()
    }

    fn cv(&self) -> String {
        self.settings().effective().config_version
    }

    fn post(&self, sub: &str, role: &str, body: &str, step: u64) -> (u16, Value, String) {
        let response = self
            .plane
            .handle(&request("POST", sub, role, body), clock(step));
        let text = String::from_utf8(response.body).unwrap();
        (
            response.status,
            serde_json::from_str(&text).unwrap_or(Value::Null),
            text,
        )
    }

    fn observe(&self, key: &Key, step: u64) -> Outcome {
        self.bans
            .lock()
            .unwrap()
            .observe(key, clock(step).monotonic)
            .unwrap()
    }
}

// settings_runtime_test.go: TestSettingsRateHotApplyPreservesCounters
#[test]
fn rate_hot_apply_preserves_counters() {
    let unit = unit_with((60, 5, 10), None);
    let key = Key::parse("ip:198.51.100.39").unwrap();
    for _ in 0..4 {
        assert!(matches!(unit.observe(&key, 1), Outcome::Clear(_)));
    }
    let body = format!(
        r#"{{"parentConfigVersion":"{}","rateLimit":{{"hard":5}}}}"#,
        unit.cv()
    );
    let (status, out, text) = unit.post("settings/overlays", "operator", &body, 2);
    assert_eq!(status, 200, "{text}");
    assert_eq!(out["applied"], true);
    assert!(
        matches!(unit.observe(&key, 3), Outcome::AutoBanned { .. }),
        "hot apply reset hits or missed hard=5"
    );
}

// settings_runtime_test.go: TestSettingsMetricsReflectApply (the counters;
// the Prometheus exposition is a recorded gap)
#[test]
fn stats_reflect_apply_and_rejection() {
    let unit = unit_with((10, 60, 120), None);
    let body = format!(
        r#"{{"parentConfigVersion":"{}","scoring":{{"challengeAt":20}}}}"#,
        unit.cv()
    );
    assert_eq!(unit.post("settings/overlays", "operator", &body, 1).0, 200);
    assert!(!unit.settings().effective().empty_overlay);
    let stale = r#"{"parentConfigVersion":"cfg-stale"}"#;
    assert_eq!(unit.post("settings/rollback", "operator", stale, 2).0, 409);
    assert_eq!(
        unit.plane.settings_stats(),
        Some(SettingsStats {
            applied: 1,
            ..SettingsStats::default()
        })
    );
}

#[test]
fn class_b_commit_rejected_when_stale_counts_rejection() {
    let unit = unit_with((10, 60, 120), None);
    let body = format!(
        r#"{{"parentConfigVersion":"{}","hardRules":{{"HR-12":"monitor"}}}}"#,
        unit.cv()
    );
    let (_, pending, _) = unit.post("settings/overlays", "operator", &body, 1);
    let tighten = format!(
        r#"{{"parentConfigVersion":"{}","scoring":{{"denyAt":60}}}}"#,
        unit.cv()
    );
    assert_eq!(
        unit.post("settings/overlays", "operator", &tighten, 2).0,
        200
    );
    let id = pending["approvalId"].as_str().unwrap();
    let (status, _, text) = unit.post(&format!("approvals/{id}"), "approver", "", 3);
    assert_eq!(
        (status, text.as_str()),
        (409, format!("{STALE}\n").as_str())
    );
    let stats = unit.plane.settings_stats().unwrap();
    assert_eq!((stats.applied, stats.rejected), (1, 1));
}

/// A class B proposal stores Go's 7-day default lifetime from the trusted
/// clock, in whole seconds.
#[test]
fn class_b_lifetime_defaults_to_seven_days() {
    let unit = unit_with((10, 60, 120), None);
    let body = format!(
        r#"{{"parentConfigVersion":"{}","hardRules":{{"HR-12":"monitor"}}}}"#,
        unit.cv()
    );
    let (_, pending, _) = unit.post("settings/overlays", "operator", &body, 1);
    assert_eq!(
        unit.plane.settings_pending_age(clock(4).monotonic),
        Duration::from_secs(3)
    );
    let id = pending["approvalId"].as_str().unwrap();
    assert_eq!(
        unit.post(&format!("approvals/{id}"), "approver", "", 4).0,
        200
    );
    assert_eq!(
        unit.plane.settings_pending_age(clock(5).monotonic),
        Duration::ZERO
    );
    let active = unit.settings().active().unwrap();
    let created = i64::try_from(UNIX + 1).unwrap();
    assert_eq!(active.created_at, Timestamp::from_unix(created).unwrap());
    assert_eq!(
        active.expires_at,
        Timestamp::from_unix(created + MAX_LOOSEN_SECS)
    );
    assert_eq!(active.mutation_class, "B");
    assert_eq!(active.approved_by, "appr-1");
}

/// Go answered 200 after the change even when the sink failed. Here the
/// records come first: no record, no change, and the rate limiter keeps its
/// previous cutpoints.
#[test]
fn apply_audit_failure_changes_nothing() {
    let unit = unit_with((60, 5, 10), None);
    *unit.sink.fail.lock().unwrap() = Some("config.changed");
    let body = format!(
        r#"{{"parentConfigVersion":"{}","rateLimit":{{"hard":5}}}}"#,
        unit.cv()
    );
    let (status, _, text) = unit.post("settings/overlays", "operator", &body, 1);
    assert_eq!((status, text.as_str()), (503, "audit unavailable\n"));
    assert!(unit.settings().active().is_none());
    let key = Key::parse("ip:198.51.100.40").unwrap();
    for _ in 0..5 {
        assert!(matches!(unit.observe(&key, 2), Outcome::Clear(_)));
    }
    assert_eq!(unit.plane.settings_stats().unwrap().applied, 0);
}

/// A pending action whose `approval.requested` record fails is withdrawn,
/// so no approver can commit an unaudited proposal.
#[test]
fn pending_audit_failure_withdraws_the_action() {
    let unit = unit_with((10, 60, 120), None);
    *unit.sink.fail.lock().unwrap() = Some("approval.requested");
    let body = format!(
        r#"{{"parentConfigVersion":"{}","hardRules":{{"HR-12":"monitor"}}}}"#,
        unit.cv()
    );
    let (status, _, _) = unit.post("settings/overlays", "operator", &body, 1);
    assert_eq!(status, 503);
    assert!(unit.plane.approvals.pending(clock(2).monotonic).is_empty());
}

/// A proposal whose record fails is not applied or queued.
#[test]
fn proposal_audit_failure_refuses() {
    let unit = unit_with((10, 60, 120), None);
    *unit.sink.fail.lock().unwrap() = Some("settings.overlay.proposed");
    let body = format!(
        r#"{{"parentConfigVersion":"{}","scoring":{{"challengeAt":20}}}}"#,
        unit.cv()
    );
    assert_eq!(unit.post("settings/overlays", "operator", &body, 1).0, 503);
    assert!(unit.settings().active().is_none());
    assert!(unit.plane.approvals.pending(clock(2).monotonic).is_empty());
}

/// An overlay approved after its lifetime ended is refused and recorded.
#[test]
fn commit_after_expiry_is_refused() {
    let unit = unit_with((10, 60, 120), None);
    let body = format!(
        r#"{{"parentConfigVersion":"{}","hardRules":{{"HR-12":"monitor"}},"expiresInSec":2}}"#,
        unit.cv()
    );
    let (_, pending, _) = unit.post("settings/overlays", "operator", &body, 1);
    let id = pending["approvalId"].as_str().unwrap();
    let (status, _, text) = unit.post(&format!("approvals/{id}"), "approver", "", 3);
    assert_eq!((status, text.as_str()), (409, "overlay already expired\n"));
    assert!(unit.settings().active().is_none());
    assert!(unit.sink.events().contains(&(
        "settings.overlay.rejected".into(),
        "overlay already expired".into()
    )));
}

/// Go's `GC` expiry sweep, held while the kill switch is engaged.
#[test]
fn sweep_expires_the_active_overlay() {
    let unit = unit_with(
        (10, 60, 120),
        Some(
            r#"{"schemaVersion":"1.0.0","overlayId":"ovl_old","status":"active","mutationClass":"B","expiresAt":"2020-01-01T00:00:00Z","hardRules":{"HR-12":"monitor"}}"#,
        ),
    );
    unit.kill.store(true, Ordering::SeqCst);
    unit.plane.sweep_settings(clock(1)).unwrap();
    assert!(unit.settings().active().is_some());
    unit.kill.store(false, Ordering::SeqCst);
    unit.plane.sweep_settings(clock(2)).unwrap();
    assert!(unit.settings().active().is_none());
    assert_eq!(
        unit.sink.events(),
        [(
            "settings.overlay.expired".to_owned(),
            "overlayId=ovl_old".to_owned()
        )]
    );
}

/// An expiry whose record cannot be written leaves the overlay in place and
/// reports the failure.
#[test]
fn sweep_audit_failure_keeps_the_overlay() {
    let unit = unit_with(
        (10, 60, 120),
        Some(
            r#"{"schemaVersion":"1.0.0","overlayId":"ovl_old","status":"active","mutationClass":"B","expiresAt":"2020-01-01T00:00:00Z","hardRules":{"HR-12":"monitor"}}"#,
        ),
    );
    *unit.sink.fail.lock().unwrap() = Some("settings.overlay.expired");
    assert!(unit.plane.sweep_settings(clock(1)).is_err());
    assert!(unit.settings().active().is_some());
    let effective = unit.plane.handle(
        &request("GET", "settings/effective", "auditor", ""),
        clock(2),
    );
    assert_eq!(effective.status, 503);
}

/// Go overflowed its duration for an enormous class A lifetime; it is refused.
#[test]
fn unrepresentable_lifetime_is_refused() {
    let unit = unit_with((10, 60, 120), None);
    let body = format!(
        r#"{{"parentConfigVersion":"{}","scoring":{{"challengeAt":20}},"expiresInSec":{}}}"#,
        unit.cv(),
        i64::MAX
    );
    let (status, _, text) = unit.post("settings/overlays", "operator", &body, 1);
    assert_eq!(
        (status, text.as_str()),
        (400, "expiresInSec out of range\n")
    );
    assert!(unit.settings().active().is_none());
}

/// The ban store's limiter bounds the window (here one hour); a Go-valid
/// window above it is rejected like a runtime validation failure.
#[test]
fn rate_limiter_refusal_rejects_the_change() {
    let unit = unit_with((10, 60, 120), None);
    let body = format!(
        r#"{{"parentConfigVersion":"{}","rateLimit":{{"windowSec":7200}}}}"#,
        unit.cv()
    );
    let (status, out, text) = unit.post("settings/overlays", "operator", &body, 1);
    let (status, text) = match out["approvalId"].as_str() {
        Some(id) => {
            let (status, _, text) = unit.post(&format!("approvals/{id}"), "approver", "", 2);
            (status, text)
        }
        None => (status, text),
    };
    let message = "settings: rate limiter refused effective rate limit 7200/60/120";
    assert_eq!((status, text), (409, format!("{message}\n")));
    assert!(unit.settings().active().is_none());
    assert!(
        unit.sink
            .events()
            .contains(&("settings.overlay.rejected".into(), message.into()))
    );
}

/// Without an attached engine every settings route is the plain 404 after
/// authentication, and no settings approval can exist.
#[test]
fn unattached_settings_routes_are_not_found() {
    let plane = plane(bans(10, 60, 120), Box::new(Sink::default()));
    for (method, sub, role) in [
        ("GET", "settings/effective", "auditor"),
        ("GET", "settings/schema", "auditor"),
        ("GET", "settings/overlays", "auditor"),
        ("POST", "settings/overlays", "operator"),
        ("POST", "settings/dry-run", "operator"),
        ("POST", "settings/rollback", "operator"),
    ] {
        let response = plane.handle(&request(method, sub, role, "{}"), clock(1));
        assert_eq!(response.status, 404, "{method} {sub}");
    }
    assert!(plane.settings_stats().is_none());
}

/// The policy view's overlay slot follows every change.
#[test]
fn active_slot_mirrors_changes() {
    let scratch = Scratch::new();
    let slot = Arc::new(RwLock::new(None));
    let store = OverlayFileStore::open(scratch.0.join("overlay")).unwrap();
    let settings = Settings::new(boot(10, 60, 120), Arc::new(AtomicBool::new(false)), store)
        .with_active_slot(slot.clone());
    let plane = plane(bans(10, 60, 120), Box::new(Sink::default()))
        .with_settings(settings, clock(0))
        .unwrap();
    let cv = plane.settings().unwrap().effective().config_version;
    let body = format!(r#"{{"parentConfigVersion":"{cv}","scoring":{{"challengeAt":20}}}}"#);
    let response = plane.handle(
        &request("POST", "settings/overlays", "operator", &body),
        clock(1),
    );
    assert_eq!(response.status, 200);
    let active = slot.read().unwrap().clone().unwrap();
    assert_eq!(active.mutation_class, "A");
    assert_eq!(active.status, "active");
}

#[test]
fn time_mask_covers_go_and_rust_instants() {
    assert_eq!(
        mask_times(r#"{"a":"2026-11-19T13:15:25.704979832Z","b":"2027-01-15T08:00:00Z","c":"x"}"#),
        r#"{"a":"<time>","b":"<time>","c":"x"}"#
    );
}

#[test]
fn rate_level_is_hard_after_hot_apply() {
    // The Go test also asserted level 2 (hard) on the banning request.
    let unit = unit_with((60, 5, 10), None);
    let body = format!(
        r#"{{"parentConfigVersion":"{}","rateLimit":{{"hard":5}}}}"#,
        unit.cv()
    );
    assert_eq!(unit.post("settings/overlays", "operator", &body, 1).0, 200);
    let mut store = unit.bans.lock().unwrap();
    let key = Key::parse("ip:198.51.100.41").unwrap();
    let levels: Vec<Level> = (0..5)
        .map(|_| store.observe_rate(&key, clock(2).monotonic).unwrap())
        .collect();
    assert_eq!(levels[4], Level::Hard);
    assert_eq!(levels[3], Level::Normal);
}
