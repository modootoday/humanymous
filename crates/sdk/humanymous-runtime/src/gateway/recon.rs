//! Constant decision-latency floor (legacy internal/gate/recon.go
//! `constantFloor` at baseline 981d25c, SoT-21 §8, HR-30).
//!
//! Blocking until at least `floor` has elapsed since the request started makes
//! the visible latency of allow, challenge and deny converge, so timing does
//! not leak which decision path ran. This is coarse jitter defense, not a
//! constant-time guarantee. The sweep detector from the same Go file lives in
//! `network::history::sweep`; applying the floor on the response path belongs
//! to the serving composition.
use std::time::{Duration, Instant};

/// Time still to wait so that `floor` has elapsed between `start` and `now`;
/// `None` when nothing remains.
///
/// Matches Go `floor - now.Sub(start)` exactly, including a `now` earlier than
/// `start` (the gap is added to the wait) and saturating instead of
/// overflowing.
pub fn floor_remaining(start: Duration, floor: Duration, now: Duration) -> Option<Duration> {
    let remaining = match now.checked_sub(start) {
        Some(elapsed) => floor.saturating_sub(elapsed),
        None => floor.saturating_add(start - now),
    };
    (!remaining.is_zero()).then_some(remaining)
}

/// Block the calling thread until at least `floor` has elapsed since `start`
/// on the monotonic clock.
pub fn constant_floor(start: Instant, floor: Duration) {
    let elapsed = Instant::now().saturating_duration_since(start);
    if let Some(wait) = floor_remaining(Duration::ZERO, floor, elapsed) {
        std::thread::sleep(wait);
    }
}

#[cfg(test)]
mod tests;
