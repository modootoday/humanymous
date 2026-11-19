//! Ports of the admin HTTP rounds of legacy internal/gate/ws12_test.go,
//! wargame_escalation_r51_test.go (R066-R070, R079, R080) and
//! wargame_escalation_r161_test.go (R161-R167, R186, R187, R190) at baseline
//! 981d25c, and of wargame_r01_ban_test.go (TestPermanentBanIgnoresBodyApprover,
//! in `dual_control`). Every round goes through a real loopback TCP listener served by
//! `AdminPlane::serve_until`, like Go `adminDo` through `AdminHandler`.
use super::*;
use crate::gateway::admin_approval::Role;
use crate::network::history::rate::{Config, Limits};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::AsyncReadExt;
use tokio::task::JoinHandle;

/// Records appended by the plane, with an injectable disk failure.
#[derive(Clone, Default)]
struct Journal {
    records: Arc<Mutex<Vec<Record>>>,
    fail: Arc<AtomicBool>,
}

impl DurableAudit for Journal {
    fn append(&mut self, record: Record) -> io::Result<()> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(io::Error::other("injected audit failure"));
        }
        self.records.lock().unwrap().push(record);
        Ok(())
    }
}

impl Journal {
    fn events(&self) -> Vec<(String, String)> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .map(|record| (record.event_type.clone(), record.fail_reason.clone()))
            .collect()
    }

    fn has(&self, event: &str) -> bool {
        self.events().iter().any(|(kind, _)| kind == event)
    }
}

fn scope() -> Scope {
    Scope {
        installation: "installation".into(),
        node: "node".into(),
    }
}

fn ban_store() -> Arc<Mutex<Store>> {
    let limits = Limits::new(64, 64, 128, Duration::from_secs(60), 1 << 20, 64).unwrap();
    let rate = Config::new(Duration::from_secs(60), 998, 1000, limits).unwrap();
    Arc::new(Mutex::new(Store::new(rate, 64).unwrap()))
}

/// Go `banStack` plus `seedAdmins`: a served plane, its ban store, its audit
/// journal and one bearer token per role.
struct Stack {
    plane: Arc<AdminPlane>,
    bans: Arc<Mutex<Store>>,
    journal: Journal,
    addr: SocketAddr,
    tokens: BTreeMap<&'static str, String>,
    _stop: watch::Sender<Option<ShutdownRequest>>,
    task: JoinHandle<io::Result<()>>,
}

impl Drop for Stack {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn stack() -> Stack {
    stack_with(|plane| plane).await
}

/// [`stack`] with the plane adjusted (for example an attached erasure hold)
/// before it is served.
async fn stack_with(build: impl FnOnce(AdminPlane) -> AdminPlane) -> Stack {
    let bans = ban_store();
    let journal = Journal::default();
    let plane = Arc::new(build(AdminPlane::new(
        "test",
        scope(),
        b"k",
        bans.clone(),
        Box::new(journal.clone()),
    )));
    let mut tokens = BTreeMap::new();
    for role in [Role::Auditor, Role::Operator, Role::Approver, Role::Dpo] {
        let token = format!("{}-token-0123456789", role.as_str());
        plane
            .auth()
            .add(&token, &format!("{}-1", role.as_str()), role)
            .unwrap();
        tokens.insert(role.as_str(), token);
    }
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (stop, shutdown) = watch::channel(None);
    let task = tokio::spawn(plane.clone().serve_until(listener, shutdown));
    Stack {
        plane,
        bans,
        journal,
        addr,
        tokens,
        _stop: stop,
        task,
    }
}

struct Reply {
    status: u16,
    content_type: String,
    body: String,
}

impl Reply {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap()
    }
}

impl Stack {
    fn token(&self, role: &str) -> &str {
        &self.tokens[role]
    }

    /// Go `adminDo`: one request with an optional bearer token over TCP.
    async fn admin_do(&self, token: &str, method: &str, sub: &str, body: &str) -> Reply {
        self.raw(method, &format!("{PREFIX}{sub}"), token, body)
            .await
    }

    async fn raw(&self, method: &str, target: &str, token: &str, body: &str) -> Reply {
        let mut request = format!(
            "{method} {target} HTTP/1.1\r\nHost: admin\r\nContent-Length: {}\r\n",
            body.len()
        );
        if !token.is_empty() {
            request.push_str(&format!("Authorization: Bearer {token}\r\n"));
        }
        if !body.is_empty() {
            request.push_str("Content-Type: application/json\r\n");
        }
        request.push_str("\r\n");
        request.push_str(body);
        let mut stream = TcpStream::connect(self.addr).await.unwrap();
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).await.unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let (head, body) = text.split_once("\r\n\r\n").unwrap();
        let mut lines = head.split("\r\n");
        let status = lines.next().unwrap().split(' ').nth(1).unwrap();
        let content_type = lines
            .find_map(|line| line.strip_prefix("Content-Type: "))
            .unwrap()
            .to_owned();
        Reply {
            status: status.parse().unwrap(),
            content_type,
            body: body.to_owned(),
        }
    }

    /// Whether the shared ban store holds an active ban for `key`.
    fn banned(&self, key: &str) -> Option<ban::Entry> {
        self.bans
            .lock()
            .unwrap()
            .check(&Key::parse(key).unwrap(), Duration::from_secs(1))
            .cloned()
    }

    fn now(&self) -> Duration {
        Duration::from_secs(1)
    }
}

