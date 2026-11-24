//! Pass on the gateway for the attestation-floor step-up proof (owner
//! decision #4, option 1; owner choice 2026-11-24 to mount Pass on the
//! gateway). Legacy reference: cmd/server pass_handler.go (the Pass API) and
//! internal/gate control.go `handleStepUp` with gate.go `hasValidStepUp` at
//! baseline 981d25c.
//!
//! Go ran Pass on the Core, which handed the client a session-bound receipt;
//! the gate exchanged it for a fingerprint-bound `hmn_su` cookie. The
//! runtime's browser profile is cookie-free and its session is the RIT
//! session, so the ported `gateway::pass_flow` is mounted here instead:
//! `/_humanymous/pass/{new,pow,attest,solve}` are reserved routes,
//! authenticated by request integrity, and the RIT session id is the Pass
//! session. A solve that marks the session solved records a step-up proof in
//! the server-side sticky store, bound to Go's `tokenBind` (header
//! fingerprint and client subnet) for `step_up_ttl` (Go: 30 minutes). No
//! receipt, token or cookie is issued.
//!
//! Kept from the ported flow: puzzle, proof-of-work and attestation
//! preflight, velocity limits, the duplicate-trace refusal and
//! [`MotorEvidence::Off`] only (accessibility hard rule, owner decision #9).
//!
//! Deliberate differences:
//! - The page (`/pass`) and the KPI view are not served here; a client drives
//!   the API with RIT-signed requests.
//! - The flow's score is computed over the gateway session's collected client
//!   report; it is returned as the flow returns it and never changes the
//!   served path's verdicts (detection freeze).
//! - Responses are written as local answers (the flow's `Content-Type` is not
//!   carried).
use super::{Application, Decision};
use crate::gateway::Rejection;
use crate::gateway::control_plane::Now;
use crate::gateway::http::Request;
use crate::gateway::observatory::Body;
use crate::gateway::pass_flow::{self, Backend, MotorEvidence, PassFlow};
use humanymous_core::signals::report::{ClientReport, ScoreResult, SessionReport, Timestamp};
use std::collections::BTreeMap;
use std::time::Duration;

/// Reserved prefix of the gateway Pass API.
pub const PREFIX: &str = "/_humanymous/pass/";

/// Pass-scored session reports kept at once.
const MAX_REPORTS: usize = 4096;
/// A kept report older than this may be evicted under pressure.
const REPORT_STALE: Duration = Duration::from_secs(3600);

/// The mounted flow, its Pass-scored reports and the step-up proof lifetime.
pub(super) struct Mounted {
    flow: PassFlow,
    /// Network-side signals the flow folded (velocity, solved), per session.
    reports: BTreeMap<String, (SessionReport, Duration)>,
    step_up_ttl: Duration,
}

impl Mounted {
    /// A flow with a per-process master key (as Go's Core) and
    /// [`MotorEvidence::Off`].
    pub(super) fn new(step_up_ttl: Duration) -> Result<Self, Rejection> {
        let refused = Rejection::new(503, "configuration");
        let mut master_key = vec![0_u8; 32];
        getrandom::fill(&mut master_key).map_err(|_| refused)?;
        let flow = pass_flow::mount(pass_flow::Config {
            master_key,
            web_dir: std::path::PathBuf::new(),
            step_up: None,
            motor_evidence: MotorEvidence::Off,
        })
        .map_err(|_| refused)?;
        Ok(Self {
            flow,
            reports: BTreeMap::new(),
            step_up_ttl,
        })
    }
}

/// Flow path for a gateway Pass route.
pub(super) fn flow_path(path: &str) -> Option<&'static str> {
    Some(match path.strip_prefix(PREFIX)? {
        "new" => pass_flow::NEW_PATH,
        "pow" => pass_flow::POW_PATH,
        "attest" => pass_flow::ATTEST_PATH,
        "solve" => pass_flow::SOLVE_PATH,
        _ => return None,
    })
}

