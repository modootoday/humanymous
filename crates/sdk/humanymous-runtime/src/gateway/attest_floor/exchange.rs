//! The `/stepup` receipt-to-proof exchange (legacy `ControlPlane.handleStepUp`
//! in internal/gate/control.go at baseline 981d25c).
//!
//! The Pass server hands a solved client a session-bound receipt. The client
//! presents it here; the gate, which sees the live socket, re-binds a step-up
//! proof to this client binding and returns it as the `hmn_su` cookie. The
//! receipt alone grants nothing, and a proof minted here cannot be lifted to
//! another binding.
//!
//! Kept from Go: the method, session and key checks and their order, every
//! status and body, the typed refusal reasons and their audit messages, the
//! 30 minute proof lifetime and the cookie attributes. The control-plane
//! flood guard that Go ran first is `gateway::control_flood`, applied by the
//! caller. Serving the path and pseudonymizing the audit actor are also the
//! caller's.
//!
//! Deliberate difference, pinned by a test: a receipt Rust cannot reissue a
//! proof for (an unencodable session or binding) answers 500 with no cookie;
//! Go minted a proof from whatever fields it had.
use crate::gateway::token::{Reason, TokenKey};

/// Step-up proof cookie (Go `stepUpCookie`).
pub const STEP_UP_COOKIE: &str = "hmn_su";
/// Receipt request header (Go `stepUpReceiptHeader`).
pub const RECEIPT_HEADER: &str = "X-Hmn-Stepup-Receipt";
/// Lifetime of a minted step-up proof, seconds.
pub const STEP_UP_TTL: u64 = 30 * 60;

const POST_ONLY: &str = "POST only\n";
const NOT_CONFIGURED: &str = "step-up not configured\n";
const NO_SESSION: &str = "no session\n";
const REFUSED: &str = "invalid or missing step-up receipt\n";
const MINTED: &str = "{\"ok\":true}\n";
const UNAVAILABLE: &str = "step-up proof unavailable\n";

const BAD_RECEIPT_NOTE: &str = "step-up: missing/invalid Pass receipt (bad_sig)";
const SID_MISMATCH_NOTE: &str = "step-up receipt sid mismatch \u{2014} Core Pass likely not co-served in the Gate cookie jar (hsid diverged); real human may loop on the attested route";
const EXPIRED_NOTE: &str = "step-up receipt expired (check Core\u{2194}Gate clock skew / NTP; the client can re-solve for a fresh receipt)";
const MINTED_NOTE: &str = "step-up proof minted (Pass solved)";

/// Typed receipt check result (Go `receiptReason`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptReason {
    /// Valid, unexpired and bound to the request session.
    Ok,
    /// Absent, forged, malformed or of another token class.
    Bad,
    /// Past its expiry even with the skew leeway.
    Expired,
    /// Valid receipt for a different session (diverged cookie jar).
    SidMismatch,
}

impl ReceiptReason {
    /// Go wire text (`""`, `bad_sig`, `expired`, `sid_mismatch`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "",
            Self::Bad => "bad_sig",
            Self::Expired => "expired",
            Self::SidMismatch => "sid_mismatch",
        }
    }
}

/// Classify a receipt for `sid` at `now` (Go `verifyStepUpReceipt`).
pub fn receipt_reason(key: &TokenKey, receipt: Option<&str>, sid: &str, now: u64) -> ReceiptReason {
    match receipt.map(|receipt| key.verify_receipt(receipt, sid, now)) {
        Some(Ok(())) => ReceiptReason::Ok,
        Some(Err(Reason::Expired)) => ReceiptReason::Expired,
        Some(Err(Reason::BindingMismatch)) => ReceiptReason::SidMismatch,
        None | Some(Err(Reason::Malformed | Reason::BadSignature)) => ReceiptReason::Bad,
    }
}

/// One `/stepup` request, already parsed by the caller.
#[derive(Debug, Clone, Copy)]
pub struct StepUpRequest<'a> {
    /// HTTP method.
    pub method: &'a str,
    /// The `hsid` session cookie, if any.
    pub sid: Option<&'a str>,
    /// The [`RECEIPT_HEADER`] value, if any.
    pub receipt: Option<&'a str>,
    /// Live client binding (`token::token_bind`).
    pub bind: &'a str,
    /// Current epoch label (`Epochs::current`).
    pub epoch: &'a str,
    /// Current time, seconds.
    pub now: u64,
}

/// Audit record fields the legacy handler emitted; the actor is the session
/// pseudonym, route class and tenant are `control`, mode `enforce`, key `k1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepUpAudit {
    /// Event type.
    pub event_type: &'static str,
    /// Verdict field.
    pub verdict: &'static str,
    /// Fail-reason text.
    pub fail_reason: &'static str,
}

/// What to answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepUpResponse {
    /// HTTP status.
    pub status: u16,
    /// Exact body: `text/plain` for refusals, `application/json` for 200.
    pub body: &'static str,
    /// The minted proof for the `hmn_su` cookie ([`set_cookie`]).
    pub proof: Option<String>,
    /// Record to append before answering, if any.
    pub audit: Option<StepUpAudit>,
}

impl StepUpResponse {
    fn plain(status: u16, body: &'static str) -> Self {
        Self {
            status,
            body,
            proof: None,
            audit: None,
        }
    }
}

/// Exchange a receipt for a binding-bound step-up proof. `key` is the shared
/// token key; without one the endpoint is not configured.
pub fn redeem(key: Option<&TokenKey>, request: StepUpRequest<'_>) -> StepUpResponse {
    if request.method != "POST" {
        return StepUpResponse::plain(405, POST_ONLY);
    }
    let Some(key) = key else {
        return StepUpResponse::plain(501, NOT_CONFIGURED);
    };
    let sid = request.sid.unwrap_or("");
    if sid.is_empty() {
        return StepUpResponse::plain(400, NO_SESSION);
    }
    let reason = receipt_reason(key, request.receipt, sid, request.now);
    if reason != ReceiptReason::Ok {
        let fail_reason = match reason {
            ReceiptReason::SidMismatch => SID_MISMATCH_NOTE,
            ReceiptReason::Expired => EXPIRED_NOTE,
            ReceiptReason::Ok | ReceiptReason::Bad => BAD_RECEIPT_NOTE,
        };
        return StepUpResponse {
            audit: Some(StepUpAudit {
                event_type: "token.binding_mismatch",
                verdict: "deny",
                fail_reason,
            }),
            ..StepUpResponse::plain(403, REFUSED)
        };
    }
    let Some(exp) = request.now.checked_add(STEP_UP_TTL) else {
        return StepUpResponse::plain(500, UNAVAILABLE);
    };
    match key.issue_step_up(sid, request.bind, request.epoch, exp) {
        Ok(proof) => StepUpResponse {
            status: 200,
            body: MINTED,
            proof: Some(proof),
            audit: Some(StepUpAudit {
                event_type: "token.issued",
                verdict: "allow",
                fail_reason: MINTED_NOTE,
            }),
        },
        Err(_) => StepUpResponse::plain(500, UNAVAILABLE),
    }
}

/// `Set-Cookie` value for a minted proof, as Go `http.SetCookie` wrote it.
pub fn set_cookie(proof: &str) -> String {
    format!("{STEP_UP_COOKIE}={proof}; Path=/; HttpOnly; Secure; SameSite=Lax")
}
