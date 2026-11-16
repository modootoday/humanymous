//! Solve-time analysis for PoW submissions (legacy cmd/server powhandler.go,
//! baseline 981d25c, SoT-15).
//!
//! A valid solution returned faster than a browser JS solver could compute it
//! reveals a native or GPU solver. The observation is permanent for the
//! session: a later resubmit of the same valid nonce after a plausible delay
//! never earns the `pow.solved` upgrade (wargame round R1). The floor is the
//! frozen Go constant; this module only ports it.
use std::time::Duration;

/// Conservative ceiling on browser JS hashing throughput, hashes per second.
pub const CEILING_HASHES_PER_SEC: f64 = 3_000_000.0;

/// Fastest plausible browser solve at `difficulty`; `None` outside 0..=63.
pub fn plausible_browser_solve(difficulty: i32) -> Option<Duration> {
    let shift = u32::try_from(difficulty).ok().filter(|d| *d < 64)?;
    let attempts = (1u64 << shift) as f64;
    Some(Duration::from_secs_f64(attempts / CEILING_HASHES_PER_SEC))
}

/// Result of one verified-or-not submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The nonce did not verify, or no challenge was issued; nothing changes.
    Rejected,
    /// Verified, but this session has solved faster than a browser could;
    /// carries the elapsed milliseconds for the `l7.pow.too_fast` signal.
    TooFast {
        /// Milliseconds since issuance (0 if issuance time is unknown).
        elapsed_ms: u64,
    },
    /// Verified at a plausible pace; the `pow.solved` upgrade may apply.
    Solved,
}

/// Per-session PoW solve-time state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SolveTime {
    issued: Option<(Duration, i32)>,
    too_fast: bool,
}

impl SolveTime {
    /// Record that a challenge at `difficulty` was issued at `now`.
    pub fn issue(&mut self, now: Duration, difficulty: i32) {
        self.issued = Some((now, difficulty));
    }

    /// Server-held issue-time difficulty, if a challenge was issued.
    pub fn difficulty(&self) -> Option<i32> {
        self.issued.map(|(_, d)| d)
    }

    /// Whether the session ever revealed a super-human solve.
    pub fn too_fast(&self) -> bool {
        self.too_fast
    }

    /// Decide a submission whose nonce `verified` against the issue-time
    /// difficulty, at `now`. A too-fast observation is sticky.
    pub fn submit(&mut self, verified: bool, now: Duration) -> Outcome {
        let Some((issued, difficulty)) = self.issued.filter(|(_, d)| *d > 0) else {
            return Outcome::Rejected;
        };
        if !verified {
            return Outcome::Rejected;
        }
        let elapsed = now.saturating_sub(issued);
        // An unrepresentable difficulty has no plausible floor: fail closed.
        let too_fast_now = plausible_browser_solve(difficulty).is_none_or(|floor| elapsed < floor);
        if too_fast_now || self.too_fast {
            self.too_fast = true;
            return Outcome::TooFast {
                elapsed_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
            };
        }
        Outcome::Solved
    }
}

#[cfg(test)]
mod tests;
