//! Native timer/dispatch adapter for the pure ordered probe ledger.
use crate::{child::Shared, navigation};
use humanymous_browser::navigation::{Outcome, Ticket};
use std::{
    io,
    time::{Duration, Instant},
};
#[derive(Default)]
pub(super) struct Probe {
    pending: Option<(Ticket, Instant)>,
}
impl Probe {
    pub(super) fn tick(&mut self, state: &Shared) -> io::Result<()> {
        let next = {
            let mut state = state
                .lock()
                .map_err(|_| io::Error::other("state poisoned"))?;
            if state.revoked
                || state.closed
                || state.navigation.probe.as_ref().is_none_or(|p| p.finished())
            {
                return Ok(());
            }
            if self.pending.is_none() {
                if let Some(ticket) = state.navigation.transaction.current() {
                    self.pending = Some((ticket, Instant::now()));
                } else {
                    if let Some(probe) = &mut state.navigation.probe {
                        probe.abort();
                    }
                    return Ok(());
                }
            }
            let Some((ticket, started)) = self.pending else {
                return Ok(());
            };
            if state.navigation.transaction.current() != Some(ticket) {
                if let Some(probe) = &mut state.navigation.probe {
                    probe.abort();
                }
                self.pending = None;
                return Ok(());
            }
            if state.navigation.transaction.outcome(ticket).is_none()
                && started.elapsed() > Duration::from_secs(5)
            {
                state.navigation.finish(ticket, Outcome::ProbeTimeout, 0, 0);
            }
            let Some(outcome) = state.navigation.transaction.outcome(ticket) else {
                return Ok(());
            };
            let Some(probe) = &mut state.navigation.probe else {
                return Err(io::Error::other("probe authority missing"));
            };
            probe.observe(ticket, outcome);
            self.pending = None;
            probe
                .next_route()
                .map(|route| (route, state.browser.clone()))
        };
        let missing_browser = next.as_ref().is_some_and(|(_, browser)| browser.is_none());
        if let Some((route, Some(browser))) = next {
            match navigation::queue(browser, route, state.clone(), false) {
                Ok(ticket) => self.pending = Some((ticket, Instant::now())),
                Err(error) => {
                    // A post-task rejection has a terminal ticket. Preserve it and
                    // collect later independent route results rather than exiting early.
                    let mut state = state
                        .lock()
                        .map_err(|_| io::Error::other("state poisoned"))?;
                    if let Some(ticket) = state
                        .navigation
                        .transaction
                        .current()
                        .filter(|t| t.route() == route)
                        && state.navigation.transaction.outcome(ticket).is_some()
                    {
                        self.pending = Some((ticket, Instant::now()));
                    } else {
                        if let Some(probe) = &mut state.navigation.probe {
                            probe.abort();
                        }
                        return Err(error);
                    }
                }
            }
        } else if missing_browser
            && let Ok(mut state) = state.lock()
            && let Some(probe) = &mut state.navigation.probe
        {
            probe.abort();
        }
        Ok(())
    }
}
