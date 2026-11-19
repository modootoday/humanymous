//! Key-gated `hmn_vt` verdict tokens and route presets on the served path
//! (owner decision #4, issue #75 comment 7017558784; legacy internal/gate
//! `verdictTokenGate`, `upgradeGate`, `hasValidToken`, `hasValidStepUp`,
//! `applyVerdict`'s attestation floor and the control plane's token issuance
//! at baseline 981d25c).
//!
//! With a token key installed, a scored ALLOW forwarded to the origin carries
//! `Set-Cookie: hmn_vt=…; Path=/; HttpOnly; Secure; SameSite=Lax`, minted for
//! the RIT session, the header fingerprint plus client subnet binding, the
//! current wall-clock epoch and a 30-minute expiry, with a `token.issued`
//! record appended before the effect. The verdict-token gate then runs
//! between the credential gates and the upgrade gate, and the upgrade gate
//! sees Go `hasValidToken`. Without a key nothing is issued and an upgrade on
//! an enforcing route stays denied (HR-26), as Go without a token key.
//!
//! With route presets installed (Go `-routes` file), each proxied path
//! resolves to a preset: its `enforce` flag feeds the gates and its
//! attestation floor demotes an unproven scored ALLOW to a step-up challenge
//! under `gate.attest_floor` enforce, or writes the would-challenge audit
//! under monitor or shadow.
//!
//! Kept from Go, pinned against `tests/fixtures/gateway/gate-modes-go-golden.json`:
//! the token bytes for a fixed key, session, binding, expiry and epoch; the
//! cookie attributes; the verdict-token gate records and fast path; the floor
//! records and the 401 step-up status.
//!
//! Deliberate differences, each pinned by tests:
//! - Go minted the token on the control plane's `/collect` scoring ALLOW;
//!   here the served path's scored ALLOW mints it, for the RIT session, and
//!   its record carries the request's tenant and route class.
//! - The key is configured (hex, at least 32 bytes) or absent; Go fell back to
//!   a per-boot random key. A key needs the trusted client address (PROXY v2)
//!   for the subnet binding; installing one without it is refused.
//! - A `monitor` or `off` preset makes the gates non-enforcing but never lets
//!   a scored DENY or CHALLENGE through: scoring enforcement is unchanged.
//! - The step-up answer is the runtime's JSON verdict body with status 401,
//!   not Go's interstitial page (no `/__hmn/loader.js` is served here).
//! - A cleared token's `Set-Cookie` and a minted token ride only on proxied
//!   complete responses; native-stream responses and local answers carry
//!   none.
//! - Records carry no subject pseudonym, as the other gates.
use super::Application;
use crate::audit::record::events::{EVENT_ENF_CHALLENGE_ISSUED, EVENT_TOKEN_ISSUED};
use crate::audit::record::{Actor, Record};
use crate::gateway::Rejection;
use crate::gateway::attest_floor::FloorMode;
use crate::gateway::ban::gate::Mode;
use crate::gateway::gates::{self, Route, Subject, Token};
use crate::gateway::http::Request;
use crate::gateway::route_policy::{RoutePolicy, RouteTable};
use crate::gateway::token::{self, Epochs, STEP_UP_COOKIE, TokenKey, VERDICT_COOKIE};

/// Verdict-token lifetime (Go `NewVerdictStore(30 * time.Minute)`).
pub const VERDICT_TTL_SECS: u64 = 30 * 60;

/// Installed token key and route presets.
#[derive(Default)]
pub(super) struct Verdicts {
    key: Option<(TokenKey, Epochs)>,
    routes: Option<RouteTable>,
}

/// The value of the first cookie named `name` (Go `r.Cookie`).
pub fn cookie<'a>(req: &'a Request, name: &str) -> Option<&'a str> {
    req.headers
        .iter()
        .filter(|(field, _)| field.eq_ignore_ascii_case("cookie"))
        .flat_map(|(_, value)| value.split(';'))
        .find_map(|pair| {
            let (field, value) = pair.trim().split_once('=')?;
            (field == name).then_some(value)
        })
}

