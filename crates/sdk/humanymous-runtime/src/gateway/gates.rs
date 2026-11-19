//! Edge-path gates after the spoof-header gate (legacy internal/gate gates.go
//! `agentAuthGate`, `patGate`, `webauthnGate`, `trustUpgrade`,
//! `verdictTokenGate`, `upgradeGate`, `sweepGate` and
//! `auditAttestationFloor` at baseline 981d25c; PLAN-08 R2/R3, SoT-21 §3,
//! §5 and §8, HR-26, HR-28 and HR-30).
//!
//! This is a pure decision kernel. Credential verification, token checks,
//! the credential fan-out registry and the sweep detector are the existing
//! kernels (`credential`, `token`, `attest_floor`, `network::history::sweep`);
//! the caller passes them in as closures so each one runs exactly when Go ran
//! it (for example a Private Access Token is spent only on an enforcing route,
//! and the sweep detector counts nothing on a route the gate skips). Each
//! function returns the records in emission order and the effect; the caller
//! appends every record durably before it acts. Go's `ServeHTTP` order is
//! agent auth, PAT, WebAuthn, verdict token, upgrade, sweep, then the verdict.
//!
//! Kept from Go, pinned against `tests/fixtures/gateway/gates-go-golden.json`:
//! - Event types, actions, modes, rules, `key_id` `k1`, fail reasons and the
//!   route name as `config_version` of a verdict-token fast path.
//! - A deny is 403 with `X-Hmn-Incident: see-audit-log` and the static deny
//!   body; a trust upgrade forwards with no behavioural scoring.
//! - An invalid verdict token never denies: enforcing, it clears the cookie
//!   (`CLEAR_VERDICT_COOKIE`) and re-scores; monitor and shadow only record.
//! - A Web Bot Auth forgery denies only on an enforcing route; a trusted
//!   signature trust-upgrades only there; a neutral signature is recorded and
//!   continues. PAT and WebAuthn never deny.
//! - The attestation floor declines the verdict-token fast path on an
//!   attested route without a step-up proof (enforce) or records
//!   `would_challenge` and forwards (monitor, shadow).
//!
//! Deliberate differences, each pinned by tests:
//! - Records use the Runtime verdict vocabulary `ALLOW`/`DENY` (Go wrote
//!   `allow`/`deny`), as the ban and spoof gates do.
//! - A neutral Web Bot Auth record names the keyid only when it is on the
//!   allowlist, and a request carrying only one of the two signature fields
//!   is neutral (recorded) where Go ignored it: the credential kernel never
//!   echoes unmatched attacker bytes and refuses half signatures.
//! - A fan-out registry that cannot admit an observation withholds the
//!   WebAuthn trust upgrade on every route without a record
//!   (`attest_floor::fanout`).
//! - An enforcing sweep gate whose detector cannot count (budget, clock or an
//!   empty session) is an error the caller refuses (fail closed); monitor and
//!   shadow continue. Go's unbounded detector could not fail.
use crate::audit::record::events::{
    EVENT_AGENT_FORGED, EVENT_AGENT_UNKNOWN, EVENT_AGENT_VERIFIED, EVENT_ENF_ALLOW,
    EVENT_ENF_CHALLENGE_ISSUED, EVENT_PAT_VERIFIED, EVENT_RECON_PROBING,
    EVENT_TOKEN_BINDING_MISMATCH, EVENT_UPGRADE_NO_VERDICT, EVENT_WEBAUTHN_VERIFIED,
};
use crate::audit::record::{Actor, Record};
use crate::gateway::attest_floor::{self, FloorMode, fanout};
use crate::gateway::ban::gate::Mode;
use crate::gateway::credential::{privacy_pass, web_bot_auth, webauthn};
use crate::gateway::token::Reason;
use crate::network::history::Unavailable;

/// HTTP status of a gate deny (legacy `deny`).
pub const DENY_STATUS: u16 = 403;
/// Incident pointer set on every gate deny.
pub use crate::gateway::guard::INCIDENT_HEADER;
/// Static body of a gate deny (legacy `deny`).
pub use crate::gateway::guard::SPOOF_DENY_BODY as DENY_BODY;
/// `Set-Cookie` value that clears a verdict token that did not verify, as Go's
/// `http.SetCookie` rendered `MaxAge: -1`.
pub const CLEAR_VERDICT_COOKIE: &str = "hmn_vt=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Lax";

/// Request context copied into records; no raw address or header bytes.
#[derive(Debug, Clone, Copy)]
pub struct Subject<'a> {
    /// Tenant (Go `NodeID`).
    pub tenant: &'a str,
    /// Subject pseudonym of the session (Go `pseudonym(sid, sid)`); empty
    /// when none is available.
    pub pseudonym: &'a str,
    /// Coarse route class (`request::route_class`).
    pub route_class: &'a str,
}

