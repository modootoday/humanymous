//! Bounded shutdown facts, distinct from cancellation intent or supervisor kill.
use serde::Serialize;
use std::{fmt, io};
/// Complete shutdown evidence, returned only after request work and audit flush.
#[derive(Serialize)]
pub struct ExitReport {
    /// Both owned drain and final audit flush completed successfully.
    pub stopped: bool,
    /// All owned asynchronous request work ended successfully before flush.
    pub drained: bool,
    /// The final synchronous audit flush actually returned success.
    pub audit_flushed: bool,
}
/// Failed shutdown phases, without interpolating underlying sensitive errors.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExitStage {
    /// Listener work ended with an error; final flush succeeded.
    Drain,
    /// Listener drain succeeded; final audit flush failed.
    AuditFlush,
    /// Both phases returned errors.
    DrainAndAuditFlush,
}
/// An unsuccessful shutdown; these facts never constitute a Stopped receipt.
#[derive(Debug, Serialize)]
pub struct ExitFailure {
    /// Exactly the phases which returned errors.
    pub stage: ExitStage,
    /// True only when the listener drain actually returned success.
    pub drained: bool,
    /// True only when the final audit flush actually returned success.
    pub audit_flushed: bool,
}
impl fmt::Display for ExitFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "service shutdown failed at {:?}", self.stage)
    }
}
impl std::error::Error for ExitFailure {}
pub(super) fn finish(
    drain: io::Result<()>,
    flush: io::Result<()>,
) -> Result<ExitReport, ExitFailure> {
    let drained = drain.is_ok();
    let audit_flushed = flush.is_ok();
    let stage = match (drained, audit_flushed) {
        (true, true) => {
            return Ok(ExitReport {
                stopped: true,
                drained,
                audit_flushed,
            });
        }
        (false, true) => ExitStage::Drain,
        (true, false) => ExitStage::AuditFlush,
        (false, false) => ExitStage::DrainAndAuditFlush,
    };
    Err(ExitFailure {
        stage,
        drained,
        audit_flushed,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_accounting_preserves_both_observed_phases_without_secret_errors() {
        let failure = finish(
            Err(io::Error::other("private path or credential")),
            Err(io::Error::other("private WAL path")),
        )
        .err()
        .unwrap();
        assert_eq!(failure.stage, ExitStage::DrainAndAuditFlush);
        assert!(!failure.drained && !failure.audit_flushed);
        let failure = finish(Err(io::Error::other("secret")), Ok(()))
            .err()
            .unwrap();
        assert_eq!(failure.stage, ExitStage::Drain);
        assert!(!failure.drained && failure.audit_flushed);
        assert!(!serde_json::to_string(&failure).unwrap().contains("secret"));
        let failure = finish(Ok(()), Err(io::Error::other("secret")))
            .err()
            .unwrap();
        assert_eq!(failure.stage, ExitStage::AuditFlush);
        assert!(failure.drained && !failure.audit_flushed);
        assert!(finish(Ok(()), Ok(())).unwrap().stopped);
    }
}
