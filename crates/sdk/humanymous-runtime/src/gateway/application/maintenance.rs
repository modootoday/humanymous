//! Composite in-memory sweep of the serving owner (legacy internal/gate
//! gate.go `Server.GC` and the cmd/gate one-minute maintenance ticker at
//! baseline 981d25c).
//!
//! Go swept the verdict store, the ban store (bans, strikes and rate
//! windows), the sweep detector, the shadow anomaly state and the credential
//! fan-out tracker once a minute, so fingerprint- and address-keyed maps could
//! not grow without bound under bot churn. Here the one serialized
//! [`Application`] owner sweeps every kernel it composes: the session epoch
//! (expired sessions) and the installed ban store, on the same one-minute
//! cadence, between requests in both accept loops. Kernels the served path
//! does not compose (verdict store, sweep detector, anomaly shadow, credential
//! fan-out) are not held by this owner and keep their own tested `gc`.
//!
//! Deliberate differences:
//! - A sweep never moves the session epoch's clock and never poisons it: a
//!   wall clock behind the last observed request sweeps nothing, and the next
//!   request still fails closed on the rollback as before.
//! - A ban store sweep the rate history refuses, or a poisoned store lock, is
//!   reported as `503 ban_ledger`; the accept loop keeps serving because the
//!   ban gate itself refuses on the same ledger (fail closed), as Go ignored
//!   GC outcomes.
use super::Application;
use crate::gateway::Rejection;
use std::time::Duration;

/// Legacy cmd/gate maintenance ticker period.
pub(super) const INTERVAL: Duration = Duration::from_secs(60);

/// Ticker for the accept loops; the first sweep is one period after start and
/// late ticks are delayed rather than bunched.
pub(super) fn ticker(period: Duration) -> tokio::time::Interval {
    let mut ticker = tokio::time::interval_at(tokio::time::Instant::now() + period, period);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    ticker
}

impl Application {
    /// Period of the accept loops' maintenance sweep.
    pub fn sweep_interval(&self) -> Duration {
        self.sweep_interval
    }

    /// Shorten the sweep period so a test can observe the tick (tests).
    #[cfg(test)]
    pub(in crate::gateway) fn set_sweep_interval(&mut self, period: Duration) {
        self.sweep_interval = period;
    }

    /// Legacy `Server.GC`: sweep every in-memory kernel this owner composes.
    /// `now_ms` is the wall clock in UTC milliseconds; the ban store is swept
    /// at its own monotonic epoch.
    pub fn gc(&mut self, now_ms: u64) -> Result<(), Rejection> {
        let ban_now = self.bans.as_ref().map(|bans| bans.epoch.elapsed());
        self.gc_at(now_ms, ban_now)
    }

    /// [`Application::gc`] with an explicit ban-store instant (tests).
    pub(in crate::gateway) fn gc_at(
        &mut self,
        now_ms: u64,
        ban_now: Option<Duration>,
    ) -> Result<(), Rejection> {
        self.sessions.sweep(now_ms);
        if let (Some(bans), Some(now)) = (self.bans.as_ref(), ban_now) {
            let ledger = Rejection::new(503, "ban_ledger");
            bans.store
                .lock()
                .map_err(|_| ledger)?
                .gc(now)
                .map_err(|_| ledger)?;
        }
        Ok(())
    }

    /// Sessions held by the serving epoch (tests).
    #[cfg(test)]
    pub(in crate::gateway) fn held_sessions(&self) -> usize {
        self.sessions.len()
    }

    /// One maintenance tick from an accept loop. An unavailable clock skips
    /// the session sweep only.
    pub(super) fn maintenance_tick(&mut self) {
        let result = match crate::authority::now_ms() {
            Ok(now) => self.gc(now),
            Err(_) => {
                let ban_now = self.bans.as_ref().map(|bans| bans.epoch.elapsed());
                self.gc_at(0, ban_now)
            }
        };
        // The ban gate refuses on the same ledger, so serving stays fail closed.
        let _ledger_refused = result.is_err();
    }
}