/// The matched route's enforcement facts (legacy `routePolicy`).
#[derive(Debug, Clone, Copy)]
pub struct Route<'a> {
    /// Effective enforcement: the route enforces and no global monitor is on.
    pub enforcing: bool,
    /// The route carries the attestation floor.
    pub attest_floor: bool,
    /// Preset name, recorded as the fast path's `config_version`.
    pub name: &'a str,
}

/// What the caller does after a gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Run the next gate (and finally the verdict).
    Continue,
    /// Answer 403 with the incident pointer; the origin is never contacted.
    Deny,
    /// Trusted fast path: forward to the origin without behavioural scoring.
    Forward,
}

/// Records to append, in order, before the effect.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// Audit records in emission order.
    pub records: Vec<Record>,
    /// What to do with the request.
    pub effect: Effect,
    /// Add `Set-Cookie: CLEAR_VERDICT_COOKIE` to the eventual response.
    pub clear_verdict_cookie: bool,
}

impl Outcome {
    fn new(records: Vec<Record>, effect: Effect) -> Self {
        Self {
            records,
            effect,
            clear_verdict_cookie: false,
        }
    }

    fn proceed() -> Self {
        Self::new(Vec::new(), Effect::Continue)
    }
}

/// Result of checking the presented verdict token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    /// No cookie, or an empty value.
    Absent,
    /// Present but not valid for this session and client binding.
    Invalid(Reason),
    /// Valid for this session and client binding.
    Valid,
}

/// Fields of the attestation-floor audit record (legacy
/// `auditAttestationFloor`) the subject does not carry.
#[derive(Debug, Clone, Copy)]
pub struct FloorAudit<'a> {
    /// Actor pseudonym (Go `pseudonym(sid, clientIP)`).
    pub actor_pseudonym: &'a str,
    /// Session pseudonym (Go `pseudonym(sid, sid)`).
    pub session_pseudonym: &'a str,
    /// Request authority.
    pub host: &'a str,
    /// Effective configuration version.
    pub config_version: &'a str,
}

/// Attestation-floor inputs of the verdict-token gate.
#[derive(Debug, Clone, Copy)]
pub struct Floor<'a> {
    /// `gate.attest_floor` mode.
    pub mode: FloorMode,
    /// The request carries a valid step-up proof for this session.
    pub step_up: bool,
    /// Audit fields for a monitor or shadow note.
    pub audit: FloorAudit<'a>,
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Shadow => "shadow",
        Mode::Enforce => "enforce",
        Mode::Off | Mode::Monitor => "monitor",
    }
}

fn floor_mode_name(mode: FloorMode) -> &'static str {
    match mode {
        FloorMode::Shadow => "shadow",
        FloorMode::Enforce => "enforce",
        FloorMode::Off | FloorMode::Monitor => "monitor",
    }
}

/// Go `tokenReason` spelling.
pub fn reason_name(reason: Reason) -> &'static str {
    match reason {
        Reason::Malformed => "malformed",
        Reason::BadSignature => "bad_sig",
        Reason::Expired => "expired",
        Reason::BindingMismatch => "binding_mismatch",
    }
}

fn record(subject: &Subject<'_>, event: &str, mode: &str) -> Record {
    Record {
        event_type: event.into(),
        actor: Actor {
            kind: "subject".into(),
            id_pseudonym: subject.pseudonym.into(),
        },
        tenant_id: subject.tenant.into(),
        route_class: subject.route_class.into(),
        enforcement_mode: mode.into(),
        key_id: "k1".into(),
        ..Default::default()
    }
}

fn deny_record(subject: &Subject<'_>, event: &str, rule: &str, reason: String) -> Record {
    Record {
        verdict: "DENY".into(),
        triggered_rules: Some(vec![rule.into()]),
        enforcement_action: "block".into(),
        fail_reason: reason,
        ..record(subject, event, "enforce")
    }
}

/// Legacy `trustUpgrade`: the audited ALLOW fast path shared by the
/// credential gates.
fn trust_upgrade(subject: &Subject<'_>, event: &str, rule: &str, reason: String) -> Outcome {
    let rec = Record {
        verdict: "ALLOW".into(),
        triggered_rules: Some(vec![rule.into()]),
        enforcement_action: "pass".into(),
        fail_reason: reason,
        ..record(subject, event, "enforce")
    };
    Outcome::new(vec![rec], Effect::Forward)
}

