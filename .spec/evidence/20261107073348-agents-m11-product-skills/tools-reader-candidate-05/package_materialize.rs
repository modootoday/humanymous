//! Explicit reviewed-allowlist materialization, not source or provider authentication.
use super::package_stage::{output_path, selection};
use crate::maintenance::selected_inputs::{Clock, Limits, SystemClock, capture_source_file};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs::OpenOptions, io::Write, path::{Path, PathBuf}};

const ROOT_BINDING: &str = "__FROZEN_CANONICAL_ROOT__";
const MAX_READ: u64 = 2 * selection::MAX_REQUEST + selection::MAX_TEXT + selection::MAX_INPUTS as u64 + 2;

#[derive(Serialize)]
pub struct Receipt {
    pub schema: &'static str,
    pub kind: String,
    pub reviewed_allowlist_sha256: String,
    pub reviewed_root: PathBuf,
    pub repo_root: PathBuf,
    pub declared_source_commit: String,
    pub request_path: PathBuf,
    pub request_sha256: String,
    pub request_bytes: usize,
    pub inputs: Vec<selection::Pin>,
    pub actual_read_bytes: u64,
    pub elapsed_micros: u128,
    pub raw_pins_match_review: bool,
    pub materialization_complete: bool,
    pub canonical_tree_complete: bool,
    pub source_commit_verified: bool,
    pub owner_identity_verified: bool,
    pub provider_identity_qualified: bool,
    pub qualified_complete: bool,
    pub staging_executed: bool,
    pub identity_status: &'static str,
}

fn hash(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }

/// Deserialize with the EXISTING strict type, then bind only an explicit reviewed
/// marker or an already identical root. All remaining validation uses decode.
fn bind_reviewed(
    bytes: &[u8], frozen_root: &Path, source_commit: &str, kind: &str,
) -> Result<(selection::Request, PathBuf), &'static str> {
    if bytes.len() as u64 > selection::MAX_REQUEST { return Err("materialize_allowlist_cap"); }
    let mut request: selection::Request = serde_json::from_slice(bytes)
        .map_err(|_| "materialize_allowlist_json")?;
    let reviewed_root = request.repo_root.clone();
    if reviewed_root != Path::new(ROOT_BINDING) && reviewed_root != frozen_root {
        return Err("materialize_unreviewed_root_rebind");
    }
    if request.source_commit != source_commit || request.kind.name() != kind {
        return Err("materialize_epoch_or_kind_mismatch");
    }
    request.repo_root = frozen_root.to_path_buf();
    let bound = serde_json::to_vec(&request).map_err(|_| "materialize_request_json")?;
    let validated = selection::decode(&bound)?;
    Ok((validated, reviewed_root))
}

