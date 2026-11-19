//! SoT-39 settings write plane of the admin listener: the `settings/*` routes
//! and the approval-backed commits of legacy internal/gate/admin_settings.go
//! and settings_runtime.go (with the `SetSettingsStore`, `SettingsEffective`
//! and `resolvePath` parts of gate.go) at baseline 981d25c, ported under owner
//! decision B1 (issue #75 comment 6979694428).
//!
//! [`AdminPlane::with_settings`] attaches a [`Settings`]: the boot input the
//! effective posture resolves from, the live kill-switch flag and the
//! file-backed overlay store of `gateway::settings::overlay`. Without it every
//! settings route answers the plain 404 after the role check, as before.
//!
//! An overlay is operator-supplied runtime content. This module adds no
//! default, bound, weight, threshold, hard-rule predicate, verdict band or
//! signal ID: validation, classification, resolution and the configuration
//! version are the frozen ports in `gateway::settings::overlay`, and the
//! effective posture of an empty overlay is the built-in policy.
//!
//! Kept from Go (pinned against
//! `tests/fixtures/gateway/settings-admin-go-golden.json`):
//! - Routes, roles and order: `GET settings/effective`, `settings/schema` and
//!   `settings/overlays` for any role; `POST settings/overlays` (propose),
//!   `settings/dry-run` and `settings/rollback` for operator and DPO.
//! - Propose: the body is decoded like Go's `settingsProposeBody` (only the
//!   first JSON value, case-folded keys, unknown members and a client
//!   `mutationClass` ignored, a type mismatch is `400 bad request`). Refusals
//!   in Go order: stale or empty parent (409), overlay bounds (400 with the
//!   validator text), runtime validation of the candidate (400), class C
//!   without `integrityConfirm=DISABLE-INTEGRITY` (400), lifetime caps (B and
//!   D at most 7 days, default 7 days; C at most 24 hours, default 24 hours;
//!   A none unless asked), then the kill switch refusing B, C and D (409).
//!   The class is always server computed. Class A applies at once; B, C and
//!   D become a `settings.overlay` pending action a distinct approver must
//!   commit, binding the overlay id, parent version, class, exact overlay
//!   JSON and confirmation phrase into the approval ticket.
//! - Commit re-checks the parent version, bounds, class C confirmation and
//!   expiry, recomputes the class against the posture at commit time and
//!   validates the runtime effect; a stale commit answers 409 and records
//!   `settings.overlay.rejected`.
//! - Rollback clears the active overlay with the same CAS. Clearing an
//!   overlay stored as class A (undoing a tightening) needs an approver
//!   (`settings.overlay.rollback`); any other class clears at once; an empty
//!   overlay is a no-op.
//! - Dry-run never changes state: it validates and classifies the overlay and
//!   re-bands up to the newest 1000 scoring and enforcement audit records
//!   under the proposed thresholds and hard-rule demotions. Below 50 records
//!   it reports `usable: false` and no numeric deltas.
//! - Expiry: before every read or write of the posture (and on
//!   [`AdminPlane::sweep_settings`]) an active overlay past its `expiresAt`
//!   is cleared and `settings.overlay.expired` recorded, except while the
//!   kill switch is engaged.
//! - Runtime validation: the effective rate window and soft cutpoint must be
//!   at least 1 and hard at least soft (`settings: invalid effective rate
//!   limit W/S/H`). The rate is hot-applied to the shared ban store's limiter,
//!   which keeps its rolling hits. An attach whose stored overlay fails this
//!   check is refused, like Go `SetSettingsStore`.
//! - Audit event types, actors, modes and reasons equal Go's, including the
//!   `dual-control requester→approver` suffix.
//!
//! Deliberate fail-closed differences, each pinned by a test:
//! - Mutations are serialized, so the parent-version check and the change it
//!   guards are atomic (Go read and wrote the store without a lock).
//! - Records are written before the change. If one cannot be written the
//!   change is not made (the rate limiter is restored) and the request
//!   answers 503; Go wrote them after the change and ignored sink failure. A
//!   pending action whose `approval.requested` record fails is withdrawn.
//! - The rate limiter of the ban store can refuse a Go-valid rate (for
//!   example a window above its resource bound, or a clock that went
//!   backwards); that refusal rejects the change like a runtime validation
//!   failure (`settings: rate limiter refused effective rate limit W/S/H`).
//!   Go's ledger accepted any rate.
//! - Times are whole seconds of the trusted clock; Go stamped nanoseconds.
//!   A lifetime that does not fit an RFC 3339 year (Go overflowed its
//!   duration) is refused with `expiresInSec out of range`.
//! - Without a chain ([`AdminPlane::with_chain`]) the dry-run sees no audit
//!   records and reports `n: 0`, `usable: false`; an unreadable chain answers
//!   503.
//! - Records carry the plane's keyed operator pseudonym.
//!
//! Not ported here (recorded ledger gaps): Prometheus exposition of the
//! settings counters, the pending-approval age gauge and the configuration
//! version label (owner pending); [`Settings::stats`] and
//! [`AdminPlane::settings_pending_age`] keep the values. The served path
//! reads per-gate modes from the overlay this plane publishes through
//! [`Settings::with_active_slot`] (`application::modes`, owner decision #3).
use super::{AdminPlane, Clock, JSON, Response, audit_unavailable, go_escape, system};
use crate::audit::record::{Actor, Record};
use crate::gateway::admin_approval::{Operator, PendingAction, Role};
use crate::gateway::ban::Store;
use crate::gateway::route_policy::{RoutePolicy, RouteTable};
use crate::gateway::settings::overlay::{
    BootInput, Effective, Mode, MutationClass, Overlay, OverlayFileStore, ProposeBody, Timestamp,
    classify, decode_rollback_parent, resolve, schema_json, validate,
};
use humanymous_core::signals::wire::quote_json_go;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use std::time::Duration;