/// Legacy `agentAuthGate`, run only when an agent key directory is installed.
/// `verdict` is the Web Bot Auth verification of this request.
pub fn agent_auth(
    subject: &Subject<'_>,
    route: &Route<'_>,
    verdict: &web_bot_auth::Verdict,
) -> Outcome {
    use web_bot_auth::Verdict;
    match verdict {
        Verdict::Absent => Outcome::proceed(),
        Verdict::Forged { keyid } if !route.enforcing => {
            let rec = Record {
                fail_reason: format!(
                    "forged Web Bot Auth signature for allowlisted keyid {keyid} (monitor: not blocked)"
                ),
                ..record(subject, EVENT_AGENT_FORGED, "monitor")
            };
            Outcome::new(vec![rec], Effect::Continue)
        }
        Verdict::Forged { keyid } => Outcome::new(
            vec![deny_record(
                subject,
                EVENT_AGENT_FORGED,
                "web-bot-auth",
                format!("forged Web Bot Auth signature for allowlisted keyid {keyid}"),
            )],
            Effect::Deny,
        ),
        Verdict::Trusted { .. } if !route.enforcing => Outcome::proceed(),
        Verdict::Trusted { keyid } => trust_upgrade(
            subject,
            EVENT_AGENT_VERIFIED,
            "web-bot-auth",
            format!("verified Web Bot Auth agent {keyid}"),
        ),
        Verdict::Neutral { keyid, .. } => {
            let rec = Record {
                fail_reason: format!(
                    "unknown/unsupported agent signature keyid {}",
                    keyid.as_deref().unwrap_or("")
                ),
                ..record(subject, EVENT_AGENT_UNKNOWN, "enforce")
            };
            Outcome::new(vec![rec], Effect::Continue)
        }
    }
}

/// Legacy `patGate`, run only when PAT issuers are installed. `verify` runs
/// (and may spend the token) only on an enforcing route, as in Go.
pub fn pat(
    subject: &Subject<'_>,
    route: &Route<'_>,
    verify: impl FnOnce() -> privacy_pass::Verdict,
) -> Outcome {
    if !route.enforcing {
        return Outcome::proceed();
    }
    let privacy_pass::Verdict::Verified { key_id } = verify() else {
        return Outcome::proceed();
    };
    let issuer = key_id.get(..12).unwrap_or(&key_id);
    trust_upgrade(
        subject,
        EVENT_PAT_VERIFIED,
        "privacy-pass",
        format!("verified Private Access Token, issuer {issuer}"),
    )
}

/// Legacy `webauthnGate`, run only when a credential registry is installed.
/// `verify` runs only on an enforcing route; `fanout` counts a verified
/// credential (Go `credCorr.Observe`) and decides whether its upgrade holds.
pub fn webauthn(
    subject: &Subject<'_>,
    route: &Route<'_>,
    verify: impl FnOnce() -> webauthn::Verdict,
    fanout: impl FnOnce(&str) -> fanout::Decision,
) -> Outcome {
    if !route.enforcing {
        return Outcome::proceed();
    }
    let webauthn::Verdict::Verified { credential_id } = verify() else {
        return Outcome::proceed();
    };
    match fanout(&credential_id) {
        fanout::Decision::Upgrade => {}
        fanout::Decision::Withheld { note, .. } => {
            let rec = Record {
                fail_reason: note,
                ..record(subject, EVENT_WEBAUTHN_VERIFIED, "enforce")
            };
            return Outcome::new(vec![rec], Effect::Continue);
        }
        fanout::Decision::Unavailable => return Outcome::proceed(),
    }
    trust_upgrade(
        subject,
        EVENT_WEBAUTHN_VERIFIED,
        "webauthn",
        format!(
            "verified WebAuthn assertion, credential {}",
            short(&credential_id)
        ),
    )
}

/// Go `credID[:16]`, cut back to a character boundary.
fn short(credential: &str) -> &str {
    let mut end = credential.len().min(16);
    while !credential.is_char_boundary(end) {
        end -= 1;
    }
    &credential[..end]
}

