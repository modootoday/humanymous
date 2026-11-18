//! Decision-latency floor against timing oracles, ported from legacy
//! `internal/gate/recon.go` `constantFloor` (oracle 981d25c, SoT-21 section 8,
//! HR-30). The sweep detector half of that file is
//! `network::history::sweep`.
//!
//! A refusal (block or challenge) waits until at least the floor has elapsed
//! since the decision started, so the visible latency of the decision paths
//! converges and does not leak which checks ran. This is coarse jitter
//! defence, not a constant-time guarantee. Composition into the gate response
//! path is not done here.
use std::time::Duration;
use tokio::time::Instant;

/// Go `decFloor` default (2 ms).
pub const DECISION_FLOOR: Duration = Duration::from_millis(2);

/// Time still owed before `floor` has elapsed since `start` (zero once it
/// has, or when the clock reads earlier than `start`).
pub fn remaining(start: Instant, floor: Duration, now: Instant) -> Duration {
    floor.saturating_sub(now.saturating_duration_since(start))
}

/// Go `constantFloor`: wait until at least `floor` has elapsed since `start`.
pub async fn constant_floor(start: Instant, floor: Duration) {
    if !remaining(start, floor, Instant::now()).is_zero() {
        tokio::time::sleep_until(start + floor).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaining_is_the_unelapsed_part_of_the_floor() {
        let start = Instant::now();
        let floor = Duration::from_millis(10);
        assert_eq!(remaining(start, floor, start), floor);
        assert_eq!(
            remaining(start, floor, start + Duration::from_millis(4)),
            Duration::from_millis(6)
        );
        assert_eq!(remaining(start, floor, start + floor), Duration::ZERO);
        assert_eq!(
            remaining(start, floor, start + Duration::from_secs(1)),
            Duration::ZERO
        );
        // A clock reading before the start owes the whole floor, never more.
        let later = start + Duration::from_millis(50);
        assert_eq!(remaining(later, floor, start), floor);
        assert_eq!(remaining(start, Duration::ZERO, start), Duration::ZERO);
    }

    #[tokio::test]
    async fn constant_floor_waits_out_the_floor_and_never_extends_it() {
        let floor = Duration::from_millis(20);
        let start = Instant::now();
        constant_floor(start, floor).await;
        assert!(start.elapsed() >= floor);
        // A decision that already took longer than the floor is not delayed.
        let past = Instant::now() - Duration::from_millis(50);
        let before = Instant::now();
        constant_floor(past, floor).await;
        assert!(before.elapsed() < Duration::from_secs(5));
        assert_eq!(remaining(past, floor, before), Duration::ZERO);
        assert_eq!(DECISION_FLOOR, Duration::from_millis(2));
    }
}
