//! Pure local navigation authority. CEF/OS receipts must be supplied by adapters.
use crate::internal_urls::{InternalPage, Route};

/// Identity captured before posting a native navigation operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    generation: u64,
    route: Route,
}
impl Ticket {
    /// Monotonic identity; no handle or native permission is represented.
    pub fn generation(self) -> u64 {
        self.generation
    }
    /// Exact typed local route.
    pub fn route(self) -> Route {
        self.route
    }
}
/// Terminal observations are separate from request acceptance and commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The user explicitly stopped this transaction.
    Stopped,
    /// Native UI dispatcher refused the task.
    TaskRejected,
    /// The task ran without a usable main frame.
    FrameUnavailable,
    /// CEF reported a main document error.
    LoadError,
    /// Fixed local documents returned an unexpected main-frame load status.
    LoadStatusRejected,
    /// Bounded inspection completed; identity=false is a failed observation.
    Content {
        /// True only when the bounded native document identity check matched.
        identity: bool,
    },
    /// Source exceeded its permitted allocation or inspection bound.
    SourceBudgetExceeded,
    /// The current probe step exceeded its deadline.
    ProbeTimeout,
}
/// One current request plus the independently observed committed page.
#[derive(Debug, Default)]
pub struct Navigation {
    generation: u64,
    current: Option<Ticket>,
    committed: Option<Ticket>,
    outcome: Option<Outcome>,
    revoked: bool,
}
impl Navigation {
    /// Supersede pending work, without inventing a committed page.
    pub fn begin(&mut self, route: Route) -> Result<Ticket, &'static str> {
        if self.revoked {
            return Err("navigation authority revoked");
        }
        if self.generation >= 64 {
            return Err("navigation request budget exhausted");
        }
        self.generation += 1;
        let ticket = Ticket {
            generation: self.generation,
            route,
        };
        self.current = Some(ticket);
        self.outcome = None;
        Ok(ticket)
    }
    /// Current identity, including a completed transaction, while authority is live.
    pub fn current(&self) -> Option<Ticket> {
        if self.revoked { None } else { self.current }
    }
    /// Only this identity can still perform or complete pending work.
    pub fn pending(&self) -> Option<Ticket> {
        self.current().filter(|_| self.outcome.is_none())
    }
    /// A native browse may only continue the currently pending exact target.
    pub fn permits_browse(&self, route: Route) -> bool {
        self.pending().is_some_and(|ticket| ticket.route() == route)
    }
    /// Last actual commit supplied by the native adapter; acceptance is insufficient.
    pub fn committed(&self) -> Option<Ticket> {
        self.committed
    }
    /// Accept a commit only once for the current pending identity.
    pub fn observe_commit(&mut self, ticket: Ticket) -> bool {
        if self.pending() != Some(ticket) || self.committed == Some(ticket) {
            return false;
        }
        self.committed = Some(ticket);
        true
    }
    /// Record a single terminal result. Successful inspection requires a commit.
    pub fn finish(&mut self, ticket: Ticket, outcome: Outcome) -> bool {
        if self.pending() != Some(ticket) {
            return false;
        }
        if matches!(outcome, Outcome::Content { .. }) && self.committed != Some(ticket) {
            return false;
        }
        self.outcome = Some(outcome);
        true
    }
    /// Terminal result for exactly this identity; stale tickets have no authority.
    pub fn outcome(&self, ticket: Ticket) -> Option<Outcome> {
        if self.current() == Some(ticket) {
            self.outcome
        } else {
            None
        }
    }
    /// Irreversible revocation; diagnostic commit history is not erased.
    pub fn revoke(&mut self) {
        self.revoked = true;
    }
}

