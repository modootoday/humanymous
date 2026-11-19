//! Request-smuggling classification and audit records (legacy
//! internal/gate/smuggle.go and the guard.go spoof record at baseline 981d25c;
//! HR-23, HR-27b).
//!
//! The Rust HTTP parser already refuses every ambiguous framing with
//! `http_framing` before a request exists. This kernel classifies a header
//! list the way Go did so the refusal can carry a stable reason, and builds the
//! audit records. Records are DENY/block/enforce and never carry header values.
use crate::audit::record::{Actor, Record};

/// Legacy smuggle reason classes, in Go precedence order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Both Content-Length and Transfer-Encoding.
    TeClConflict,
    /// More than one Content-Length, identical or not.
    AmbiguousLength,
    /// Transfer-Encoding present but not exactly `chunked` once.
    TeNotChunked,
    /// A header value with an embedded CR or LF.
    ObsFold,
}

impl Reason {
    /// Legacy audit spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TeClConflict => "te_cl_conflict",
            Self::AmbiguousLength => "ambiguous_len",
            Self::TeNotChunked => "te_not_chunked",
            Self::ObsFold => "obs_fold",
        }
    }
}

fn values<'a>(headers: &'a [(String, String)], name: &'a str) -> impl Iterator<Item = &'a str> {
    headers
        .iter()
        .filter(move |(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// Classify framing ambiguity exactly like Go's `smuggleScan`.
pub fn scan(headers: &[(String, String)]) -> Option<Reason> {
    let te: Vec<&str> = values(headers, "Transfer-Encoding").collect();
    let cl = values(headers, "Content-Length").count();
    if !te.is_empty() && cl > 0 {
        return Some(Reason::TeClConflict);
    }
    if cl > 1 {
        return Some(Reason::AmbiguousLength);
    }
    if te.len() > 1 || te.iter().any(|v| !v.trim().eq_ignore_ascii_case("chunked")) {
        return Some(Reason::TeNotChunked);
    }
    if headers
        .iter()
        .any(|(_, v)| v.contains('\r') || v.contains('\n'))
    {
        return Some(Reason::ObsFold);
    }
    None
}

/// Legacy record shape: the session pseudonym is the subject actor and the
/// key version is `k1`, as Go's `smuggleRecord` and `spoofRecord` set them.
fn deny(event: &str, tenant: &str, session: &str, rule: &str, reason: String) -> Record {
    Record {
        event_type: event.into(),
        actor: Actor {
            kind: "subject".into(),
            id_pseudonym: session.into(),
        },
        tenant_id: tenant.into(),
        key_id: "k1".into(),
        verdict: "DENY".into(),
        triggered_rules: Some(vec![rule.into()]),
        enforcement_action: "block".into(),
        enforcement_mode: "enforce".into(),
        fail_reason: reason,
        ..Default::default()
    }
}

/// HR-23 audit record for a detected smuggling attempt.
pub fn smuggle_record(tenant: &str, session_pseudonym: &str, reason: Reason) -> Record {
    deny(
        "net.smuggle",
        tenant,
        session_pseudonym,
        "HR-23",
        format!("request framing ambiguity: {}", reason.as_str()),
    )
}

/// HR-27b audit record for forged inbound trust headers (`guard::spoof_scan`).
/// Only static header names from the internal list are recorded.
pub fn spoof_record(tenant: &str, session_pseudonym: &str, found: &[&'static str]) -> Record {
    let mut record = deny(
        "hdr.internal_header_ingress",
        tenant,
        session_pseudonym,
        "HR-27b",
        format!("inbound internal headers: {}", found.join(",")),
    );
    record.route_class = "html".into();
    record
}

#[cfg(test)]
mod tests;
