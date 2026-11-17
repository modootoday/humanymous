use super::{
    OwnedLab, Target,
    report::{NetworkRow, Step},
};
use crate::report::Status;
use humanymous_core::signals::report::ClientReport;
use humanymous_defender::lab::{
    LabError, OwnedRuntime, ProcessReceipt, Request, Response, Session, SnapshotDelta,
};
use serde_json::{Value, json};

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Baseline,
    Selenium,
    MissingReport,
    LongWindow,
    ForgedPass,
    Replay,
    Body,
    Target,
    HeaderOrder,
    Expired,
    MissingToken,
    MissingSession,
    DuplicateLength,
    AdminSpoof,
    Tls,
    H2,
    Collector,
    Restart,
    Method,
}
pub(super) struct Spec {
    pub id: &'static str,
    pub tier: &'static str,
    pub class: &'static str,
    pub kind: Option<Kind>,
    pub reason: Option<&'static str>,
}

pub(super) fn catalog() -> Vec<Spec> {
    use Kind::*;
    let mut result: Vec<_> = [
        (
            "synthetic-zero-motor-allow",
            "control",
            "synthetic_control",
            Baseline,
        ),
        ("synthetic-selenium-deny", "T1", "attack", Selenium),
        ("missing-client-report", "T0", "attack", MissingReport),
        (
            "synthetic-long-window-challenge",
            "control",
            "synthetic_accessibility",
            LongWindow,
        ),
        ("forged-pass-server-signal", "T2", "attack", ForgedPass),
        ("rit-replay-after-control", "T0", "attack", Replay),
        ("rit-body-mutation", "T0", "attack", Body),
        ("rit-query-target-mutation", "T0", "attack", Target),
        ("rit-header-order-mutation", "T0", "attack", HeaderOrder),
        ("rit-expired-deadline", "T0", "attack", Expired),
        ("rit-missing-token", "T0", "attack", MissingToken),
        ("missing-session", "T0", "attack", MissingSession),
        ("duplicate-content-length", "T0", "attack", DuplicateLength),
        ("forwarded-local-admin-forgery", "T0", "attack", AdminSpoof),
        ("required-tls-refusal", "runtime", "capability_control", Tls),
        ("required-h2-refusal", "runtime", "capability_control", H2),
        (
            "required-collector-refusal",
            "runtime",
            "capability_control",
            Collector,
        ),
        ("restart-stale-session", "T0", "attack", Restart),
        ("method-route-mutation", "T0", "attack", Method),
    ]
    .into_iter()
    .map(|(id, tier, class, kind)| Spec {
        id,
        tier,
        class,
        kind: Some(kind),
        reason: None,
    })
    .collect();
    for (id, class, reason) in [
        (
            "ordinary-browser-human",
            "human",
            "Requires actual ordinary browser, qualified collector and human participation",
        ),
        (
            "assistive-technology-human",
            "accessibility",
            "Requires actual AT user interaction and recovery evidence; synthetic events are not participants",
        ),
        (
            "accessible-pass-human",
            "accessibility",
            "Actual accessible Pass recovery/participant producer is not available in this runtime profile",
        ),
        (
            "signed-remote-installation",
            "runtime",
            "Not an HTTP1 row: product-lab target-attestation cases evaluate the pinned-key fresh-nonce answer separately",
        ),
        (
            "coherent-T4-browser",
            "ceiling",
            "Real coherent browser/human T4 is an intentional detection ceiling, not a synthetic detection success",
        ),
        (
            "full-gate-original-tls-h2",
            "runtime",
            "Loopback HTTP1 child is not a qualified full Gate with original TLS/H2 capture",
        ),
        (
            "audit-outage-network-injection",
            "runtime",
            "No safe isolated live audit fault producer is wired into this supervisor contract",
        ),
    ] {
        result.push(Spec {
            id,
            tier: if class == "ceiling" { "T4" } else { "runtime" },
            class,
            kind: None,
            reason: Some(reason),
        });
    }
    result
}

