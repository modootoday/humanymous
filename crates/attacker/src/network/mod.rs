//! Real owned-runtime HTTP case execution, never full Gate/human qualification.
mod cases;
mod report;
use humanymous_core::signals::report::ClientReport;
use humanymous_defender::lab::{
    LabError, OwnedRuntime, Request, Response, SnapshotDelta, VerifiedSnapshot,
};
pub use report::{NetworkReport, NetworkRow, NetworkTotals, Step};

/// The Gate a catalog case talks to. The supervisor-owned child implements it
/// inside this crate; the Docker lab implements it for a Gate in another
/// container on an internal network. Either way the target is owned by the lab.
pub trait Target {
    /// Volatile issued capability; never logged or serialized.
    type Session;
    /// The fixed synthetic header profile for the target's installation.
    fn request(&self, target: &str, body: Vec<u8>) -> Request;
    /// Request a fresh session from the Gate.
    fn issue(&mut self) -> Result<Self::Session, LabError>;
    /// Bind a synthetic report to the issued session.
    fn bind_report(&self, session: &Self::Session, report: &mut ClientReport);
    /// Sign the exact current request bytes with a live deadline.
    fn sign(
        &self,
        request: &mut Request,
        session: &Self::Session,
        counter: u64,
    ) -> Result<(), LabError>;
    /// Sign with an already expired deadline (negative case).
    fn sign_expired(
        &self,
        request: &mut Request,
        session: &Self::Session,
        counter: u64,
    ) -> Result<(), LabError>;
    /// One framed request and its parsed response.
    fn exchange(&mut self, request: &Request) -> Result<Response, LabError>;
    /// Exact (possibly malformed) request bytes and the parsed response.
    fn exchange_raw(&mut self, bytes: &[u8]) -> Result<Response, LabError>;
    /// Replace the Gate process with a fresh one of the same installation.
    fn restart(&mut self) -> Result<(), LabError>;
}

/// One scheduled catalog entry as a remote lab schedules it.
pub struct Entry {
    /// Stable catalog identity.
    pub id: &'static str,
    /// Cost tier or control/runtime scope.
    pub tier: &'static str,
    /// Attack, control, accessibility, capability or ceiling class.
    pub class: &'static str,
    /// Ordered step identities and roles; empty when unsupported.
    pub plan: &'static [(&'static str, &'static str)],
    /// Missing capability for unsupported entries.
    pub reason: Option<&'static str>,
}

/// The complete network catalog in seed order, including unsupported rows.
pub fn entries(seed: u64) -> Vec<Entry> {
    let mut specifications = cases::catalog();
    crate::catalog::shuffle(&mut specifications, seed);
    specifications
        .into_iter()
        .map(|spec| Entry {
            id: spec.id,
            tier: spec.tier,
            class: spec.class,
            plan: spec.kind.map_or(&[][..], cases::step_plan),
            reason: spec.reason,
        })
        .collect()
}

/// Execute one supported case's steps against `target`. Steps are appended as
/// observed, including a failed transport; the caller owns effect accounting.
pub fn execute_case(
    target: &mut impl Target,
    id: &str,
    steps: &mut Vec<Step>,
) -> Result<(), LabError> {
    let kind = cases::catalog()
        .into_iter()
        .find(|spec| spec.id == id)
        .and_then(|spec| spec.kind)
        .ok_or(LabError("unknown or unsupported catalog case"))?;
    cases::execute_steps(target, steps, kind)
}

