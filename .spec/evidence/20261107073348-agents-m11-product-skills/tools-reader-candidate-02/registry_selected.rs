//! Explicit single-registry content observation, never live app or writer authority.
use super::{Report, validate};
use crate::selected_inputs::{Clock, FrozenSelection, Input, Limits, Root, SystemClock, capture_source_file};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub repo_root: PathBuf,
    pub source_commit: String,
    pub path: String,
    pub owner: String,
    pub presence: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Serialize)]
pub struct Receipt {
    pub report: Report,
    pub declared_source_commit: String,
    pub captured_sha256: String,
    pub actual_read_bytes: u64,
    pub content_checks_complete: bool,
    pub source_commit_verified: bool,
    pub provider_identity_qualified: bool,
    pub writer_authorized: bool,
    pub qualified_complete: bool,
    pub identity_status: &'static str,
}

/// Input is a bounded request captured by the caller with the same original clock.
pub fn inspect_bytes(bytes: &[u8], clock: &mut SystemClock) -> Result<Receipt, Box<dyn std::error::Error>> {
    if bytes.len() > 2 * 1024 * 1024 { return Err("registry request exceeds cap".into()); }
    let request: Request = serde_json::from_slice(bytes).map_err(|_| "invalid registry request")?;
    if request.schema != "humanymous.thread-registry-source.v1"
        || request.path != ".agents/sessions/THREADS.md" || request.owner != "agents-meta"
        || request.presence != "required" || request.bytes > 2 * 1024 * 1024
        || request.sha256.len() != 64
        || !request.sha256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || ![40, 64].contains(&request.source_commit.len())
        || !request.source_commit.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    { return Err("invalid finite registry source contract".into()); }
    if request.repo_root.to_str().is_none_or(|root| {
        root.contains(['*', '?', '[', ']']) || root.chars().any(char::is_control)
    }) { return Err("unsafe explicit source root".into()); }
    FrozenSelection::new(
        vec![Root { id: "repo".into(), path: request.repo_root.clone() }],
        vec![Input { root: "repo".into(), path: request.path.clone() }], true, &Limits::default(),
    ).map_err(|_| "invalid explicit registry root")?;
    let (captured, read) = capture_source_file(
        &request.repo_root.join(&request.path), request.bytes.max(1), clock,
    )?;
    let captured_sha256 = format!("{:x}", Sha256::digest(&captured));
    if captured.len() as u64 != request.bytes || captured_sha256 != request.sha256 {
        return Err("registry captured source mismatch".into());
    }
    let text = std::str::from_utf8(&captured).map_err(|_| "registry input is not UTF-8")?;
    let report = validate(text);
    if clock.elapsed() >= Limits::default().max_elapsed { return Err("registry deadline".into()); }
    let complete = report.errors.is_empty();
    Ok(Receipt {
        report, declared_source_commit: request.source_commit, captured_sha256,
        actual_read_bytes: read.actual_read_bytes, content_checks_complete: complete,
        source_commit_verified: false, provider_identity_qualified: false,
        writer_authorized: false, qualified_complete: false,
        identity_status: "unsupported_source_identity; trusted-checkout content observation only",
    })
}

/// Actual explicit request-file entrypoint; no relative request fallback.
pub fn check_request_file(path: &std::path::Path) -> Result<Receipt, Box<dyn std::error::Error>> {
    let mut clock = SystemClock::default();
    let (bytes, request_read) = capture_source_file(path, 2 * 1024 * 1024, &mut clock)?;
    let mut receipt = inspect_bytes(&bytes, &mut clock)?;
    receipt.actual_read_bytes = receipt.actual_read_bytes.checked_add(request_read.actual_read_bytes)
        .ok_or("source command accounting overflow")?;
    if receipt.actual_read_bytes > 10 * 1024 * 1024 + 6 || clock.elapsed() >= Limits::default().max_elapsed {
        return Err("source command budget exhausted".into());
    }
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_snapshot_or_ambient_fallback_on_missing_contract() {
        assert!(inspect_bytes(b"{}", &mut SystemClock::default()).is_err());
    }
}
