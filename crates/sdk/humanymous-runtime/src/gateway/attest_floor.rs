//! Attestation floor (ceiling-guard #1) for high-value routes.
//!
//! Port of the legacy `attestFloor` branches in internal/gate/gates.go
//! (verdictTokenGate and applyVerdict) at baseline 981d25c. A detection-plane
//! ALLOW, whether carried by a verdict token or by a sticky session verdict,
//! does not reach the origin of an attested route unless the request also
//! holds a valid step-up proof. In enforce mode the ALLOW is demoted to a
//! step-up CHALLENGE (HTTP 401 interstitial), never to a DENY. Monitor and
//! shadow modes forward and report what the floor would have done.
//!
//! This is a request-path decision only. Scores, thresholds, signal IDs and
//! verdict bands are unchanged; route presets and HTTP serving are wired by the
//! runtime owner.
use super::token::{Epochs, TokenKey};
use FloorMode as Mode;

/// Mode of the `gate.attest_floor` module setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloorMode {
    /// Demote an unproven ALLOW to a step-up challenge.
    Enforce,
    /// Forward, and report that step-up would be required.
    Monitor,
    /// Forward, and report that step-up would be required.
    Shadow,
    /// Floor disabled.
    Off,
}

/// What the gate does with the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Forward to the origin.
    Forward,
    /// Serve the step-up interstitial (HTTP 401); the origin is not contacted.
    StepUpChallenge,
    /// Not an ALLOW on this path; the ordinary verdict pipeline decides.
    Pipeline,
}

/// Request facts the floor needs. Token checks are done by [`Credentials`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    /// The matched route preset carries the attestation floor.
    pub attest_floor: bool,
    /// The route is in enforcing mode.
    pub enforcing: bool,
    /// The request carries a verdict token that verifies for this session.
    pub verdict_token: bool,
    /// The session's sticky verdict is ALLOW.
    pub sticky_allow: bool,
    /// The request carries a step-up proof that verifies for this session.
    pub step_up: bool,
}

/// Decision plus the audit note the legacy gate emitted in monitor/shadow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// What to do with the request.
    pub decision: Decision,
    /// Set when monitor/shadow forwarded a request the floor would have held.
    pub would_require_step_up: Option<&'static str>,
}

const TOKEN_NOTE: &str = "verdict-token fast path would require step-up";
const SCORING_NOTE: &str = "scoring allow would require step-up";

/// Decide one request. The verdict-token fast path is tried first, then the
/// sticky verdict, in the legacy order.
pub fn decide(request: Request, mode: FloorMode) -> Outcome {
    let floor = request.enforcing && request.attest_floor && !request.step_up;
    let mut note = None;
    if request.verdict_token {
        if !floor {
            return Outcome {
                decision: Decision::Forward,
                would_require_step_up: None,
            };
        }
        match mode {
            Mode::Enforce => {}
            Mode::Monitor | Mode::Shadow => {
                return Outcome {
                    decision: Decision::Forward,
                    would_require_step_up: Some(TOKEN_NOTE),
                };
            }
            Mode::Off => {
                return Outcome {
                    decision: Decision::Forward,
                    would_require_step_up: None,
                };
            }
        }
    }
    if !request.sticky_allow {
        return Outcome {
            decision: Decision::Pipeline,
            would_require_step_up: note,
        };
    }
    let decision = if floor {
        match mode {
            Mode::Enforce => Decision::StepUpChallenge,
            Mode::Monitor | Mode::Shadow => {
                note = Some(SCORING_NOTE);
                Decision::Forward
            }
            Mode::Off => Decision::Forward,
        }
    } else {
        Decision::Forward
    };
    Outcome {
        decision,
        would_require_step_up: note,
    }
}

/// Token verification inputs shared by the verdict and step-up checks.
pub struct Credentials<'a> {
    /// Server-only token key.
    pub key: &'a TokenKey,
    /// Accepted epochs.
    pub epochs: &'a Epochs,
    /// Client binding (`token_bind`).
    pub bind: &'a str,
    /// Session id.
    pub sid: &'a str,
    /// Current time, seconds.
    pub now: u64,
}

impl Credentials<'_> {
    /// Whether `token` is a valid verdict token for this session and binding.
    pub fn verdict(&self, token: Option<&str>) -> bool {
        token.is_some_and(|token| {
            !token.is_empty()
                && self
                    .key
                    .verify_verdict(token, self.bind, self.sid, self.now, &self.accepted())
                    .is_ok()
        })
    }

    /// Whether `token` is a valid step-up proof for this session and binding.
    pub fn step_up(&self, token: Option<&str>) -> bool {
        token.is_some_and(|token| {
            !token.is_empty()
                && self
                    .key
                    .verify_step_up(token, self.bind, self.sid, self.now, &self.accepted())
                    .is_ok()
        })
    }

    fn accepted(&self) -> Vec<String> {
        self.epochs.accepted(self.now)
    }
}

#[cfg(test)]
mod tests;