/// `Set-Cookie` value carrying a minted verdict token (Go `http.SetCookie`
/// with `Path=/`, `HttpOnly`, `Secure`, `SameSite=Lax`).
pub fn verdict_cookie(token: &str) -> String {
    format!("{VERDICT_COOKIE}={token}; Path=/; HttpOnly; Secure; SameSite=Lax")
}

fn floor_mode(mode: Mode) -> FloorMode {
    match mode {
        Mode::Enforce => FloorMode::Enforce,
        Mode::Monitor => FloorMode::Monitor,
        Mode::Shadow => FloorMode::Shadow,
        Mode::Off => FloorMode::Off,
    }
}

/// What the served path does with a scored ALLOW.
pub(super) enum Allowed {
    /// Forward, adding these `Set-Cookie` values to the origin response.
    Forward(Vec<String>),
    /// Answer the step-up challenge (attestation floor enforced).
    StepUp,
}

impl Application {
    /// Mint and verify `hmn_vt` with `key` (Go `WithTokenKey`). Refused
    /// without the trusted client address the token binding needs.
    pub fn install_verdict_token_key(&mut self, key: TokenKey) -> Result<(), Rejection> {
        if self.edge_policy().is_none() {
            return Err(Rejection::new(503, "configuration"));
        }
        self.verdicts.key = Some((key, Epochs::default()));
        Ok(())
    }

    /// Resolve proxied paths through these presets (Go `Config.Routes`). A
    /// preset with the attestation floor on the catch-all prefix is refused.
    pub fn install_route_presets(&mut self, routes: RouteTable) -> Result<(), Rejection> {
        crate::gateway::route_policy::refuse_attested_catch_all(&routes.routes)
            .map_err(|_| Rejection::new(503, "configuration"))?;
        self.verdicts.routes = Some(routes);
        Ok(())
    }

    /// The preset for `path`; without presets every route enforces with no
    /// floor, as before presets were installed.
    pub(super) fn route_preset(&self, path: &str) -> Option<RoutePolicy> {
        self.verdicts
            .routes
            .as_ref()
            .map(|routes| routes.resolve(path))
    }

    /// The gate route facts for `path`.
    pub(super) fn gate_route(&self, path: &str) -> (bool, bool, &'static str) {
        self.route_preset(path).map_or((true, false, ""), |preset| {
            (preset.enforce, preset.attest_floor, preset.name)
        })
    }

    fn binding(&self, req: &Request) -> Option<String> {
        let client = self.client?;
        let key = token::bind_key(
            req.header("user-agent").unwrap_or(""),
            req.header("accept-language").unwrap_or(""),
            req.header("sec-ch-ua").unwrap_or(""),
        );
        Some(token::token_bind(
            key.as_deref(),
            &token::client_subnet(client),
        ))
    }

    /// Go `hasValidToken`, ignoring the gate's mode.
    pub(super) fn token_state(&self, req: &Request, sid: &str, now_s: u64) -> Option<Token> {
        let (key, epochs) = self.verdicts.key.as_ref()?;
        let Some(value) = cookie(req, VERDICT_COOKIE).filter(|value| !value.is_empty()) else {
            return Some(Token::Absent);
        };
        let Some(bind) = self.binding(req) else {
            return Some(Token::Invalid(token::Reason::Malformed));
        };
        Some(
            match key.verify_verdict(value, &bind, sid, now_s, &epochs.accepted(now_s)) {
                Ok(()) => Token::Valid,
                Err(reason) => Token::Invalid(reason),
            },
        )
    }

    /// Go `hasValidStepUp`.
    pub(super) fn step_up(&self, req: &Request, sid: &str, now_s: u64) -> bool {
        let (Some((key, epochs)), Some(bind)) = (self.verdicts.key.as_ref(), self.binding(req))
        else {
            return false;
        };
        cookie(req, STEP_UP_COOKIE)
            .filter(|value| !value.is_empty())
            .is_some_and(|value| {
                key.verify_step_up(value, &bind, sid, now_s, &epochs.accepted(now_s))
                    .is_ok()
            })
    }

