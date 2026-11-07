//! Consumer-side contract for a future windowed CEF adapter, not CEF execution.
//!
//! No public transition can assert native attachment or sandbox qualification.
//! A concrete reviewed native adapter must supply that implementation later.

/// Current target-surface state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// No native child exists.
    Detached,
    /// Native creation has been requested; no ready receipt exists.
    Attaching,
    /// Cleanup requested; stale input is rejected.
    Closing,
    /// Terminal consumer state. Does not attest OS child destruction.
    Closed,
}

/// Identity invalidated on cancellation or close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Epoch(u64);

/// Bounded physical child size plus scale factor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    /// Physical pixel width.
    pub width: u32,
    /// Physical pixel height.
    pub height: u32,
    /// Native display scale factor.
    pub scale: f64,
}

impl Geometry {
    /// Validate before making an OS/CEF resize call.
    pub fn validate(self) -> Result<Self, &'static str> {
        if !(1..=16384).contains(&self.width)
            || !(1..=16384).contains(&self.height)
            || !self.scale.is_finite()
            || !(0.25..=8.0).contains(&self.scale)
        {
            return Err("invalid native child geometry");
        }
        Ok(self)
    }
}

/// Future adapter obligations. These are requirements, not claimed capabilities.
pub const NATIVE_REQUIREMENTS: &[&str] = &[
    "verified sandbox bootstrap and engine artifacts",
    "scoped parent lease and UI-thread creation",
    "windowed child resize and DPI propagation",
    "keyboard focus transfer and focus restoration",
    "native IME composition and candidate placement",
    "OS accessibility subtree and actions",
    "before-close receipt, crash cleanup and cancellation",
];

/// Generation guard used before forwarding any native operation.
#[derive(Debug)]
pub struct SurfaceLifecycle {
    epoch: u64,
    phase: Phase,
    cleanup_pending: bool,
}

impl Default for SurfaceLifecycle {
    fn default() -> Self {
        Self {
            epoch: 0,
            phase: Phase::Detached,
            cleanup_pending: false,
        }
    }
}

impl SurfaceLifecycle {
    /// Target state; no state in this slice means browser ready.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Start a creation transaction. Actual CEF creation is not implemented here.
    pub fn begin_attach(&mut self, geometry: Geometry) -> Result<Epoch, &'static str> {
        geometry.validate()?;
        if self.phase != Phase::Detached {
            return Err("surface is not detached");
        }
        self.epoch = self.epoch.checked_add(1).ok_or("generation exhausted")?;
        self.phase = Phase::Attaching;
        self.cleanup_pending = true;
        Ok(Epoch(self.epoch))
    }

    /// Reject stale or non-live callbacks before native handle access.
    pub fn check_pending(&self, epoch: Epoch) -> Result<(), &'static str> {
        if epoch.0 != self.epoch || self.phase != Phase::Attaching {
            return Err("stale or inactive child transaction");
        }
        Ok(())
    }

    /// Cancel creation and revoke callback identity before native cleanup.
    pub fn cancel(&mut self, epoch: Epoch) -> Result<(), &'static str> {
        self.check_pending(epoch)?;
        self.epoch = self.epoch.checked_add(1).ok_or("generation exhausted")?;
        self.phase = Phase::Closing;
        Ok(())
    }

    /// Consumer is closed; native cleanup remains explicitly outstanding.
    pub fn close(&mut self) -> Result<(), &'static str> {
        if self.phase == Phase::Closed {
            return Ok(());
        }
        self.epoch = self.epoch.checked_add(1).ok_or("generation exhausted")?;
        self.phase = Phase::Closed;
        Ok(())
    }

    /// Whether native cleanup still lacks a real receipt.
    pub fn cleanup_pending(&self) -> bool {
        self.cleanup_pending
    }

    /// Always false until a concrete native adapter has been qualified.
    pub fn browser_ready(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn geometry() -> Geometry {
        Geometry {
            width: 1000,
            height: 600,
            scale: 1.5,
        }
    }

    #[test]
    fn cancellation_revokes_callbacks_without_inventing_cleanup_receipt() {
        let mut surface = SurfaceLifecycle::default();
        let epoch = surface.begin_attach(geometry()).unwrap();
        assert!(surface.check_pending(epoch).is_ok());
        assert!(!surface.browser_ready());
        assert!(surface.begin_attach(geometry()).is_err());
        surface.cancel(epoch).unwrap();
        assert!(surface.check_pending(epoch).is_err());
        assert!(surface.cancel(epoch).is_err());
        assert_eq!(surface.phase(), Phase::Closing);
        assert!(surface.cleanup_pending());
        surface.close().unwrap();
        surface.close().unwrap();
        assert_eq!(surface.phase(), Phase::Closed);
        assert!(surface.cleanup_pending());
        assert!(!surface.browser_ready());
    }

    #[test]
    fn resize_dpi_rejects_non_finite_empty_or_unbounded_geometry() {
        for scale in [f64::NAN, f64::INFINITY, 0.0, 8.1] {
            assert!(
                Geometry {
                    scale,
                    ..geometry()
                }
                .validate()
                .is_err()
            );
        }
        for width in [0, 16385, u32::MAX] {
            assert!(
                Geometry {
                    width,
                    ..geometry()
                }
                .validate()
                .is_err()
            );
        }
        assert!(geometry().validate().is_ok());
    }

    #[test]
    fn close_without_creation_has_no_native_cleanup_or_reopen() {
        let mut surface = SurfaceLifecycle::default();
        surface.close().unwrap();
        assert!(!surface.cleanup_pending());
        assert!(surface.begin_attach(geometry()).is_err());
    }
}
