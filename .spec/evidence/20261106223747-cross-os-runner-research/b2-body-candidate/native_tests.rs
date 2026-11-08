//! REQUIRED native fixture SOURCE; no local execution or body approval implied.
#![forbid(unsafe_code)]

use super::*;
use std::{io::{Read, Write}, process::{Command, Stdio}, thread, time::{Duration, Instant}};

fn launch(helper: QualificationHelper) -> OwnedTree {
    // current_exe/cwd are locators ONLY. Root must independently admit this exact
    // immutable helper artifact, ordinary actor and ORIGINAL outer supervisor.
    let executable = std::env::current_exe().expect("actual fixture executable locator");
    let cwd = std::env::current_dir().expect("explicit allocated fixture cwd");
    let limits = BudgetLimits::new(Duration::from_secs(5), Duration::from_secs(10), Duration::from_secs(3)).unwrap();
    match OwnedTree::launch(&executable, &cwd, helper, limits) {
        Ok(tree) => tree,
        Err(retained) => panic!("native fixture launch refused: {:?}", retained.observation().first_failure()),
    }
}

fn wait_population_zero(tree: &mut OwnedTree) {
    loop {
        let observation = tree.poll().expect("actual native query");
        if observation.direct_exit().is_some() && observation.active_processes() == Some(0) { return; }
        assert!(Instant::now() < observation.effective_cleanup(), "effective original cleanup expired; retained owner not complete");
        thread::sleep(Duration::from_millis(2));
    }
}

fn terminal(tree: OwnedTree) -> TerminalTree {
    match tree.settle() { Ok(terminal) => terminal, Err(_) => panic!("actual native terminal conjunction missing") }
}

#[test]
#[ignore = "required actual B2 parent: genuine Windows2025 source-bound ordinary fixture with independent original outer owner"]
fn actual_job_list_membership_and_natural_zero_exit_are_observed() {
    let mut tree = launch(QualificationHelper::ExitZero);
    wait_population_zero(&mut tree);
    let result = terminal(tree);
    assert_eq!(result.observation().direct_exit(), Some(0));
    assert_eq!(result.observation().active_processes(), Some(0));
    assert!(result.execution_succeeded());
}

#[test]
#[ignore = "required actual B2 parent: terminal259 is valid only after actual process signal"]
fn actual_signaled_exit_259_is_terminal_and_not_still_active() {
    let mut tree = launch(QualificationHelper::Exit259);
    wait_population_zero(&mut tree);
    let result = terminal(tree);
    assert_eq!(result.observation().direct_exit(), Some(259));
    assert!(!result.execution_succeeded());
}

#[test]
#[ignore = "required actual B2 parent: ordinary CreateProcess descendant keeps Job nonempty after direct exit"]
fn actual_direct_exit_does_not_complete_a_live_descendant_job() {
    let mut tree = launch(QualificationHelper::LeaveDescendant);
    loop {
        let observation = tree.poll().unwrap();
        if observation.direct_exit() == Some(0) {
            assert!(observation.active_processes().is_some_and(|count| count > 0));
            break;
        }
        assert!(Instant::now() < observation.original_instants().1);
        thread::sleep(Duration::from_millis(2));
    }
    let retained = match tree.settle() { Err(retained) => retained, Ok(_) => panic!("direct exit falsely accepted as Job0") };
    let mut tree = retained.into_owner();
    tree.request_stop().unwrap();
    wait_population_zero(&mut tree);
    assert!(!terminal(tree).execution_succeeded());
}

#[test]
#[ignore = "required actual B2 parent: recovery moves SAME owners without fresh budget or PID lookup"]
fn actual_retained_recovery_keeps_original_pair_and_first_stop() {
    let tree = launch(QualificationHelper::Wait);
    let original = tree.observation().original_instants();
    let retained = match tree.settle() { Err(retained) => retained, Ok(_) => panic!("running helper settled") };
    let mut tree = retained.into_owner();
    tree.request_stop().unwrap();
    let first = tree.observation().first_failure().unwrap();
    tree.request_stop().unwrap();
    assert_eq!(tree.observation().first_failure().unwrap().at, first.at);
    assert_eq!(tree.observation().original_instants(), original);
    wait_population_zero(&mut tree);
    let result = terminal(tree);
    assert!(!result.execution_succeeded());
    assert_eq!(result.observation().original_instants(), original);
}

