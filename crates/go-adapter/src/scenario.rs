//! Pure evaluation of real lab observations; no transport or authority clone.
use crate::{Case, Outcome, probe};
use serde::Deserialize;

/// Exact request target for the pinned binary echo case.
pub const BINARY_TARGET: &str = "/dump/request?q=1%2B2";
/// Exact binary entity, including invalid UTF-8, from the pilot contract.
pub const BINARY_ENTITY: &[u8] = &[0, 255, 13, 10, 65, 127, 128];
/// Exact HEAD fixture target; repeated cookie query members must survive.
pub const HEAD_TARGET: &str = "/response-headers?Set-Cookie=a%3D1&Set-Cookie=b%3D2";

/// Observations supplied by the actual lab. Intentionally not Debug/Serialize.
/// No defaults allow omitted facts to become successful measurements.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseObservation {
    /// Selected case from the fixed contract inventory.
    pub case: Case,
    /// Actual decoded HTTP status, or no response for cancellation/kernel denial.
    pub http_status: Option<u16>,
    /// Already-bounded decoded outer response entity; never reconstructed text.
    pub decoded_entity: Vec<u8>,
    /// Individual outer header pairs; repeated cookies are not coalesced.
    pub header_pairs: Vec<(String, String)>,
    /// Independent measured contact count, absent when measurement failed.
    pub origin_contacts: Option<u32>,
    /// Actual fixture artifact/process identity verified by the lab.
    pub actual_host_verified: bool,
    /// Cancellation observed before final input and origin handoff.
    pub pre_handoff_cancelled: bool,
    /// Contact observation remained live past the original request deadline.
    pub observed_past_deadline: bool,
    /// Client performed a write half-close after its incomplete entity.
    pub write_half_closed: bool,
    /// Both backend and witness paths were denied by measured kernel policy.
    pub backend_and_witness_kernel_denied: bool,
}

/// Non-sensitive failed/missing check. The outer lab ledger retains the case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvaluationError {
    /// Static diagnostic; never interpolates headers, bodies or credentials.
    pub code: &'static str,
    /// Checks attempted, including the failing or missing observation check.
    pub assertions: u32,
}
fn require(value: bool, code: &'static str, checks: &mut u32) -> Result<(), EvaluationError> {
    *checks += 1;
    if value {
        Ok(())
    } else {
        Err(EvaluationError {
            code,
            assertions: *checks,
        })
    }
}

/// Produce a successful outcome only for a complete matching measured case.
/// This evaluates evidence supplied by the trusted lab, not its authenticity.
pub fn evaluate(o: &CaseObservation) -> Result<Outcome, EvaluationError> {
    let mut checks = 0;
    require(
        o.actual_host_verified,
        "actual_host_unverified",
        &mut checks,
    )?;
    checks += 1;
    let contacts = o.origin_contacts.ok_or(EvaluationError {
        code: "origin_counter_missing",
        assertions: checks,
    })?;
    let positive = matches!(o.case, Case::BinaryAllow | Case::ResponseSemantics);
    require(
        contacts == u32::from(positive),
        "origin_contacts_mismatch",
        &mut checks,
    )?;
    match o.case {
        Case::BinaryAllow => require(
            probe::binary_dump(
                o.http_status.unwrap_or(0),
                &o.decoded_entity,
                "POST",
                BINARY_TARGET,
                BINARY_ENTITY,
            ),
            "binary_dump_mismatch",
            &mut checks,
        )?,
        Case::ResponseSemantics => require(
            probe::head_cookies(
                o.http_status.unwrap_or(0),
                o.header_pairs.iter().map(|(n, v)| (n.as_str(), v.as_str())),
                &o.decoded_entity,
            ),
            "head_cookie_mismatch",
            &mut checks,
        )?,
        Case::DependencyDown | Case::ForgedProvenance => {
            require(
                o.http_status == Some(422),
                "capability_status_mismatch",
                &mut checks,
            )?;
            require(
                o.decoded_entity == br#"{"error":"required_capability"}"#,
                "capability_reason_mismatch",
                &mut checks,
            )?;
        }
        Case::IncompleteBody => {
            require(o.write_half_closed, "half_close_unobserved", &mut checks)?;
            require(
                o.http_status == Some(400),
                "incomplete_status_mismatch",
                &mut checks,
            )?;
            require(
                o.decoded_entity == br#"{"error":"truncated_body"}"#,
                "incomplete_reason_mismatch",
                &mut checks,
            )?;
        }
        Case::Upgrade => {
            require(
                o.http_status == Some(400),
                "upgrade_status_mismatch",
                &mut checks,
            )?;
            require(
                o.decoded_entity == br#"{"error":"http_framing"}"#,
                "upgrade_reason_mismatch",
                &mut checks,
            )?;
        }
        Case::Cancellation | Case::OriginBypass => {
            require(
                o.http_status.is_none() && o.decoded_entity.is_empty() && o.header_pairs.is_empty(),
                "unexpected_http_response",
                &mut checks,
            )?;
            if o.case == Case::Cancellation {
                require(
                    o.pre_handoff_cancelled,
                    "pre_handoff_cancel_unobserved",
                    &mut checks,
                )?;
                require(
                    o.observed_past_deadline,
                    "deadline_observation_missing",
                    &mut checks,
                )?;
            } else {
                require(
                    o.backend_and_witness_kernel_denied,
                    "kernel_denial_unobserved",
                    &mut checks,
                )?;
            }
        }
    }
    Ok(Outcome {
        case: o.case,
        assertions: checks,
        failures: 0,
        origin_contacts: contacts,
        handler_response_verified: positive,
        actual_host: o.actual_host_verified,
    })
}

#[cfg(test)]
mod tests;
