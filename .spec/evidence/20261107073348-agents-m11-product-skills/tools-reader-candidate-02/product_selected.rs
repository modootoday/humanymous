//! Fixed five-file product skill content check; source identity remains unqualified.
use super::{Report, validate};
use crate::{inventory::FileRecord, selected_inputs::{Clock, FrozenSelection, Input, Limits, Root, SystemClock, capture_source_file}};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub repo_root: PathBuf,
    pub source_commit: String,
    pub inputs: [Pin; 5],
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
    pub qualified_complete: bool,
    pub runtime_qualified: bool,
    pub identity_status: &'static str,
}
const PATHS: [&str; 5] = [
    "skills/SKILL.md", "skills/references/installation.md", "skills/references/operations.md",
    "skills/references/adapters.md", "skills/references/qualification.md",
];

/// Fixed canonical profile only. A different reference closure needs a new
/// explicit profile decision; it is never discovered by recursively walking.
pub fn inspect_bytes(bytes: &[u8], clock: &mut SystemClock) -> Result<Receipt, Box<dyn std::error::Error>> {
    if bytes.len() > 2 * 1024 * 1024 { return Err("product request exceeds cap".into()); }
    let request: Request = serde_json::from_slice(bytes).map_err(|_| "invalid product request")?;
    if request.schema != "humanymous.product-skill-source.v1"
        || ![40, 64].contains(&request.source_commit.len())
        || !request.source_commit.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    { return Err("invalid product source declaration".into()); }
    if request.repo_root.to_str().is_none_or(|root| {
        root.contains(['*', '?', '[', ']']) || root.chars().any(char::is_control)
    }) { return Err("unsafe explicit source root".into()); }
    FrozenSelection::new(
        vec![Root { id: "repo".into(), path: request.repo_root.clone() }],
        PATHS.iter().map(|p| Input { root: "repo".into(), path: (*p).into() }).collect(),
        true, &Limits::default(),
    ).map_err(|_| "invalid explicit product root")?;
    let total = request.inputs.iter().try_fold(0_u64, |n, pin| n.checked_add(pin.bytes))
        .ok_or("product declared size overflow")?;
    if total > 8 * 1024 * 1024 { return Err("product bundle exceeds cap".into()); }
    let mut texts = Vec::with_capacity(5);
    let mut files = Vec::with_capacity(5);
    let mut actual = 0_u64;
    for (index, pin) in request.inputs.iter().enumerate() {
        let cap = if index == 0 { 256 * 1024 } else { 2 * 1024 * 1024 };
        if pin.path != PATHS[index] || pin.owner != "agents-meta" || pin.presence != "required"
            || pin.bytes > cap || pin.sha256.len() != 64
            || !pin.sha256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        { return Err("product finite input pin mismatch".into()); }
        let (captured, read) = capture_source_file(
            &request.repo_root.join(PATHS[index]), pin.bytes.max(1), clock,
        )?;
        actual = actual.checked_add(read.actual_read_bytes).ok_or("product accounting overflow")?;
        let sha256 = format!("{:x}", Sha256::digest(&captured));
        if captured.len() as u64 != pin.bytes || sha256 != pin.sha256 {
            return Err("product captured source mismatch".into());
        }
        files.push(FileRecord { path: pin.path.clone(), bytes: captured.len(), sha256 });
        texts.push(String::from_utf8(captured).map_err(|_| "product input is not UTF-8")?);
    }
    let resources = PATHS.iter().skip(1).zip(texts.iter().skip(1))
        .map(|(path, text)| (path.strip_prefix("skills/").expect("fixed path").into(), text.clone()))
        .collect::<BTreeMap<_, _>>();
    let report = validate(&texts[0], &resources);
    if clock.elapsed() >= Limits::default().max_elapsed { return Err("product deadline".into()); }
    let complete = report.errors.is_empty();
    Ok(Receipt {
        report, declared_source_commit: request.source_commit, files, actual_read_bytes: actual,
        content_checks_complete: complete, source_commit_verified: false,
        provider_identity_qualified: false, qualified_complete: false, runtime_qualified: false,
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
    fn incomplete_contract_does_not_discover_a_product_root() {
        assert!(inspect_bytes(b"{}", &mut SystemClock::default()).is_err());
    }
}
