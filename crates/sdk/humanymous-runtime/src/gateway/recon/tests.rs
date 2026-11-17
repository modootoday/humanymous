//! Unit coverage for legacy internal/gate/recon.go `constantFloor` (baseline
//! 981d25c had no direct Go test; the cases pin its arithmetic).
use super::*;

const MS: Duration = Duration::from_millis(1);

#[test]
fn remaining_is_floor_minus_elapsed() {
    let start = Duration::from_secs(100);
    assert_eq!(floor_remaining(start, 50 * MS, start), Some(50 * MS));
    assert_eq!(
        floor_remaining(start, 50 * MS, start + 20 * MS),
        Some(30 * MS)
    );
}

#[test]
fn nothing_remains_once_the_floor_has_elapsed() {
    let start = Duration::from_secs(100);
    assert_eq!(floor_remaining(start, 50 * MS, start + 50 * MS), None);
    assert_eq!(floor_remaining(start, 50 * MS, start + 80 * MS), None);
    assert_eq!(floor_remaining(start, Duration::ZERO, start), None);
}

#[test]
fn a_clock_behind_start_extends_the_wait_like_go() {
    let start = Duration::from_secs(100);
    assert_eq!(
        floor_remaining(start, 50 * MS, start - 10 * MS),
        Some(60 * MS)
    );
    assert_eq!(
        floor_remaining(Duration::MAX, Duration::MAX, Duration::ZERO),
        Some(Duration::MAX)
    );
}

#[test]
fn constant_floor_blocks_until_the_floor() {
    let start = Instant::now();
    constant_floor(start, 20 * MS);
    assert!(start.elapsed() >= 20 * MS);

    let floor = Duration::from_secs(1);
    if let Some(late) = Instant::now().checked_sub(2 * floor) {
        let before = Instant::now();
        constant_floor(late, floor);
        assert!(before.elapsed() < floor, "an elapsed floor must not block");
    }
}
