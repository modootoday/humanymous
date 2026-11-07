//! Actual direct-child boundary, requiring independent outer native supervision.
//! No SCM/accounts, pipe EOF, worker-join, native-close or tree-cleanup claim.
#![cfg(all(feature = "runtime-service-pilot", feature = "service-pilot"))]

use humanymous_cli::runtime_host::watchdog_fixture::WRITER_ENTRY;
use humanymous_test_support::process::{CaptureDeadlines, CaptureIssue, RetainedDirectPipeAttempt};
use std::{
    process::Command,
    sync::Mutex,
    time::{Duration, Instant},
};

// Preserve unresolved actual owners for the independently supervised test
// process/controller. No Drop, panic or failed assertion turns this into cleanup.
static RECOVERY: Mutex<Vec<RetainedDirectPipeAttempt>> = Mutex::new(Vec::new());

fn fail_owned(mut owner: RetainedDirectPipeAttempt, cleanup: Instant, reason: &'static str) -> ! {
    // This borrowed shared stop request can itself block: the external native
    // controller, not this loop or a detached thread, owns that final ceiling.
    let _ = owner.request_stop();
    while Instant::now() < cleanup {
        if owner.poll().status().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    match owner.settle_closing_pipes() {
        Ok(_) => panic!("{reason}; recovery observed direct child exit, fixture failed"),
        Err(failure) => {
            RECOVERY
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(failure.into_owner());
            panic!("{reason}; actual owner retained, cleanup unknown; outer recovery required");
        }
    }
}

#[test]
#[ignore = "requires allocated independent native outer supervisor; prefix/spawn/stop may block"]
fn final_delivery_watchdog_with_retained_unread_pipes() {
    // Routing opt-in only: this string does NOT prove native supervision or
    // genuine hosted identity. The external run/source/controller evidence does.
    assert_eq!(
        std::env::var("HUMANYMOUS_OWNED_WATCHDOG_BOUNDARY").as_deref(),
        Ok("1")
    );
    let original = Instant::now();
    let run = original + Duration::from_secs(8);
    let cleanup = original + Duration::from_secs(12);
    for (case, expected_exit) in [
        ("positive", 0),
        ("blocked-stdout", 74),
        ("blocked-stderr", 74),
        ("late-completion", 74),
    ] {
        let deadlines = CaptureDeadlines::from_original(run, cleanup).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_humanymous-service-fixture"));
        command.args(["--watchdog-boundary", case]);
        let mut owner = RetainedDirectPipeAttempt::start(&mut command, deadlines)
            .unwrap_or_else(|_| panic!("shared spawn failed before returning an actual owner"));
        owner.close_input();
        let marker = if case == "blocked-stderr" {
            owner.read_stderr_prefix(WRITER_ENTRY)
        } else {
            owner.read_stdout_prefix(WRITER_ENTRY)
        };
        if marker.is_err() {
            fail_owned(owner, cleanup, "writer-entry observation failed");
        }
        // Only the fixed prefix was read; pressure/report remainder stays held
        // unread. Entry is not a claim that the kernel blocked a particular write.
        loop {
            let progress = owner.poll();
            if progress.retained_pipe_ends() != [false, true, true] {
                fail_owned(owner, cleanup, "required unread pipe ownership changed");
            }
            if progress.status().is_some() {
                break;
            }
            if !progress.issues().is_empty() || Instant::now() >= run {
                fail_owned(
                    owner,
                    cleanup,
                    "original run deadline or observation failed",
                );
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let closed = match owner.settle_closing_pipes() {
            Ok(closed) => closed,
            Err(failure) => fail_owned(failure.into_owner(), cleanup, "direct child unsettled"),
        };
        assert!(closed.pid() > 0);
        assert_eq!(closed.status().code(), Some(expected_exit), "{case}");
        assert_eq!(closed.released_pipe_ends(), 3);
        assert_eq!(
            closed.prefixes_observed(),
            if case == "blocked-stderr" {
                [false, true]
            } else {
                [true, false]
            }
        );
        let expected_issues = if expected_exit == 0 {
            vec![]
        } else {
            vec![CaptureIssue::UnsuccessfulExit]
        };
        assert_eq!(closed.issues(), expected_issues.as_slice(), "{case}");
        assert!(
            RECOVERY
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_empty()
        );
        println!(
            "{}",
            serde_json::json!({
                "schema":"humanymous.watchdog-boundary-observation.v1", "case":case,
            "actual_exit":closed.status().code(),"writer_entry_observed":closed.prefixes_observed().into_iter().any(|observed| observed),
            "parent_pipe_ends_released":closed.released_pipe_ends(),"runtime_completion_simulated":true,
                "kernel_blocked_write_proven":false,"tree_cleanup_proven":false,
                "product_qualified":false
            })
        );
    }
}