/// The catalog's immutable expectation for one case: `None` for an unknown or
/// unsupported case, otherwise whether the observed steps and effects match.
pub fn expectation_matches(id: &str, steps: &[Step], effects: &SnapshotDelta) -> Option<bool> {
    let kind = cases::catalog()
        .into_iter()
        .find(|spec| spec.id == id)?
        .kind?;
    Some(
        steps.len() == cases::step_plan(kind).len()
            && steps
                .iter()
                .zip(cases::step_plan(kind))
                .all(|(step, (id, role))| step.id == *id && step.role == *role)
            && cases::matches_effects(kind, steps, effects),
    )
}

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
    /// A scripted remote Gate: answers by request path, counts restarts.
    struct Scripted {
        restarts: usize,
        replies: Vec<u16>,
    }
    impl Target for Scripted {
        type Session = ();
        fn request(&self, target: &str, body: Vec<u8>) -> Request {
            Request {
                method: "POST".into(),
                target: target.into(),
                headers: vec![
                    ("Host".into(), "owned.test".into()),
                    ("Content-Length".into(), body.len().to_string()),
                ],
                body,
            }
        }
        fn issue(&mut self) -> Result<(), LabError> {
            Ok(())
        }
        fn bind_report(&self, _: &(), _: &mut ClientReport) {}
        fn sign(&self, request: &mut Request, _: &(), _: u64) -> Result<(), LabError> {
            request
                .headers
                .push(("x-humanymous-token".into(), "t".into()));
            Ok(())
        }
        fn sign_expired(&self, request: &mut Request, s: &(), n: u64) -> Result<(), LabError> {
            self.sign(request, s, n)
        }
        fn exchange(&mut self, _: &Request) -> Result<Response, LabError> {
            let status = self.replies.remove(0);
            Ok(Response {
                status,
                headers: Vec::new(),
                body: Vec::new(),
            })
        }
        fn exchange_raw(&mut self, _: &[u8]) -> Result<Response, LabError> {
            self.exchange(&self.request("/", Vec::new()))
        }
        fn restart(&mut self) -> Result<(), LabError> {
            self.restarts += 1;
            Ok(())
        }
    }

    #[test]
    fn remote_entries_keep_the_whole_denominator_and_step_plans() {
        let entries = entries(42);
        assert_eq!(entries.len(), cases::catalog().len());
        assert_eq!(entries.iter().filter(|e| e.plan.is_empty()).count(), 7);
        assert!(
            entries
                .iter()
                .all(|e| e.plan.is_empty() == e.reason.is_some())
        );
        let ids: Vec<_> = entries.iter().map(|e| e.id).collect();
        let owned: Vec<_> = run(&mut Unavailable, 42)
            .rows
            .into_iter()
            .map(|row| row.id)
            .collect();
        assert_eq!(ids, owned);
    }

    #[test]
    fn remote_execution_follows_the_plan_and_only_exact_effects_match() {
        let mut target = Scripted {
            restarts: 0,
            replies: vec![200, 200, 403],
        };
        let mut steps = Vec::new();
        execute_case(&mut target, "rit-replay-after-control", &mut steps).unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(target.restarts, 0);
        let event = |seq, verdict: &str| humanymous_defender::lab::AuditEvent {
            seq,
            event_type: if verdict == "COLLECTED" {
                "runtime.collect"
            } else {
                "runtime.decision"
            }
            .into(),
            verdict: verdict.into(),
            risk_score: 0,
            rules: Vec::new(),
            record_hash: "a".repeat(64),
        };
        let good = SnapshotDelta {
            origin_effects: 1,
            audit: vec![event(1, "COLLECTED"), event(2, "ALLOW")],
        };
        assert_eq!(
            expectation_matches("rit-replay-after-control", &steps, &good),
            Some(true)
        );
        let doubled = SnapshotDelta {
            origin_effects: 2,
            audit: good.audit.clone(),
        };
        assert_eq!(
            expectation_matches("rit-replay-after-control", &steps, &doubled),
            Some(false)
        );
        assert_eq!(
            expectation_matches("rit-replay-after-control", &steps[..2], &good),
            Some(false)
        );
        assert_eq!(
            expectation_matches("coherent-T4-browser", &steps, &good),
            None
        );
        assert_eq!(expectation_matches("unknown", &steps, &good), None);
        assert!(execute_case(&mut target, "coherent-T4-browser", &mut steps).is_err());

        let mut target = Scripted {
            restarts: 0,
            replies: vec![401],
        };
        let mut steps = Vec::new();
        execute_case(&mut target, "restart-stale-session", &mut steps).unwrap();
        assert_eq!(target.restarts, 1);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].id, "stale-epoch-request");
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