/// [`Backend`] over the gateway session's collected client report, the
/// Pass-scored reports and the frozen engine. The reports live only here:
/// they never reach the served path's scoring.
struct GatewayBackend<'a> {
    engine: &'a humanymous_core::scoring::engine::Engine,
    client: ClientReport,
    reports: &'a mut BTreeMap<String, (SessionReport, Duration)>,
    mono: Duration,
}

impl Backend for GatewayBackend<'_> {
    fn ensure(&mut self, _session: &str, _now: Timestamp) {}
    fn report(&self, session: &str) -> Option<SessionReport> {
        let kept = self
            .reports
            .get(session)
            .map(|(report, _)| report.clone())
            .unwrap_or_default();
        Some(SessionReport {
            client: self.client.clone(),
            ..kept
        })
    }
    fn store_scored(&mut self, session: &str, report: SessionReport, _now: Timestamp) {
        if !self.reports.contains_key(session) && self.reports.len() >= MAX_REPORTS {
            let now = self.mono;
            self.reports
                .retain(|_, (_, seen)| now.saturating_sub(*seen) <= REPORT_STALE);
            if self.reports.len() >= MAX_REPORTS {
                return;
            }
        }
        self.reports.insert(session.to_owned(), (report, self.mono));
    }
    fn score(&mut self, report: &mut SessionReport) -> Option<ScoreResult> {
        self.engine.score(report).ok()
    }
}

fn timestamp(now_ms: u64) -> Timestamp {
    let offset = time::Duration::milliseconds(i64::try_from(now_ms).unwrap_or(i64::MAX));
    Timestamp(
        time::OffsetDateTime::UNIX_EPOCH
            .checked_add(offset)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH),
    )
}

impl Application {
    /// Whether the Pass API is mounted.
    pub(super) fn pass_mounted(&self) -> bool {
        self.pass.is_some()
    }

    /// Answer one RIT-authorized Pass request of session `sid`.
    pub(super) fn serve_pass(
        &mut self,
        req: &Request,
        flow_path: &'static str,
        sid: &str,
        now_ms: u64,
    ) -> Result<Decision, Rejection> {
        let monotonic = self.monotonic();
        let binding = self.client_binding(req);
        let client = self.sessions.report(sid);
        let Some(mounted) = self.pass.as_mut() else {
            return Err(Rejection::new(403, "reserved_route"));
        };
        let cookie = format!("{}=\"{}\"", pass_flow::SESSION_COOKIE, sid);
        let red_team = req.header("x-hm-redteam");
        let request = pass_flow::Request {
            method: &req.method,
            path: flow_path,
            cookie: Some(&cookie),
            red_team,
            fingerprint: &binding,
            body: &req.body,
        };
        let now = Now {
            wall: timestamp(now_ms),
            mono: monotonic,
        };
        let Mounted {
            flow,
            reports,
            step_up_ttl,
        } = mounted;
        let mut backend = GatewayBackend {
            engine: &self.engine,
            client,
            reports,
            mono: monotonic,
        };
        let response = flow
            .handle(&mut backend, &request, now)
            .ok_or(Rejection::new(403, "reserved_route"))?;
        let solved = flow_path == pass_flow::SOLVE_PATH
            && response.status == 200
            && flow
                .store()
                .with_session(sid, monotonic, |state| state.solved)
                .unwrap_or(false);
        let ttl = *step_up_ttl;
        if solved {
            // Go `handleStepUp` audited the minted proof before setting it.
            let record = crate::audit::record::Record {
                event_type: "token.issued".into(),
                actor: crate::audit::record::Actor {
                    kind: "subject".into(),
                    ..Default::default()
                },
                tenant_id: self.config.scope.tenant.clone(),
                route_class: "control".into(),
                verdict: "ALLOW".into(),
                enforcement_mode: "enforce".into(),
                fail_reason: "step-up proof minted (Pass solved)".into(),
                key_id: "k1".into(),
                ..Default::default()
            };
            self.append_gate_record(record)?;
            self.remember_step_up(sid, &binding, ttl);
        }
        let Body::Bytes(body) = response.body else {
            return Err(Rejection::new(503, "pass_unavailable"));
        };
        Ok(Decision::Local(response.status, body))
    }
}
