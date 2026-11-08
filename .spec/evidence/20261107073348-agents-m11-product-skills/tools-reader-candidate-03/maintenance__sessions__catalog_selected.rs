//! Fixed lane projection content check; no script execution or source authority.
use crate::maintenance::sessions::catalog::{Catalog, Report, validate_projections};
use crate::maintenance::{inventory::FileRecord, selected_inputs::{Clock, FrozenSelection, Input, Limits, Root, SystemClock, capture_source_file}};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub repo_root: PathBuf,
    pub source_commit: String,
    pub expected_lane_count: u16,
    pub inputs: [Pin; 4],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
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
    pub files: Vec<FileRecord>,
    pub actual_read_bytes: u64,
    pub content_checks_complete: bool,
    pub source_commit_verified: bool,
    pub provider_identity_qualified: bool,
    pub writer_authorized: bool,
    pub qualified_complete: bool,
    pub identity_status: &'static str,
}

const PATHS: [&str; 4] = [
    ".agents/sessions/lanes.json", ".agents/sessions/LANES.md",
    "scripts/agents/session-board.ps1", "scripts/agents/session-board.sh",
];
const CAPS: [u64; 4] = [256 * 1024, 2 * 1024 * 1024, 2 * 1024 * 1024, 2 * 1024 * 1024];

/// The caller supplies bounded request bytes from its command's original clock.
/// Root must bind request capture and these reads to that same command budget.
pub fn inspect_bytes(
    bytes: &[u8], clock: &mut SystemClock,
) -> Result<Receipt, Box<dyn std::error::Error>> {
    if bytes.len() > 2 * 1024 * 1024 { return Err("catalog request exceeds cap".into()); }
    let request: Request = serde_json::from_slice(bytes).map_err(|_| "invalid catalog request")?;
    if request.schema != "humanymous.lane-projection-source.v1"
        || request.expected_lane_count == 0 || request.expected_lane_count > 128
        || ![40, 64].contains(&request.source_commit.len())
        || !request.source_commit.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    { return Err("invalid catalog source declaration".into()); }
    if request.repo_root.to_str().is_none_or(|root| {
        root.contains(['*', '?', '[', ']']) || root.chars().any(char::is_control)
    }) { return Err("unsafe explicit source root".into()); }
    FrozenSelection::new(
        vec![Root { id: "repo".into(), path: request.repo_root.clone() }],
        PATHS.iter().map(|p| Input { root: "repo".into(), path: (*p).into() }).collect(),
        true, &Limits::default(),
    ).map_err(|_| "invalid explicit catalog root")?;
    let mut texts = Vec::with_capacity(4);
    let mut files = Vec::with_capacity(4);
    let mut actual = 0_u64;
    for (index, pin) in request.inputs.iter().enumerate() {
        if pin.path != PATHS[index] || pin.owner != "agents-meta" || pin.presence != "required"
            || pin.bytes > CAPS[index] || pin.sha256.len() != 64
            || !pin.sha256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        { return Err("catalog finite input pin mismatch".into()); }
        let (captured, read) = capture_source_file(
            &request.repo_root.join(PATHS[index]), pin.bytes.max(1), clock,
        )?;
        actual = actual.checked_add(read.actual_read_bytes).ok_or("catalog accounting overflow")?;
        let sha256 = format!("{:x}", Sha256::digest(&captured));
        if captured.len() as u64 != pin.bytes || sha256 != pin.sha256 {
            return Err("catalog captured source mismatch".into());
        }
        files.push(FileRecord { path: pin.path.clone(), bytes: captured.len(), sha256 });
        texts.push(String::from_utf8(captured).map_err(|_| "catalog input is not UTF-8")?);
    }
    if clock.elapsed() >= Limits::default().max_elapsed { return Err("catalog deadline".into()); }
    let catalog = Catalog::parse(&texts[0]).map_err(|_| "invalid lane catalog")?;
    let mut report = validate_projections(&catalog, &texts[1], &texts[2], &texts[3]);
    if report.lane_count != usize::from(request.expected_lane_count) {
        report.errors.push("declared lane count mismatch".into());
    }
    let complete = report.errors.is_empty();
    Ok(Receipt {
        report, declared_source_commit: request.source_commit, files, actual_read_bytes: actual,
        content_checks_complete: complete, source_commit_verified: false,
        provider_identity_qualified: false, writer_authorized: false, qualified_complete: false,
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
    fn missing_or_duplicate_fields_do_not_become_empty_success() {
        let mut clock = SystemClock::default();
        assert!(inspect_bytes(br#"{"schema":"x","schema":"y"}"#, &mut clock).is_err());
        assert!(inspect_bytes(br#"{}"#, &mut clock).is_err());
    }
}