/// Ordered bounded diagnostic probe; terminal failures do not erase later results.
#[derive(Debug, Default)]
pub struct ProbeLedger {
    observed: usize,
    last_generation: u64,
    failed: bool,
    aborted: bool,
}
impl ProbeLedger {
    /// Required sequence includes both initial document and return to the owner.
    pub const ROUTES: [Route; InternalPage::ALL.len() + 2] = {
        let mut routes = [Route::NewTab; InternalPage::ALL.len() + 2];
        let mut index = 0;
        while index < InternalPage::ALL.len() {
            routes[index + 1] = Route::Internal(InternalPage::ALL[index]);
            index += 1;
        }
        routes
    };
    /// Next expected observation; an aborted probe cannot start more work.
    pub fn next_route(&self) -> Option<Route> {
        if self.aborted {
            None
        } else {
            Self::ROUTES.get(self.observed).copied()
        }
    }
    /// Consume one expected, fresh identity. Failure is sticky, but collect all steps.
    pub fn observe(&mut self, ticket: Ticket, outcome: Outcome) -> bool {
        if self.next_route() != Some(ticket.route()) || ticket.generation() <= self.last_generation
        {
            self.abort();
            return false;
        }
        self.last_generation = ticket.generation();
        self.observed += 1;
        self.failed |= outcome != (Outcome::Content { identity: true });
        true
    }
    /// Superseded/malformed sequence is incomplete and can never pass.
    pub fn abort(&mut self) {
        self.aborted = true;
        self.failed = true;
    }
    /// Number of actual terminal observations, never the number of scheduled tasks.
    pub fn observed(&self) -> usize {
        self.observed
    }
    /// No more work is scheduled, including abort. This is not a success predicate.
    pub fn finished(&self) -> bool {
        self.aborted || self.observed == Self::ROUTES.len()
    }
    /// Only all seven correctly ordered positive observations pass.
    pub fn passed(&self) -> bool {
        !self.aborted && !self.failed && self.observed == Self::ROUTES.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stop_rejects_late_commit_and_content_without_revoking_future_requests() {
        let mut navigation = Navigation::default();
        let ticket = navigation.begin(Route::NewTab).unwrap();
        assert!(navigation.finish(ticket, Outcome::Stopped));
        assert!(!navigation.permits_browse(Route::NewTab));
        assert!(!navigation.observe_commit(ticket));
        assert!(!navigation.finish(ticket, Outcome::Content { identity: true }));
        let next = navigation.begin(Route::NewTab).unwrap();
        assert_ne!(next, ticket);
        assert!(navigation.observe_commit(next));
    }
    #[test]
    fn supersession_revoke_and_terminal_failure_reject_captured_work() {
        let mut navigation = Navigation::default();
        let old = navigation.begin(Route::NewTab).unwrap();
        let current = navigation.begin(Route::NewTab).unwrap();
        assert!(navigation.permits_browse(Route::NewTab));
        assert!(!navigation.permits_browse(Route::Internal(InternalPage::Gpu)));
        assert!(!navigation.observe_commit(old));
        assert!(!navigation.finish(old, Outcome::LoadError));
        assert!(navigation.finish(current, Outcome::TaskRejected));
        assert_eq!(navigation.pending(), None);
        assert!(!navigation.permits_browse(Route::NewTab));
        assert!(!navigation.observe_commit(current));
        assert!(!navigation.finish(current, Outcome::Content { identity: true }));
        let next = navigation
            .begin(Route::Internal(InternalPage::Gpu))
            .unwrap();
        navigation.revoke();
        assert!(!navigation.observe_commit(next));
        assert!(!navigation.finish(next, Outcome::LoadError));
        assert!(navigation.begin(Route::NewTab).is_err());
    }
    #[test]
    fn accepted_or_uncommitted_inspection_never_changes_committed_address() {
        let mut navigation = Navigation::default();
        let first = navigation.begin(Route::NewTab).unwrap();
        assert!(!navigation.finish(first, Outcome::Content { identity: true }));
        assert!(navigation.observe_commit(first));
        assert!(navigation.finish(first, Outcome::Content { identity: true }));
        let next = navigation
            .begin(Route::Internal(InternalPage::Version))
            .unwrap();
        assert_eq!(navigation.committed(), Some(first));
        assert!(navigation.finish(next, Outcome::FrameUnavailable));
        assert_eq!(navigation.committed(), Some(first));
        for _ in 2..64 {
            navigation.begin(Route::NewTab).unwrap();
        }
        assert!(navigation.begin(Route::NewTab).is_err());
    }
    #[test]
    fn probe_collects_independent_failures_without_cleanup_becoming_a_pass() {
        for rejected in [
            Outcome::TaskRejected,
            Outcome::FrameUnavailable,
            Outcome::LoadError,
            Outcome::LoadStatusRejected,
            Outcome::Content { identity: false },
            Outcome::SourceBudgetExceeded,
            Outcome::ProbeTimeout,
        ] {
            let mut navigation = Navigation::default();
            let mut probe = ProbeLedger::default();
            assert!(!probe.passed());
            for (index, route) in ProbeLedger::ROUTES.into_iter().enumerate() {
                let ticket = navigation.begin(route).unwrap();
                let outcome = if index == 2 {
                    rejected
                } else {
                    Outcome::Content { identity: true }
                };
                assert!(probe.observe(ticket, outcome));
            }
            assert!(probe.finished());
            assert_eq!(probe.observed(), 7);
            assert!(!probe.passed());
        }
    }
    #[test]
    fn probe_requires_every_ordered_positive_observation_and_fresh_ticket() {
        let mut navigation = Navigation::default();
        let mut probe = ProbeLedger::default();
        for route in ProbeLedger::ROUTES {
            let ticket = navigation.begin(route).unwrap();
            assert!(probe.observe(ticket, Outcome::Content { identity: true }));
        }
        assert!(probe.passed());
        let mut replay = ProbeLedger::default();
        let mut first = None;
        for route in ProbeLedger::ROUTES.into_iter().take(6) {
            let ticket = navigation.begin(route).unwrap();
            first.get_or_insert(ticket);
            assert!(replay.observe(ticket, Outcome::Content { identity: true }));
        }
        // Same route as the required return, but an already consumed generation.
        assert!(!replay.observe(first.unwrap(), Outcome::Content { identity: true }));
        assert!(replay.finished());
        assert!(!replay.passed());
        assert_eq!(replay.observed(), 6);
    }
}