/// Propose and dry-run body cap (Go `MaxBytesReader` 1<<18).
pub const PROPOSE_BODY: usize = 1 << 18;
/// Rollback body cap (Go `MaxBytesReader` 1<<12).
pub const ROLLBACK_BODY: usize = 1 << 12;
/// Fewest scanned records for which the dry-run reports numbers.
pub const DRY_RUN_MIN_N: usize = 50;
/// Most audit records the dry-run scans, newest first.
pub const DRY_RUN_SCAN: usize = 1000;
/// Class C confirmation phrase.
pub const CONFIRM_PHRASE: &str = "DISABLE-INTEGRITY";
/// Longest (and default) class B and D lifetime in seconds.
pub const MAX_LOOSEN_SECS: i64 = 7 * 24 * 3600;
/// Longest (and default) class C lifetime in seconds.
pub const MAX_INTEGRITY_SECS: i64 = 24 * 3600;
/// Pending action kind of a proposed overlay.
pub const KIND_OVERLAY: &str = "settings.overlay";
/// Pending action kind of a dual-control rollback.
pub const KIND_ROLLBACK: &str = "settings.overlay.rollback";

const STALE: &str = "CAS: parentConfigVersion mismatch (stale)";
const DRY_RUN_EVENTS: [&str; 4] = [
    "scoring.evaluated",
    "enforcement.deny",
    "enforcement.challenge.issued",
    "enforcement.allow",
];

/// Lifetime in seconds for a class and a requested `expiresInSec` (Go
/// `expiryForClass`); `None` means no expiry.
pub fn expiry_for_class(class: MutationClass, wanted: i64) -> Result<Option<i64>, &'static str> {
    let capped = |cap: i64, refusal: &'static str| {
        let seconds = if wanted <= 0 { cap } else { wanted };
        if seconds > cap {
            Err(refusal)
        } else {
            Ok(Some(seconds))
        }
    };
    match class {
        MutationClass::A => Ok((wanted > 0).then_some(wanted)),
        MutationClass::B | MutationClass::D => {
            capped(MAX_LOOSEN_SECS, "class B/D expiresInSec max 7d")
        }
        MutationClass::C => capped(MAX_INTEGRITY_SECS, "class C expiresInSec max 24h"),
    }
}

/// Snapshot of the settings counters (Go `settingsRuntimeStats`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettingsStats {
    /// Overlays applied.
    pub applied: u64,
    /// Overlays rolled back to empty.
    pub rolled_back: u64,
    /// Changes rejected (stale, invalid, runtime refusal).
    pub rejected: u64,
    /// Changes that failed in the store after validation.
    pub errors: u64,
    /// Store load, apply, rollback or expiry failures.
    pub store_errors: u64,
}

#[derive(Default)]
struct Counters {
    applied: AtomicU64,
    rolled_back: AtomicU64,
    rejected: AtomicU64,
    errors: AtomicU64,
    store_errors: AtomicU64,
}

fn bump(counter: &AtomicU64) {
    counter.fetch_add(1, Ordering::Relaxed);
}

/// A refused [`AdminPlane::with_settings`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsRefusal {
    /// The effective posture of the stored overlay failed runtime validation
    /// (Go's message).
    Runtime(String),
    /// The ban store's rate limiter refused the effective rate.
    RateLimiter,
    /// The store-degraded record could not be written.
    Audit,
}

impl fmt::Display for SettingsRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(message) => f.write_str(message),
            Self::RateLimiter => f.write_str("settings: rate limiter refused the effective rate"),
            Self::Audit => f.write_str("audit unavailable"),
        }
    }
}

impl std::error::Error for SettingsRefusal {}

/// The settings engine an admin plane serves.
pub struct Settings {
    boot: BootInput,
    kill_switch: Arc<AtomicBool>,
    store: OverlayFileStore,
    mirror: Option<Arc<RwLock<Option<Overlay>>>>,
    serial: Mutex<()>,
    counters: Counters,
    #[cfg(test)]
    next_id: Mutex<Option<String>>,
}

