//! Preserve returned runtime phase facts without promoting an error to Stopped.
use humanymous_runtime::gateway::service::{ExitFailure, ExitStage};
use serde::Serialize;
use std::io;

/// Bounded failure envelope; absent phase facts mean unknown, never false success.
#[derive(Serialize)]
pub struct HostFailure<'a> {
    /// Stable CLI failure schema.
    pub schema: &'static str,
    /// Verified host generation binding.
    pub generation: &'a str,
    /// Verified executing bytes.
    pub executable_sha256: &'a str,
    /// Only a typed returned runtime failure identifies its phase.
    pub stage: Option<&'a ExitStage>,
    /// Actual drain fact if it returned; otherwise unknown.
    pub drained: Option<bool>,
    /// Actual final flush fact if it returned; otherwise unknown.
    pub audit_flushed: Option<bool>,
    /// An error never publishes a successful stopped receipt.
    pub stopped: bool,
    /// The fixture and partial runtime remain unqualified as a whole product.
    pub product_qualified: bool,
}

/// Extract only the shared typed error; never serialize an arbitrary error payload.
pub fn describe_failure<'a>(
    error: &'a io::Error,
    generation: &'a str,
    executable_sha256: &'a str,
) -> HostFailure<'a> {
    let failure = error
        .get_ref()
        .and_then(|value| value.downcast_ref::<ExitFailure>());
    HostFailure {
        schema: "humanymous.runtime-host.failure.v1",
        generation,
        executable_sha256,
        stage: failure.map(|f| &f.stage),
        drained: failure.map(|f| f.drained),
        audit_flushed: failure.map(|f| f.audit_flushed),
        stopped: false,
        product_qualified: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pure_failure_keeps_partial_flush_success_and_unknown_separate() {
        let error = io::Error::other(ExitFailure {
            stage: ExitStage::Drain,
            drained: false,
            audit_flushed: true,
        });
        let report = describe_failure(&error, "generation", "digest");
        assert_eq!(report.drained, Some(false));
        assert_eq!(report.audit_flushed, Some(true));
        assert!(!report.stopped);
        let unknown = io::Error::other("do-not-export-an-arbitrary-error");
        let report = describe_failure(&unknown, "generation", "digest");
        assert!(
            report.drained.is_none() && report.audit_flushed.is_none() && report.stage.is_none()
        );
        assert!(
            !serde_json::to_string(&report)
                .unwrap()
                .contains("do-not-export")
        );
    }
}
