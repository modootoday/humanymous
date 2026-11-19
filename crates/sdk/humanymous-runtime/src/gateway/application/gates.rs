//! Credential, upgrade and sweep gates on the served path (legacy
//! internal/gate gate.go `ServeHTTP` after the spoof gate: `agentAuthGate`,
//! `patGate`, `webauthnGate`, `upgradeGate` and `sweepGate` at baseline
//! 981d25c). The decisions are `gateway::gates` over the existing credential,
//! fan-out and sweep kernels; this module only holds the installed
//! capabilities, appends the records durably before acting and maps the
//! effect onto a [`Decision`].
//!
//! Every capability is opt-in, as in Go where each was nil by default: Web
//! Bot Auth needs an installed key directory, PAT installed issuers, WebAuthn
//! an installed registry and fan-out tracker, and the sweep gate an installed
//! detector with its `gate.recon_sweep` mode. The service binary installs
//! none of them; provisioning and default modes are owner questions.
//!
//! Kept from Go: the gate order (agent auth, PAT, WebAuthn, upgrade, sweep,
//! then the verdict), the records, the 403 deny with the incident pointer
//! and the trusted fast path that forwards without behavioural scoring.
//!
//! Deliberate differences, each pinned by `gateway::tests::gates`:
//! - The gates run after session authorization (request integrity) and
//!   session admission, immediately before scoring, not before them: the
//!   served path has no request that skips request integrity, and a
//!   credential is a trust upgrade over scoring only. The session is the
//!   signed RIT session, not the legacy `hsid` cookie.
//! - Every inventoried route enforces and none carries the attestation
//!   floor: route presets and the kill switch are not on the served path.
//! - The verdict-token gate is not composed: nothing on the served path
//!   issues `hmn_vt` (the gate control plane that minted it is not composed),
//!   so the upgrade gate's `hasValidToken` is always false, as in Go without
//!   a token key.
//! - Records carry no subject pseudonym, as the ban and spoof gates. A trust
//!   upgrade's record also carries the request id and commitment of the
//!   ALLOW it authorizes, so an `upstream.error` record links to it.
//! - A deny after authorization carries the counter acknowledgement fields,
//!   as every other response to an authorized request does.
//! - WebAuthn needs the trusted client address (PROXY v2) for its fan-out
//!   subnet; installing it without one is refused. The sweep detector and
//!   the fan-out tracker run on a monotonic epoch; an enforcing sweep
//!   detector that cannot count answers 503 `sweep_unavailable`.
//! - The sweep binding is the header fingerprint (`token::bind_key`); a
//!   request without a User-Agent is not counted, as in Go.
use super::{Application, Decision, GateResponse};
use crate::audit::record::Record;
use crate::gateway::Rejection;
use crate::gateway::attest_floor::fanout::{self, CredentialFanout};
use crate::gateway::ban::gate::Mode;
use crate::gateway::credential::{privacy_pass, web_bot_auth, webauthn};
use crate::gateway::gates::{self, Effect, Outcome, Route, Subject};
use crate::gateway::http::Request;
use crate::gateway::token::{self, Nonces};
use crate::network::history::sweep::{Detector, Sweep};
use std::time::{Duration, Instant};

/// Go `NewNonceCache(time.Hour)` for Web Bot Auth signature nonces.
pub const AGENT_NONCE_TTL: Duration = Duration::from_secs(60 * 60);

/// Installed gate capabilities; all absent by default.
#[derive(Default)]
pub(super) struct Installed {
    agent: Option<(web_bot_auth::KeyDirectory, Nonces)>,
    pat: Option<privacy_pass::Verifier>,
    webauthn: Option<(webauthn::Registry, CredentialFanout)>,
    sweep: Option<(Detector, Mode)>,
    /// Monotonic origin of the sweep detector and fan-out tracker clocks.
    epoch: Option<Instant>,
}

impl Installed {
    fn epoch(&mut self) -> Instant {
        *self.epoch.get_or_insert_with(Instant::now)
    }

    /// Sweep the detector and the fan-out tracker (legacy `Server.GC`).
    pub(super) fn gc(&mut self) -> Result<(), Rejection> {
        let Some(epoch) = self.epoch else {
            return Ok(());
        };
        let now = epoch.elapsed();
        let refused = Rejection::new(503, "sweep_unavailable");
        if let Some((detector, _)) = self.sweep.as_mut() {
            detector.gc(now).map_err(|_| refused)?;
        }
        if let Some((_, fanout)) = self.webauthn.as_mut() {
            fanout.gc(now).map_err(|_| refused)?;
        }
        Ok(())
    }
}

/// What the served path does after the gates.
pub(super) enum Gated {
    /// Score the request.
    Continue,
    /// Answer with this decision.
    Answer(Decision),
    /// Trusted fast path: forward without scoring; the request linkage must
    /// be stamped on the trust-upgrade record (see `Application::run_gates`).
    Forward,
}

impl Application {
    /// Verify Web Bot Auth signatures against `directory` (Go `AgentKeys`).
    pub fn install_agent_keys(&mut self, directory: web_bot_auth::KeyDirectory) {
        let nonces = Nonces::new(AGENT_NONCE_TTL, token::MAX_NONCES);
        self.installed.agent = Some((directory, nonces));
    }

    /// Verify Private Access Tokens from these issuers (Go `PATIssuers`).
    pub fn install_pat_issuers(&mut self, verifier: privacy_pass::Verifier) {
        self.installed.pat = Some(verifier);
    }