/// Caller supplies a review-pinned allowlist and explicit frozen extraction.
/// Changed input bytes, including LICENSE, fail; there is no repin/retry branch.
pub fn materialize(
    allowlist: &Path, reviewed_sha256: &str, frozen_root: &Path,
    source_commit: &str, kind: &str, new_request: &Path,
) -> Result<Receipt, Box<dyn std::error::Error>> {
    let mut clock = SystemClock::default();
    let (allowlist_bytes, allowlist_read) = capture_source_file(allowlist, selection::MAX_REQUEST, &mut clock)?;
    let allowlist_sha256 = hash(&allowlist_bytes);
    if allowlist_sha256 != reviewed_sha256 { return Err("materialize_review_hash_mismatch".into()); }
    let (mut request, reviewed_root) = bind_reviewed(&allowlist_bytes, frozen_root, source_commit, kind)?;
    let destination = output_path(&request.repo_root, new_request)?;
    let mut actual = allowlist_read.actual_read_bytes;
    for pin in &mut request.files {
        let (captured, read) = capture_source_file(
            &request.repo_root.join(&pin.path), pin.bytes.max(1), &mut clock,
        )?;
        actual = actual.checked_add(read.actual_read_bytes).ok_or("materialize_read_overflow")?;
        if actual > MAX_READ { return Err("materialize_read_cap".into()); }
        // Derive the outgoing raw pin ONLY after matching the reviewed expectation.
        // Never normalize license/text bytes or replace a mismatching expectation.
        let sha256 = selection::verify_pin(pin, &captured)?;
        pin.bytes = captured.len() as u64;
        pin.sha256 = sha256;
    }
    let mut outgoing = serde_json::to_vec_pretty(&request)?;
    outgoing.push(b'\n');
    if outgoing.len() as u64 > selection::MAX_REQUEST { return Err("materialize_request_cap".into()); }
    // Reuse the same strict parser for exactly the bytes that will be staged.
    selection::decode(&outgoing)?;
    if clock.elapsed() >= Limits::default().max_elapsed { return Err("materialize_deadline".into()); }
    let mut file = OpenOptions::new().write(true).create_new(true).open(&destination)?;
    file.write_all(&outgoing)?;
    file.sync_all()?;
    drop(file);
    let (read_back, read) = capture_source_file(&destination, outgoing.len() as u64, &mut clock)?;
    actual = actual.checked_add(read.actual_read_bytes).ok_or("materialize_read_overflow")?;
    if actual > MAX_READ || read_back != outgoing || clock.elapsed() >= Limits::default().max_elapsed {
        return Err("materialize_readback_or_budget".into());
    }
    let request_sha256 = hash(&read_back);
    Ok(Receipt {
        schema: "humanymous.stage-request-materialization.v1",
        kind: request.kind.name().into(), reviewed_allowlist_sha256: allowlist_sha256,
        reviewed_root, repo_root: request.repo_root, declared_source_commit: request.source_commit,
        request_path: destination, request_sha256, request_bytes: read_back.len(), inputs: request.files,
        actual_read_bytes: actual, elapsed_micros: clock.elapsed().as_micros(),
        raw_pins_match_review: true, materialization_complete: true,
        canonical_tree_complete: false, source_commit_verified: false,
        owner_identity_verified: false, provider_identity_qualified: false,
        qualified_complete: false, staging_executed: false,
        identity_status: "unsupported_source_identity; review-hash and selected raw-byte matches only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changed_license_is_refused_without_refresh_or_normalization() {
        let original = b"Reviewed license\n";
        let pin = selection::Pin { path: "LICENSE".into(), owner: "repository".into(),
            presence: "required".into(), bytes: original.len() as u64, sha256: hash(original) };
        assert_eq!(selection::verify_pin(&pin, original).unwrap(), pin.sha256);
        assert!(selection::verify_pin(&pin, b"Reviewed license\r\n").is_err());
        assert!(selection::verify_pin(&pin, b"reviewed license\n").is_err());
        assert_eq!(pin.sha256, hash(original));
        assert_eq!(pin.bytes, original.len() as u64);
    }
    #[test]
    fn duplicate_or_unknown_allowlist_fields_still_refuse() {
        let root = std::env::temp_dir();
        assert!(bind_reviewed(br#"{"schema":"x","schema":"y"}"#, &root, "a", "product").is_err());
        assert!(bind_reviewed(br#"{"unexpected":"PRIVATE_SENTINEL"}"#, &root, "a", "product").is_err());
    }
    #[test]
    fn root_binding_is_exact_and_does_not_guess_an_unrelated_checkout() {
        let value = serde_json::json!({"schema":"humanymous.stage-eval-source.v1",
            "repo_root":"__UNREVIEWED_OTHER_ROOT__", "source_commit":"a".repeat(40), "kind":"product",
            "input_selection_complete":true,"expected_skill_names":["fixture"],"files":[]});
        let error = bind_reviewed(&serde_json::to_vec(&value).unwrap(), &std::env::temp_dir(), &"a".repeat(40), "product").unwrap_err();
        assert_eq!(error, "materialize_unreviewed_root_rebind");
    }
    #[test]
    fn reviewed_marker_binds_explicit_root_without_changing_pins() {
        let mut paths = vec!["LICENSE".to_owned(), "skills/SKILL.md".into(), ".agents/evals/product/README.md".into()];
        for case in selection::Kind::Product.cases() {
            for tail in ["prompt.md", "graders/behavior.md", "graders/discovery.md"] {
                paths.push(format!(".agents/evals/product/{case}/{tail}"));
            }
        }
        let value = serde_json::json!({"schema":"humanymous.stage-eval-source.v1",
            "repo_root":ROOT_BINDING,"source_commit":"a".repeat(40),"kind":"product",
            "input_selection_complete":true,"expected_skill_names":["fixture"],
            "files":paths.iter().map(|path|serde_json::json!({"path":path,
                "owner":if path=="LICENSE" {"repository"} else {"agents-meta"},
                "presence":"required","bytes":0,"sha256":hash(b"")})).collect::<Vec<_>>()});
        let root = std::env::temp_dir();
        let (request, reviewed_root) = bind_reviewed(&serde_json::to_vec(&value).unwrap(), &root, &"a".repeat(40), "product").unwrap();
        assert_eq!(request.repo_root, root);
        assert_eq!(reviewed_root, Path::new(ROOT_BINDING));
        assert!(request.files.iter().all(|pin| pin.sha256 == hash(b"") && pin.bytes == 0));
    }
}
