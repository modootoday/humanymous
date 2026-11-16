//! Lab-only external-input score receipts (legacy cmd/server
//! external_input_receipt.go at baseline 981d25c).
//!
//! The channel is off unless an absolute, real receipt directory is configured.
//! It writes one `core.score.json` per run directory, exclusively, and never
//! goes through the operational logger. A receipt binds the run label, a
//! session reference (never the raw session id), the server-authoritative
//! score, the detector WASM digest and a digest of the canonical report.
//!
//! The report digest is taken over the Rust wire encoding of the report; byte
//! parity of that encoding with Go is a differential-parity question, not a
//! property of this writer.
use crate::gateway::logging::canonical_external_input_label;
use humanymous_core::signals::observation::Source;
use humanymous_core::signals::report::{ScoreResult, SessionReport};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Receipt schema written to `core.score.json`.
pub const RECEIPT_SCHEMA: &str = "humanymous.external-input-core-score/v1";
/// Domain of the score-trace digest.
pub const TRACE_SCHEMA: &str = "humanymous.external-input-score-trace/v1";
/// Upper bound on the detector WASM read at configuration.
pub const MAX_DETECTOR_BYTES: u64 = 8 * 1024 * 1024;
/// Upper bound on one encoded receipt.
pub const MAX_RECEIPT_BYTES: usize = 16 * 1024;
/// Public path the detector is served from.
pub const DETECTOR_PATH: &str = "/static/detector.wasm";
/// File name of the receipt inside the receipt directory.
pub const RECEIPT_FILE: &str = "core.score.json";

/// Why configuration or publication was refused; static, never echoes input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Receipt directory is relative.
    Relative,
    /// Receipt directory is missing, a symlink or not a directory.
    Directory,
    /// `detector.wasm` is missing, unreadable, empty or too large.
    Detector,
    /// Label is canonical but not `external-input/<run>/<profile>`.
    Label,
    /// The report carries no WASM-collected signal or no engine version.
    NoWasmEvidence,
    /// The report or receipt could not be encoded within its byte policy.
    Encoding,
    /// The receipt already exists or could not be written.
    Write,
}

/// The receipt as written to disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    /// Always `RECEIPT_SCHEMA`.
    pub schema_version: String,
    /// Canonical `external-input/<run>/<profile>` label.
    pub run_label: String,
    /// Run segment of the label.
    pub run_id: String,
    /// Profile segment of the label.
    pub profile_id: String,
    /// First 8 bytes of SHA-256 of the session id, hex; never the raw id.
    pub session_ref: String,
    /// Server-authoritative risk score.
    pub risk_score: f64,
    /// Server-authoritative verdict.
    pub verdict: String,
    /// Hard rule that fired, if any.
    pub hard_rule_fired: String,
    /// Scoring policy version.
    pub policy_version: String,
    /// Public detector path.
    pub detector_path: String,
    /// Hex SHA-256 of the configured detector WASM.
    #[serde(rename = "detectorWasmSha256")]
    pub detector_wasm_hash: String,
    /// Client-reported detector engine version.
    pub engine_version: String,
    /// Number of WASM-collected signals in the report.
    #[serde(rename = "wasmSignalCount")]
    pub wasm_signal_count: usize,
    /// Hex SHA-256 of the encoded report.
    #[serde(rename = "reportSha256")]
    pub report_sha256: String,
    /// Hex SHA-256 over the NUL-joined trace fields.
    #[serde(rename = "scoreTraceSha256")]
    pub score_trace_sha256: String,
    /// Always false: the score is taken, not recomputed.
    pub score_recomputed: bool,
    /// Always true.
    pub server_authoritative: bool,
}

/// Configured receipt channel.
#[derive(Debug, Clone)]
pub struct Writer {
    directory: PathBuf,
    detector_hash: String,
}

impl Writer {
    /// Configure the channel. `None` (or an empty path) disables it.
    pub fn configure(directory: Option<&Path>, web_dir: &Path) -> Result<Option<Self>, Refusal> {
        let Some(directory) = directory.filter(|d| !d.as_os_str().is_empty()) else {
            return Ok(None);
        };
        if !directory.is_absolute() {
            return Err(Refusal::Relative);
        }
        let info = fs::symlink_metadata(directory).map_err(|_| Refusal::Directory)?;
        if !info.is_dir() {
            return Err(Refusal::Directory);
        }
        let detector = read_detector(&web_dir.join("detector.wasm"))?;
        Ok(Some(Self {
            directory: directory.to_path_buf(),
            detector_hash: hex::encode(Sha256::digest(&detector)),
        }))
    }

