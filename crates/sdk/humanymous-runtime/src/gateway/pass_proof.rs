//! Pass submission model, anti-replay trace digest, the real-event pre-filter
//! and its timing statistics (legacy cmd/server pass_proof.go at baseline
//! 981d25c, ported under owner decision B1 with Go-captured goldens).
//!
//! [`real_event_ok`] is an exact port of `realEventOK`. It rejects a Pass on
//! interaction richness and timing regularity, which conflicts with the
//! accessibility rule that Pass is never gated on motor richness or speed.
//! The conflict is recorded for the owner; the served Pass flow (`pass_flow`)
//! keeps it off unless explicitly configured.
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
    pub moves: i64,
    /// Coalesced pointer sub-samples.
    pub coalesced: i64,
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
    pub keys: i64,
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

/// Legacy `realEventOK`, exact: `Ok` or the Go rejection reason.
///
/// Accepts a pointer channel (at least 5 moves and 20 px of path) or a
/// keyboard channel (at least 3 keys). Rejects untrusted events, too little
/// interaction, near-uniform key timing (4+ intervals, deviation under 0.4 ms)
/// and, for pointer-only input, uniform event timing, missing coalesced
/// sub-samples, a raw stream under 10 samples, non-monotonic raw timestamps
/// and uniform raw spacing. Accessibility conflict: see the module notes.
pub fn real_event_ok(proof: &PassProof) -> Result<(), &'static str> {
    if !proof.trusted {
        return Err("untrusted events");
    }
    let pointer = proof.moves >= 5 && proof.path_len >= 20.0;
    let keyboard = proof.keys >= 3;
    if !pointer && !keyboard {
        return Err("insufficient interaction");
    }
    if keyboard && proof.key_durs.len() >= 4 && stddev(&proof.key_durs) < 0.4 {
        return Err("uniform key timing");
    }
    if pointer && !keyboard {
        if proof.durations.len() >= 5 && stddev(&proof.durations) < 0.5 {
            return Err("uniform event timing");
        }
        if proof.coalesced != 0 && proof.coalesced <= proof.moves {
            return Err("no coalesced sub-samples");
        }
        if proof.raw_t.len() < 10 {
            return Err("missing raw input stream");
        }
        let mut diffs = Vec::with_capacity(proof.raw_t.len() - 1);
        for pair in proof.raw_t.windows(2) {
            let delta = pair[1] - pair[0];
            if delta < 0.0 {
                return Err("non-monotonic raw timestamps");
            }
            diffs.push(delta);
        }
        if stddev(&diffs) < 0.15 {
            return Err("uniform raw sample spacing");
        }
    }
    Ok(())
}

/// Legacy `meanFloats`: arithmetic mean, 0 for no samples.
pub fn mean(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.iter().fold(0.0, |sum, x| sum + x) / samples.len() as f64
}

/// Legacy `allIntegerMs`: every sample within 1e-6 of a whole millisecond;
/// false for no samples.
pub fn all_integer_ms(samples: &[f64]) -> bool {
    !samples.is_empty()
        && samples.iter().all(|x| {
            let fraction = x - x.floor();
            !(fraction > 1e-6 && fraction < 1.0 - 1e-6)
        })
}

/// Legacy `stddev`: population standard deviation, 0 for no samples.
pub fn stddev(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let count = samples.len() as f64;
    let mean = samples.iter().fold(0.0, |sum, x| sum + x) / count;
    let variance = samples.iter().fold(0.0, |sum, x| {
        let delta = x - mean;
        sum + delta * delta
    });
    (variance / count).sqrt()
}

#[cfg(test)]
mod tests;
