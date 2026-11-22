//! Read-only admin `policy` route and the operator console page of legacy
//! internal/gate/admin.go (`adminPolicy`, `adminConsole`) and console.go at
//! baseline 981d25c.
//!
//! [`AdminPlane::with_policy`] attaches a [`Policy`]: the boot policy slice
//! the legacy server hashed, the live kill-switch flag and the active
//! settings overlay slot, all shared and only read here. Without it `GET
//! policy` answers the plain 404 after authentication, as before.
//!
//! Kept from Go:
//! - `policy` reports `globalMonitor` (boot posture), `killSwitch` (live),
//!   `effectiveMonitor` (either of the two), `configVersion` (the keyed
//!   `cfg-…` version of the effective policy with the active overlay,
//!   `settings::configsign::BootPolicy::config_version`), `routes` (a
//!   `(default)` balanced row, then one row per configured prefix with the
//!   preset name after the unknown-name fallback and `enforce` cleared under
//!   the boot global monitor), `rateLimit` (window seconds, soft and hard
//!   after the legacy defaults), `retentionDays` (hot, warm, cold),
//!   `settingsWrite: false` and `settingsReadPath`. Map keys are sorted and
//!   the route row fields keep Go's struct order.
//! - There is no write verb: `POST policy` matches no route and answers 404.
//! - The console page is `gateway::console::page()`, the Go
//!   `styledConsoleHTML` byte for byte, as `text/html; charset=utf-8` with
//!   `Cache-Control: no-store` and `X-Frame-Options: DENY`, at the bare
//!   prefix and at `console`.
//!
//! Deliberate fail-closed differences, each pinned by a test:
//! - Go served the console page to any method without authentication (a
//!   top-level navigation cannot present a bearer header) and, when a dev
//!   console token was set, injected that bearer token into the page. Here
//!   the page is an authenticated `GET` read like every other admin route, so
//!   an unauthenticated request keeps the deny-by-default 404 and no
//!   credential is ever written into a response. A browser reaches it through
//!   a front end that presents the operator's bearer header.
//! - Go listed configured routes in map iteration order, which varies between
//!   calls; here they follow prefix byte order after the `(default)` row, one
//!   of the orders Go could produce.
//! - This route only reads: it never changes the overlay, the kill switch,
//!   any route preset or any scoring value (detection freeze).
use super::{AdminPlane, Response};
use crate::audit::lifecycle::RetentionPolicy;
use crate::gateway::console;
use crate::gateway::route_policy::{PRESET_BALANCED, RoutePolicy, preset_by_name};
use crate::gateway::settings::configsign::BootPolicy;
use crate::gateway::settings::overlay::Overlay;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError, RwLock};

/// `Content-Type` of the console page (Go `adminConsole`).
pub const HTML: &str = "text/html; charset=utf-8";
/// `settingsReadPath` of the policy view (SoT-39: Settings is the write plane).
pub const SETTINGS_READ_PATH: &str = "GET /settings/effective";

/// The shared, read-only inputs of the policy view.
pub struct Policy {
    boot: BootPolicy,
    kill_switch: Arc<AtomicBool>,
    active: Arc<RwLock<Option<Overlay>>>,
}

impl Policy {
    /// `(kill switch, effective monitor)` now: the live flag, and the boot
    /// global monitor or the kill switch (Go `monitorOn`).
    pub(super) fn switches(&self) -> (bool, bool) {
        let kill = self.kill_switch.load(Ordering::SeqCst);
        (kill, self.boot.global_monitor || kill)
    }

    /// The signed effective-policy version now (Go `ConfigVersion`).
    pub(super) fn version(&self) -> String {
        let kill = self.kill_switch.load(Ordering::SeqCst);
        let active = self.active.read().unwrap_or_else(PoisonError::into_inner);
        self.boot.config_version(kill, active.as_ref())
    }

