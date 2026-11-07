//! Real owned-runtime HTTP case execution, never full Gate/human qualification.
mod cases;
mod report;
use humanymous_defender::lab::{LabError, OwnedRuntime, VerifiedSnapshot};
pub use report::{NetworkReport, NetworkRow, NetworkTotals, Step};

/// Implemented by the Docker lab supervisor that owns custody and counted origin.
/// Production evidence must use real processes and authenticated Log recovery;
/// a mock implementation can test accounting only and cannot qualify a pilot.
pub trait OwnedLab {
    /// Start a fresh child with the same provisioned installation and revision.
    fn spawn(&mut self) -> Result<OwnedRuntime, LabError>;
    /// Recover authenticated audit after child stop and sample the owned origin.
    fn snapshot(&mut self) -> Result<VerifiedSnapshot, LabError>;
}

/// Execute the complete catalog, retaining every unavailable/error denominator.
pub fn run(lab: &mut impl OwnedLab, seed: u64) -> NetworkReport {
    let mut specifications = cases::catalog();
    crate::catalog::shuffle(&mut specifications, seed);
    let rows = specifications
        .into_iter()
        .map(|spec| cases::execute(lab, spec))
        .collect();
    NetworkReport::new(seed, rows)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::report::Status;
    use humanymous_defender::lab::{ProcessReceipt, SnapshotDelta};
    pub(crate) struct Unavailable;
    impl OwnedLab for Unavailable {
        fn spawn(&mut self) -> Result<OwnedRuntime, LabError> {
            Err(LabError("test-only unavailable child"))
        }
        fn snapshot(&mut self) -> Result<VerifiedSnapshot, LabError> {
            Err(LabError("test-only unavailable audit"))
        }
    }

    fn attempted_failure() -> NetworkReport {
        // Synthetic failed ledger only: no child/network success is manufactured.
        let mut report = run(&mut Unavailable, 42);
        let row = report
            .rows
            .iter_mut()
            .find(|row| row.id == "rit-replay-after-control")
            .unwrap();
        row.processes.push(ProcessReceipt {
            runtime_sha256: "1".repeat(64),
            supervisor_sha256: "2".repeat(64),
            installation: "unit-only-failure".into(),
            revision: "r1".into(),
            binding: "supervisor-owned-child/private-pipe/same-loopback-only-namespace".into(),
        });
        row.steps = [
            ("collect-asserted-fixture", "setup"),
            ("protected-request", "synthetic_control"),
            ("replayed-request", "attack"),
        ]
        .into_iter()
        .map(|(id, role)| Step {
            id: id.into(),
            role: role.into(),
            http_status: Some(200),
            response_verdict: None,
            error: None,
        })
        .collect();
        row.reason = Some("unit-only audit recovery failure".into());
        assert!(!report.scoped_passed);
        report.validate().unwrap();
        report
    }

    #[test]
    fn attempted_execution_cannot_be_skipped_or_transport_failure_evaluated() {
        let mut report = attempted_failure();
        let row = report
            .rows
            .iter_mut()
            .find(|row| !row.steps.is_empty())
            .unwrap();
        row.status = Status::Skipped;
        let report = NetworkReport::new(report.seed, report.rows);
        assert!(report.validate().is_err());

        let mut report = attempted_failure();
        let row = report
            .rows
            .iter_mut()
            .find(|row| !row.steps.is_empty())
            .unwrap();
        row.status = Status::Evaluated;
        row.reason = None;
        row.expectation_matches = Some(false);
        row.effects = Some(SnapshotDelta {
            origin_effects: 0,
            audit: Vec::new(),
        });
        row.steps[2].http_status = None;
        row.steps[2].error = Some("unit-only transport failure".into());
        let report = NetworkReport::new(report.seed, report.rows);
        assert!(report.validate().is_err());
    }

    #[test]
    fn transport_failure_ends_steps_and_cannot_claim_a_response_verdict() {
        let mut report = attempted_failure();
        let row = report
            .rows
            .iter_mut()
            .find(|row| !row.steps.is_empty())
            .unwrap();
        row.steps[1].http_status = None;
        row.steps[1].error = Some("unit-only transport failure".into());
        assert!(report.validate().is_err());

        let row = report
            .rows
            .iter_mut()
            .find(|row| !row.steps.is_empty())
            .unwrap();
        row.steps.truncate(2);
        report.validate().unwrap();
        let row = report
            .rows
            .iter_mut()
            .find(|row| !row.steps.is_empty())
            .unwrap();
        row.steps[1].response_verdict = Some("DENY".into());
        assert!(report.validate().is_err());

        for error in ["", " \t\r\n"] {
            let row = report
                .rows
                .iter_mut()
                .find(|row| !row.steps.is_empty())
                .unwrap();
            row.steps[1].response_verdict = None;
            row.steps[1].error = Some(error.into());
            assert!(report.validate().is_err());
        }
        let mut report = attempted_failure();
        report
            .rows
            .iter_mut()
            .find(|row| !row.steps.is_empty())
            .unwrap()
            .reason = Some(" \t\r\n".into());
        assert!(report.validate().is_err());
    }
    #[test]
    fn absent_lab_cannot_turn_into_success_or_disappear_from_denominator() {
        let report = run(&mut Unavailable, 42);
        assert_eq!(report.totals.scheduled, cases::catalog().len());
        assert_eq!(report.totals.errors, 19);
        assert_eq!(report.totals.unsupported, 7);
        assert!(!report.scoped_passed);
        report.validate().unwrap();
        let alternate = run(&mut Unavailable, 43);
        assert_ne!(
            report.rows.iter().map(|row| &row.id).collect::<Vec<_>>(),
            alternate.rows.iter().map(|row| &row.id).collect::<Vec<_>>()
        );
    }
    #[test]
    fn verifier_refuses_fabricated_totals_participants_missing_cases_and_passes() {
        let mut report = run(&mut Unavailable, 42);
        report.totals.evaluated = 19;
        assert!(report.validate().is_err());
        let mut report = run(&mut Unavailable, 42);
        report.human_executed = true;
        assert!(report.validate().is_err());
        let mut report = run(&mut Unavailable, 42);
        report.rows.pop();
        assert!(report.validate().is_err());
        let mut report = run(&mut Unavailable, 42);
        report.scoped_passed = true;
        assert!(report.validate().is_err());
        let mut report = run(&mut Unavailable, 42);
        report
            .rows
            .iter_mut()
            .find(|row| row.status == crate::report::Status::Error)
            .unwrap()
            .status = crate::report::Status::Evaluated;
        assert!(report.validate().is_err());
    }
}