/// Legacy `verdictTokenGate`, run only when a token key is installed. `token`
/// checks the presented cookie; it is not called when the mode is off.
pub fn verdict_token(
    subject: &Subject<'_>,
    route: &Route<'_>,
    mode: Mode,
    token: impl FnOnce() -> Token,
    floor: &Floor<'_>,
) -> Outcome {
    if mode == Mode::Off {
        return Outcome::proceed();
    }
    let enforce = mode == Mode::Enforce;
    match token() {
        Token::Absent => Outcome::proceed(),
        Token::Invalid(reason) if !enforce => {
            let rec = Record {
                enforcement_action: "would_clear_and_rescore".into(),
                fail_reason: format!("verdict token {} (audit only)", reason_name(reason)),
                ..record(subject, EVENT_TOKEN_BINDING_MISMATCH, mode_name(mode))
            };
            Outcome::new(vec![rec], Effect::Continue)
        }
        Token::Invalid(reason) => {
            let rec = Record {
                fail_reason: format!(
                    "verdict token {} \u{2014} cleared, re-scoring",
                    reason_name(reason)
                ),
                ..record(subject, EVENT_TOKEN_BINDING_MISMATCH, "enforce")
            };
            Outcome {
                clear_verdict_cookie: true,
                ..Outcome::new(vec![rec], Effect::Continue)
            }
        }
        Token::Valid if !enforce => {
            let rec = Record {
                verdict: "ALLOW".into(),
                enforcement_action: "would_fast_path".into(),
                ..record(subject, EVENT_ENF_ALLOW, mode_name(mode))
            };
            Outcome::new(vec![rec], Effect::Continue)
        }
        Token::Valid if !route.enforcing => Outcome::proceed(),
        Token::Valid => {
            let decided = attest_floor::decide(
                attest_floor::Request {
                    attest_floor: route.attest_floor,
                    enforcing: true,
                    verdict_token: true,
                    sticky_allow: false,
                    step_up: floor.step_up,
                },
                floor.mode,
            );
            if decided.decision != attest_floor::Decision::Forward {
                return Outcome::proceed();
            }
            let mut records = Vec::new();
            if let Some(note) = decided.would_require_step_up {
                records.push(attestation_floor_audit(
                    subject.tenant,
                    subject.route_class,
                    floor.mode,
                    note,
                    &floor.audit,
                ));
            }
            records.push(Record {
                verdict: "ALLOW".into(),
                enforcement_action: "pass".into(),
                config_version: route.name.into(),
                ..record(subject, EVENT_ENF_ALLOW, "enforce")
            });
            Outcome::new(records, Effect::Forward)
        }
    }
}

/// Legacy `upgradeGate`: a WebSocket/SSE upgrade on an enforcing route needs a
/// valid fingerprint-bound verdict token (HR-26). `valid_token` is Go
/// `hasValidToken`, which ignores the verdict-token gate's mode.
pub fn upgrade(subject: &Subject<'_>, route: &Route<'_>, valid_token: bool) -> Outcome {
    if subject.route_class != "upgrade" || !route.enforcing || valid_token {
        return Outcome::proceed();
    }
    Outcome::new(
        vec![deny_record(
            &Subject {
                route_class: "upgrade",
                ..*subject
            },
            EVENT_UPGRADE_NO_VERDICT,
            "HR-26",
            "upgrade with no prior fingerprint-bound verdict".into(),
        )],
        Effect::Deny,
    )
}

/// Legacy `sweepGate` (HR-30), run only when a sweep detector is installed.
/// `observe` counts this (binding, session) and reports whether the binding
/// is sweeping; it is not called when the mode is off or an enforcing gate
/// meets a non-enforcing route.
pub fn sweep(
    subject: &Subject<'_>,
    route: &Route<'_>,
    mode: Mode,
    observe: impl FnOnce() -> Result<bool, Unavailable>,
) -> Result<Outcome, Unavailable> {
    if mode == Mode::Off || mode == Mode::Enforce && !route.enforcing {
        return Ok(Outcome::proceed());
    }
    let sweeping = match observe() {
        Ok(sweeping) => sweeping,
        Err(refusal) if mode == Mode::Enforce => return Err(refusal),
        Err(_) => return Ok(Outcome::proceed()),
    };
    if !sweeping {
        return Ok(Outcome::proceed());
    }
    let reason = "decision-probing sweep (many sessions, one fingerprint)";
    let mut rec = deny_record(subject, EVENT_RECON_PROBING, "HR-30", reason.into());
    if mode != Mode::Enforce {
        rec.enforcement_mode = mode_name(mode).into();
        rec.enforcement_action = "would_block".into();
        rec.fail_reason = format!("{reason} (audit only)");
        return Ok(Outcome::new(vec![rec], Effect::Continue));
    }
    Ok(Outcome::new(vec![rec], Effect::Deny))
}

/// Legacy `auditAttestationFloor`: what the floor would have done while its
/// mode is monitor or shadow. Never changes the outcome.
pub fn attestation_floor_audit(
    tenant: &str,
    route_class: &str,
    mode: FloorMode,
    reason: &str,
    audit: &FloorAudit<'_>,
) -> Record {
    Record {
        event_type: EVENT_ENF_CHALLENGE_ISSUED.into(),
        actor: Actor {
            kind: "subject".into(),
            id_pseudonym: audit.actor_pseudonym.into(),
        },
        tenant_id: tenant.into(),
        session_pseudonym: audit.session_pseudonym.into(),
        host: audit.host.into(),
        route_class: route_class.into(),
        verdict: "ALLOW".into(),
        enforcement_action: "would_challenge".into(),
        enforcement_mode: floor_mode_name(mode).into(),
        fail_reason: format!("{reason} (audit only)"),
        key_id: "k1".into(),
        config_version: audit.config_version.into(),
        ..Default::default()
    }
}

#[cfg(test)]
pub(in crate::gateway) mod tests;
