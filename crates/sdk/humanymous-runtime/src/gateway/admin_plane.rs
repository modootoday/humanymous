//! Authenticated admin HTTP plane: the ban, approval, erasure-request,
//! kill-switch-request, policy-read and console routes of legacy
//! `internal/gate/admin.go` (`handleAdmin`, `AdminHandler` and the route table)
//! at baseline 981d25c.
//!
//! The plane serves only its own loopback listener under [`PREFIX`]; the public
//! application listener keeps refusing every `/_humanymous/` path it does not
//! own, so the admin surface is never reachable there. It composes
//! `gateway::admin_approval` (bearer roles and the pending two-person store),
//! `gateway::ban` (the bounded ban store, shared by `Arc` so an edge gate can
//! consult the same entries) and an `authority::decision::DurableAudit` sink.
//!
//! Kept from Go:
//! - Paths outside [`PREFIX`] answer `404 page not found`. A missing or unknown
//!   bearer token answers the same 404 and records `admin.auth_fail`.
//! - Every authenticated request records `admin.access` before dispatch.
//! - Dispatch walks an ordered table of method, path matcher, capability and
//!   handler. The first match wins; a matched route the role may not use
//!   answers `403 role <role> not permitted`; no match answers 404.
//! - Request bodies are decoded like Go `encoding/json` into the legacy
//!   structs: field names match case-insensitively, unknown fields (such as a
//!   forged `Approver`) are ignored, a type mismatch is a 400, and only the
//!   first JSON value is read. Body caps are the Go `MaxBytesReader` limits.
//! - Error bodies are `text/plain` with a trailing newline, JSON bodies are
//!   `application/json` with sorted map keys, a trailing newline and Go's
//!   HTML-safe escaping. Status codes, messages and refusal order are Go's.
//! - A permanent (`DurationSec` 0), effectively permanent (at least
//!   [`ban::MAX_BULK_TERM`]) or broad (`cidr:`) ban becomes a pending action a
//!   distinct approver must commit. Erasure needs a DPO; the kill switch needs
//!   an approver. Bulk bans skip invalid and broad keys and apply the rest.
//!
//! Deliberate fail-closed differences from Go, each pinned by tests:
//! - Committing an approved permanent or broad ban also consumes a retained
//!   signed dual-control permit (`audit::control`, `Action::PermanentBan`)
//!   staged with [`AdminPlane::retain_ban_permit`] for that exact key. Without
//!   one the commit answers 409 and nothing is banned: two bearer principals
//!   alone never create a permanent ban.
//! - An approved kill switch answers 409: its effect needs the signed one-way
//!   node kill, which this plane does not hold. An approved erasure is
//!   scheduled into the hold queue only when [`AdminPlane::with_erasure`] is
//!   attached and a ledger-issued erasure permit for that subject is staged
//!   with [`AdminPlane::retain_erasure_permit`]; otherwise it answers 409
//!   (see [`erasure`]). Their pending requests and every approval refusal
//!   match Go.
//! - A permanent ban has no lift action (owner decision 2026-11-16, decision
//!   2): lifting one answers 409 and the ban stays. A temporary ban cannot
//!   overwrite a permanent one.
//! - Keys are canonicalized by `ban::Key`. Reasons and incidents longer than
//!   256 bytes or with control characters are a 400; an empty reason is stored
//!   as [`DEFAULT_REASON`] because the store refuses empty reasons.
//! - Mutations are audited before their effect; an audit failure answers 503
//!   and applies nothing. A failure after the record answers 503.
//! - Records carry a keyed operator pseudonym, not the Go vault pseudonym.
//! - The integrity, proof, audit, incident, keys and checkpoints reads are
//!   served only when a shared audit chain is attached with
//!   [`AdminPlane::with_chain`] (see [`chain`]); otherwise they answer 404
//!   after authentication.
//! - The erasure list and cancel routes are served only with an attached
//!   erasure hold; otherwise they answer 404 after authentication.
//! - The read-only `policy` route is served only when a [`policy::Policy`]
//!   is attached with [`AdminPlane::with_policy`]; otherwise it answers 404
//!   after authentication. The console page is an authenticated `GET` read
//!   (Go served it without authentication and could inject a dev bearer
//!   token into it); see [`policy`].
//! - Every response carries `Cache-Control: no-store`,
//!   `X-Content-Type-Options: nosniff` and `X-Frame-Options: DENY`.
//! - The settings routes (`settings/effective`, `settings/schema`,
//!   `settings/overlays`, `settings/dry-run`, `settings/rollback`) and the
//!   `settings.overlay` and `settings.overlay.rollback` approval commits are
//!   served only when a [`settings::Settings`] engine is attached with
//!   [`AdminPlane::with_settings`] (see [`settings`]); otherwise they answer
//!   404 after authentication (Go answered 503 without a store).
//! - `GET metrics` is Go's Prometheus exposition for any role (owner decision
//!   #5); see [`metrics`]. The virtual-USB routes are not served here; they
//!   answer 404 after authentication.
//! - JSON numbers use Go `encoding/json` text (`1`, not `1.0`).
//!
//! The operator ops routes of legacy cmd/server/ops.go (liveness, score
//! explanation and counters) share this listener under [`ops::OPS_PREFIX`]
//! with their own operator token when attached by [`AdminPlane::with_ops`];
//! see [`ops`].
//!
//! The behavioral-model routes (`GET models`, `POST models/promote`, `POST
//! models/rollback`) and the `ml.model.promote` approval commit are served
//! only when a [`models::Models`] lifecycle is attached with
//! [`AdminPlane::with_models`] (owner decision #12; see [`models`]);
//! otherwise they answer 404 after authentication.
//!
//! `gateway::service` composes the plane on the separate loopback admin TCP
//! listener of the private configuration's `admin` object (owner decision
//! B01 #1), with its bearer tokens, the ban store it shares with the edge ban
//! gate and, with `admin.settings_dir`, the settings engine. The ops token,
//! chain, erasure hold, policy view and model lifecycle are not provisioned
//! there.
pub mod chain;
pub mod erasure;
pub mod metrics;
pub mod models;
pub mod ops;
pub mod policy;
pub mod settings;