    /// The verdict-token gate, when a key is installed.
    pub(super) fn verdict_token_gate(
        &self,
        req: &Request,
        sid: &str,
        now_s: u64,
        subject: &Subject<'_>,
        route: &Route<'_>,
    ) -> Option<gates::Outcome> {
        let state = self.token_state(req, sid, now_s)?;
        let floor = gates::Floor {
            mode: floor_mode(self.gate_mode(super::Gate::AttestFloor)),
            step_up: self.step_up(req, sid, now_s),
            audit: gates::FloorAudit {
                actor_pseudonym: "",
                session_pseudonym: "",
                host: req.header("host").unwrap_or(""),
                config_version: &self.config.scope.revision,
            },
        };
        Some(gates::verdict_token(
            subject,
            route,
            self.gate_mode(super::Gate::VerdictToken),
            || state,
            &floor,
        ))
    }

    /// Legacy `applyVerdict` for a scored ALLOW on a proxied route: the
    /// attestation floor, then token issuance. Records are appended before
    /// the effect.
    pub(super) fn allowed(
        &mut self,
        req: &Request,
        sid: &str,
        now_s: u64,
        route_class: &str,
    ) -> Result<Allowed, Rejection> {
        let path = req.target.split('?').next().unwrap_or("");
        let (enforcing, attest_floor, _) = self.gate_route(path);
        let tenant = self.config.scope.tenant.clone();
        if enforcing && attest_floor && !self.step_up(req, sid, now_s) {
            let mode = floor_mode(self.gate_mode(super::Gate::AttestFloor));
            let audit = gates::FloorAudit {
                actor_pseudonym: "",
                session_pseudonym: "",
                host: req.header("host").unwrap_or(""),
                config_version: &self.config.scope.revision,
            };
            match mode {
                FloorMode::Enforce => {
                    let record = Record {
                        event_type: EVENT_ENF_CHALLENGE_ISSUED.into(),
                        actor: Actor {
                            kind: "subject".into(),
                            id_pseudonym: String::new(),
                        },
                        tenant_id: tenant,
                        host: audit.host.into(),
                        route_class: route_class.into(),
                        verdict: "ALLOW".into(),
                        enforcement_action: "challenge_pow".into(),
                        enforcement_mode: "enforce".into(),
                        key_id: "k1".into(),
                        config_version: audit.config_version.into(),
                        ..Default::default()
                    };
                    self.append_all(vec![record])?;
                    return Ok(Allowed::StepUp);
                }
                FloorMode::Monitor | FloorMode::Shadow => {
                    let record = gates::attestation_floor_audit(
                        &tenant,
                        route_class,
                        mode,
                        "scoring allow would require step-up",
                        &audit,
                    );
                    self.append_all(vec![record])?;
                }
                FloorMode::Off => {}
            }
        }
        let issued = match (self.verdicts.key.as_ref(), self.binding(req)) {
            (Some((key, epochs)), Some(bind)) => key
                .issue_verdict(sid, &bind, &epochs.current(now_s), now_s + VERDICT_TTL_SECS)
                .ok(),
            _ => None,
        };
        let Some(minted) = issued else {
            return Ok(Allowed::Forward(Vec::new()));
        };
        let record = Record {
            event_type: EVENT_TOKEN_ISSUED.into(),
            actor: Actor {
                kind: "subject".into(),
                id_pseudonym: String::new(),
            },
            tenant_id: self.config.scope.tenant.clone(),
            route_class: route_class.into(),
            verdict: "ALLOW".into(),
            enforcement_mode: "enforce".into(),
            key_id: "k1".into(),
            ..Default::default()
        };
        self.append_all(vec![record])?;
        Ok(Allowed::Forward(vec![verdict_cookie(&minted)]))
    }
}