    /// `boot` is the boot configuration slice (global monitor, route presets,
    /// rate thresholds and the verdict-token key the version is keyed with),
    /// `kill_switch` the live kill-switch flag and `active` the active
    /// settings overlay, if any. The view only reads them.
    pub fn new(
        boot: BootPolicy,
        kill_switch: Arc<AtomicBool>,
        active: Arc<RwLock<Option<Overlay>>>,
    ) -> Self {
        Self {
            boot,
            kill_switch,
            active,
        }
    }
}

/// One route row (Go `routeRow`, field order kept).
#[derive(Serialize)]
struct RouteRow {
    prefix: String,
    preset: &'static str,
    enforce: bool,
    #[serde(rename = "failClosed")]
    fail_closed: bool,
    #[serde(rename = "syncScore")]
    sync_score: bool,
    inject: bool,
}

impl RouteRow {
    fn new(prefix: &str, preset: RoutePolicy, global_monitor: bool) -> Self {
        Self {
            prefix: prefix.to_owned(),
            preset: preset.name,
            enforce: preset.enforce && !global_monitor,
            fail_closed: preset.fail_closed,
            sync_score: preset.sync_score,
            inject: preset.inject,
        }
    }
}

/// Go `rateLimit` map (sorted keys).
#[derive(Serialize)]
struct RateLimit {
    hard: i64,
    soft: i64,
    #[serde(rename = "windowSec")]
    window_sec: i64,
}

/// Go `RetentionPolicy.Days` map (sorted keys).
#[derive(Serialize)]
struct RetentionDays {
    cold: i128,
    hot: i128,
    warm: i128,
}

/// Go `adminPolicy` map, fields in sorted key order.
#[derive(Serialize)]
struct PolicyView {
    #[serde(rename = "configVersion")]
    config_version: String,
    #[serde(rename = "effectiveMonitor")]
    effective_monitor: bool,
    #[serde(rename = "globalMonitor")]
    global_monitor: bool,
    #[serde(rename = "killSwitch")]
    kill_switch: bool,
    #[serde(rename = "rateLimit")]
    rate_limit: RateLimit,
    #[serde(rename = "retentionDays")]
    retention_days: RetentionDays,
    routes: Vec<RouteRow>,
    #[serde(rename = "settingsReadPath")]
    settings_read_path: &'static str,
    #[serde(rename = "settingsWrite")]
    settings_write: bool,
}

impl AdminPlane {
    /// Serve the read-only `policy` route from `policy`.
    pub fn with_policy(mut self, policy: Policy) -> Self {
        self.policy = Some(policy);
        self
    }

    pub(super) fn policy_view(&self) -> Response {
        let Some(policy) = &self.policy else {
            return Response::not_found();
        };
        let boot = &policy.boot;
        let kill = policy.kill_switch.load(Ordering::SeqCst);
        let config_version = {
            let active = policy.active.read().unwrap_or_else(PoisonError::into_inner);
            boot.config_version(kill, active.as_ref())
        };
        let global = boot.global_monitor;
        let mut routes = vec![RouteRow::new("(default)", PRESET_BALANCED, global)];
        routes.extend(
            boot.routes
                .iter()
                .map(|(prefix, name)| RouteRow::new(prefix, preset_by_name(name), global)),
        );
        let [hot, warm, cold] = RetentionPolicy::default().days();
        Response::json(&PolicyView {
            config_version,
            effective_monitor: global || kill,
            global_monitor: global,
            kill_switch: kill,
            rate_limit: RateLimit {
                hard: boot.rate_hard(),
                soft: boot.rate_soft(),
                window_sec: boot.rate_window_sec(),
            },
            retention_days: RetentionDays { cold, hot, warm },
            routes,
            settings_read_path: SETTINGS_READ_PATH,
            settings_write: false,
        })
    }

    pub(super) fn console_page() -> Response {
        Response {
            status: 200,
            content_type: HTML,
            body: console::page().as_bytes().to_vec(),
        }
    }
}

#[cfg(test)]
mod tests;
