//! Pass submission model and anti-replay trace digest (legacy cmd/server
//! pass_proof.go at baseline 981d25c).
//!
//! Only the wire model and the digest are ported. The legacy real-event
//! pre-filter (`realEventOK`) gates a Pass on pointer and keyboard timing
//! microstructure; whether and how such a gate may exist is an accessibility
//! decision reserved for the owner, so it is deliberately not ported here.
use crate::gateway::pass_store::TraceDigest;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write;

/// Client Pass submission, with the legacy JSON field names.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PassProof {
    /// Challenge bucket the solve claims.
    pub bucket: u64,
    /// Nonce of the issued challenge instance.
    pub challenge_nonce: String,
    /// Rate-limited attestation token, when required.
    pub attest_token: String,
    /// Per-row shifts of the solution.
    pub offsets: Vec<i64>,
    /// Client claim that every event was trusted.
    pub trusted: bool,
    /// Distinct pointer moves.
    pub moves: u64,
    /// Coalesced pointer sub-samples.
    pub coalesced: u64,
    /// Inter-event intervals in milliseconds.
    pub durations: Vec<f64>,
    /// Pointer path length in pixels.
    pub path_len: f64,
    /// Raw coalesced sample timestamps in milliseconds.
    pub raw_t: Vec<f64>,
    /// Touch or pen pressure samples.
    pub pressures: Vec<f64>,
    /// `mouse`, `touch` or `pen`.
    pub pointer_type: String,
    /// Distinct navigation keydowns (keyboard lane).
    pub keys: u64,
    /// Inter-key intervals in milliseconds.
    pub key_durs: Vec<f64>,
    /// Device-motion magnitude samples.
    pub motion: Vec<f64>,
}

/// Fingerprint the raw motor evidence for anti-replay, byte-identical to Go.
///
/// Timings are quantized to 1 ms and pressures to 0.05 before hashing, so an
/// exact or sub-quantum replay collides even around a fresh challenge. Offsets
/// and the nonce are excluded on purpose. Deliberate difference: a non-finite
/// or out-of-range sample returns `None` (Go converted it to an
/// implementation-defined integer).
pub fn trace_digest(proof: &PassProof) -> Option<TraceDigest> {
    let mut text = String::new();
    for (samples, quantum) in [
        (&proof.raw_t, 1.0),
        (&proof.durations, 1.0),
        (&proof.pressures, 0.05),
        (&proof.key_durs, 1.0),
    ] {
        for sample in samples {
            let q = (sample / quantum).round();
            if !q.is_finite() || q.abs() >= 9.0e18 {
                return None;
            }
            let _ = write!(text, "{},", q as i64);
        }
        text.push('|');
    }
    Some(TraceDigest(Sha256::digest(text.as_bytes()).into()))
}

#[cfg(test)]
mod tests;
