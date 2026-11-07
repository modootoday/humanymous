//! Pure diagnostic classification; never native ownership or terminal proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ExpectedOutcome {
    ParentPass,
    MissingGrantRefusal,
    LiveDeadlineTermination,
}
impl ExpectedOutcome {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::ParentPass => "parent-pass",
            Self::MissingGrantRefusal => "missing-grant-refusal",
            Self::LiveDeadlineTermination => "live-parent-original-deadline-termination",
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Facts {
    pub setup_ok: bool,
    pub assigned: bool,
    pub granted: bool,
    pub natural: bool,
    pub run_expired: bool,
    pub live_at_deadline: bool,
    pub exit_code: Option<i32>,
    pub parent_exited: bool,
    pub job_empty: bool,
    pub cleanup_timely: bool,
    pub termination_requested: bool,
    pub termination_failed: bool,
    pub observation_failed: bool,
    pub active_before_termination: Option<u32>,
    pub probe_marker: Option<bool>,
    pub probe_timely: bool,
}
pub(super) struct Classification {
    pub parent_passed: bool,
    pub case_qualified: bool,
}
pub(super) fn classify(expected: ExpectedOutcome, f: Facts) -> Classification {
    let cleanup = f.parent_exited
        && f.job_empty
        && f.cleanup_timely
        && !f.termination_failed
        && !f.observation_failed;
    let common = f.setup_ok && f.assigned && cleanup;
    let parent_passed =
        common && f.granted && f.natural && !f.run_expired && f.exit_code == Some(0);
    let case_qualified = match expected {
        ExpectedOutcome::ParentPass => parent_passed,
        ExpectedOutcome::MissingGrantRefusal => {
            common
                && !f.granted
                && f.natural
                && !f.run_expired
                && f.exit_code == Some(0)
                && f.probe_marker == Some(true)
                && f.probe_timely
        }
        ExpectedOutcome::LiveDeadlineTermination => {
            common
                && f.granted
                && !f.natural
                && f.run_expired
                && f.live_at_deadline
                && f.exit_code == Some(1)
                && f.termination_requested
                && f.active_before_termination.is_some_and(|n| n > 0)
                && f.probe_marker == Some(true)
                && f.probe_timely
        }
    };
    Classification {
        parent_passed,
        case_qualified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn complete() -> Facts {
        Facts {
            setup_ok: true,
            assigned: true,
            granted: true,
            natural: true,
            run_expired: false,
            live_at_deadline: false,
            exit_code: Some(0),
            parent_exited: true,
            job_empty: true,
            cleanup_timely: true,
            termination_requested: true,
            termination_failed: false,
            observation_failed: false,
            active_before_termination: Some(0),
            probe_marker: None,
            probe_timely: true,
        }
    }
    #[test]
    fn ordinary_profile_does_not_accept_a_missing_grant_or_negative_exit() {
        let mut f = complete();
        assert!(classify(ExpectedOutcome::ParentPass, f).case_qualified);
        f.granted = false;
        assert!(!classify(ExpectedOutcome::ParentPass, f).case_qualified);
        f.granted = true;
        f.exit_code = Some(1);
        assert!(!classify(ExpectedOutcome::ParentPass, f).case_qualified);
    }
    #[test]
    fn missing_grant_requires_explicit_refusal_marker_and_known_cleanup() {
        let mut f = complete();
        f.granted = false;
        f.probe_marker = Some(true);
        let result = classify(ExpectedOutcome::MissingGrantRefusal, f);
        assert!(result.case_qualified);
        assert!(!result.parent_passed);
        f.probe_marker = None;
        assert!(!classify(ExpectedOutcome::MissingGrantRefusal, f).case_qualified);
        f.probe_marker = Some(true);
        f.parent_exited = false;
        assert!(!classify(ExpectedOutcome::MissingGrantRefusal, f).case_qualified);
    }
    #[test]
    fn timeout_alone_is_never_terminal_or_a_qualified_live_fault() {
        let mut f = complete();
        f.natural = false;
        f.run_expired = true;
        f.live_at_deadline = true;
        f.exit_code = Some(1);
        f.active_before_termination = Some(1);
        f.probe_marker = Some(true);
        assert!(classify(ExpectedOutcome::LiveDeadlineTermination, f).case_qualified);
        for change in [0, 1, 2, 3, 4, 5, 6] {
            let mut missing = f;
            match change {
                0 => missing.exit_code = None,
                1 => missing.parent_exited = false,
                2 => missing.job_empty = false,
                3 => missing.cleanup_timely = false,
                4 => missing.active_before_termination = None,
                5 => missing.probe_marker = None,
                _ => missing.probe_timely = false,
            }
            assert!(!classify(ExpectedOutcome::LiveDeadlineTermination, missing).case_qualified);
        }
    }
    #[test]
    fn errors_or_already_empty_job_cannot_become_live_termination_evidence() {
        let mut f = complete();
        f.natural = false;
        f.run_expired = true;
        f.live_at_deadline = true;
        f.exit_code = Some(1);
        f.probe_marker = Some(true);
        assert!(!classify(ExpectedOutcome::LiveDeadlineTermination, f).case_qualified);
        f.active_before_termination = Some(1);
        f.observation_failed = true;
        assert!(!classify(ExpectedOutcome::LiveDeadlineTermination, f).case_qualified);
        f.observation_failed = false;
        f.termination_failed = true;
        assert!(!classify(ExpectedOutcome::LiveDeadlineTermination, f).case_qualified);
    }
}
