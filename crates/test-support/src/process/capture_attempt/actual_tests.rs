//! Ignored actual-process parents: only exact allocated runs under independent
//! outer supervision. Fixed helpers have finite bodies and create no descendants.
use super::*;
use std::{sync::atomic::Ordering, time::Duration};

fn budget(run: Duration) -> CaptureDeadlines {
    let original = Instant::now();
    CaptureDeadlines::from_original(original + run, original + Duration::from_secs(7)).unwrap()
}

// Retain every failed owner. Defer all test assertions until the actual child
// and workers settle. OS stop/final joins can block: outer supervision REQUIRED.
fn settle(mut owner: CaptureAttempt) -> CapturedDirectChild {
    let mut requested_stop = false;
    loop {
        let progress = owner.poll();
        if progress.issues.contains(&CaptureIssue::RunExpired)
            && !progress.direct_child_exited
            && !requested_stop
        {
            let _stop_result = owner.request_stop();
            requested_stop = true;
        }
        match owner.settle_joined() {
            Ok(result) => return result,
            Err(retained) => owner = retained.into_owner(),
        }
        // This does not renew either deadline. A late settlement remains failed.
        thread::sleep(Duration::from_millis(5));
    }
}

fn started(result: Result<CaptureAttempt, CaptureStartFailure>) -> CaptureAttempt {
    match result {
        Ok(owner) => owner,
        Err(CaptureStartFailure::NotStarted(_)) => panic!("fixture was not started"),
        Err(CaptureStartFailure::Retained(retained)) => {
            let result = settle(retained.into_owner());
            assert_eq!(result.io_worker_counts().0, result.io_worker_counts().1);
            panic!("fixture setup failed after retained owner settled");
        }
    }
}

fn normal(mode: &str, limit: usize) -> CaptureAttempt {
    let limits = CaptureLimits::new(limit, limit).unwrap();
    let deadlines = budget(Duration::from_secs(3));
    started(CaptureAttempt::start(
        &mut super::super::tests::fixture(mode),
        Vec::new(),
        limits,
        deadlines,
    ))
}

fn assert_joined(result: &CapturedDirectChild) {
    assert_eq!(result.io_worker_counts(), (3, 3));
    assert!(!result.issues().contains(&CaptureIssue::CleanupExpired));
}

#[test]
#[ignore = "actual subprocess parent; requires explicit allocation and outer supervision"]
fn retained_normal_stdout_stderr_and_actual_joins() {
    let result = settle(normal("both", 4096));
    assert_joined(&result);
    assert!(result.execution_succeeded());
    assert_eq!(result.status().code(), Some(0));
    assert!(String::from_utf8_lossy(result.stdout()).contains("stdout fixture"));
    assert!(String::from_utf8_lossy(result.stderr()).contains("stderr fixture"));
}

#[test]
#[ignore = "synthetic pending-input subprocess parent; requires allocation and outer supervision"]
fn retained_synthetic_pending_stdin_expires_original_run() {
    let limits = CaptureLimits::new(4096, 4096).unwrap();
    let deadlines = budget(Duration::from_secs(1));
    let mut owner = started(CaptureAttempt::start_synthetic(
        &mut super::super::tests::fixture("hang"),
        vec![b'x'; 8 * 1024 * 1024],
        limits,
        deadlines,
    ));
    let mut entered_pending = false;
    loop {
        let progress = owner.poll();
        entered_pending |=
            owner.writer_observation.load(Ordering::Acquire) == 1 && !progress.input_completed;
        if progress.issues.contains(&CaptureIssue::RunExpired) || progress.direct_child_exited {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let observation = owner.writer_observation.clone();
    let result = settle(owner);
    assert_joined(&result);
    assert!(entered_pending);
    assert_eq!(observation.load(Ordering::Acquire), 2);
    assert!(result.issues().contains(&CaptureIssue::RunExpired));
    assert!(result.issues().contains(&CaptureIssue::ForcedStop));
    assert!(!result.execution_succeeded());
    eprintln!(
        "synthetic_input_bytes=8388608 writer_entered_pending=true writer_operation_completed=true joined_io_workers=3 actual_exit_code={:?} production_payload_support=false kernel_blocked_certainty=false tree_coverage=unknown",
        result.status().code(),
    );
}

#[test]
#[ignore = "actual stdout cap subprocess parent; requires allocation and outer supervision"]
fn retained_stdout_cap_is_sticky() {
    let result = settle(normal("flood", 1024));
    assert_joined(&result);
    assert!(result.issues().contains(&CaptureIssue::StdoutFailed));
    assert!(!result.execution_succeeded());
}

#[test]
#[ignore = "actual stderr cap subprocess parent; requires allocation and outer supervision"]
fn retained_stderr_cap_is_sticky() {
    let result = settle(normal("stderr-flood", 1024));
    assert_joined(&result);
    assert!(result.issues().contains(&CaptureIssue::StderrFailed));
    assert!(!result.execution_succeeded());
}

#[test]
#[ignore = "actual stop-classification subprocess parent; requires allocation and outer supervision"]
fn retained_stop_request_preserves_observed_exit_zero() {
    let mut owner = normal("both", 4096);
    loop {
        let progress = owner.poll();
        if progress.direct_child_exited || progress.issues.contains(&CaptureIssue::RunExpired) {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let observed_zero_before_request = owner.status.is_some_and(|status| status.code() == Some(0));
    let stop_result = owner.request_stop();
    let result = settle(owner);
    assert_joined(&result);
    assert!(observed_zero_before_request);
    assert!(stop_result.is_ok());
    assert_eq!(result.status().code(), Some(0));
    assert!(result.issues().contains(&CaptureIssue::ForcedStop));
    assert!(!result.execution_succeeded());
}
