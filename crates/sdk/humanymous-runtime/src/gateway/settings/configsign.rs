//! Signed configuration version of the running policy (legacy
//! internal/gate/configsign.go at 981d25c; SoT-28 WS8, SoT-39 §4.4).
//!
//! Legacy `Server.configVersion` returned `SettingsEffective().ConfigVersion`:
//! the boot posture (global monitor, the live kill switch, route presets and
//! the rate window/soft/hard after the `rlWindow`/`rlSoft`/`rlHard` defaults)
//! resolved with the active RuntimeOverlay and keyed by the verdict-token HMAC
//! key. A free-form label would let an unapproved policy run under an approved
//! name; the keyed hash changes whenever the effective policy changes. The
//! legacy overlay-free fallback was reachable only for a nil server, which then
//! dereferenced nil, so it is not ported.
//!
//! [`BootPolicy::config_version`] is a pure kernel over the frozen overlay
//! resolver. Owner decision #8 (issue #75 comment 7017558784) binds it in
//! place of `scope.revision`: [`SignedPolicy`] is the shared live handle
//! (boot slice, kill-switch flag, active overlay slot) whose
//! [`SignedPolicy::version`] the served path stamps as `config_version` once
//! installed with `Application::install_signed_policy`, as Go
//! `Server.configVersion()` stamped every gate decision record. The admin
//! `policy` view reports the same value. The Prometheus label stays owner
//! pending with the rest of the metrics.
use super::overlay::{BootInput, Overlay, resolve};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError, RwLock};

/// Legacy `DefaultRateWindow` (10 s) in nanoseconds.
pub const DEFAULT_RATE_WINDOW_NANOS: i64 = 10_000_000_000;
/// Legacy `DefaultRateSoft`.
pub const DEFAULT_RATE_SOFT: i64 = 60;
/// Legacy `DefaultRateHard`.
pub const DEFAULT_RATE_HARD: i64 = 120;

/// Boot configuration slice the legacy server hashed. Rate fields keep the
/// legacy raw form: non-positive values select the defaults.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BootPolicy {
    /// Boot global monitor posture.
    pub global_monitor: bool,
    /// Route prefix to preset name.
    pub routes: BTreeMap<String, String>,
    /// Rate window as a Go `time.Duration` (nanoseconds).
    pub rate_window_nanos: i64,
    /// Soft rate threshold.
    pub rate_soft: i64,
    /// Hard rate threshold.
    pub rate_hard: i64,
    /// Verdict-token HMAC key; empty keys hash like the legacy nil key.
    pub hmac_key: Vec<u8>,
}

impl BootPolicy {
    /// Legacy `int(rlWindow(cfg).Seconds())`: the default for a non-positive
    /// window, then Go's float seconds truncated toward zero.
    pub fn rate_window_sec(&self) -> i64 {
        let nanos = if self.rate_window_nanos > 0 {
            self.rate_window_nanos
        } else {
            DEFAULT_RATE_WINDOW_NANOS
        };
        let seconds = (nanos / 1_000_000_000) as f64 + (nanos % 1_000_000_000) as f64 / 1e9;
        seconds as i64
    }

    /// Legacy `rlSoft`.
    pub fn rate_soft(&self) -> i64 {
        if self.rate_soft > 0 {
            self.rate_soft
        } else {
            DEFAULT_RATE_SOFT
        }
    }

    /// Legacy `rlHard`.
    pub fn rate_hard(&self) -> i64 {
        if self.rate_hard > 0 {
            self.rate_hard
        } else {
            DEFAULT_RATE_HARD
        }
    }

    /// Legacy `settingsEffectiveFor` boot input with the live kill switch.
    pub fn boot_input(&self, kill_switch: bool) -> BootInput {
        BootInput {
            global_monitor: self.global_monitor,
            kill_switch,
            routes: self.routes.clone(),
            rate_window_sec: self.rate_window_sec(),
            rate_soft: self.rate_soft(),
            rate_hard: self.rate_hard(),
            hmac_key: self.hmac_key.clone(),
        }
    }

    /// Legacy `configVersion`: the `cfg-…` version of the effective policy
    /// under `kill_switch` and the store's active overlay, if any.
    pub fn config_version(&self, kill_switch: bool, active: Option<&Overlay>) -> String {
        resolve(&self.boot_input(kill_switch), active).config_version
    }
}

/// Live inputs of the signed effective-policy version, shared with the admin
/// plane: the boot slice, the kill-switch flag and the active overlay slot.
/// Cloning shares the same flag and slot.
#[derive(Clone, Debug)]
pub struct SignedPolicy {
    boot: Arc<BootPolicy>,
    kill_switch: Arc<AtomicBool>,
    active: Arc<RwLock<Option<Overlay>>>,
}

impl SignedPolicy {
    /// Bind `boot` to the live `kill_switch` flag and `active` overlay slot.
    pub fn new(
        boot: BootPolicy,
        kill_switch: Arc<AtomicBool>,
        active: Arc<RwLock<Option<Overlay>>>,
    ) -> Self {
        Self {
            boot: Arc::new(boot),
            kill_switch,
            active,
        }
    }

    /// Legacy `Server.configVersion()`: the `cfg-…` version of the policy in
    /// force now.
    pub fn version(&self) -> String {
        let kill = self.kill_switch.load(Ordering::SeqCst);
        let active = self.active.read().unwrap_or_else(PoisonError::into_inner);
        self.boot.config_version(kill, active.as_ref())
    }
}

#[cfg(test)]
mod tests;