use crate::audit::control::{Action, RetainedActionPermit, Scope};
use crate::audit::record::{Actor, Record};
use crate::authority::decision::DurableAudit;
use crate::gateway::admin_approval::{AdminAuth, ApprovalStore, Operator, Role};
use crate::gateway::ban::{self, Key, Source, Store};
use crate::gateway::http::{self, Request};
use crate::gateway::service::ShutdownRequest;
use hmac::{Hmac, KeyInit, Mac};
use humanymous_core::signals::wire::json_float_go;
use serde::Serialize;
use serde_json::{Map, Value, json};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

/// Admin route prefix (Go `ControlPath + "admin/"`, here under the runtime's
/// reserved control prefix).
pub const PREFIX: &str = "/_humanymous/admin/";
/// Pending approval lifetime (Go `NewApprovalStore(cfg.TokenKey, 10*time.Minute)`).
pub const APPROVAL_TTL: Duration = Duration::from_secs(10 * 60);
/// Stored reason when a request leaves it empty.
pub const DEFAULT_REASON: &str = "operator ban";
/// Most staged permanent-ban permits.
pub const MAX_STAGED_PERMITS: usize = 64;
/// Whole-connection budget on the admin listener.
pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);

const TEXT: &str = "text/plain; charset=utf-8";
const JSON: &str = "application/json";
const BAN_BODY: usize = 1 << 16;
const BULK_BODY: usize = 1 << 18;
const KILL_BODY: usize = 1 << 12;
const ERASURE_BODY: usize = 1 << 16;
const BAD_KEY: &str = "ban key must be ip:<addr>, fp:<hash>, or cidr:<addr/mask>";
const PSEUDONYM_DOMAIN: &[u8] = b"humanymous.admin.operator/1\0";

/// One admin response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    /// HTTP status.
    pub status: u16,
    /// `Content-Type` value.
    pub content_type: &'static str,
    /// Exact body bytes.
    pub body: Vec<u8>,
}

impl Response {
    /// Go `http.Error`: plain text with a trailing newline.
    fn text(status: u16, message: &str) -> Self {
        Self {
            status,
            content_type: TEXT,
            body: format!("{message}\n").into_bytes(),
        }
    }

    /// Go `http.NotFound`.
    fn not_found() -> Self {
        Self::text(404, "404 page not found")
    }

    /// Go `writeJSON`: `json.NewEncoder(w).Encode(v)`.
    fn json(value: &impl Serialize) -> Self {
        let mut bytes = Vec::new();
        let encoded = value.serialize(&mut serde_json::Serializer::with_formatter(
            &mut bytes, GoNumbers,
        ));
        match encoded {
            Ok(()) => Self {
                status: 200,
                content_type: JSON,
                body: go_escape(bytes),
            },
            Err(_) => Self::text(500, "response encoding failed"),
        }
    }
}

/// Number text of Go `encoding/json` (`strconv.AppendFloat` shortest, `e`
/// form outside `[1e-6, 1e21)`). `serde_json` writes non-finite floats as
/// `null` before reaching the formatter.
struct GoNumbers;

impl serde_json::ser::Formatter for GoNumbers {
    fn write_f64<W: ?Sized + io::Write>(&mut self, writer: &mut W, value: f64) -> io::Result<()> {
        writer.write_all(json_float_go(value).as_deref().unwrap_or("null").as_bytes())
    }

    fn write_f32<W: ?Sized + io::Write>(&mut self, writer: &mut W, value: f32) -> io::Result<()> {
        // Go encodes a float32 with 32-bit shortest digits.
        let shortest: f64 = value.to_string().parse().unwrap_or(f64::NAN);
        self.write_f64(writer, shortest)
    }
}

/// Go's encoder escapes `<`, `>`, `&`, U+2028 and U+2029 inside strings and
/// ends with a newline. These bytes appear only inside JSON strings.
fn go_escape(bytes: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 1);
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'<' => out.extend_from_slice(b"\\u003c"),
            b'>' => out.extend_from_slice(b"\\u003e"),
            b'&' => out.extend_from_slice(b"\\u0026"),
            0xE2 if bytes.get(index + 1) == Some(&0x80)
                && matches!(bytes.get(index + 2), Some(0xA8 | 0xA9)) =>
            {
                out.extend_from_slice(if bytes[index + 2] == 0xA8 {
                    b"\\u2028"
                } else {
                    b"\\u2029"
                });
                index += 2;
            }
            byte => out.push(byte),
        }
        index += 1;
    }
    out.push(b'\n');
    out
}