    /// Verify WebAuthn assertions (Go `WebAuthnCreds`) with the credential
    /// fan-out cap. Refused unless the client address comes from the trusted
    /// edge, which the fan-out subnet needs.
    pub fn install_webauthn(
        &mut self,
        registry: webauthn::Registry,
        fanout: CredentialFanout,
    ) -> Result<(), Rejection> {
        if self.edge_policy().is_none() {
            return Err(Rejection::new(503, "configuration"));
        }
        self.installed.epoch();
        self.installed.webauthn = Some((registry, fanout));
        Ok(())
    }

    /// Run the decision-probing sweep gate (HR-30) in `mode`.
    pub fn install_sweep(&mut self, detector: Detector, mode: Mode) {
        self.installed.epoch();
        self.installed.sweep = Some((detector, mode));
    }

    /// Set the recovered client address the edge would supply (tests).
    #[cfg(test)]
    pub(in crate::gateway) fn set_client(&mut self, client: Option<std::net::IpAddr>) {
        self.client = client;
    }

    fn append_all(&mut self, records: Vec<Record>) -> Result<(), Rejection> {
        for record in records {
            if self.audit.append(record).is_err() {
                self.sessions.poison();
                return Err(Rejection::new(503, "durable_audit"));
            }
        }
        Ok(())
    }

    /// Run one gate outcome, if the gate ran: append its records, then
    /// report whether it ended the chain. `linkage` is stamped on a trust upgrade's records.
    fn settle(
        &mut self,
        outcome: Option<Outcome>,
        linkage: (&str, &str),
    ) -> Result<Option<Gated>, Rejection> {
        let Some(mut outcome) = outcome else {
            return Ok(None);
        };
        if outcome.effect == Effect::Forward {
            for record in &mut outcome.records {
                record.request_id = linkage.0.into();
                record.correlation_id = linkage.1.into();
            }
        }
        self.append_all(outcome.records)?;
        Ok(match outcome.effect {
            Effect::Continue => None,
            Effect::Forward => Some(Gated::Forward),
            Effect::Deny => Some(Gated::Answer(Decision::Gate(GateResponse {
                status: gates::DENY_STATUS,
                fields: vec![(gates::INCIDENT_HEADER.0, gates::INCIDENT_HEADER.1.into())],
                body: gates::DENY_BODY,
            }))),
        })
    }

    /// The gates of Go `ServeHTTP` between route resolution and the verdict,
    /// for one authorized proxied request of session `sid` at wall clock
    /// `now_ms`. `linkage` is the request id and commitment of the decision.
    pub(super) fn run_gates(
        &mut self,
        req: &Request,
        sid: &str,
        now_ms: u64,
        linkage: (&str, &str),
    ) -> Result<Gated, Rejection> {
        let tenant = self.config.scope.tenant.clone();
        let class = crate::gateway::request::route_class(&req.headers);
        let subject = Subject {
            tenant: &tenant,
            pseudonym: "",
            route_class: class.as_str(),
        };
        let route = Route {
            enforcing: true,
            attest_floor: false,
            name: "",
        };
        let wall = Duration::from_millis(now_ms);
        let path = req.target.split('?').next().unwrap_or("");
        let agent = self.installed.agent.as_mut().map(|(directory, nonces)| {
            let presented = web_bot_auth::Request {
                authority: req.header("host").unwrap_or(""),
                method: &req.method,
                path,
                signature_input: req.header("signature-input"),
                signature: req.header("signature"),
            };
            let verdict = web_bot_auth::verify(&presented, Some(directory), nonces, wall);
            gates::agent_auth(&subject, &route, &verdict)
        });
        if let Some(gated) = self.settle(agent, linkage)? {
            return Ok(gated);
        }
        let pat = self.installed.pat.as_mut().map(|verifier| {
            let header = req.header("authorization");
            gates::pat(&subject, &route, || verifier.verify(header, wall))
        });
        if let Some(gated) = self.settle(pat, linkage)? {
            return Ok(gated);
        }
        let monotonic = self.installed.epoch.map(|epoch| epoch.elapsed());
        let subnet = self.client.map(token::client_subnet);
        let webauthn = match (self.installed.webauthn.as_mut(), monotonic) {
            (Some((registry, tracker)), Some(now)) => Some(gates::webauthn(
                &subject,
                &route,
                || registry.verify(req.header("webauthn-assertion")),
                |credential| match &subnet {
                    Some(subnet) => tracker.observe(credential, subnet, sid, false, now),
                    // No attributed client: no fan-out count, no fast path.
                    None => fanout::Decision::Unavailable,
                },
            )),
            _ => None,
        };
        if let Some(gated) = self.settle(webauthn, linkage)? {
            return Ok(gated);
        }
        let upgrade = Some(gates::upgrade(&subject, &route, false));
        if let Some(gated) = self.settle(upgrade, linkage)? {
            return Ok(gated);
        }
        let binding = token::bind_key(
            req.header("user-agent").unwrap_or(""),
            req.header("accept-language").unwrap_or(""),
            req.header("sec-ch-ua").unwrap_or(""),
        )
        .unwrap_or_default();
        let sweep = match (self.installed.sweep.as_mut(), monotonic) {
            (Some((detector, mode)), Some(now)) => Some(
                gates::sweep(&subject, &route, *mode, || {
                    detector
                        .observe(&binding, sid, now)
                        .map(|counted| matches!(counted, Sweep::Counted { sweeping: true, .. }))
                })
                .map_err(|_| Rejection::new(503, "sweep_unavailable"))?,
            ),
            _ => None,
        };
        if let Some(gated) = self.settle(sweep, linkage)? {
            return Ok(gated);
        }
        Ok(Gated::Continue)
    }
}