fn expected(
    kind: Kind,
) -> (
    &'static [u16],
    u64,
    &'static [&'static str],
    Option<&'static str>,
) {
    use Kind::*;
    match kind {
        Baseline => (&[200, 200], 1, &["COLLECTED", "ALLOW"], None),
        Selenium => (&[200, 403], 0, &["COLLECTED", "DENY"], Some("HR-1")),
        MissingReport => (&[403], 0, &["CHALLENGE"], Some("HR-10")),
        LongWindow => (&[200, 403], 0, &["COLLECTED", "CHALLENGE"], Some("HR-12")),
        ForgedPass => (&[400, 403], 0, &["CHALLENGE"], Some("HR-10")),
        Replay => (&[200, 200, 403], 1, &["COLLECTED", "ALLOW"], None),
        Body | Target | HeaderOrder | Expired | MissingToken => {
            (&[200, 403], 0, &["COLLECTED"], None)
        }
        Method => (&[200, 409], 0, &["COLLECTED"], None),
        MissingSession | Restart => (&[401], 0, &[], None),
        DuplicateLength => (&[400], 0, &[], None),
        AdminSpoof => (&[403], 0, &[], None),
        Tls | H2 | Collector => (&[422], 0, &[], None),
    }
}

/// Ordered identities and roles are part of the catalog, including controls.
/// Errors may retain a prefix; evaluated rows must retain the complete plan.
pub(super) fn step_plan(kind: Kind) -> &'static [(&'static str, &'static str)] {
    use Kind::*;
    match kind {
        MissingSession | AdminSpoof | Tls | H2 | Collector => &[("untrusted-request", "attack")],
        DuplicateLength => &[("ambiguous-framing", "attack")],
        Restart => &[("stale-epoch-request", "attack")],
        MissingReport => &[("protected-request", "attack")],
        ForgedPass => &[
            ("collect-asserted-fixture", "attack"),
            ("protected-request", "attack"),
        ],
        Baseline | LongWindow => &[
            ("collect-asserted-fixture", "setup"),
            ("protected-request", "synthetic_control"),
        ],
        Replay => &[
            ("collect-asserted-fixture", "setup"),
            ("protected-request", "synthetic_control"),
            ("replayed-request", "attack"),
        ],
        Selenium | Body | Target | HeaderOrder | Expired | MissingToken | Method => &[
            ("collect-asserted-fixture", "setup"),
            ("protected-request", "attack"),
        ],
    }
}

pub(super) fn matches(spec: &Spec, row: &NetworkRow) -> bool {
    let Some(kind) = spec.kind else {
        return false;
    };
    let Some(effects) = &row.effects else {
        return false;
    };
    matches_effects(kind, &row.steps, effects)
}

/// Exact statuses, origin delta, audit verdict sequence, rule and event types.
pub(super) fn matches_effects(kind: Kind, steps: &[Step], effects: &SnapshotDelta) -> bool {
    let (statuses, origin, verdicts, rule) = expected(kind);
    steps
        .iter()
        .map(|step| step.http_status)
        .eq(statuses.iter().copied().map(Some))
        && steps.iter().all(|step| step.error.is_none())
        && effects.origin_effects == origin
        && effects
            .audit
            .iter()
            .map(|event| event.verdict.as_str())
            .eq(verdicts.iter().copied())
        && rule.is_none_or(|rule| {
            effects
                .audit
                .iter()
                .any(|event| event.rules.iter().any(|actual| actual == rule))
        })
        && effects.audit.iter().all(|event| {
            event.event_type
                == if event.verdict == "COLLECTED" {
                    "runtime.collect"
                } else {
                    "runtime.decision"
                }
        })
}

fn fixture<T: Target>(
    target: &T,
    session: &T::Session,
    selenium: bool,
    long: bool,
    forged: bool,
) -> Result<ClientReport, LabError> {
    let raw = if selenium {
        include_str!("../../../sdk/humanymous-core/tests/scoring/fixtures/frozen/selenium-hr1.json")
    } else {
        include_str!(
            "../../../sdk/humanymous-core/tests/scoring/fixtures/frozen/human-chrome-allow.json"
        )
    };
    let value: Value = serde_json::from_str(raw).map_err(|_| LabError("frozen fixture JSON"))?;
    let mut report: ClientReport = serde_json::from_value(value["report"]["client"].clone())
        .map_err(|_| LabError("frozen client fixture"))?;
    target.bind_report(session, &mut report);
    if long {
        report.behavior = Default::default();
        report.behavior.duration_s = 60.0;
    }
    if forged {
        report.signals.get_or_insert_default().push(
            humanymous_core::signals::observation::Signal::new(
                "l7.pass.solved",
                json!(true),
                humanymous_core::signals::observation::Verdict::Ok,
                1.0,
                humanymous_core::signals::observation::Source::Wasm,
                "",
            ),
        );
    }
    Ok(report)
}

