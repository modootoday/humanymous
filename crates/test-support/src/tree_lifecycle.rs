//! Pure ownership model only: no spawning, native proof, cleanup or Drop supervisor.
//! Callers supply symbolic observations; this module cannot qualify a backend.

/// Independent terminal obligations, including reaping separately from emptiness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalFact {
    /// SDK exited and was reaped, or was proved never started.
    SdkSettled,
    /// The exact owned native containment has no live population.
    PopulationEmpty,
    /// All required supervisor and adopted-descendant waits are complete.
    RequiredReapsComplete,
    /// All owned readers have joined and no pipe worker remains.
    ReadersJoined,
    /// No further signal or stop operation can be issued for this invocation.
    StopOperationsSettled,
    /// The exact allocation can now be released without losing cleanup ownership.
    AllocationReleaseReady,
}

impl TerminalFact {
    fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// Symbolic execution outcome; clean teardown does not imply SDK success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Execution {
    /// SDK success is not yet established.
    Unknown,
    /// SDK returned a nonzero status or setup/capture failed.
    Failed,
    /// SDK success and complete bounded capture were independently observed.
    Succeeded,
}

/// A retained symbolic owner, including partially started allocations.
///
/// This intentionally has no custom Drop: it is not a native lifetime adapter.
/// An actual adapter must first supply the reviewed recovery-supervisor contract.
#[must_use = "retain the owner until terminal obligations are satisfied"]
pub struct Lifecycle<Owner> {
    owner: Owner,
    facts: u8,
    cleanup_required: bool,
    execution: Execution,
}

/// Terminal model result. This is not authenticated native evidence.
pub struct Terminal<Owner> {
    /// Owner returned to its adapter for final release.
    pub owner: Owner,
    /// Execution outcome, separate from the terminal cleanup conjunction.
    pub execution: Execution,
}

impl<Owner> Lifecycle<Owner> {
    /// Retain an actual adapter-owned value at the first partial allocation.
    pub fn retain(owner: Owner) -> Self {
        Self {
            owner,
            facts: 0,
            cleanup_required: false,
            execution: Execution::Unknown,
        }
    }

    /// Borrow retained ownership, including after a failed observation.
    pub fn owner(&mut self) -> &mut Owner {
        &mut self.owner
    }

    /// Record a separately established terminal obligation.
    pub fn observe(&mut self, fact: TerminalFact) {
        self.facts |= fact.bit();
    }

    /// Invalidate a previously observed fact when its proof becomes uncertain.
    pub fn invalidate(&mut self, fact: TerminalFact) {
        self.facts &= !fact.bit();
        self.cleanup_required = true;
    }

    /// Request cleanup from any started state, without consuming ownership.
    pub fn require_cleanup(&mut self) {
        self.cleanup_required = true;
    }

    /// Whether cleanup was requested; repeated requests cannot reset state.
    pub fn cleanup_required(&self) -> bool {
        self.cleanup_required
    }

    /// Record execution separately. Failure is irreversible for this attempt.
    pub fn execution(&mut self, observation: Execution) {
        if self.execution != Execution::Failed {
            self.execution = observation;
        }
    }

    /// Return the SAME owner on every incomplete terminal conjunction.
    pub fn finish(self) -> Result<Terminal<Owner>, Self> {
        if self.facts != 0b11_1111 {
            return Err(self);
        }
        Ok(Terminal {
            owner: self.owner,
            execution: self.execution,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    const FACTS: [TerminalFact; 6] = [
        TerminalFact::SdkSettled,
        TerminalFact::PopulationEmpty,
        TerminalFact::RequiredReapsComplete,
        TerminalFact::ReadersJoined,
        TerminalFact::StopOperationsSettled,
        TerminalFact::AllocationReleaseReady,
    ];

    struct Owner(Rc<Cell<usize>>);
    impl Drop for Owner {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn every_missing_fact_returns_actual_owner_without_dropping_it() {
        for missing in FACTS {
            let drops = Rc::new(Cell::new(0));
            let mut state = Lifecycle::retain(Owner(drops.clone()));
            for fact in FACTS {
                if fact != missing {
                    state.observe(fact);
                }
            }
            state.require_cleanup();
            let mut retained = state.finish().err().expect("missing obligation");
            assert_eq!(drops.get(), 0);
            assert!(retained.cleanup_required());
            let _ = retained.owner();
            retained.observe(missing);
            let terminal = retained.finish().ok().expect("all obligations");
            assert_eq!(terminal.execution, Execution::Unknown);
            assert_eq!(drops.get(), 0);
            drop(terminal);
            assert_eq!(drops.get(), 1);
        }
    }

    #[test]
    fn all_sixty_four_fact_combinations_require_the_full_conjunction() {
        for mask in 0_u8..64 {
            let mut state = Lifecycle::retain(());
            for fact in FACTS {
                if mask & fact.bit() != 0 {
                    state.observe(fact);
                }
            }
            assert_eq!(state.finish().is_ok(), mask == 63);
        }
    }

    #[test]
    fn partial_start_and_late_unknown_keep_the_same_owner() {
        let mut state = Lifecycle::retain(String::from("private owner"));
        state.require_cleanup();
        state.require_cleanup();
        for fact in FACTS {
            state.observe(fact);
        }
        state.invalidate(TerminalFact::ReadersJoined);
        let mut state = state.finish().err().expect("reader unknown");
        assert_eq!(state.owner(), "private owner");
        state.observe(TerminalFact::ReadersJoined);
        assert!(state.finish().is_ok());
    }

    #[test]
    fn cleanup_completion_does_not_promote_failed_execution() {
        let mut state = Lifecycle::retain(());
        state.execution(Execution::Failed);
        state.execution(Execution::Succeeded);
        for fact in FACTS {
            state.observe(fact);
        }
        assert_eq!(state.finish().ok().unwrap().execution, Execution::Failed);
    }
}