impl Settings {
    /// `boot` is the boot posture (global monitor, route presets, the rate
    /// window and cutpoints the ban store's limiter was built with, and the
    /// configuration-version key); its `kill_switch` is ignored in favour of
    /// the live `kill_switch` flag. `store` holds the active overlay.
    pub fn new(boot: BootInput, kill_switch: Arc<AtomicBool>, store: OverlayFileStore) -> Self {
        Self {
            boot,
            kill_switch,
            store,
            mirror: None,
            serial: Mutex::new(()),
            counters: Counters::default(),
            #[cfg(test)]
            next_id: Mutex::new(None),
        }
    }

    /// Keep `slot` equal to the active overlay (for example the slot read by
    /// the `policy` view) after every change.
    pub fn with_active_slot(mut self, slot: Arc<RwLock<Option<Overlay>>>) -> Self {
        self.mirror = Some(slot);
        self
    }

    /// Effective posture of the active overlay (Go `SettingsEffective`). It
    /// does not expire an overlay; the admin routes do.
    pub fn effective(&self) -> Effective {
        self.effective_for(self.store.active().as_ref())
    }

    /// The active overlay, if any.
    pub fn active(&self) -> Option<Overlay> {
        self.store.active()
    }

    /// Route policy under the effective routes and global monitor when an
    /// overlay is active, otherwise the boot table (Go `resolvePath`).
    pub fn resolve_path(&self, raw_path: &str) -> RoutePolicy {
        let mut table = RouteTable {
            routes: self.boot.routes.clone(),
            global_monitor: self.boot.global_monitor,
        };
        if self.store.active().is_some() {
            let effective = self.effective();
            if !effective.routes.is_empty() {
                table.routes = effective.routes;
            }
            if effective.global_monitor {
                table.global_monitor = true;
            }
        }
        table.resolve(raw_path)
    }

    /// Counter snapshot.
    pub fn stats(&self) -> SettingsStats {
        let load = |counter: &AtomicU64| counter.load(Ordering::Relaxed);
        SettingsStats {
            applied: load(&self.counters.applied),
            rolled_back: load(&self.counters.rolled_back),
            rejected: load(&self.counters.rejected),
            errors: load(&self.counters.errors),
            store_errors: load(&self.counters.store_errors),
        }
    }

    fn kill(&self) -> bool {
        self.kill_switch.load(Ordering::SeqCst)
    }

    fn lock(&self) -> MutexGuard<'_, ()> {
        self.serial.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn effective_for(&self, active: Option<&Overlay>) -> Effective {
        let boot = BootInput {
            kill_switch: self.kill(),
            ..self.boot.clone()
        };
        resolve(&boot, active)
    }

    /// Go `settingsEffectiveCandidate`: the overlay as if active.
    fn candidate(&self, overlay: &Overlay) -> Effective {
        let mut candidate = overlay.clone();
        candidate.status = "active".into();
        self.effective_for(Some(&candidate))
    }

    /// Go `validateSettingsRuntime` for a rate-configurable ban ledger.
    fn validate_runtime(effective: &Effective) -> Result<(), String> {
        if effective.rate_window_sec < 1
            || effective.rate_soft < 1
            || effective.rate_hard < effective.rate_soft
        {
            return Err(format!(
                "settings: invalid effective rate limit {}/{}/{}",
                effective.rate_window_sec, effective.rate_soft, effective.rate_hard
            ));
        }
        Ok(())
    }

    fn publish(&self) {
        if let Some(slot) = &self.mirror {
            *slot.write().unwrap_or_else(PoisonError::into_inner) = self.store.active();
        }
    }

    fn overlay_id(&self) -> Option<String> {
        #[cfg(test)]
        if let Some(id) = self
            .next_id
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            return Some(id);
        }
        let mut bytes = [0_u8; 8];
        getrandom::fill(&mut bytes).ok()?;
        Some(format!("ovl_{}", hex::encode(bytes)))
    }

    /// Use `id` for the next built overlay (differential replay only).
    #[cfg(test)]
    pub(super) fn inject_overlay_id(&self, id: &str) {
        *self.next_id.lock().unwrap() = Some(id.to_owned());
    }
}

/// Why a settings change did not happen.
enum Failure {
    /// A record could not be written: 503.
    Audit,
    /// Rejected or failed with Go's message: 409.
    Conflict(String),
    /// The runtime could not be reconfigured outside a change request: 503.
    Unavailable,
}

impl Failure {
    fn response(self) -> Response {
        match self {
            Self::Audit => audit_unavailable(),
            Self::Conflict(message) => Response::text(409, &message),
            Self::Unavailable => Response::text(503, "settings runtime unavailable"),
        }
    }
}

/// Where a [`AdminPlane::transition`] stopped.
enum Step {
    Rate,
    Audit,
    Store(String),
}

fn rate_triple(effective: &Effective) -> (i64, i64, i64) {
    (
        effective.rate_window_sec,
        effective.rate_soft,
        effective.rate_hard,
    )
}

/// Hot-apply the effective rate to the ban store's limiter (Go
/// `syncSettingsRuntime` through `RateConfigurableBanLedger.ConfigureRate`).
fn configure_rate(store: &mut Store, effective: &Effective, now: Duration) -> Result<(), ()> {
    let window = u64::try_from(effective.rate_window_sec).map_err(|_| ())?;
    let soft = usize::try_from(effective.rate_soft).map_err(|_| ())?;
    let hard = usize::try_from(effective.rate_hard).map_err(|_| ())?;
    store
        .limiter()
        .configure(Duration::from_secs(window), soft, hard, now)
        .map(|_| ())
        .map_err(|_| ())
}

