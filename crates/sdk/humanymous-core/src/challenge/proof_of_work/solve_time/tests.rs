//! Ports legacy cmd/server/powhandler_launder_test.go (baseline 981d25c) at
//! kernel level, plus the floor and boundary cases.
use super::*;
use crate::challenge::proof_of_work::{issue, solve, verify};

const MS: fn(u64) -> Duration = Duration::from_millis;

#[test]
fn floor_matches_go() {
    // Go: float64(1<<d) / 3_000_000 seconds, truncated to a Duration.
    assert_eq!(plausible_browser_solve(16).unwrap().as_nanos(), 21_845_333);
    assert_eq!(plausible_browser_solve(0).unwrap().as_nanos(), 333);
    assert!(plausible_browser_solve(63).is_some());
    assert_eq!(plausible_browser_solve(64), None);
    assert_eq!(plausible_browser_solve(-1), None);
}

#[test]
fn too_fast_cannot_be_laundered_by_delayed_resubmit() {
    let key = b"test-master-key-0123456789abcdef";
    let sid = "launder-sid";
    let difficulty = 16;
    let bucket = 1_000;
    let challenge = issue(key, sid, difficulty, bucket);
    let nonce = solve(&challenge.seed, difficulty, 1 << 24).expect("solvable");
    let verified = verify(key, sid, difficulty, bucket, bucket, &nonce);
    assert!(verified);

    let mut state = SolveTime::default();
    let start = Duration::from_secs(10);
    state.issue(start, difficulty);
    assert_eq!(
        state.submit(verified, start + Duration::from_micros(50)),
        Outcome::TooFast { elapsed_ms: 0 },
        "instant submit is too fast"
    );
    let floor = plausible_browser_solve(difficulty).unwrap();
    assert!(
        matches!(
            state.submit(verified, start + floor + MS(50)),
            Outcome::TooFast { .. }
        ),
        "a delayed resubmit stays flagged"
    );
    assert!(state.too_fast());
}

#[test]
fn plausible_solve_is_solved_and_boundary_is_inclusive() {
    let mut state = SolveTime::default();
    state.issue(Duration::ZERO, 16);
    let floor = plausible_browser_solve(16).unwrap();
    assert_eq!(
        state.submit(true, floor),
        Outcome::Solved,
        "elapsed == floor is plausible"
    );
    assert!(!state.too_fast());
}

#[test]
fn unverified_or_unissued_submissions_change_nothing() {
    let mut state = SolveTime::default();
    assert_eq!(
        state.submit(true, MS(1)),
        Outcome::Rejected,
        "no challenge issued"
    );
    state.issue(Duration::ZERO, 0);
    assert_eq!(
        state.submit(true, MS(1)),
        Outcome::Rejected,
        "difficulty 0 never verifies"
    );
    state.issue(Duration::ZERO, 16);
    assert_eq!(state.submit(false, Duration::ZERO), Outcome::Rejected);
    assert!(!state.too_fast(), "an invalid nonce records no tell");
    assert_eq!(state.difficulty(), Some(16));
}

#[test]
fn unrepresentable_difficulty_and_clock_regression_fail_closed() {
    let mut huge = SolveTime::default();
    huge.issue(Duration::ZERO, 80);
    assert!(matches!(
        huge.submit(true, Duration::from_secs(1_000)),
        Outcome::TooFast { .. }
    ));
    let mut regressed = SolveTime::default();
    regressed.issue(Duration::from_secs(5), 16);
    assert_eq!(
        regressed.submit(true, Duration::from_secs(1)),
        Outcome::TooFast { elapsed_ms: 0 },
        "a clock regression counts as an instant solve"
    );
}
