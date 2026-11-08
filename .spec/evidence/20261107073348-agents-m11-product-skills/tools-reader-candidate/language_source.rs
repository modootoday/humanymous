//! Explicit language-source contract; trusted cooperative observations only.
use super::{Preservation, Report, inspect};
use crate::selected_inputs::{
    Clock, FrozenSelection, Input, Limits, MAX_REQUEST_BYTES, Root, SystemClock,
    capture_request_file,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt, path::{Component, Path, PathBuf}};

const MAX_INPUT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_COMMAND_BYTES: u64 = 3 * (MAX_INPUT_BYTES + 1);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub path: String,
    pub owner: String,
    pub presence: Presence,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Presence {
    Required,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub operation: String,
    pub repo_root: PathBuf,
    pub source_commit: String,
    pub source: Expected,
    pub preservation: Expected,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    RequestTooLarge,
    Json { line: usize, column: usize },
    Schema,
    SourceCommit,
    Selection,
    InputContract,
    RequestPath,
    Capture { input: &'static str, code: String },
    Mismatch,
    Utf8,
    Preservation,
    Budget,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "language source contract: {self:?}")
    }
}
impl std::error::Error for Error {}

fn hex(value: &str, bytes: usize) -> bool {
    value.len() == bytes && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Fixed language operation, not a generalized repository-reader framework.
pub fn decode(bytes: &[u8]) -> Result<Request, Error> {
    if bytes.len() as u64 > MAX_REQUEST_BYTES {
        return Err(Error::RequestTooLarge);
    }
    let request: Request = serde_json::from_slice(bytes).map_err(|e| Error::Json {
        line: e.line(), column: e.column(),
    })?;
    if request.schema != "humanymous.language-source-request.v1"
        || request.operation != "language-check"
    {
        return Err(Error::Schema);
    }
    if !hex(&request.source_commit, 40) && !hex(&request.source_commit, 64) {
        return Err(Error::SourceCommit);
    }
    if request.repo_root.to_str().is_none_or(|root| {
        root.contains(['*', '?', '[', ']']) || root.chars().any(char::is_control)
    }) {
        return Err(Error::Selection);
    }
    for input in [&request.source, &request.preservation] {
        if input.path.contains(['*', '?', '[', ']'])
            || input.owner.is_empty() || input.owner.len() > 128
            || !input.owner.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_/".contains(&b))
            || input.presence != Presence::Required
            || input.bytes > MAX_INPUT_BYTES || !hex(&input.sha256, 64)
        {
            return Err(Error::InputContract);
        }
    }
    if !(request.source.path.ends_with(".md") || request.source.path.ends_with(".rs"))
        || !request.preservation.path.ends_with(".json")
    {
        return Err(Error::InputContract);
    }
    // Reuse existing absolute-root, traversal, control and case-collision rules.
    FrozenSelection::new(
        vec![Root { id: "repo".into(), path: request.repo_root.clone() }],
        [&request.source, &request.preservation].into_iter()
            .map(|input| Input { root: "repo".into(), path: input.path.clone() }).collect(),
        true, &Limits::default(),
    ).map_err(|_| Error::Selection)?;
    Ok(request)
}

#[derive(Debug, Serialize)]
pub struct Receipt {
    pub schema: &'static str,
    pub declared_source_commit: String,
    pub request_sha256: String,
    pub source_sha256: String,
    pub preservation_sha256: String,
    pub actual_read_bytes: u64,
    pub elapsed_micros: u128,
    pub report: Report,
    pub content_matches_declared_manifest: bool,
    pub source_commit_verified: bool,
    pub provider_identity_qualified: bool,
    pub qualified_complete: bool,
    pub execution_authorized: bool,
    pub runtime_qualified: bool,
    pub refusal: &'static str,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn verify<'a>(expected: &Expected, bytes: &'a [u8]) -> Result<&'a str, Error> {
    if bytes.len() as u64 != expected.bytes || digest(bytes) != expected.sha256 {
        return Err(Error::Mismatch);
    }
    std::str::from_utf8(bytes).map_err(|_| Error::Utf8)
}

/// Existing capture checks all ancestors/reparse points and metadata before and
/// after open/read. It does NOT establish race-resistant provider identity or
/// authenticate a caller's Git manifest. Never promote content matches to either.
pub fn check_request_file(path: &Path) -> Result<Receipt, Error> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::CurDir | Component::ParentDir)) {
        return Err(Error::RequestPath);
    }
    let mut clock = SystemClock::default();
    let (request_bytes, request_read) = capture_request_file(path, &mut clock)
        .map_err(|e| Error::Capture { input: "request", code: e.to_string() })?;
    let request = decode(&request_bytes)?;
    let (source_bytes, source_read) = capture_request_file(
        &request.repo_root.join(&request.source.path), &mut clock,
    ).map_err(|e| Error::Capture { input: "source", code: e.to_string() })?;
    let source = verify(&request.source, &source_bytes)?;
    let (preservation_bytes, preservation_read) = capture_request_file(
        &request.repo_root.join(&request.preservation.path), &mut clock,
    ).map_err(|e| Error::Capture { input: "preservation", code: e.to_string() })?;
    let preservation = verify(&request.preservation, &preservation_bytes)?;
    let records: Vec<Preservation> = serde_json::from_str(preservation).map_err(|_| Error::Preservation)?;
    if records.len() > 4096 { return Err(Error::Preservation); }
    for record in records.iter().filter(|record| record.path == request.source.path) {
        if !source.lines().any(|line| digest(line.as_bytes()) == record.line_sha256) {
            return Err(Error::Preservation);
        }
    }
    let report = inspect(&request.source.path, source, &records);
    let actual = request_read.actual_read_bytes.checked_add(source_read.actual_read_bytes)
        .and_then(|n| n.checked_add(preservation_read.actual_read_bytes)).ok_or(Error::Budget)?;
    let elapsed = clock.elapsed();
    if actual > MAX_COMMAND_BYTES || elapsed >= Limits::default().max_elapsed {
        return Err(Error::Budget);
    }
    Ok(Receipt {
        schema: "humanymous.language-source-receipt.v1",
        declared_source_commit: request.source_commit,
        request_sha256: digest(&request_bytes),
        source_sha256: digest(&source_bytes),
        preservation_sha256: digest(&preservation_bytes),
        actual_read_bytes: actual, elapsed_micros: elapsed.as_micros(), report,
        content_matches_declared_manifest: true,
        source_commit_verified: false, provider_identity_qualified: false,
        qualified_complete: false, execution_authorized: false, runtime_qualified: false,
        refusal: "unsupported_source_identity: cooperative capture cannot authenticate Git source or exclude hostile path replacement",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> serde_json::Value {
        serde_json::json!({
            "schema":"humanymous.language-source-request.v1", "operation":"language-check",
            "repo_root":std::env::temp_dir(), "source_commit":"a".repeat(40),
            "source":{"path":"fixture.md","owner":"synthetic","presence":"required","bytes":0,"sha256":digest(b"")},
            "preservation":{"path":"fixture.json","owner":"synthetic","presence":"required","bytes":2,"sha256":digest(b"[]")}
        })
    }
    #[test]
    fn strict_fixed_contract_has_no_live_or_unknown_fallback() {
        assert!(decode(&serde_json::to_vec(&request()).unwrap()).is_ok());
        for (key, value) in [("live_mode", serde_json::json!(true)), ("operation", serde_json::json!("session-claim"))] {
            let mut r = request(); r[key] = value;
            assert!(decode(&serde_json::to_vec(&r).unwrap()).is_err());
        }
        assert!(decode(br#"{"schema":"x","schema":"y"}"#).is_err());
    }
    #[test]
    fn finite_selection_rejects_escape_alias_and_optional_presence() {
        for path in ["../fixture.md", "/fixture.md", "a\\fixture.md", "*.md", "a/./fixture.md"] {
            let mut r = request(); r["source"]["path"] = path.into();
            assert!(decode(&serde_json::to_vec(&r).unwrap()).is_err());
        }
        let mut r = request(); r["source"]["presence"] = "optional".into();
        assert!(decode(&serde_json::to_vec(&r).unwrap()).is_err());
        let mut r = request(); r["source"]["bytes"] = (MAX_INPUT_BYTES + 1).into();
        assert!(decode(&serde_json::to_vec(&r).unwrap()).is_err());
    }
    #[test]
    fn verification_uses_exact_captured_bytes_without_normalizing() {
        let r = decode(&serde_json::to_vec(&request()).unwrap()).unwrap();
        assert_eq!(verify(&r.preservation, b"[]"), Ok("[]"));
        assert_eq!(verify(&r.preservation, b"[]\r\n"), Err(Error::Mismatch));
        assert_eq!(verify(&r.source, b"changed"), Err(Error::Mismatch));
    }
    #[test]
    fn errors_do_not_echo_supplied_payloads() {
        let error = decode(br#"{"schema":"PRIVATE_SENTINEL"}"#).unwrap_err();
        assert!(!error.to_string().contains("PRIVATE_SENTINEL"));
        assert_eq!(check_request_file(Path::new("relative.json")).unwrap_err(), Error::RequestPath);
    }
}