fn now_nanos(clock: Clock) -> i128 {
    i128::from(clock.unix) * 1_000_000_000
}

fn dual(reason: &mut String, requester: &str, approver: &str) {
    if !approver.is_empty() {
        reason.push_str(&format!(" dual-control {requester}\u{2192}{approver}"));
    }
}

/// A JSON object with sorted keys from already-encoded Go JSON values.
fn raw_object(mut fields: Vec<(&str, String)>) -> Response {
    fields.sort_by(|left, right| left.0.cmp(right.0));
    let mut body = String::from("{");
    for (index, (key, value)) in fields.iter().enumerate() {
        if index > 0 {
            body.push(',');
        }
        body.push_str(&quote_json_go(key));
        body.push(':');
        body.push_str(value);
    }
    body.push('}');
    Response {
        status: 200,
        content_type: JSON,
        body: go_escape(body.into_bytes()),
    }
}

fn encoding_failed() -> Response {
    Response::text(500, "response encoding failed")
}

fn effective_json(effective: &Effective) -> Option<String> {
    effective
        .to_json()
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

fn build_overlay(
    body: &ProposeBody,
    id: String,
    operator: &str,
    parent: &str,
    created: Timestamp,
) -> Overlay {
    Overlay {
        schema_version: "1.0.0".into(),
        overlay_id: id,
        created_at: created,
        created_by: operator.into(),
        status: "pending".into(),
        parent_config_version: parent.into(),
        gates: body.gates.clone(),
        hard_rules: body.hard_rules.clone(),
        scoring: body.scoring,
        weight_multipliers: body.weight_multipliers.clone(),
        net_policy: body.net_policy.clone(),
        routes: body.routes.clone(),
        rate_limit: body.rate_limit,
        global_monitor: body.global_monitor,
        ..Overlay::default()
    }
}

impl AdminPlane {
    /// Serve the settings routes from `settings`. Like Go `SetSettingsStore`,
    /// a store load problem is recorded as `settings.store.degraded`, the
    /// effective posture must pass runtime validation, and its rate is
    /// hot-applied to the ban store before the plane serves it.
    pub fn with_settings(
        mut self,
        settings: Settings,
        clock: Clock,
    ) -> Result<Self, SettingsRefusal> {
        if let Some(error) = settings.store.load_error() {
            bump(&settings.counters.store_errors);
            self.emit(self.settings_record(
                "settings.store.degraded",
                system(),
                "monitor",
                format!("load: {error}"),
            ))
            .map_err(|_| SettingsRefusal::Audit)?;
        }
        let effective = settings.effective();
        Settings::validate_runtime(&effective).map_err(SettingsRefusal::Runtime)?;
        configure_rate(&mut self.store(), &effective, clock.monotonic)
            .map_err(|()| SettingsRefusal::RateLimiter)?;
        settings.publish();
        self.settings = Some(settings);
        Ok(self)
    }

    /// The attached settings engine, for example to resolve request paths.
    pub fn settings(&self) -> Option<&Settings> {
        self.settings.as_ref()
    }

    /// Counter snapshot of the attached settings engine.
    pub fn settings_stats(&self) -> Option<SettingsStats> {
        self.settings.as_ref().map(Settings::stats)
    }

    /// Age of the oldest pending settings approval (Go
    /// `settingsPendingAgeSeconds`); zero when there is none.
    pub fn settings_pending_age(&self, now: Duration) -> Duration {
        self.approvals
            .pending(now)
            .iter()
            .filter(|action| action.kind == KIND_OVERLAY || action.kind == KIND_ROLLBACK)
            .map(|action| now.saturating_sub(action.created))
            .max()
            .unwrap_or_default()
    }

    /// Expire the active overlay when it is past its lifetime (Go `GC` calling
    /// `expireActiveOverlayIfNeeded`). Call it on the maintenance tick.
    pub fn sweep_settings(&self, clock: Clock) -> io::Result<()> {
        let Some(settings) = &self.settings else {
            return Ok(());
        };
        let _serial = settings.lock();
        self.expire(settings, clock)
            .map_err(|failure| match failure {
                Failure::Audit => io::Error::other("audit unavailable"),
                _ => io::Error::other("settings runtime unavailable"),
            })
    }

    fn settings_record(&self, event: &str, actor: Actor, mode: &str, reason: String) -> Record {
        Record {
            enforcement_mode: mode.into(),
            ..self.record(event, actor, "", reason)
        }
    }

    /// Go `settingsApplyRejected`: count and record the refusal.
    fn reject(&self, settings: &Settings, message: String) -> Failure {
        bump(&settings.counters.rejected);
        let record = self.settings_record(
            "settings.overlay.rejected",
            system(),
            "monitor",
            message.clone(),
        );
        match self.emit(record) {
            Ok(()) => Failure::Conflict(message),
            Err(_) => Failure::Audit,
        }
    }

    /// Go `recordSettingsStoreError` plus `settingsApplyError`.
    fn store_failure(&self, settings: &Settings, phase: &str, message: String) -> Failure {
        bump(&settings.counters.store_errors);
        bump(&settings.counters.errors);
        let _ = self.emit(self.settings_record(
            "settings.store.degraded",
            system(),
            "monitor",
            format!("{phase}: {message}"),
        ));
        Failure::Conflict(message)
    }

    /// Hot-apply the rate, write the records, then persist; a failure at any
    /// step restores the previous rate and leaves the overlay unchanged.
    fn transition(
        &self,
        settings: &Settings,
        overlay: Option<&mut Overlay>,
        next: &Effective,
        records: Vec<Record>,
        clock: Clock,
    ) -> Result<(), Step> {
        let mut bans = self.store();
        let previous = settings.effective();
        let changed = rate_triple(&previous) != rate_triple(next);
        if changed {
            configure_rate(&mut bans, next, clock.monotonic).map_err(|()| Step::Rate)?;
        }
        let restore = |bans: &mut Store| {
            if changed {
                let _ = configure_rate(bans, &previous, clock.monotonic);
            }
        };
        for record in records {
            if self.emit(record).is_err() {
                restore(&mut bans);
                return Err(Step::Audit);
            }
        }
        if let Err(error) = settings.store.set_active(overlay) {
            restore(&mut bans);
            return Err(Step::Store(error.to_string()));
        }
        settings.publish();
        Ok(())
    }

    fn rate_refused(&self, settings: &Settings, next: &Effective) -> Failure {
        let (window, soft, hard) = rate_triple(next);
        self.reject(
            settings,
            format!("settings: rate limiter refused effective rate limit {window}/{soft}/{hard}"),
        )
    }

    /// Go `expireActiveOverlayIfNeeded`.
    fn expire(&self, settings: &Settings, clock: Clock) -> Result<(), Failure> {
        if settings.kill() {
            return Ok(());
        }
        let Some(active) = settings.store.active() else {
            return Ok(());
        };
        let Some(expires) = active.expires_at else {
            return Ok(());
        };
        if now_nanos(clock) <= expires.unix_nanos() {
            return Ok(());
        }
        let next = settings.effective_for(None);
        let record = self.settings_record(
            "settings.overlay.expired",
            system(),
            "enforce",
            format!("overlayId={}", active.overlay_id),
        );
        match self.transition(settings, None, &next, vec![record], clock) {
            Ok(()) => Ok(()),
            Err(Step::Audit) => Err(Failure::Audit),
            Err(Step::Rate) => Err(Failure::Unavailable),
            Err(Step::Store(message)) => Err(self.store_failure(settings, "expire", message)),
        }
    }

    /// Go `settingsEffectiveFresh`.
    fn fresh(&self, settings: &Settings, clock: Clock) -> Result<Effective, Failure> {
        self.expire(settings, clock)?;
        Ok(settings.effective())
    }

    /// Go `applyOverlayCAS`; returns the new configuration version.
    fn apply_overlay(
        &self,
        settings: &Settings,
        mut overlay: Overlay,
        parent: &str,
        requester: &str,
        approver: &str,
        clock: Clock,
    ) -> Result<String, Failure> {
        let current = settings.effective();
        if parent.is_empty() || parent != current.config_version {
            return Err(self.reject(settings, STALE.into()));
        }
        if let Err(error) = validate(Some(&overlay)) {
            return Err(self.reject(settings, error.to_string()));
        }
        let class = classify(&current, Some(&overlay));
        overlay.mutation_class = class.as_str().into();
        if class == MutationClass::C && overlay.expires_at.is_none() {
            return Err(self.reject(settings, "class C requires expiresAt".into()));
        }
        if overlay
            .expires_at
            .is_some_and(|expires| expires.unix_nanos() <= now_nanos(clock))
        {
            return Err(self.reject(settings, "overlay already expired".into()));
        }
        overlay.status = "active".into();
        overlay.approved_by = approver.into();
        let next = settings.candidate(&overlay);
        if let Err(message) = Settings::validate_runtime(&next) {
            return Err(self.reject(settings, message));
        }
        let mut reason = format!(
            "settings.overlay applied class={} id={}",
            class.as_str(),
            overlay.overlay_id
        );
        dual(&mut reason, requester, approver);
        let records = vec![
            self.settings_record(
                "settings.overlay.applied",
                self.actor(requester),
                "enforce",
                reason,
            ),
            self.settings_record(
                "config.changed",
                self.actor(requester),
                "enforce",
                format!("config_version={}", next.config_version),
            ),
        ];
        match self.transition(settings, Some(&mut overlay), &next, records, clock) {
            Ok(()) => {}
            Err(Step::Rate) => return Err(self.rate_refused(settings, &next)),
            Err(Step::Audit) => return Err(Failure::Audit),
            Err(Step::Store(message)) => {
                return Err(self.store_failure(settings, "apply", message));
            }
        }
        bump(&settings.counters.applied);
        Ok(next.config_version)
    }

    /// Go `clearOverlayCAS`; returns the new configuration version.
    fn clear_overlay(
        &self,
        settings: &Settings,
        parent: &str,
        requester: &str,
        approver: &str,
        clock: Clock,
    ) -> Result<String, Failure> {
        let current = settings.effective();
        if parent.is_empty() || parent != current.config_version {
            return Err(self.reject(settings, STALE.into()));
        }
        let next = settings.effective_for(None);
        if let Err(message) = Settings::validate_runtime(&next) {
            return Err(self.reject(settings, message));
        }
        let mut reason = "settings.overlay rolled back to empty".to_owned();
        dual(&mut reason, requester, approver);
        let records = vec![
            self.settings_record(
                "settings.overlay.rolled_back",
                self.actor(requester),
                "enforce",
                reason,
            ),
            self.settings_record(
                "config.changed",
                self.actor(requester),
                "enforce",
                format!("config_version={}", next.config_version),
            ),
        ];
        match self.transition(settings, None, &next, records, clock) {
            Ok(()) => {}
            Err(Step::Rate) => return Err(self.rate_refused(settings, &next)),
            Err(Step::Audit) => return Err(Failure::Audit),
            Err(Step::Store(message)) => {
                return Err(self.store_failure(settings, "rollback", message));
            }
        }
        bump(&settings.counters.rolled_back);
        Ok(next.config_version)
    }

    /// Create a pending settings action and record it; the action is
    /// withdrawn when its record cannot be written.
    fn settings_pending(
        &self,
        kind: &str,
        params: &BTreeMap<String, String>,
        operator: &Operator,
        reason: impl FnOnce(&str) -> String,
        clock: Clock,
    ) -> Result<PendingAction, Response> {
        let action = self
            .approvals
            .create(kind, params, &operator.id, Role::Approver, clock.monotonic)
            .map_err(|error| Response::text(503, &error.to_string()))?;
        let record = self.settings_record(
            "approval.requested",
            self.actor(&operator.id),
            "enforce",
            reason(&action.id),
        );
        if self.emit(record).is_err() {
            self.approvals.withdraw(&action.id);
            return Err(audit_unavailable());
        }
        Ok(action)
    }

    pub(super) fn settings_effective(&self, clock: Clock) -> Response {
        let Some(settings) = &self.settings else {
            return Response::not_found();
        };
        let _serial = settings.lock();
        let effective = match self.fresh(settings, clock) {
            Ok(effective) => effective,
            Err(failure) => return failure.response(),
        };
        let Some(effective) = effective_json(&effective) else {
            return encoding_failed();
        };
        let mut fields = vec![
            ("effective", effective),
            ("storeAttached", "true".to_owned()),
        ];
        if let Some(warning) = settings.store.load_error() {
            fields.push(("storeWarning", quote_json_go(&warning)));
        }
        raw_object(fields)
    }

    pub(super) fn settings_schema(&self) -> Response {
        if self.settings.is_none() {
            return Response::not_found();
        }
        Response {
            status: 200,
            content_type: JSON,
            body: go_escape(schema_json()),
        }
    }

    pub(super) fn settings_overlays(&self, clock: Clock) -> Response {
        let Some(settings) = &self.settings else {
            return Response::not_found();
        };
        let _serial = settings.lock();
        let effective = match self.fresh(settings, clock) {
            Ok(effective) => effective,
            Err(failure) => return failure.response(),
        };
        let active = match settings.store.active() {
            None => Some("null".to_owned()),
            Some(overlay) => overlay
                .to_json()
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok()),
        };
        let (Some(active), Some(effective)) = (active, effective_json(&effective)) else {
            return encoding_failed();
        };
        raw_object(vec![("active", active), ("effective", effective)])
    }

    pub(super) fn settings_propose(
        &self,
        body: &[u8],
        operator: &Operator,
        clock: Clock,
    ) -> Response {
        let Some(settings) = &self.settings else {
            return Response::not_found();
        };
        let _serial = settings.lock();
        let bad = || Response::text(400, "bad request");
        if body.len() > PROPOSE_BODY {
            return bad();
        }
        let Ok(request) = ProposeBody::from_json(body) else {
            return bad();
        };
        let effective = match self.fresh(settings, clock) {
            Ok(effective) => effective,
            Err(failure) => return failure.response(),
        };
        let parent = request.parent_config_version.clone();
        if parent.is_empty() || parent != effective.config_version {
            return Response::text(409, STALE);
        }
        let mut overlay =
            match self.proposed_overlay(settings, &request, operator, &effective, clock) {
                Ok(overlay) => overlay,
                Err(response) => return response,
            };
        if let Err(message) = Settings::validate_runtime(&settings.candidate(&overlay)) {
            return Response::text(400, &message);
        }
        let class = classify(&effective, Some(&overlay));
        overlay.mutation_class = class.as_str().into();
        if class == MutationClass::C && request.integrity_confirm != CONFIRM_PHRASE {
            return Response::text(400, "class C requires integrityConfirm=DISABLE-INTEGRITY");
        }
        overlay.expires_at = match expiry_for_class(class, request.expires_in_sec) {
            Err(message) => return Response::text(400, message),
            Ok(None) => None,
            Ok(Some(seconds)) => match i64::try_from(clock.unix)
                .ok()
                .and_then(|now| now.checked_add(seconds))
                .and_then(Timestamp::from_unix)
            {
                Some(expires) => Some(expires),
                None => return Response::text(400, "expiresInSec out of range"),
            },
        };
        if settings.kill() && class != MutationClass::A {
            return Response::text(409, "kill switch engaged: loosening overlays blocked");
        }
        let id = overlay.overlay_id.clone();
        if self
            .emit(self.settings_record(
                "settings.overlay.proposed",
                self.actor(&operator.id),
                "enforce",
                format!("class={} overlayId={id}", class.as_str()),
            ))
            .is_err()
        {
            return audit_unavailable();
        }
        if class == MutationClass::A {
            return match self.apply_overlay(settings, overlay, &parent, &operator.id, "", clock) {
                Ok(version) => Response::json(&json!({
                    "ok": true,
                    "applied": true,
                    "class": class.as_str(),
                    "overlayId": id,
                    "configVersion": version,
                })),
                Err(failure) => failure.response(),
            };
        }
        // Go ignores the marshal error; an unencodable overlay then commits
        // nothing ("missing overlay body").
        let raw = overlay
            .to_json()
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .unwrap_or_default();
        let params = BTreeMap::from([
            ("overlayId".to_owned(), id.clone()),
            ("parentConfigVersion".to_owned(), parent),
            ("mutationClass".to_owned(), class.as_str().to_owned()),
            ("body".to_owned(), raw),
            ("integrityConfirm".to_owned(), request.integrity_confirm),
        ]);
        let action = match self.settings_pending(
            KIND_OVERLAY,
            &params,
            operator,
            |pending| {
                format!(
                    "settings.overlay pending class={} id={pending}",
                    class.as_str()
                )
            },
            clock,
        ) {
            Ok(action) => action,
            Err(response) => return response,
        };
        Response::json(&json!({
            "ok": true,
            "pending": true,
            "approvalId": action.id,
            "class": class.as_str(),
            "overlayId": id,
            "needsRole": Role::Approver.as_str(),
        }))
    }

    /// Go `buildOverlayFromPropose` then `settings.Validate`.
    fn proposed_overlay(
        &self,
        settings: &Settings,
        request: &ProposeBody,
        operator: &Operator,
        effective: &Effective,
        clock: Clock,
    ) -> Result<Overlay, Response> {
        let created = i64::try_from(clock.unix)
            .ok()
            .and_then(Timestamp::from_unix)
            .ok_or_else(|| Response::text(503, "clock unavailable"))?;
        let id = settings
            .overlay_id()
            .ok_or_else(|| Response::text(400, "overlay id entropy unavailable"))?;
        let overlay = build_overlay(
            request,
            id,
            &operator.id,
            &effective.config_version,
            created,
        );
        validate(Some(&overlay)).map_err(|error| Response::text(400, &error.to_string()))?;
        Ok(overlay)
    }

    pub(super) fn settings_dry_run(
        &self,
        body: &[u8],
        operator: &Operator,
        clock: Clock,
    ) -> Response {
        let Some(settings) = &self.settings else {
            return Response::not_found();
        };
        let _serial = settings.lock();
        if body.len() > PROPOSE_BODY {
            return Response::text(400, "bad request");
        }
        let Ok(request) = ProposeBody::from_json(body) else {
            return Response::text(400, "bad request");
        };
        let effective = match self.fresh(settings, clock) {
            Ok(effective) => effective,
            Err(failure) => return failure.response(),
        };
        let overlay = match self.proposed_overlay(settings, &request, operator, &effective, clock) {
            Ok(overlay) => overlay,
            Err(response) => return response,
        };
        let class = classify(&effective, Some(&overlay));
        let records = match &self.chain {
            None => Vec::new(),
            Some(chain) => match chain.log().records() {
                Ok(records) => records,
                Err(_) => return audit_unavailable(),
            },
        };
        let (mut challenge, mut deny) = (effective.challenge_at, effective.deny_at);
        if let Some(scoring) = &overlay.scoring {
            challenge = scoring.challenge_at.unwrap_or(challenge);
            deny = scoring.deny_at.unwrap_or(deny);
        }
        let (mut n, mut allow, mut challenged, mut denied, mut demotions) = (0, 0, 0, 0, 0);
        for record in records.iter().rev() {
            if n >= DRY_RUN_SCAN {
                break;
            }
            if !DRY_RUN_EVENTS.contains(&record.event_type.as_str()) {
                continue;
            }
            n += 1;
            // Go converts the integer score to float64 exactly.
            let risk = record.risk_score as f64;
            let mut band = if risk >= deny {
                2
            } else if risk >= challenge {
                1
            } else {
                0
            };
            let mut demoted = false;
            for rule in record.triggered_rules.iter().flatten() {
                if overlay.hard_rules.get(rule) == Some(&Mode::Monitor) {
                    demoted = true;
                    demotions += 1;
                }
            }
            if demoted {
                band = 0;
            }
            match band {
                2 => denied += 1,
                1 => challenged += 1,
                _ => allow += 1,
            }
        }
        let mut out = json!({
            "class": class.as_str(),
            "n": n,
            "minN": DRY_RUN_MIN_N,
            "usable": n >= DRY_RUN_MIN_N,
            "confidence": "approximate",
            "note": "aggregate-only; not a freeze claim; empty store \u{2192} usable=false",
        });
        if n >= DRY_RUN_MIN_N
            && let Value::Object(fields) = &mut out
        {
            fields.insert("allowDelta".into(), json!(allow));
            fields.insert("challengeDelta".into(), json!(challenged));
            fields.insert("denyDelta".into(), json!(denied));
            fields.insert("hardRuleDemoteHits".into(), json!(demotions));
        }
        if self
            .emit(self.settings_record(
                "settings.dry_run.completed",
                self.actor(&operator.id),
                "monitor",
                format!("n={n} class={}", class.as_str()),
            ))
            .is_err()
        {
            return audit_unavailable();
        }
        Response::json(&out)
    }

    pub(super) fn settings_rollback(
        &self,
        body: &[u8],
        operator: &Operator,
        clock: Clock,
    ) -> Response {
        let Some(settings) = &self.settings else {
            return Response::not_found();
        };
        let _serial = settings.lock();
        if body.len() > ROLLBACK_BODY {
            return Response::text(400, "bad request");
        }
        let Ok(parent) = decode_rollback_parent(body) else {
            return Response::text(400, "bad request");
        };
        let effective = match self.fresh(settings, clock) {
            Ok(effective) => effective,
            Err(failure) => return failure.response(),
        };
        if parent.is_empty() || parent != effective.config_version {
            return Response::text(409, STALE);
        }
        if effective.empty_overlay {
            return Response::json(&json!({"ok": true, "noop": true, "note": "already empty"}));
        }
        // Undoing a tightening (stored class A) loosens and needs an approver.
        let dual_control = settings
            .store
            .active()
            .is_some_and(|active| active.mutation_class == MutationClass::A.as_str());
        if dual_control {
            let params = BTreeMap::from([
                ("parentConfigVersion".to_owned(), parent),
                ("action".to_owned(), "clear".to_owned()),
            ]);
            return match self.settings_pending(
                KIND_ROLLBACK,
                &params,
                operator,
                |pending| format!("settings rollback pending id={pending}"),
                clock,
            ) {
                Ok(action) => Response::json(&json!({
                    "ok": true,
                    "pending": true,
                    "approvalId": action.id,
                    "needsRole": Role::Approver.as_str(),
                })),
                Err(response) => response,
            };
        }
        match self.clear_overlay(settings, &parent, &operator.id, "", clock) {
            Ok(version) => Response::json(&json!({
                "ok": true,
                "rolledBack": true,
                "configVersion": version,
            })),
            Err(failure) => failure.response(),
        }
    }

    /// Go `commitSettingsOverlay` after a successful approval.
    pub(super) fn commit_settings(
        &self,
        action: &PendingAction,
        approver: &Operator,
        clock: Clock,
    ) -> Response {
        let Some(settings) = &self.settings else {
            return Response::text(500, "unknown action kind");
        };
        let _serial = settings.lock();
        let param = |name: &str| action.params.get(name).map_or("", String::as_str);
        if action.requester == approver.id {
            return Response::text(409, "approver must differ from requester");
        }
        let raw = param("body");
        if raw.is_empty() {
            return Response::text(409, "missing overlay body");
        }
        let overlay = match Overlay::from_json(raw.as_bytes()) {
            Ok(overlay) => overlay,
            Err(error) => return Response::text(409, &error.to_string()),
        };
        if param("mutationClass") == MutationClass::C.as_str()
            && param("integrityConfirm") != CONFIRM_PHRASE
        {
            return Response::text(409, "class C requires integrityConfirm");
        }
        match self.apply_overlay(
            settings,
            overlay,
            param("parentConfigVersion"),
            &action.requester,
            &approver.id,
            clock,
        ) {
            Ok(version) => Response::json(&json!({
                "ok": true,
                "committed": KIND_OVERLAY,
                "configVersion": version,
                "overlayId": param("overlayId"),
            })),
            Err(failure) => failure.response(),
        }
    }

    /// Go `clearOverlayCAS` for an approved rollback.
    pub(super) fn commit_settings_rollback(
        &self,
        action: &PendingAction,
        approver: &Operator,
        clock: Clock,
    ) -> Response {
        let Some(settings) = &self.settings else {
            return Response::text(500, "unknown action kind");
        };
        let _serial = settings.lock();
        let parent = action
            .params
            .get("parentConfigVersion")
            .map_or("", String::as_str);
        match self.clear_overlay(settings, parent, &action.requester, &approver.id, clock) {
            Ok(version) => Response::json(&json!({
                "ok": true,
                "committed": KIND_ROLLBACK,
                "configVersion": version,
            })),
            Err(failure) => failure.response(),
        }
    }
}

#[cfg(test)]
mod tests;