#[test]
#[ignore = "required actual B2 parent: controlled post-create validation refusal retains real Job/process/thread owners"]
fn actual_post_create_validation_refusal_keeps_owned_cleanup_target() {
    let tree = launch(QualificationHelper::Wait);
    let original = tree.observation().original_instants();
    let retained = tree.reject_after_actual_create();
    assert_eq!(retained.observation().first_failure().unwrap().issue, Issue::Containment);
    let mut tree = retained.into_owner();
    tree.request_stop().unwrap();
    wait_population_zero(&mut tree);
    let result = terminal(tree);
    assert_eq!(result.observation().original_instants(), original);
    assert_eq!(result.observation().first_failure().unwrap().issue, Issue::Containment);
    assert!(!result.execution_succeeded());
}

#[test]
#[ignore = "required actual B2 parent: separate outer Job observes nested owner death, never duplicates inner Job handle"]
fn actual_owner_death_kills_nested_job_without_forging_join_proof() {
    let mut tree = launch(QualificationHelper::OwnerDies);
    wait_population_zero(&mut tree);
    let result = terminal(tree);
    assert_eq!(result.observation().direct_exit(), Some(71));
    assert!(!result.execution_succeeded());
    // No reader/worker/journal/provider fact is produced by this minimum profile.
}

#[test]
#[ignore = "required actual B2 parent: run separately with console-attached and redirected parent; outer fixture verifies no leaked marker"]
fn actual_detached_cui_helper_does_not_consume_parent_standard_input() {
    let mut tree = launch(QualificationHelper::ProbeStandardStreams);
    wait_population_zero(&mut tree);
    assert!(terminal(tree).execution_succeeded());
    // No marker on PARENT redirected streams must be checked by independent
    // outer fixture; this test alone does not establish that second observation.
}

// Subprocess helpers: NOT standalone required-parent passes or native authority.
#[test]
#[ignore = "B2 subprocess helper only; exact closed selector, do not run standalone"]
fn child_exit_zero() {}

#[test]
#[ignore = "B2 subprocess helper only; exact closed selector, do not run standalone"]
fn child_exit_259() { std::process::exit(259); }

#[test]
#[ignore = "B2 subprocess helper only; exact closed selector, do not run standalone"]
fn child_wait() { thread::sleep(Duration::from_secs(60)); }

#[test]
#[ignore = "B2 subprocess helper only; actual ordinary child inherits Job, no broker spawn"]
fn child_leaves_descendant() {
    let executable = std::env::current_exe().unwrap();
    let mut child = Command::new(executable).args(["--exact", "windows_supervisor::native_tests::child_wait", "--ignored", "--test-threads=1"])
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    assert!(child.try_wait().unwrap().is_none());
    // Deliberate fixture: direct parent exits with an actual live descendant.
    // Root owns Job; this Pending poll is NOT a descendant reap claim.
    std::process::exit(0);
}

#[test]
#[ignore = "B2 subprocess helper only; OS owner death closes last inner Job handle"]
fn child_owner_dies() {
    let _inner = launch(QualificationHelper::Wait);
    // Skip Rust Drop deliberately. Windows closes owned handles on process exit;
    // separate actual outer Job population observation is still REQUIRED.
    std::process::exit(71);
}

#[test]
#[ignore = "B2 subprocess helper only; ordinary std IO does not mint a console/native proof"]
fn child_probes_standard_streams() {
    let mut byte = [0];
    let count = std::io::stdin().read(&mut byte).unwrap_or(0);
    if count != 0 { std::process::exit(73); }
    let _stdout = std::io::stdout().write_all(b"b2-parent-stream-leak-marker");
    let _stderr = std::io::stderr().write_all(b"b2-parent-stream-leak-marker");
}
