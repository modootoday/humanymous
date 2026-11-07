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
mod tests {
    use super::*;
    struct Unavailable;
    impl OwnedLab for Unavailable {
        fn spawn(&mut self) -> Result<OwnedRuntime, LabError> {
            Err(LabError("test-only unavailable child"))
        }
        fn snapshot(&mut self) -> Result<VerifiedSnapshot, LabError> {
            Err(LabError("test-only unavailable audit"))
        }
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