/// Request time: monotonic for the approval and ban stores, UTC seconds for
/// signed permit windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    /// Monotonic time since the plane's epoch.
    pub monotonic: Duration,
    /// Trusted UTC Unix seconds.
    pub unix: u64,
}

/// A refused permit staging.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermitRefusal {
    /// The permit is not for `Action::PermanentBan`.
    Action,
    /// The permit's scope is not this plane's scope.
    Scope,
    /// [`MAX_STAGED_PERMITS`] permits are already staged.
    Capacity,
    /// No erasure hold is attached ([`AdminPlane::with_erasure`]).
    Unattached,
    /// The erasure permit does not bind that subject in this plane's scope.
    Binding,
}

impl fmt::Display for PermitRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Action => "permit is not a permanent-ban permit",
            Self::Scope => "permit scope does not match the admin plane",
            Self::Capacity => "too many staged permits",
            Self::Unattached => "no erasure hold is attached to the admin plane",
            Self::Binding => "erasure permit does not bind this subject in the plane scope",
        })
    }
}

impl std::error::Error for PermitRefusal {}

#[derive(Clone, Copy)]
enum Capability {
    Read,
    Operate,
    Approve,
}

#[derive(Clone, Copy)]
enum Matcher {
    Exact(&'static str),
    Prefix(&'static str),
    /// Go `adminWrapped`: prefix and suffix present; the argument is the
    /// remainder after the prefix with the suffix trimmed when it remains.
    Wrapped(&'static str, &'static str),
}

impl Matcher {
    fn capture(self, sub: &str) -> Option<&str> {
        match self {
            Self::Exact(path) => (sub == path).then_some(""),
            Self::Prefix(path) => sub.strip_prefix(path),
            Self::Wrapped(prefix, suffix) => {
                if !sub.ends_with(suffix) {
                    return None;
                }
                let rest = sub.strip_prefix(prefix)?;
                Some(rest.strip_suffix(suffix).unwrap_or(rest))
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Handler {
    ListBans,
    Metrics,
    Integrity,
    Proof,
    Audit,
    Incident,
    Keys,
    Checkpoints,
    Policy,
    Console,
    SettingsEffective,
    SettingsSchema,
    SettingsOverlays,
    SettingsPropose,
    SettingsDryRun,
    SettingsRollback,
    ModelsView,
    ModelsPromote,
    ModelsRollback,
    ListApprovals,
    ListErasures,
    Whoami,
    AddBan,
    BulkBan,
    LiftBan,
    Erasure,
    KillSwitch,
    CancelErasure,
    Approve,
}

/// Ordered dispatch and capability table (Go `adminRoutes`, served subset).
const ROUTES: [(&str, Matcher, Capability, Handler); 30] = [
    (
        "GET",
        Matcher::Exact("bans"),
        Capability::Read,
        Handler::ListBans,
    ),
    (
        "GET",
        Matcher::Exact("integrity"),
        Capability::Read,
        Handler::Integrity,
    ),
    (
        "GET",
        Matcher::Exact("proof"),
        Capability::Read,
        Handler::Proof,
    ),
    (
        "GET",
        Matcher::Exact("audit"),
        Capability::Read,
        Handler::Audit,
    ),
    (
        "GET",
        Matcher::Prefix("incidents/"),
        Capability::Read,
        Handler::Incident,
    ),
    (
        "GET",
        Matcher::Exact("policy"),
        Capability::Read,
        Handler::Policy,
    ),
    (
        "GET",
        Matcher::Exact("metrics"),
        Capability::Read,
        Handler::Metrics,
    ),
    (
        "GET",
        Matcher::Exact(""),
        Capability::Read,
        Handler::Console,
    ),
    (
        "GET",
        Matcher::Exact("console"),
        Capability::Read,
        Handler::Console,
    ),
    (
        "GET",
        Matcher::Exact("settings/effective"),
        Capability::Read,
        Handler::SettingsEffective,
    ),
    (
        "GET",
        Matcher::Exact("settings/schema"),
        Capability::Read,
        Handler::SettingsSchema,
    ),
    (
        "GET",
        Matcher::Exact("settings/overlays"),
        Capability::Read,
        Handler::SettingsOverlays,
    ),
    (
        "POST",
        Matcher::Exact("settings/overlays"),
        Capability::Operate,
        Handler::SettingsPropose,
    ),
    (
        "POST",
        Matcher::Exact("settings/dry-run"),
        Capability::Operate,
        Handler::SettingsDryRun,
    ),
    (
        "POST",
        Matcher::Exact("settings/rollback"),
        Capability::Operate,
        Handler::SettingsRollback,
    ),
    (
        "GET",
        Matcher::Exact("models"),
        Capability::Read,
        Handler::ModelsView,
    ),
    (
        "POST",
        Matcher::Exact("models/promote"),
        Capability::Operate,
        Handler::ModelsPromote,
    ),
    (
        "POST",
        Matcher::Exact("models/rollback"),
        Capability::Operate,
        Handler::ModelsRollback,
    ),
    (
        "GET",
        Matcher::Exact("approvals"),
        Capability::Read,
        Handler::ListApprovals,
    ),
    (
        "GET",
        Matcher::Exact("erasures"),
        Capability::Read,
        Handler::ListErasures,
    ),
    (
        "GET",
        Matcher::Exact("whoami"),
        Capability::Read,
        Handler::Whoami,
    ),
    (
        "GET",
        Matcher::Exact("keys"),
        Capability::Read,
        Handler::Keys,
    ),
    (
        "GET",
        Matcher::Exact("checkpoints"),
        Capability::Read,
        Handler::Checkpoints,
    ),
    (
        "POST",
        Matcher::Exact("bans"),
        Capability::Operate,
        Handler::AddBan,
    ),
    (
        "POST",
        Matcher::Exact("bans/bulk"),
        Capability::Operate,
        Handler::BulkBan,
    ),
    (
        "POST",
        Matcher::Exact("bans/lift"),
        Capability::Operate,
        Handler::LiftBan,
    ),
    (
        "POST",
        Matcher::Exact("erasure"),
        Capability::Operate,
        Handler::Erasure,
    ),
    (
        "POST",
        Matcher::Exact("killswitch"),
        Capability::Operate,
        Handler::KillSwitch,
    ),
    (
        "POST",
        Matcher::Wrapped("erasures/", "/cancel"),
        Capability::Operate,
        Handler::CancelErasure,
    ),
    (
        "POST",
        Matcher::Prefix("approvals/"),
        Capability::Approve,
        Handler::Approve,
    ),
];

/// Console view of one ban (Go `banView`, field order kept).
#[derive(Serialize)]
struct BanView {
    key: String,
    reason: String,
    source: &'static str,
    #[serde(skip_serializing_if = "String::is_empty")]
    incident: String,
    strike: u32,
    permanent: bool,
    #[serde(rename = "expiresInSec")]
    expires_in: u64,
}

fn source_name(source: Source) -> &'static str {
    match source {
        Source::Auto => "auto",
        Source::Manual => "manual",
        Source::DualControl => "dual_control",
    }
}

/// The admin plane. Share it by `Arc` with its listener task.
pub struct AdminPlane {
    node: String,
    scope: Scope,
    auth: AdminAuth,
    approvals: ApprovalStore,
    pseudonym_key: Vec<u8>,
    bans: Arc<Mutex<Store>>,
    permits: Mutex<Vec<RetainedActionPermit>>,
    audit: Mutex<Box<dyn DurableAudit>>,
    ops: Option<ops::Ops>,
    chain: Option<chain::Chain>,
    erasure: Option<erasure::Erasure>,
    policy: Option<policy::Policy>,
    settings: Option<settings::Settings>,
    models: Option<models::Models>,
}

impl AdminPlane {
    /// A plane with an empty bearer table (every request is denied until
    /// operators are added), an approval store keyed by `key`, the shared ban
    /// store and the audit sink. `scope` must match the control ledger that
    /// issues permanent-ban permits.
    pub fn new(
        node: &str,
        scope: Scope,
        key: &[u8],
        bans: Arc<Mutex<Store>>,
        audit: Box<dyn DurableAudit>,
    ) -> Self {
        Self {
            node: node.to_owned(),
            scope,
            auth: AdminAuth::new(),
            approvals: ApprovalStore::new(key, APPROVAL_TTL),
            pseudonym_key: key.to_vec(),
            bans,
            permits: Mutex::new(Vec::new()),
            audit: Mutex::new(audit),
            ops: None,
            chain: None,
            erasure: None,
            policy: None,
            settings: None,
            models: None,
        }
    }

    /// Attach the legacy operator ops routes (Go cmd/server/ops.go) to this
    /// listener under [`ops::OPS_PREFIX`]. Without them every ops path is the
    /// plain 404.
    pub fn with_ops(mut self, ops: ops::Ops) -> Self {
        self.ops = Some(ops);
        self
    }

    /// The bearer table, for seeding operators (Go `Server.Auth`).
    pub fn auth(&self) -> &AdminAuth {
        &self.auth
    }

    /// Stage a retained signed permit for one permanent or broad ban. It is
    /// consumed only by the commit of an approved pending ban whose key
    /// commitment equals the permit target.
    pub fn retain_ban_permit(&self, permit: RetainedActionPermit) -> Result<(), PermitRefusal> {
        if permit.proposal().action != Action::PermanentBan {
            return Err(PermitRefusal::Action);
        }
        if permit.proposal().scope != self.scope {
            return Err(PermitRefusal::Scope);
        }
        let mut permits = self.permits.lock().unwrap_or_else(PoisonError::into_inner);
        if permits.len() >= MAX_STAGED_PERMITS {
            return Err(PermitRefusal::Capacity);
        }
        permits.push(permit);
        Ok(())
    }

    /// Serve one parsed request (Go `AdminHandler` then `handleAdmin`).
    pub fn handle(&self, request: &Request, clock: Clock) -> Response {
        if let Some(response) = self.ops.as_ref().and_then(|ops| ops.handle(request, clock)) {
            return response;
        }
        let path = request.target.split('?').next().unwrap_or("");
        let Some(sub) = path.strip_prefix(PREFIX) else {
            return Response::not_found();
        };
        let Some(operator) = self.auth.authenticate(request.header("authorization")) else {
            // Deny by default: the surface is not even discoverable.
            let _ = self.emit(self.record(
                "admin.auth_fail",
                system(),
                "",
                format!("unauthenticated admin {} {sub}", request.method),
            ));
            return Response::not_found();
        };
        // Meta-audit every authenticated access before serving it.
        if self
            .emit(self.record(
                "admin.access",
                self.actor(&operator.id),
                &request.method,
                format!("{sub} (role={})", operator.role.as_str()),
            ))
            .is_err()
        {
            return audit_unavailable();
        }
        for (method, matcher, capability, handler) in ROUTES {
            if request.method != method {
                continue;
            }
            let Some(argument) = matcher.capture(sub) else {
                continue;
            };
            let allowed = match capability {
                Capability::Read => operator.can_read(),
                Capability::Operate => operator.can_operate(),
                Capability::Approve => operator.can_approve(),
            };
            if !allowed {
                return Response::text(
                    403,
                    &format!("role {} not permitted", operator.role.as_str()),
                );
            }
            return match handler {
                Handler::ListBans => self.list_bans(clock),
                Handler::Integrity => self.integrity(),
                Handler::Proof => self.proof(&request.target),
                Handler::Audit => self.audit_stream(&request.target),
                Handler::Incident => self.incident(argument, &operator, clock),
                Handler::Keys => self.keys(),
                Handler::Checkpoints => self.checkpoints(),
                Handler::Policy => self.policy_view(),
                Handler::Metrics => self.metrics_view(clock),
                Handler::Console => Self::console_page(),
                Handler::SettingsEffective => self.settings_effective(clock),
                Handler::SettingsSchema => self.settings_schema(),
                Handler::SettingsOverlays => self.settings_overlays(clock),
                Handler::SettingsPropose => self.settings_propose(&request.body, &operator, clock),
                Handler::SettingsDryRun => self.settings_dry_run(&request.body, &operator, clock),
                Handler::SettingsRollback => {
                    self.settings_rollback(&request.body, &operator, clock)
                }
                Handler::ModelsView => self.models_view(),
                Handler::ModelsPromote => self.models_promote(&request.body, &operator, clock),
                Handler::ModelsRollback => self.models_rollback(&request.body, &operator),
                Handler::ListApprovals => self.list_approvals(clock),
                Handler::ListErasures => self.list_erasures(clock.monotonic),
                Handler::Whoami => {
                    Response::json(&json!({"id": operator.id, "role": operator.role.as_str()}))
                }
                Handler::AddBan => self.add_ban(&request.body, &operator, clock),
                Handler::BulkBan => self.bulk_ban(&request.body, &operator, clock),
                Handler::LiftBan => self.lift_ban(&request.target, &operator, clock),
                Handler::Erasure => self.erasure(&request.body, &operator, clock),
                Handler::KillSwitch => self.kill_switch(&request.body, &operator, clock),
                Handler::CancelErasure => self.cancel_erasure(argument, &operator),
                Handler::Approve => self.approve(argument, &operator, clock),
            };
        }
        Response::not_found()
    }

    fn list_bans(&self, clock: Clock) -> Response {
        let entries = self.store().list(clock.monotonic);
        let bans: Vec<BanView> = entries
            .into_iter()
            .map(|entry| BanView {
                key: entry.key.as_str().to_owned(),
                reason: entry.reason.clone(),
                source: source_name(entry.source),
                incident: entry.incident.clone().unwrap_or_default(),
                strike: entry.strike,
                permanent: entry.permanent(),
                expires_in: entry.remaining(clock.monotonic).as_secs(),
            })
            .collect();
        Response::json(&json!({"count": bans.len(), "bans": bans}))
    }

    fn list_approvals(&self, clock: Clock) -> Response {
        let pending: Vec<Value> = self
            .approvals
            .pending(clock.monotonic)
            .into_iter()
            .map(|action| {
                json!({
                    "id": action.id,
                    "kind": action.kind,
                    "params": action.params,
                    "needsRole": action.needs_role.as_str(),
                })
            })
            .collect();
        Response::json(&json!({"count": pending.len(), "pending": pending}))
    }

    fn add_ban(&self, body: &[u8], operator: &Operator, clock: Clock) -> Response {
        let bad = || Response::text(400, "bad request");
        let Some(fields) = object(body, BAN_BODY) else {
            return bad();
        };
        let (Ok(key), Ok(why), Ok(incident), Ok(seconds)) = (
            string(&fields, "Key"),
            string(&fields, "Reason"),
            string(&fields, "Incident"),
            integer(&fields, "DurationSec"),
        ) else {
            return bad();
        };
        if key.is_empty() {
            return bad();
        }
        let Ok(parsed) = Key::parse(&key) else {
            return Response::text(400, BAD_KEY);
        };
        if seconds < 0 {
            return Response::text(400, "durationSec must be >= 0");
        }
        let (Some(why), Some(incident)) = (bounded_reason(&why), bounded_incident(&incident))
        else {
            return bad();
        };
        if seconds == 0 || seconds.unsigned_abs() >= max_term_secs() || parsed.is_broad() {
            let params = BTreeMap::from([
                ("key".to_owned(), key.clone()),
                ("reason".to_owned(), why),
                ("incident".to_owned(), incident.unwrap_or_default()),
                ("durationSec".to_owned(), seconds.to_string()),
            ]);
            return self.request_approval(
                "ban",
                &params,
                operator,
                Role::Approver,
                format!("ban pending approval: {key}"),
                clock,
            );
        }
        let term = Duration::from_secs(seconds.unsigned_abs());
        let mut store = self.store();
        if store
            .check(&parsed, clock.monotonic)
            .is_some_and(ban::Entry::permanent)
        {
            return Response::text(409, "a permanent ban is in effect for this key");
        }
        if self.emit(self.ban_record(operator, &why)).is_err() {
            return audit_unavailable();
        }
        match store.add_temporary(&parsed, &why, incident.as_deref(), term, clock.monotonic) {
            Ok(entry) => Response::json(&json!({"ok": true, "permanent": entry.permanent()})),
            Err(_) => Response::text(503, "ban store refused the ban"),
        }
    }

    fn bulk_ban(&self, body: &[u8], operator: &Operator, clock: Clock) -> Response {
        let keys_required = || Response::text(400, "keys required");
        let Some(fields) = object(body, BULK_BODY) else {
            return keys_required();
        };
        let (Ok(keys), Ok(why), Ok(seconds)) = (
            strings(&fields, "Keys"),
            string(&fields, "Reason"),
            integer(&fields, "DurationSec"),
        ) else {
            return keys_required();
        };
        if keys.is_empty() {
            return keys_required();
        }
        if seconds <= 0 || seconds.unsigned_abs() >= max_term_secs() {
            return Response::text(
                400,
                "bulk bans must be temporary and bounded (0/negative/over-cap needs dual-control)",
            );
        }
        let Some(why) = bounded_reason(&why) else {
            return Response::text(400, "bad request");
        };
        let term = Duration::from_secs(seconds.unsigned_abs());
        let (mut applied, mut skipped) = (0_u64, 0_u64);
        let mut store = self.store();
        for raw in &keys {
            let parsed = match Key::parse(raw) {
                Ok(parsed) if !parsed.is_broad() => parsed,
                _ => {
                    skipped += 1;
                    continue;
                }
            };
            if store
                .check(&parsed, clock.monotonic)
                .is_some_and(ban::Entry::permanent)
            {
                skipped += 1;
                continue;
            }
            if self.emit(self.ban_record(operator, &why)).is_err() {
                return audit_unavailable();
            }
            if store
                .add_temporary(&parsed, &why, Some("bulk"), term, clock.monotonic)
                .is_err()
            {
                return Response::text(503, "ban store refused the ban");
            }
            applied += 1;
        }
        Response::json(&json!({"ok": true, "applied": applied, "skipped": skipped}))
    }

    fn lift_ban(&self, target: &str, operator: &Operator, clock: Clock) -> Response {
        let key = query_value(target, "key");
        if key.is_empty() {
            return Response::text(400, "key required");
        }
        let parsed = Key::parse(&key).ok();
        let mut store = self.store();
        // A permanent ban has no lift action; refuse before auditing a lift.
        if parsed.as_ref().is_some_and(|parsed| {
            store
                .check(parsed, clock.monotonic)
                .is_some_and(ban::Entry::permanent)
        }) {
            return Response::text(409, "a permanent ban has no lift action");
        }
        if self
            .emit(self.record(
                "ban.lifted",
                self.actor(&operator.id),
                "pass",
                format!("manual unblock: {key}"),
            ))
            .is_err()
        {
            return audit_unavailable();
        }
        let lifted = parsed.is_some_and(|parsed| store.lift(&parsed).unwrap_or(false));
        Response::json(&json!({"ok": true, "lifted": lifted}))
    }

    fn erasure(&self, body: &[u8], operator: &Operator, clock: Clock) -> Response {
        let subject_required = || Response::text(400, "subject required");
        let Some(fields) = object(body, ERASURE_BODY) else {
            return subject_required();
        };
        let (Ok(subject), Ok(basis)) = (string(&fields, "Subject"), string(&fields, "LegalBasis"))
        else {
            return subject_required();
        };
        if subject.is_empty() {
            return subject_required();
        }
        if basis.is_empty() {
            return Response::text(400, "legal basis required (GDPR Art.17 / PIPA)");
        }
        let subject = match self.erasure_subject(subject) {
            Ok(subject) => subject,
            Err(response) => return response,
        };
        let params = BTreeMap::from([
            ("subject".to_owned(), subject),
            ("legalBasis".to_owned(), basis.clone()),
        ]);
        self.request_approval(
            "erasure",
            &params,
            operator,
            Role::Dpo,
            format!("erasure pending DPO approval; legal_basis={basis}"),
            clock,
        )
    }

    fn kill_switch(&self, body: &[u8], operator: &Operator, clock: Clock) -> Response {
        let Some(on) = object(body, KILL_BODY).and_then(|fields| boolean(&fields, "On").ok())
        else {
            return Response::text(400, "bad request");
        };
        let params = BTreeMap::from([("on".to_owned(), on.to_string())]);
        self.request_approval(
            "killswitch",
            &params,
            operator,
            Role::Approver,
            format!("kill-switch flip pending approval: on={on}"),
            clock,
        )
    }

    fn request_approval(
        &self,
        kind: &str,
        params: &BTreeMap<String, String>,
        operator: &Operator,
        needs: Role,
        reason: String,
        clock: Clock,
    ) -> Response {
        if self
            .emit(self.record("approval.requested", self.actor(&operator.id), "", reason))
            .is_err()
        {
            return audit_unavailable();
        }
        match self
            .approvals
            .create(kind, params, &operator.id, needs, clock.monotonic)
        {
            Ok(action) => Response::json(&json!({
                "pending": true,
                "approvalId": action.id,
                "needsRole": needs.as_str(),
            })),
            Err(error) => Response::text(503, &error.to_string()),
        }
    }

    fn approve(&self, id: &str, operator: &Operator, clock: Clock) -> Response {
        let (action, ticket) = match self.approvals.approve(id, operator, clock.monotonic) {
            Ok(approved) => approved,
            Err(error) => return Response::text(403, &error.to_string()),
        };
        // Erasure additionally requires the approver to be a DPO.
        if action.kind == "erasure" && !operator.can_approve_erasure() {
            return Response::text(403, "erasure approval requires the DPO role");
        }
        if self
            .emit(self.record(
                "approval.granted",
                self.actor(&operator.id),
                "",
                format!(
                    "{} approved by {} (requester {}); ticket={}",
                    action.kind,
                    operator.id,
                    action.requester,
                    &ticket[..12]
                ),
            ))
            .is_err()
        {
            return audit_unavailable();
        }
        match action.kind.as_str() {
            "ban" => self.commit_ban(&action.params, &action.requester, operator, clock),
            "erasure" => self.commit_erasure(
                &action.params,
                &action.requester,
                operator,
                clock.unix,
                clock.monotonic,
            ),
            "killswitch" => Response::text(
                409,
                "kill switch commit needs the signed node kill control; not served here",
            ),
            settings::KIND_OVERLAY => self.commit_settings(&action, operator, clock),
            settings::KIND_ROLLBACK => self.commit_settings_rollback(&action, operator, clock),
            models::KIND_PROMOTE => {
                self.commit_models_promote(&action.params, &action.requester, operator)
            }
            _ => Response::text(500, "unknown action kind"),
        }
    }

    fn commit_ban(
        &self,
        params: &BTreeMap<String, String>,
        requester: &str,
        approver: &Operator,
        clock: Clock,
    ) -> Response {
        let field = |name: &str| params.get(name).map_or("", String::as_str);
        let Ok(key) = Key::parse(field("key")) else {
            return Response::text(500, "invalid pending ban key");
        };
        let seconds: i64 = field("durationSec").parse().unwrap_or(0);
        // 0 or an at/above-cap value is permanent; never rely on overflow.
        let term = (seconds > 0 && seconds.unsigned_abs() < max_term_secs())
            .then(|| Duration::from_secs(seconds.unsigned_abs()));
        let why = bounded_reason(field("reason")).unwrap_or_else(|| DEFAULT_REASON.to_owned());
        let commitment = key.commitment();
        let permit = {
            let mut permits = self.permits.lock().unwrap_or_else(PoisonError::into_inner);
            let Some(index) = permits
                .iter()
                .position(|permit| permit.proposal().target == commitment)
            else {
                return Response::text(
                    409,
                    "permanent or broad ban needs a retained dual-control permit for this key",
                );
            };
            permits.swap_remove(index)
        };
        let mut store = self.store();
        let detail = format!(
            "{} (dual-control: {requester}\u{2192}{})",
            ban_reason("dual_control", term.is_none(), &why),
            approver.id
        );
        if self
            .emit(Record {
                fail_reason: detail,
                ..self.ban_record_for(requester, "dual_control", term.is_none(), &why)
            })
            .is_err()
        {
            self.permits
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(permit);
            return audit_unavailable();
        }
        match store.add_controlled(
            permit,
            &self.scope,
            &key,
            &why,
            term,
            clock.monotonic,
            clock.unix,
        ) {
            Ok(entry) => Response::json(&json!({
                "ok": true,
                "committed": "ban",
                "permanent": entry.permanent(),
            })),
            Err(_) => Response::text(503, "ban commit refused"),
        }
    }

    fn store(&self) -> std::sync::MutexGuard<'_, Store> {
        self.bans.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn emit(&self, record: Record) -> io::Result<()> {
        self.audit
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .append(record)
    }

    fn actor(&self, id: &str) -> Actor {
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(&self.pseudonym_key) else {
            unreachable!("HMAC accepts any key length")
        };
        mac.update(PSEUDONYM_DOMAIN);
        mac.update(id.as_bytes());
        Actor {
            kind: "operator".into(),
            id_pseudonym: hex::encode(mac.finalize().into_bytes()),
        }
    }

    fn record(&self, event: &str, actor: Actor, action: &str, reason: String) -> Record {
        Record {
            event_type: event.into(),
            actor,
            tenant_id: self.node.clone(),
            route_class: "control".into(),
            enforcement_action: action.into(),
            enforcement_mode: "enforce".into(),
            fail_reason: reason,
            key_id: "k1".into(),
            ..Record::default()
        }
    }

    fn ban_record(&self, operator: &Operator, why: &str) -> Record {
        self.ban_record_for(&operator.id, "manual", false, why)
    }

    fn ban_record_for(&self, requester: &str, source: &str, permanent: bool, why: &str) -> Record {
        Record {
            verdict: "DENY".into(),
            ..self.record(
                "ban.applied",
                self.actor(requester),
                "block",
                ban_reason(source, permanent, why),
            )
        }
    }

    /// Serve the plane on its own loopback listener until shutdown (Go: the
    /// admin handler on a separate listener, never the public one). Each
    /// connection carries one bounded request and closes.
    pub async fn serve_until(
        self: Arc<Self>,
        listener: TcpListener,
        shutdown: watch::Receiver<Option<ShutdownRequest>>,
    ) -> io::Result<()> {
        self.serve_from(listener, shutdown, Instant::now()).await
    }

    /// [`Self::serve_until`] with the monotonic `epoch` of the plane's clock
    /// given by the caller, so a ban store shared with the edge ban gate
    /// (installed with the same epoch) sees one ordered time line.
    pub async fn serve_from(
        self: Arc<Self>,
        listener: TcpListener,
        mut shutdown: watch::Receiver<Option<ShutdownRequest>>,
        epoch: Instant,
    ) -> io::Result<()> {
        if !crate::gateway::configuration::exposure::admin_listen_permitted(listener.local_addr()?)
        {
            return Err(io::Error::other("loopback only"));
        }
        loop {
            if shutdown.borrow().is_some() {
                return Ok(());
            }
            let (mut stream, peer) = tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    changed.map_err(|_| io::Error::other("shutdown controller closed"))?;
                    continue;
                }
                // Transient accept errors (EMFILE, aborted peers) are retried
                // with backoff; any other error stops the service (fail closed).
                accepted = crate::gateway::accept::accept(&listener) => accepted?,
            };
            if !peer.ip().is_loopback() {
                continue;
            }
            let _ =
                tokio::time::timeout(CONNECTION_TIMEOUT, self.connection(&mut stream, epoch)).await;
        }
    }

    async fn connection(&self, stream: &mut TcpStream, epoch: Instant) -> io::Result<()> {
        let response = match http::read(stream).await {
            Ok(request) => match crate::authority::now_ms() {
                Ok(ms) => self.handle(
                    &request,
                    Clock {
                        monotonic: epoch.elapsed(),
                        unix: ms / 1000,
                    },
                ),
                Err(_) => Response::text(503, "clock unavailable"),
            },
            Err(rejection) => Response::text(rejection.status, "bad request"),
        };
        write(stream, &response).await
    }
}

fn system() -> Actor {
    Actor {
        kind: "system".into(),
        id_pseudonym: String::new(),
    }
}

fn audit_unavailable() -> Response {
    Response::text(503, "audit unavailable")
}

/// Go `banReason`: `<source> ban (<strike 0|permanent>): <reason>`.
fn ban_reason(source: &str, permanent: bool, why: &str) -> String {
    let term = if permanent { "permanent" } else { "strike 0" };
    format!("{source} ban ({term}): {why}")
}

fn max_term_secs() -> u64 {
    ban::MAX_BULK_TERM.as_secs()
}

/// The ban store's reason rule, with Go's empty reason mapped to the default.
fn bounded_reason(text: &str) -> Option<String> {
    let text = if text.is_empty() {
        DEFAULT_REASON
    } else {
        text
    };
    (text.len() <= 256 && !text.chars().any(char::is_control)).then(|| text.to_owned())
}

/// `None` inside means no incident; the outer `None` is a refusal.
fn bounded_incident(text: &str) -> Option<Option<String>> {
    if text.is_empty() {
        return Some(None);
    }
    (text.len() <= 256 && !text.chars().any(char::is_control)).then(|| Some(text.to_owned()))
}

/// First JSON value of a capped body as an object (Go `Decode` into a struct;
/// JSON `null` leaves the struct zero).
fn object(body: &[u8], cap: usize) -> Option<Map<String, Value>> {
    if body.len() > cap {
        return None;
    }
    match serde_json::Deserializer::from_slice(body)
        .into_iter::<Value>()
        .next()?
    {
        Ok(Value::Object(fields)) => Some(fields),
        Ok(Value::Null) => Some(Map::new()),
        _ => None,
    }
}

/// Go field matching: the exact name first, then a case-insensitive match.
fn field<'a>(fields: &'a Map<String, Value>, name: &str) -> Option<&'a Value> {
    fields.get(name).or_else(|| {
        fields
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value)
    })
}