fn record(
    steps: &mut Vec<Step>,
    id: &str,
    role: &str,
    exchanged: Result<Response, LabError>,
) -> Result<(), LabError> {
    steps.push(Step {
        id: id.into(),
        role: role.into(),
        http_status: None,
        response_verdict: None,
        error: None,
    });
    let observed = steps.last_mut().ok_or(LabError("step ledger"))?;
    match exchanged {
        Ok(response) => {
            observed.http_status = Some(response.status);
            observed.response_verdict = response.verdict();
            Ok(())
        }
        Err(error) => {
            observed.error = Some(error.to_string());
            Err(error)
        }
    }
}

fn step<T: Target>(
    steps: &mut Vec<Step>,
    target: &mut T,
    id: &str,
    role: &str,
    request: &Request,
) -> Result<(), LabError> {
    let exchanged = target.exchange(request);
    record(steps, id, role, exchanged)
}

pub(super) fn execute_steps<T: Target>(
    target: &mut T,
    steps: &mut Vec<Step>,
    kind: Kind,
) -> Result<(), LabError> {
    use Kind::*;
    if matches!(kind, MissingSession | AdminSpoof | Tls | H2 | Collector) {
        let path = match kind {
            AdminSpoof => "/_humanymous/admin",
            Tls => "/tls",
            H2 => "/h2",
            Collector => "/collector",
            _ => "/effect",
        };
        let mut request = target.request(path, b"effect".to_vec());
        if matches!(kind, AdminSpoof) {
            request
                .headers
                .push(("X-Forwarded-For".into(), "127.0.0.1".into()));
        }
        return step(steps, target, "untrusted-request", "attack", &request);
    }
    if matches!(kind, DuplicateLength) {
        let request = target.request("/effect", Vec::new());
        let authority = request
            .header("host")
            .ok_or(LabError("fixture authority"))?;
        let raw = format!(
            "POST /effect HTTP/1.1\r\nHost: {authority}\r\nContent-Length: 0\r\nContent-Length: 0\r\n\r\n"
        );
        let exchanged = target.exchange_raw(raw.as_bytes());
        return record(steps, "ambiguous-framing", "attack", exchanged);
    }
    let session = target.issue()?;
    if matches!(kind, Restart) {
        let mut stale = target.request("/effect", b"effect".to_vec());
        target.sign(&mut stale, &session, 1)?;
        target.restart()?;
        return step(steps, target, "stale-epoch-request", "attack", &stale);
    }
    let mut counter = 1;
    if !matches!(kind, MissingReport) {
        let report = fixture(
            target,
            &session,
            matches!(kind, Selenium),
            matches!(kind, LongWindow),
            matches!(kind, ForgedPass),
        )?;
        let mut collect = target.request(
            "/_humanymous/collect",
            serde_json::to_vec(&report).map_err(|_| LabError("fixture serialization"))?,
        );
        target.sign(&mut collect, &session, counter)?;
        counter += 1;
        step(
            steps,
            target,
            "collect-asserted-fixture",
            if matches!(kind, ForgedPass) {
                "attack"
            } else {
                "setup"
            },
            &collect,
        )?;
    }
    let mut request = target.request("/effect", b"effect".to_vec());
    if matches!(kind, Expired) {
        target.sign_expired(&mut request, &session, counter)?;
    } else {
        target.sign(&mut request, &session, counter)?;
    }
    match kind {
        Body => request.body[0] = b'X',
        Kind::Target => request.target.push_str("?altered=1"),
        HeaderOrder => request.headers.swap(0, 1),
        MissingToken => request
            .headers
            .retain(|(name, _)| !name.eq_ignore_ascii_case("x-humanymous-token")),
        Method => request.method = "GET".into(),
        _ => {}
    }
    step(
        steps,
        target,
        "protected-request",
        if matches!(kind, Baseline | LongWindow | Replay) {
            "synthetic_control"
        } else {
            "attack"
        },
        &request,
    )?;
    if matches!(kind, Replay) {
        step(steps, target, "replayed-request", "attack", &request)?;
    }
    Ok(())
}