/// Go `extractField`.
fn approval_id(reply: &Reply) -> String {
    reply.json()["approvalId"].as_str().unwrap_or("").to_owned()
}

/// Go `assertHTTPBanRejected`: 400, neither applied nor pending.
async fn assert_http_ban_rejected(body: &str) {
    let s = stack().await;
    let w = s.admin_do(s.token("operator"), "POST", "bans", body).await;
    assert_eq!(w.status, 400, "{}", w.body);
    assert_eq!(w.content_type, TEXT);
    assert!(s.bans.lock().unwrap().list(s.now()).is_empty());
    assert!(s.plane.approvals.pending(s.now()).is_empty());
}

// --- ws12_test.go -----------------------------------------------------------

/// Go TestAdminOffPublicListener: the public application never serves the
/// admin plane. Go answers 404; the runtime ingress answers its reserved-route
/// refusal (403 `reserved_route`) for every `/_humanymous/admin/` path.
#[test]
fn admin_off_public_listener() {
    use crate::gateway::application::Application;
    let mut app = Application::new(
        crate::gateway::tests::config(),
        Default::default(),
        Box::new(Journal::default()),
    )
    .unwrap();
    for sub in ["audit", "bans", "console", "integrity"] {
        let request = Request {
            method: "GET".into(),
            target: format!("{PREFIX}{sub}"),
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
                (
                    "Authorization".into(),
                    "Bearer operator-token-0123456789".into(),
                ),
            ],
            body: Vec::new(),
        };
        let Err(rejection) = app.decide(&request, || Ok(10_000)) else {
            panic!("{sub} on the public listener must be refused");
        };
        assert_eq!((rejection.status, rejection.code), (403, "reserved_route"));
    }
}

/// Go TestAdminDenyByDefault: no token and a bad token answer 404 and the
/// failure is meta-audited; a path outside the prefix is a plain 404.
#[tokio::test]
async fn admin_deny_by_default() {
    let s = stack().await;
    let w = s.admin_do("", "GET", "audit", "").await;
    assert_eq!((w.status, w.body.as_str()), (404, "404 page not found\n"));
    assert_eq!(w.content_type, TEXT);
    let w = s.admin_do("wrong-token", "GET", "audit", "").await;
    assert_eq!(w.status, 404);
    assert!(s.journal.has("admin.auth_fail"));
    assert!(s.journal.events().contains(&(
        "admin.auth_fail".into(),
        "unauthenticated admin GET audit".into()
    )));
    let w = s
        .raw("GET", "/_humanymous/other", s.token("operator"), "")
        .await;
    assert_eq!(w.status, 404);
    // A valid token never reaches an authenticated meta-audit for that path.
    assert!(!s.journal.has("admin.access"));
}

/// Go TestAdminRBAC: auditor reads, auditor ban is 403, operator temp ban 200.
#[tokio::test]
async fn admin_rbac() {
    let s = stack().await;
    let w = s.admin_do(s.token("auditor"), "GET", "bans", "").await;
    assert_eq!(w.status, 200);
    assert_eq!(w.content_type, JSON);
    assert_eq!(w.body, "{\"bans\":[],\"count\":0}\n");
    let body = r#"{"Key":"ip:1.2.3.4","DurationSec":3600}"#;
    let w = s.admin_do(s.token("auditor"), "POST", "bans", body).await;
    assert_eq!(
        (w.status, w.body.as_str()),
        (403, "role auditor not permitted\n")
    );
    assert!(s.banned("ip:1.2.3.4").is_none());
    let w = s.admin_do(s.token("operator"), "POST", "bans", body).await;
    assert_eq!(
        (w.status, w.body.as_str()),
        (200, "{\"ok\":true,\"permanent\":false}\n")
    );
    let entry = s.banned("ip:1.2.3.4").unwrap();
    assert_eq!(entry.reason, DEFAULT_REASON);
    assert!(!entry.permanent());
    assert!(s.journal.has("ban.applied"));
}

