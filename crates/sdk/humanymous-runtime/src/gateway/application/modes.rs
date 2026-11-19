//! Per-gate module modes on the served path (legacy internal/gate gate.go
//! `gateModuleMode` over `SettingsEffective` at baseline 981d25c; owner
//! decision #3, issue #75 comment 7017558784).
//!
//! Go read every gate's mode from the effective settings overlay on each
//! request: `enforce` without a settings store, and otherwise the active
//! overlay's `gates` entry over the built-in default `enforce` for every known
//! gate (`settings.Resolve`). The runtime does the same: the admin listener's
//! settings write plane mirrors its active overlay into one shared slot
//! ([`ActiveOverlay`]) and the served path reads that slot per request, so a
//! committed, rolled back or expired overlay takes effect on the next request.
//!
//! Kept from Go, pinned against `tests/fixtures/gateway/gate-modes-go-golden.json`:
//! - Every known gate defaults to `enforce`, with or without a settings store
//!   and with an empty store; an unknown gate id is `enforce`.
//! - An overlay entry applies only while the overlay is active (status
//!   `active` or empty), exactly as `settings.Resolve` merges it.
//! - Modes are not weights or thresholds: nothing here touches scoring.
//!
//! Deliberate differences, each pinned by tests:
//! - The boot mode of `gate.ban` and `gate.spoof_header` is the startup
//!   `edge` section (default `enforce`, Go's default), and the boot mode of
//!   `gate.recon_sweep` is the one the detector was installed with; an active
//!   overlay entry overrides it. Go had no startup mode, so with a non-default
//!   startup mode an active overlay that leaves the gate unnamed keeps the
//!   startup mode where Go would have used `enforce`.
//! - A mode spelling outside the four known ones (refused by overlay
//!   validation, so unreachable from the write plane) reads as `enforce`.
use super::Application;
use crate::gateway::ban::gate::Mode;
use crate::gateway::settings::overlay::{Mode as OverlayMode, Overlay};
use std::sync::{Arc, PoisonError, RwLock};

/// The active overlay as the settings write plane last published it.
pub type ActiveOverlay = Arc<RwLock<Option<Overlay>>>;

/// Settings ids of the gates whose mode the served path reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// `gate.ban`.
    Ban,
    /// `gate.spoof_header`.
    SpoofHeader,
    /// `gate.verdict_token`.
    VerdictToken,
    /// `gate.recon_sweep`.
    ReconSweep,
    /// `gate.inject`.
    Inject,
    /// `gate.attest_floor`.
    AttestFloor,
}

impl Gate {
    /// Settings id (Go `KnownGates` spelling).
    pub fn id(self) -> &'static str {
        match self {
            Self::Ban => "gate.ban",
            Self::SpoofHeader => "gate.spoof_header",
            Self::VerdictToken => "gate.verdict_token",
            Self::ReconSweep => "gate.recon_sweep",
            Self::Inject => "gate.inject",
            Self::AttestFloor => "gate.attest_floor",
        }
    }
}

/// The overlay's mode for `id`, when an active overlay names it (the
/// `settings.Resolve` merge of `active.Gates` over the defaults).
pub fn overlay_mode(active: Option<&Overlay>, id: &str) -> Option<Mode> {
    let overlay = active.filter(|overlay| matches!(overlay.status.as_str(), "active" | ""))?;
    Some(match overlay.gates.get(id)? {
        OverlayMode::Monitor => Mode::Monitor,
        OverlayMode::Shadow => Mode::Shadow,
        OverlayMode::Off => Mode::Off,
        OverlayMode::Enforce | OverlayMode::Other(_) => Mode::Enforce,
    })
}

/// Where the served path reads gate modes from.
#[derive(Default)]
pub(super) struct Modes {
    overlay: Option<ActiveOverlay>,
}

impl Application {
    /// Read gate modes from the settings write plane's active overlay (Go
    /// `SetSettingsStore`). Without it every gate keeps its boot mode.
    pub fn install_gate_modes(&mut self, overlay: ActiveOverlay) {
        self.modes.overlay = Some(overlay);
    }

    /// Effective mode of `gate` for this request (Go `gateModuleMode`).
    pub(in crate::gateway) fn gate_mode(&self, gate: Gate) -> Mode {
        let boot = match gate {
            Gate::Ban => self.config.edge.ban,
            Gate::SpoofHeader => self.config.edge.spoof_header,
            Gate::ReconSweep => self.installed.sweep_mode().unwrap_or(Mode::Enforce),
            Gate::VerdictToken | Gate::Inject | Gate::AttestFloor => Mode::Enforce,
        };
        let Some(slot) = &self.modes.overlay else {
            return boot;
        };
        // The slot holds plain data written whole; a poisoned lock still
        // holds the last published overlay.
        let active = slot.read().unwrap_or_else(PoisonError::into_inner);
        overlay_mode(active.as_ref(), gate.id()).unwrap_or(boot)
    }
}
