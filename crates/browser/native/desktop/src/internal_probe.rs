//! Fixed-route bounded native qualification. No arbitrary URL or page script.
use crate::{child::Shared, navigation};
use humanymous_browser::internal_urls::{InternalPage, Route};
use std::{
    io,
    time::{Duration, Instant},
};
#[derive(Default)]
pub(super) struct Probe {
    index: usize,
    pending: Option<(u64, Route, Instant)>,
    done: bool,
}
impl Probe {
    pub(super) fn tick(&mut self, state: &Shared) -> io::Result<()> {
        if self.done {
            return Ok(());
        }
        let (browser, ready) = {
            let mut state = state
                .lock()
                .map_err(|_| io::Error::other("state poisoned"))?;
            if state.revoked || state.closed {
                return Ok(());
            }
            let ready = if let Some((generation, route, started)) = self.pending {
                let terminal = state.navigation.receipts.iter().any(|r| {
                    r.generation == generation
                        && matches!(
                            r.phase,
                            "content_observed" | "load_error" | "source_budget_exceeded"
                        )
                });
                if !terminal && started.elapsed() > Duration::from_secs(5) {
                    state.navigation.record(route, "probe_timeout", 0, 0, false);
                    true
                } else {
                    terminal
                }
            } else {
                state.navigation.receipts.iter().any(|r| {
                    r.route == Route::NewTab.url() && r.phase == "content_observed" && r.identity
                })
            };
            (state.browser.clone(), ready)
        };
        if !ready {
            return Ok(());
        }
        if self.index > InternalPage::ALL.len() {
            self.done = true;
            return Ok(());
        }
        let route = InternalPage::ALL
            .get(self.index)
            .copied()
            .map_or(Route::NewTab, Route::Internal);
        if let Some(browser) = browser {
            let generation = navigation::queue(browser, route, state.clone(), false)?;
            self.pending = Some((generation, route, Instant::now()));
            self.index += 1;
        }
        Ok(())
    }
}