fn string(fields: &Map<String, Value>, name: &str) -> Result<String, ()> {
    match field(fields, name) {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(text)) => Ok(text.clone()),
        Some(_) => Err(()),
    }
}

fn integer(fields: &Map<String, Value>, name: &str) -> Result<i64, ()> {
    match field(fields, name) {
        None | Some(Value::Null) => Ok(0),
        Some(Value::Number(number)) => number.as_i64().ok_or(()),
        Some(_) => Err(()),
    }
}

fn boolean(fields: &Map<String, Value>, name: &str) -> Result<bool, ()> {
    match field(fields, name) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(()),
    }
}

fn strings(fields: &Map<String, Value>, name: &str) -> Result<Vec<String>, ()> {
    match field(fields, name) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::Null => Ok(String::new()),
                Value::String(text) => Ok(text.clone()),
                _ => Err(()),
            })
            .collect(),
        Some(_) => Err(()),
    }
}

/// Go `r.URL.Query().Get(name)`: the first value of a well-formed pair, with
/// `%XX` and `+` decoded; malformed pairs are skipped; absent is empty.
fn query_value(target: &str, name: &str) -> String {
    let Some((_, query)) = target.split_once('?') else {
        return String::new();
    };
    for pair in query.split('&') {
        if pair.contains(';') {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if let (Some(key), Some(value)) = (unescape(key), unescape(value))
            && key == name
        {
            return value;
        }
    }
    String::new()
}

fn unescape(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                let hex = text.get(index + 1..index + 3)?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                index += 3;
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

async fn write(stream: &mut TcpStream, response: &Response) -> io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        413 => "Content Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "Service Unavailable",
    };
    let head = format!(
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(&response.body).await?;
    stream.shutdown().await
}

#[cfg(test)]
mod tests;
