//! Real HTTP cases use the attacker catalog and authenticated audit projection.
use crate::installation::Installation;
use humanymous_attacker::network::{self, OwnedLab};
use humanymous_defender::lab::{LabError, OwnedRuntime, VerifiedSnapshot};

impl OwnedLab for Installation {
    fn spawn(&mut self) -> Result<OwnedRuntime, LabError> {
        OwnedRuntime::spawn(&self.config)
    }
    fn snapshot(&mut self) -> Result<VerifiedSnapshot, LabError> {
        let log = self
            .open_log()
            .map_err(|_| LabError("owned authenticated Log recovery"))?;
        VerifiedSnapshot::read(&log, self.origin.effects() as u64)
    }
}
pub fn run() {
    let report = match Installation::new(false) {
        Ok(mut installation) => network::run(&mut installation, 42),
        Err(error) => {
            eprintln!("owned installation setup failed: {error}");
            network::run(&mut SetupFailed, 42)
        }
    };
    report.validate().expect("complete consistent case ledger");
    println!(
        "{}",
        serde_json::to_string(&report).expect("sanitized report serialization")
    );
    if !report.scoped_passed {
        std::process::exit(1);
    }
}
struct SetupFailed;
impl OwnedLab for SetupFailed {
    fn spawn(&mut self) -> Result<OwnedRuntime, LabError> {
        Err(LabError("actual owned installation setup failed"))
    }
    fn snapshot(&mut self) -> Result<VerifiedSnapshot, LabError> {
        Err(LabError("actual owned installation setup failed"))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn setup_failure_retains_the_entire_denominator() {
        let report = network::run(&mut SetupFailed, 42);
        assert_eq!(report.totals.scheduled, 26);
        assert_eq!(report.totals.errors, 19);
        assert_eq!(report.totals.unsupported, 7);
        assert!(!report.scoped_passed);
        report.validate().unwrap();
    }
}