/// The supervisor-owned child as a case target; a restart spawns a fresh child
/// of the same installation and retains its process receipt.
struct Owned<'a, L: OwnedLab> {
    lab: &'a mut L,
    runtime: OwnedRuntime,
    receipts: Vec<ProcessReceipt>,
}
impl<L: OwnedLab> Target for Owned<'_, L> {
    type Session = Session;
    fn request(&self, target: &str, body: Vec<u8>) -> Request {
        self.runtime.request(target, body)
    }
    fn issue(&mut self) -> Result<Session, LabError> {
        self.runtime.issue()
    }
    fn bind_report(&self, session: &Session, report: &mut ClientReport) {
        session.bind_report(report);
    }
    fn sign(&self, request: &mut Request, session: &Session, counter: u64) -> Result<(), LabError> {
        self.runtime.sign(request, session, counter)
    }
    fn sign_expired(
        &self,
        request: &mut Request,
        session: &Session,
        counter: u64,
    ) -> Result<(), LabError> {
        self.runtime.sign_expired(request, session, counter)
    }
    fn exchange(&mut self, request: &Request) -> Result<Response, LabError> {
        self.runtime.exchange(request)
    }
    fn exchange_raw(&mut self, bytes: &[u8]) -> Result<Response, LabError> {
        self.runtime.exchange_raw(bytes)
    }
    fn restart(&mut self) -> Result<(), LabError> {
        self.runtime.stop()?;
        self.runtime = self.lab.spawn()?;
        self.receipts.push(self.runtime.receipt().clone());
        Ok(())
    }
}

pub(super) fn execute(lab: &mut impl OwnedLab, spec: Spec) -> NetworkRow {
    let mut row = NetworkRow {
        id: spec.id.into(),
        tier: spec.tier.into(),
        class: spec.class.into(),
        status: Status::Error,
        steps: Vec::new(),
        processes: Vec::new(),
        effects: None,
        expectation_matches: None,
        reason: None,
    };
    let Some(kind) = spec.kind else {
        row.status = Status::Unsupported;
        row.reason = spec.reason.map(str::to_owned);
        return row;
    };
    let before = match lab.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            row.reason = Some(error.to_string());
            return row;
        }
    };
    let execution = (|| {
        let runtime = lab.spawn()?;
        let mut owned = Owned {
            receipts: vec![runtime.receipt().clone()],
            lab: &mut *lab,
            runtime,
        };
        let result = execute_steps(&mut owned, &mut row.steps, kind);
        let stopped = owned.runtime.stop();
        row.processes.append(&mut owned.receipts);
        result.and(stopped)
    })();
    let after = lab.snapshot().and_then(|snapshot| snapshot.since(&before));
    match (execution, after) {
        (Ok(()), Ok(delta)) => {
            row.effects = Some(delta);
            row.status = Status::Evaluated;
            row.expectation_matches = Some(matches(&spec, &row));
        }
        (Err(error), Err(snapshot_error)) => {
            row.reason = Some(format!(
                "execution: {error}; audit snapshot: {snapshot_error}"
            ));
        }
        (Err(error), Ok(delta)) => {
            row.reason = Some(error.to_string());
            row.effects = Some(delta);
        }
        (Ok(()), Err(error)) => row.reason = Some(error.to_string()),
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_fixed_case_has_an_expectation_and_real_human_cases_are_unavailable() {
        let cases = catalog();
        let ids: std::collections::BTreeSet<_> = cases.iter().map(|case| case.id).collect();
        assert_eq!(cases.len(), 26);
        assert_eq!(ids.len(), 26);
        for case in &cases {
            if let Some(kind) = case.kind {
                assert!(!expected(kind).0.is_empty());
                assert_eq!(step_plan(kind).len(), expected(kind).0.len(), "{}", case.id);
                assert!(case.reason.is_none());
            } else {
                assert!(case.reason.is_some());
            }
        }
        for class in ["human", "accessibility", "ceiling"] {
            assert!(
                cases
                    .iter()
                    .filter(|case| case.class == class)
                    .all(|case| case.kind.is_none())
            );
        }
    }
}
