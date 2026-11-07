//! Ignored controlled outer-fault probes. No fixture children are spawned here.
use super::*;
use std::io::Write;

fn marker(text: &str) {
    let mut output = io::stdout().lock();
    writeln!(output, "\n{text}").unwrap();
    output.flush().unwrap();
}

#[test]
#[ignore = "controlled missing-grant parent; explicit outer supervision required"]
fn missing_grant_refuses_before_workload() {
    let error = await_controlled_fixture_grant(
        "testing::process::controlled_barrier::actual_tests::missing_grant_refuses_before_workload",
    )
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    // No workload/child constructor occurs on this refused path.
    marker(CONTROLLED_MISSING_GRANT_MARKER);
}

#[test]
#[ignore = "controlled live-parent timeout probe; must be owned by outer Job"]
fn parent_remains_running_until_original_outer_deadline() {
    await_controlled_fixture_grant(
        "testing::process::controlled_barrier::actual_tests::parent_remains_running_until_original_outer_deadline",
    )
    .expect("controlled outer grant required");
    marker(CONTROLLED_LIVE_PARENT_MARKER);
    // Finite fallback exceeds original outer10s/15s, so natural completion must
    // never qualify the forced live-parent case. No descendants or IO workers.
    std::thread::sleep(Duration::from_secs(30));
    panic!("outer controller failed to terminate the declared live parent");
}
