use super::*;
use std::time::Duration;

#[test]
fn original_thresholds_are_conserved_and_late_completion_is_not_renewed() {
    let start = Instant::now();
    let run = start + Duration::from_secs(3);
    let cleanup = start + Duration::from_secs(4);
    let mut deadlines = CaptureDeadlines::from_original(run, cleanup).unwrap();
    assert_eq!(deadlines.observe(start, false, false), [None, None]);
    assert_eq!(
        deadlines.observe(run, true, false),
        [Some(CaptureIssue::RunExpired), None]
    );
    assert_eq!(
        deadlines.observe(cleanup, true, true),
        [
            Some(CaptureIssue::RunExpired),
            Some(CaptureIssue::CleanupExpired)
        ]
    );
    assert_eq!(deadlines.run, run);
    assert_eq!(deadlines.cleanup, cleanup);
    assert!(CaptureDeadlines::from_original(cleanup, run).is_err());
}

#[test]
fn child_and_output_completion_do_not_substitute_for_actual_joins() {
    let start = Instant::now();
    let run = start + Duration::from_secs(3);
    let cleanup = start + Duration::from_secs(4);
    let mut deadlines = CaptureDeadlines::from_original(run, cleanup).unwrap();
    assert_eq!(deadlines.observe(start, true, false), [None, None]);
    assert_eq!(
        deadlines.observe(cleanup, true, false),
        [None, Some(CaptureIssue::CleanupExpired)]
    );
    // Late actual join completion cannot erase the missed cleanup observation.
    assert_eq!(
        deadlines.observe(cleanup, true, true),
        [None, Some(CaptureIssue::CleanupExpired)]
    );
}

#[test]
fn timely_observed_completion_stays_timely_without_a_new_budget() {
    let start = Instant::now();
    let run = start + Duration::from_secs(3);
    let cleanup = start + Duration::from_secs(4);
    let mut deadlines = CaptureDeadlines::from_original(run, cleanup).unwrap();
    assert_eq!(deadlines.observe(start, true, true), [None, None]);
    assert_eq!(
        deadlines.observe(cleanup + Duration::from_secs(1), true, true),
        [None, None]
    );
}

#[test]
fn allocation_caps_and_exact_eof_fail_closed_without_starting_a_process() {
    assert!(CaptureLimits::new(0, 1).is_err());
    assert!(CaptureLimits::new(1, 0).is_err());
    assert!(CaptureLimits::new(usize::MAX, 1).is_err());
    assert!(CaptureLimits::new(1, usize::MAX).is_err());
    assert!(CaptureLimits::new(4096, 4096).is_ok());
    assert_eq!(read_capped(&b"abc"[..], 3).unwrap(), b"abc");
    assert!(read_capped(&b"abcd"[..], 3).is_err());
}