/// Go TestSelfLiftClosed: an unauthenticated lift is 404 and the ban stays;
/// an authenticated operator lifts it.
#[tokio::test]
async fn self_lift_closed() {
    let s = stack().await;
    let key = Key::parse("ip:9.9.9.9").unwrap();
    s.bans
        .lock()
        .unwrap()
        .add_temporary(&key, "auto", None, Duration::from_secs(3600), s.now())
        .unwrap();
    let w = s.admin_do("", "POST", "bans/lift?key=ip:9.9.9.9", "").await;
    assert_eq!(w.status, 404);
    assert!(s.banned("ip:9.9.9.9").is_some());
    let w = s
        .admin_do(s.token("operator"), "POST", "bans/lift?key=ip:9.9.9.9", "")
        .await;
    assert_eq!(
        (w.status, w.body.as_str()),
        (200, "{\"lifted\":true,\"ok\":true}\n")
    );
    assert!(s.banned("ip:9.9.9.9").is_none());
    assert!(
        s.journal
            .events()
            .contains(&("ban.lifted".into(), "manual unblock: ip:9.9.9.9".into()))
    );
}

/// Go TestDualControlPermanentBan, request half: a permanent ban is pending,
/// not applied, and the requester cannot self-approve. Without a retained
/// signed permit the distinct approval commits nothing (Rust difference).
#[tokio::test]
async fn permanent_ban_commit_without_permit_is_refused() {
    let s = stack().await;
    let req = s
        .admin_do(
            s.token("operator"),
            "POST",
            "bans",
            r#"{"Key":"ip:5.5.5.5","DurationSec":0,"Reason":"abuse"}"#,
        )
        .await;
    assert_eq!(req.status, 200);
    let id = approval_id(&req);
    assert_eq!(id.len(), 24);
    assert_eq!(
        req.json(),
        json!({"pending": true, "approvalId": id, "needsRole": "approver"})
    );
    assert!(s.banned("ip:5.5.5.5").is_none());
    let w = s
        .admin_do(s.token("operator"), "POST", &format!("approvals/{id}"), "")
        .await;
    assert_eq!(
        (w.status, w.body.as_str()),
        (403, "role operator not permitted\n")
    );
    let w = s
        .admin_do(s.token("approver"), "POST", &format!("approvals/{id}"), "")
        .await;
    assert_eq!(w.status, 409, "{}", w.body);
    assert!(s.banned("ip:5.5.5.5").is_none());
    assert!(s.journal.has("approval.granted"));
    assert!(!s.journal.has("ban.applied"));
    // The approval was consumed: a second commit finds nothing pending.
    let w = s
        .admin_do(s.token("approver"), "POST", &format!("approvals/{id}"), "")
        .await;
    assert_eq!(
        (w.status, w.body.as_str()),
        (403, "no such pending action\n")
    );
}

// --- wargame_escalation_r51_test.go ----------------------------------------