    /// Hex SHA-256 of the configured detector WASM.
    pub fn detector_hash(&self) -> &str {
        &self.detector_hash
    }

    /// Build the receipt for one scored report. A non-canonical label is not
    /// an external-input run and yields `Ok(None)`.
    pub fn receipt(
        &self,
        label: &str,
        sid: &str,
        result: &ScoreResult,
        report: &SessionReport,
    ) -> Result<Option<Receipt>, Refusal> {
        let Some(label) = canonical_external_input_label(label) else {
            return Ok(None);
        };
        let parts: Vec<&str> = label.split('/').collect();
        let [_, run, profile] = parts.as_slice() else {
            return Err(Refusal::Label);
        };
        if run.is_empty() || profile.is_empty() {
            return Err(Refusal::Label);
        }
        let encoded = serde_json::to_vec(report).map_err(|_| Refusal::Encoding)?;
        let report_sha256 = hex::encode(Sha256::digest(&encoded));
        let wasm = report
            .all_signals()
            .iter()
            .filter(|s| s.collected == Source::Wasm)
            .count();
        if wasm < 1 || report.client.engine_version.is_empty() {
            return Err(Refusal::NoWasmEvidence);
        }
        let session_ref = hex::encode(&Sha256::digest(sid.as_bytes())[..8]);
        let trace = [
            TRACE_SCHEMA,
            label,
            &session_ref,
            &format!("{:.1}", result.risk_score),
            &result.verdict,
            &result.hard_rule_fired,
            &result.policy_version,
            &self.detector_hash,
            &report.client.engine_version,
            &wasm.to_string(),
            &report_sha256,
        ]
        .join("\0");
        Ok(Some(Receipt {
            schema_version: RECEIPT_SCHEMA.into(),
            run_label: label.into(),
            run_id: (*run).into(),
            profile_id: (*profile).into(),
            session_ref,
            risk_score: result.risk_score,
            verdict: result.verdict.clone(),
            hard_rule_fired: result.hard_rule_fired.clone(),
            policy_version: result.policy_version.clone(),
            detector_path: DETECTOR_PATH.into(),
            detector_wasm_hash: self.detector_hash.clone(),
            engine_version: report.client.engine_version.clone(),
            wasm_signal_count: wasm,
            report_sha256,
            score_trace_sha256: hex::encode(Sha256::digest(trace.as_bytes())),
            score_recomputed: false,
            server_authoritative: true,
        }))
    }

    /// Build and publish the receipt exclusively; an existing receipt is
    /// never overwritten.
    pub fn write(
        &self,
        label: &str,
        sid: &str,
        result: &ScoreResult,
        report: &SessionReport,
    ) -> Result<(), Refusal> {
        let Some(receipt) = self.receipt(label, sid, result, report)? else {
            return Ok(());
        };
        let mut raw = serde_json::to_vec(&receipt).map_err(|_| Refusal::Encoding)?;
        raw.push(b'\n');
        if raw.len() > MAX_RECEIPT_BYTES {
            return Err(Refusal::Encoding);
        }
        write_exclusive(&self.directory.join(RECEIPT_FILE), &raw)
    }
}

fn read_detector(path: &Path) -> Result<Vec<u8>, Refusal> {
    let file = File::open(path).map_err(|_| Refusal::Detector)?;
    let mut bytes = Vec::new();
    file.take(MAX_DETECTOR_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Refusal::Detector)?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_DETECTOR_BYTES {
        return Err(Refusal::Detector);
    }
    Ok(bytes)
}

/// Write to a fresh temporary file, sync it, then hard-link it into place so
/// an existing destination is never replaced.
fn write_exclusive(destination: &Path, raw: &[u8]) -> Result<(), Refusal> {
    let directory = destination.parent().ok_or(Refusal::Write)?;
    let mut nonce = [0u8; 8];
    getrandom::fill(&mut nonce).map_err(|_| Refusal::Write)?;
    let temporary = directory.join(format!(".core-score-{}", hex::encode(nonce)));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| Refusal::Write)?;
        file.write_all(raw).map_err(|_| Refusal::Write)?;
        file.sync_all().map_err(|_| Refusal::Write)?;
        drop(file);
        fs::hard_link(&temporary, destination).map_err(|_| Refusal::Write)
    })();
    let _ = fs::remove_file(&temporary);
    result
}

#[cfg(test)]
mod tests;