#[tokio::test]
async fn r066_http_empty_ip_ban_400() {
    assert_http_ban_rejected(r#"{"Key":"ip:","DurationSec":60}"#).await;
}

#[tokio::test]
async fn r067_http_newline_ip_ban_400() {
    assert_http_ban_rejected(r#"{"Key":"ip:1.2.3.4\n","DurationSec":60}"#).await;
}

#[tokio::test]
async fn r068_http_garbage_cidr_ban_400() {
    assert_http_ban_rejected(r#"{"Key":"cidr:nope","DurationSec":0}"#).await;
}

#[tokio::test]
async fn r069_http_short_fp_ban_400() {
    assert_http_ban_rejected(r#"{"Key":"fp:x","DurationSec":60}"#).await;
}

#[tokio::test]
async fn r070_bulk_skips_invalid_keys() {
    let s = stack().await;
    let body = r#"{"Keys":["ip:","ip:203.0.113.50","fp:ab","fp:abcd1234"],"DurationSec":3600,"Reason":"r70"}"#;
    let w = s
        .admin_do(s.token("operator"), "POST", "bans/bulk", body)
        .await;
    assert_eq!(w.status, 200, "{}", w.body);
    assert_eq!(w.json(), json!({"ok": true, "applied": 2, "skipped": 2}));
    assert!(s.banned("ip:203.0.113.50").is_some());
    assert_eq!(
        s.banned("fp:abcd1234").and_then(|entry| entry.incident),
        Some("bulk".into())
    );
    assert!(Key::parse("ip:").is_err());
    assert_eq!(s.bans.lock().unwrap().list(s.now()).len(), 2);
}

#[tokio::test]
async fn r079_erasure_requires_dpo_not_approver() {
    let s = stack().await;
    let req = s
        .admin_do(
            s.token("operator"),
            "POST",
            "erasure",
            r#"{"Subject":"subj-r79","LegalBasis":"test"}"#,
        )
        .await;
    assert_eq!(req.status, 200);
    assert!(req.body.contains("approvalId"));
    assert_eq!(req.json()["needsRole"], "dpo");
    let id = approval_id(&req);
    let w = s
        .admin_do(s.token("approver"), "POST", &format!("approvals/{id}"), "")
        .await;
    assert_eq!(
        (w.status, w.body.as_str()),
        (403, "approver lacks the required role\n")
    );
}

#[tokio::test]
async fn r080_killswitch_requires_distinct_approver() {
    let s = stack().await;
    let req = s
        .admin_do(s.token("operator"), "POST", "killswitch", r#"{"On":true}"#)
        .await;
    let id = approval_id(&req);
    assert!(!id.is_empty(), "{}", req.body);
    let w = s
        .admin_do(s.token("operator"), "POST", &format!("approvals/{id}"), "")
        .await;
    assert_ne!(w.status, 200);
    assert_eq!(w.status, 403);
}

// --- wargame_escalation_r161_test.go ---------------------------------------

#[tokio::test]
async fn r161_pending_ban_not_enforced_until_commit() {
    let s = stack().await;
    let req = s
        .admin_do(
            s.token("operator"),
            "POST",
            "bans",
            r#"{"Key":"ip:203.0.113.161","DurationSec":0,"Reason":"r161"}"#,
        )
        .await;
    assert!(req.body.contains("pending"), "{}", req.body);
    assert!(s.banned("ip:203.0.113.161").is_none());
}

#[tokio::test]
async fn r164_body_approver_still_ignored_on_ipv6_permanent() {
    let s = stack().await;
    let body =
        r#"{"Key":"ip:2001:db8::164","DurationSec":0,"Approver":"forged","Operator":"forged"}"#;
    let w = s.admin_do(s.token("operator"), "POST", "bans", body).await;
    assert!(w.body.contains("pending"), "{}", w.body);
    let pending = s.plane.approvals.pending(s.now());
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].requester, "operator-1");
    assert!(!pending[0].params.values().any(|value| value == "forged"));
}

#[tokio::test]
async fn r165_fp_ban_temporary_direct() {
    let s = stack().await;
    let w = s
        .admin_do(
            s.token("operator"),
            "POST",
            "bans",
            r#"{"Key":"fp:deadbeefcafebabe","DurationSec":120,"Reason":"r165"}"#,
        )
        .await;
    assert_eq!(w.status, 200);
    assert!(!w.body.contains("pending"), "{}", w.body);
    let entry = s.banned("fp:deadbeefcafebabe").unwrap();
    assert_eq!(entry.until, Some(entry.created + Duration::from_secs(120)));
}

/// Go R166 soft-checks the chain through `GET integrity`. This plane has no
/// chain attached, so that route answers 404 here and the meta-audit record of
/// the auditor's ban-list access is asserted directly; the chain-backed half
/// is `chain::tests::wargame_r166_meta_audit_on_ban_list_keeps_the_chain_intact`.
#[tokio::test]
async fn r166_meta_audit_on_ban_list() {
    let s = stack().await;
    let w = s.admin_do(s.token("auditor"), "GET", "bans", "").await;
    assert_eq!(w.status, 200);
    let records = s.journal.records.lock().unwrap().clone();
    let access = records
        .iter()
        .find(|record| record.event_type == "admin.access")
        .unwrap();
    assert_eq!(access.fail_reason, "bans (role=auditor)");
    assert_eq!(access.enforcement_action, "GET");
    assert_eq!(access.route_class, "control");
    assert_eq!(access.actor.kind, "operator");
    assert_eq!(access.actor.id_pseudonym.len(), 64);
    assert_ne!(access.actor.id_pseudonym, "auditor-1");
    drop(records);
    let w = s.admin_do(s.token("auditor"), "GET", "integrity", "").await;
    assert_eq!(w.status, 404);
}

#[tokio::test]
async fn r167_killswitch_pending_before_commit() {
    let s = stack().await;
    let req = s
        .admin_do(s.token("operator"), "POST", "killswitch", r#"{"On":true}"#)
        .await;
    assert!(req.body.contains("pending"), "{}", req.body);
    let pending = s.plane.approvals.pending(s.now());
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, "killswitch");
    assert_eq!(pending[0].params["on"], "true");
    // The plane holds no kill effect; even a distinct approval commits nothing.
    let w = s
        .admin_do(
            s.token("approver"),
            "POST",
            &format!("approvals/{}", approval_id(&req)),
            "",
        )
        .await;
    assert_eq!(w.status, 409, "{}", w.body);
}

#[tokio::test]
async fn r186_http_reject_unknown_scheme() {
    assert_http_ban_rejected(r#"{"Key":"asn:64500","DurationSec":60}"#).await;
}

#[tokio::test]
async fn r187_http_reject_bare_ip() {
    assert_http_ban_rejected(r#"{"Key":"1.2.3.4","DurationSec":60}"#).await;
}

#[tokio::test]
async fn r190_operator_lift_other_key_noop() {
    let s = stack().await;
    let w = s
        .admin_do(
            s.token("operator"),
            "POST",
            "bans/lift?key=ip:203.0.113.190",
            "",
        )
        .await;
    assert!(w.status < 500);
    assert_eq!(w.json(), json!({"ok": true, "lifted": false}));
    let w = s
        .admin_do(s.token("operator"), "POST", "bans/lift?key=garbage", "")
        .await;
    assert_eq!(w.json(), json!({"ok": true, "lifted": false}));
    let w = s
        .admin_do(s.token("operator"), "POST", "bans/lift", "")
        .await;
    assert_eq!((w.status, w.body.as_str()), (400, "key required\n"));
}

// --- Rust boundaries -------------------------------------------------------

/// The served route table: reads for any role, unknown routes and methods 404
/// after authentication, approvals listed with their needed role.
#[tokio::test]
async fn route_table_follows_go_dispatch() {
    let s = stack().await;
    let w = s.admin_do(s.token("dpo"), "GET", "whoami", "").await;
    assert_eq!(w.json(), json!({"id": "dpo-1", "role": "dpo"}));
    for (method, sub) in [("GET", "policy"), ("PUT", "bans"), ("GET", "bans/lift")] {
        let w = s.admin_do(s.token("operator"), method, sub, "").await;
        assert_eq!(w.status, 404, "{method} {sub}");
    }
    let w = s
        .admin_do(
            s.token("operator"),
            "POST",
            "erasure",
            r#"{"Subject":"s","LegalBasis":"GDPR Art.17"}"#,
        )
        .await;
    let id = approval_id(&w);
    let list = s.admin_do(s.token("auditor"), "GET", "approvals", "").await;
    assert_eq!(
        list.json(),
        json!({"count": 1, "pending": [{
            "id": id, "kind": "erasure", "needsRole": "dpo",
            "params": {"legalBasis": "GDPR Art.17", "subject": "s"},
        }]})
    );
    // Auditors and operators cannot approve; a DPO reaches the commit, which
    // needs the vault erasure permit this plane does not hold.
    let w = s
        .admin_do(s.token("auditor"), "POST", &format!("approvals/{id}"), "")
        .await;
    assert_eq!(w.status, 403);
    let w = s
        .admin_do(s.token("dpo"), "POST", &format!("approvals/{id}"), "")
        .await;
    assert_eq!(w.status, 409, "{}", w.body);
}

/// Go `encoding/json` struct decoding and the handler refusal order.
#[tokio::test]
async fn ban_bodies_decode_like_go() {
    let s = stack().await;
    let op = s.token("operator");
    for (body, status, text) in [
        ("", 400, "bad request\n"),
        ("null", 400, "bad request\n"),
        ("[]", 400, "bad request\n"),
        (
            r#"{"Key":"ip:1.2.3.4","DurationSec":"60"}"#,
            400,
            "bad request\n",
        ),
        (
            r#"{"Key":"ip:1.2.3.4","DurationSec":60.5}"#,
            400,
            "bad request\n",
        ),
        (r#"{"Key":7}"#, 400, "bad request\n"),
        (
            r#"{"Key":"ip:x","DurationSec":-1}"#,
            400,
            &format!("{BAD_KEY}\n"),
        ),
        (
            r#"{"Key":"ip:1.2.3.4","DurationSec":-1}"#,
            400,
            "durationSec must be >= 0\n",
        ),
        (
            r#"{"Key":"ip:1.2.3.4","DurationSec":60,"Reason":"a\u0007b"}"#,
            400,
            "bad request\n",
        ),
    ] {
        let w = s.admin_do(op, "POST", "bans", body).await;
        assert_eq!((w.status, w.body.as_str()), (status, text), "{body}");
    }
    let oversized = format!(
        r#"{{"Key":"ip:1.2.3.4","DurationSec":60,"Reason":"{}"}}"#,
        "x".repeat(BAN_BODY)
    );
    assert_eq!(
        object(oversized.as_bytes(), BAN_BODY),
        None,
        "Go MaxBytesReader cap"
    );
    // Case-insensitive names, ignored unknown fields and trailing data.
    let w = s
        .admin_do(
            op,
            "POST",
            "bans",
            r#"{"key":"ip:192.0.2.9","durationsec":60,"Extra":1} trailing"#,
        )
        .await;
    assert_eq!(w.status, 200, "{}", w.body);
    assert!(s.banned("ip:192.0.2.9").is_some());
    // Effectively permanent and broad bans are pending, never applied.
    let cap = max_term_secs();
    for body in [
        format!(r#"{{"Key":"ip:192.0.2.10","DurationSec":{cap}}}"#),
        r#"{"Key":"cidr:198.51.100.0/24","DurationSec":60}"#.to_owned(),
    ] {
        let w = s.admin_do(op, "POST", "bans", &body).await;
        assert_eq!(w.json()["pending"], true, "{body}");
    }
    assert!(s.banned("ip:192.0.2.10").is_none());
    for (body, text) in [
        (r#"{"Keys":[]}"#, "keys required\n"),
        (r#"{"Keys":[1]}"#, "keys required\n"),
        (
            r#"{"Keys":["ip:192.0.2.11"],"DurationSec":0}"#,
            "bulk bans must be temporary and bounded (0/negative/over-cap needs dual-control)\n",
        ),
    ] {
        let w = s.admin_do(op, "POST", "bans/bulk", body).await;
        assert_eq!((w.status, w.body.as_str()), (400, text), "{body}");
    }
    for (body, text) in [
        (r#"{"Subject":""}"#, "subject required\n"),
        (
            r#"{"Subject":"s"}"#,
            "legal basis required (GDPR Art.17 / PIPA)\n",
        ),
    ] {
        let w = s.admin_do(op, "POST", "erasure", body).await;
        assert_eq!((w.status, w.body.as_str()), (400, text), "{body}");
    }
    let w = s
        .admin_do(op, "POST", "killswitch", r#"{"On":"yes"}"#)
        .await;
    assert_eq!((w.status, w.body.as_str()), (400, "bad request\n"));
    let w = s.admin_do(op, "POST", "killswitch", "null").await;
    assert_eq!(w.json()["pending"], true);
}

/// Mutations are audited before their effect; an audit failure answers 503
/// and applies nothing, while unauthenticated requests stay 404.
#[tokio::test]
async fn audit_failure_fails_closed() {
    let s = stack().await;
    s.journal.fail.store(true, Ordering::SeqCst);
    let w = s
        .admin_do(
            s.token("operator"),
            "POST",
            "bans",
            r#"{"Key":"ip:192.0.2.20","DurationSec":60}"#,
        )
        .await;
    assert_eq!((w.status, w.body.as_str()), (503, "audit unavailable\n"));
    assert!(s.banned("ip:192.0.2.20").is_none());
    let w = s.admin_do("", "GET", "bans", "").await;
    assert_eq!(w.status, 404);
}

#[test]
fn go_encoder_escaping_and_query_decoding() {
    assert_eq!(
        go_escape(br#"{"a":"<b>&"}"#.to_vec()),
        b"{\"a\":\"\\u003cb\\u003e\\u0026\"}\n".to_vec()
    );
    assert_eq!(
        go_escape("\"\u{2028}\u{2029}\u{e9}\"".as_bytes().to_vec()),
        b"\"\\u2028\\u2029\xc3\xa9\"\n".to_vec()
    );
    assert_eq!(query_value("/x?key=ip:1.2.3.4", "key"), "ip:1.2.3.4");
    assert_eq!(
        query_value("/x?key=ip%3A2001%3Adb8%3A%3A1", "key"),
        "ip:2001:db8::1"
    );
    assert_eq!(query_value("/x?key=a+b&key=c", "key"), "a b");
    assert_eq!(query_value("/x?key=%zz&key=ok", "key"), "ok");
    assert_eq!(query_value("/x?a;key=bad&key=ok", "key"), "ok");
    assert_eq!(query_value("/x", "key"), "");
}

#[tokio::test]
async fn listener_must_be_loopback() {
    let plane = Arc::new(AdminPlane::new(
        "test",
        scope(),
        b"k",
        ban_store(),
        Box::new(Journal::default()),
    ));
    let listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
    let (_stop, shutdown) = watch::channel(None);
    let error = plane.serve_until(listener, shutdown).await.unwrap_err();
    assert_eq!(error.to_string(), "loopback only");
}

/// Commits with a retained signed permit from a real control ledger. The two
/// fixture administrators stand in for the out-of-band signers.
#[cfg(unix)]
mod dual_control {
    use super::*;
    use crate::audit::control::{
        Administrators, Approval, ApprovalRequest, ControlAuthority, LedgerBootstrap,
        NativeAnchorRetainer, RetentionRequirement,
    };
    use crate::audit::keys::custody::Owner;
    use crate::audit::keys::vault::ErasurePermit;
    use ed25519_dalek::{Signer, SigningKey};
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Dir(PathBuf);

    impl Dir {
        fn new() -> Self {
            use std::os::unix::fs::PermissionsExt;
            let path = std::env::temp_dir().join(format!(
                "humanymous-admin-plane-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            Self(std::fs::canonicalize(path).unwrap())
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn administrators(scope: Scope) -> Administrators {
        Administrators::new(
            scope,
            BTreeMap::from([
                (
                    "one".into(),
                    SigningKey::from_bytes(&[11; 32]).verifying_key().to_bytes(),
                ),
                (
                    "two".into(),
                    SigningKey::from_bytes(&[12; 32]).verifying_key().to_bytes(),
                ),
            ]),
            100,
        )
        .unwrap()
    }

    struct Ledger {
        authority: ControlAuthority,
        retainer: NativeAnchorRetainer,
        _dirs: (Dir, Dir),
    }

    impl Ledger {
        fn new(scope: Scope) -> Self {
            let local = Dir::new();
            let independent = Dir::new();
            let authority = ControlAuthority::open(
                &local.0,
                Owner::CurrentUser,
                &[13; 32],
                administrators(scope.clone()),
                20,
                LedgerBootstrap::New,
            )
            .unwrap();
            let retainer = NativeAnchorRetainer::open(
                &independent.0,
                Owner::CurrentUser,
                scope,
                authority.public_key(),
                authority.anchor(),
                true,
            )
            .unwrap();
            Self {
                authority,
                retainer,
                _dirs: (local, independent),
            }
        }

        fn permit(&mut self, action: Action, target: String) -> RetainedActionPermit {
            let now = crate::authority::now_ms().unwrap() / 1000;
            let proposal = self
                .authority
                .propose(action, target, now, now + 90)
                .unwrap();
            let approvals = signatures(&proposal);
            self.authority
                .authorize_retained_action(
                    ApprovalRequest {
                        proposal: &proposal,
                        approvals: &approvals,
                        now,
                    },
                    &mut self.retainer,
                    RetentionRequirement::FileSynced,
                )
                .unwrap()
        }

        fn ban_permit(&mut self, key: &str) -> RetainedActionPermit {
            self.permit(Action::PermanentBan, Key::parse(key).unwrap().commitment())
        }

        /// An exact-subject erasure permit issued by the ledger after two
        /// signed approvals (`ControlAuthority::issue_erasure`).
        fn erasure_permit(&mut self, subject: &str) -> ErasurePermit {
            let now = crate::authority::now_ms().unwrap() / 1000;
            let target = crate::audit::control::target_commitment(subject.as_bytes());
            let proposal = self
                .authority
                .propose(Action::EraseSubject, target, now, now + 90)
                .unwrap();
            let approvals = signatures(&proposal);
            self.authority
                .issue_erasure(&proposal, &approvals, now, subject)
                .unwrap()
                .0
        }
    }

    fn signatures(proposal: &crate::audit::control::Proposal) -> Vec<Approval> {
        [("one", 11u8), ("two", 12u8)]
            .iter()
            .map(|(id, seed)| Approval {
                administrator: (*id).into(),
                signature: hex::encode(
                    SigningKey::from_bytes(&[*seed; 32])
                        .sign(&proposal.signing_bytes().unwrap())
                        .to_bytes(),
                ),
            })
            .collect()
    }

    async fn request_permanent(s: &Stack, key: &str, reason: &str) -> String {
        let body = format!(r#"{{"Key":"{key}","DurationSec":0,"Reason":"{reason}"}}"#);
        let req = s.admin_do(s.token("operator"), "POST", "bans", &body).await;
        assert!(req.body.contains("pending"), "{}", req.body);
        approval_id(&req)
    }

    async fn commit(s: &Stack, id: &str) -> Reply {
        s.admin_do(s.token("approver"), "POST", &format!("approvals/{id}"), "")
            .await
    }

    /// Go TestDualControlPermanentBan: request, refused self-approval, then a
    /// distinct approver commits and the ban applies.
    #[tokio::test]
    async fn dual_control_permanent_ban() {
        let s = stack().await;
        let mut ledger = Ledger::new(scope());
        s.plane
            .retain_ban_permit(ledger.ban_permit("ip:5.5.5.5"))
            .unwrap();
        let id = request_permanent(&s, "ip:5.5.5.5", "abuse").await;
        assert!(s.banned("ip:5.5.5.5").is_none());
        let w = s
            .admin_do(s.token("operator"), "POST", &format!("approvals/{id}"), "")
            .await;
        assert_ne!(w.status, 200);
        let w = commit(&s, &id).await;
        assert_eq!(
            (w.status, w.body.as_str()),
            (
                200,
                "{\"committed\":\"ban\",\"ok\":true,\"permanent\":true}\n"
            )
        );
        let entry = s.banned("ip:5.5.5.5").unwrap();
        assert!(entry.permanent());
        assert_eq!(entry.source, Source::DualControl);
        assert!(s.journal.events().iter().any(|(kind, reason)| kind == "ban.applied"
            && reason
                == "dual_control ban (permanent): abuse (dual-control: operator-1\u{2192}approver-1)"));
    }

    /// Go R162: after the commit the shared ban store reports the key banned
    /// (the store an edge gate consults; edge wiring is not in this plane).
    #[tokio::test]
    async fn r162_commit_then_ban_store_denies() {
        let s = stack().await;
        let mut ledger = Ledger::new(scope());
        s.plane
            .retain_ban_permit(ledger.ban_permit("ip:203.0.113.162"))
            .unwrap();
        let id = request_permanent(&s, "ip:203.0.113.162", "r162").await;
        assert_eq!(commit(&s, &id).await.status, 200);
        assert!(
            s.banned("ip:203.0.113.162")
                .is_some_and(|entry| entry.permanent())
        );
    }

    /// Go R163 lifts the committed permanent ban. Owner decision 2026-11-16
    /// (decision 2): a permanent ban has no lift action, so the lift is
    /// refused and the ban stays; a temporary ban cannot downgrade it either.
    #[tokio::test]
    async fn r163_lift_after_permanent_commit_is_refused() {
        let s = stack().await;
        let mut ledger = Ledger::new(scope());
        s.plane
            .retain_ban_permit(ledger.ban_permit("ip:203.0.113.163"))
            .unwrap();
        let id = request_permanent(&s, "ip:203.0.113.163", "r163").await;
        assert_eq!(commit(&s, &id).await.status, 200);
        let lift = s
            .admin_do(
                s.token("operator"),
                "POST",
                "bans/lift?key=ip:203.0.113.163",
                "",
            )
            .await;
        assert_eq!(
            (lift.status, lift.body.as_str()),
            (409, "a permanent ban has no lift action\n")
        );
        let w = s
            .admin_do(
                s.token("operator"),
                "POST",
                "bans",
                r#"{"Key":"ip:203.0.113.163","DurationSec":60}"#,
            )
            .await;
        assert_eq!(w.status, 409);
        let w = s
            .admin_do(
                s.token("operator"),
                "POST",
                "bans/bulk",
                r#"{"Keys":["ip:203.0.113.163"],"DurationSec":60}"#,
            )
            .await;
        assert_eq!(w.json(), json!({"ok": true, "applied": 0, "skipped": 1}));
        assert!(
            s.banned("ip:203.0.113.163")
                .is_some_and(|entry| entry.permanent())
        );
        assert!(!s.journal.has("ban.lifted"));
    }

    /// A permit binds one key: it cannot commit another key's pending ban,
    /// and staging refuses other actions and other scopes.
    #[tokio::test]
    async fn permit_binds_key_action_and_scope() {
        let s = stack().await;
        let mut ledger = Ledger::new(scope());
        s.plane
            .retain_ban_permit(ledger.ban_permit("ip:192.0.2.30"))
            .unwrap();
        let id = request_permanent(&s, "ip:192.0.2.31", "other").await;
        assert_eq!(commit(&s, &id).await.status, 409);
        assert!(s.banned("ip:192.0.2.31").is_none());
        let kill = ledger.permit(
            Action::KillSwitch,
            Key::parse("ip:192.0.2.30").unwrap().commitment(),
        );
        assert_eq!(s.plane.retain_ban_permit(kill), Err(PermitRefusal::Action));
        let mut foreign = Ledger::new(Scope {
            installation: "other".into(),
            node: "node".into(),
        });
        assert_eq!(
            s.plane
                .retain_ban_permit(foreign.ban_permit("ip:192.0.2.30")),
            Err(PermitRefusal::Scope)
        );
        // The matching permit still commits a broad ban with a finite term.
        s.plane
            .retain_ban_permit(ledger.ban_permit("cidr:198.51.100.0/24"))
            .unwrap();
        let w = s
            .admin_do(
                s.token("operator"),
                "POST",
                "bans",
                r#"{"Key":"cidr:198.51.100.0/24","DurationSec":60}"#,
            )
            .await;
        let w = commit(&s, &approval_id(&w)).await;
        assert_eq!(w.json()["permanent"], false);
        let entry = s.banned("cidr:198.51.100.0/24").unwrap();
        assert_eq!(entry.until, Some(entry.created + Duration::from_secs(60)));
    }

    /// Go TestPermanentBanIgnoresBodyApprover (wargame r01): forged `Operator`
    /// and `Approver` body fields leave the permanent ban pending, the
    /// requester cannot self-approve, and only a distinct approver bearer
    /// commits it. Rust difference: the commit also consumes the retained
    /// signed permit for this key. The ban is observed in the shared store an
    /// edge gate consults (edge wiring is not in this plane).
    #[tokio::test]
    async fn r01_permanent_ban_ignores_body_approver() {
        let s = stack().await;
        let mut ledger = Ledger::new(scope());
        s.plane
            .retain_ban_permit(ledger.ban_permit("ip:198.51.100.77"))
            .unwrap();
        let body = r#"{"Key":"ip:198.51.100.77","Reason":"wargame r01","DurationSec":0,"Operator":"evil","Approver":"already-approved"}"#;
        let aw = s.admin_do(s.token("operator"), "POST", "bans", body).await;
        assert_eq!(aw.status, 200, "{}", aw.body);
        let id = approval_id(&aw);
        assert_eq!(aw.json()["pending"], true, "{}", aw.body);
        assert!(!id.is_empty(), "{}", aw.body);
        assert_eq!(aw.json()["needsRole"], "approver");
        assert!(
            s.banned("ip:198.51.100.77").is_none(),
            "pending permanent ban must not enforce until Approver commits"
        );
        let pending = s.plane.approvals.pending(s.now());
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].requester, "operator-1");
        assert!(
            !pending[0]
                .params
                .values()
                .any(|value| value == "evil" || value == "already-approved")
        );
        let own = s
            .admin_do(s.token("operator"), "POST", &format!("approvals/{id}"), "")
            .await;
        assert_ne!(own.status, 200, "operator must not self-approve");
        let ok = commit(&s, &id).await;
        assert_eq!(ok.status, 200, "{}", ok.body);
        assert!(ok.body.contains("committed"), "{}", ok.body);
        let entry = s
            .banned("ip:198.51.100.77")
            .expect("after Approver commit the ban must enforce");
        assert!(entry.permanent());
        assert_eq!(entry.source, Source::DualControl);
    }

    mod erasure;
}

mod wargame;
