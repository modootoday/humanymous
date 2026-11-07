//! Fixed cooperative capture/CLI parents, independently owned by a real Windows Job.
//! No arbitrary/untrusted launch guarantee, account/SCM operation or privilege.
use crate::actor::ControlledFixtureJob;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};
mod outcome;
use outcome::{ExpectedOutcome, Facts, classify};

const CASES: &[(&str, &str)] = &[
    ("normal", "retained_normal_stdout_stderr_and_actual_joins"),
    (
        "pending-input",
        "retained_synthetic_pending_stdin_expires_original_run",
    ),
    ("stdout-cap", "retained_stdout_cap_is_sticky"),
    ("stderr-cap", "retained_stderr_cap_is_sticky"),
    (
        "stop-after-zero",
        "retained_stop_request_preserves_observed_exit_zero",
    ),
];
struct Profile {
    selector: String,
    executable_prefix: &'static str,
    expected: ExpectedOutcome,
}
fn profile(case: &str) -> Option<Profile> {
    let fault = match case {
        "outer-missing-grant" => Some((
            "missing_grant_refuses_before_workload",
            ExpectedOutcome::MissingGrantRefusal,
        )),
        "outer-live-timeout" => Some((
            "live_parent_waits_for_original_outer_deadline",
            ExpectedOutcome::LiveDeadlineTermination,
        )),
        _ => None,
    };
    if let Some((name, expected)) = fault {
        return Some(Profile {
            selector: format!("testing::process::controlled_barrier::actual_tests::{name}"),
            executable_prefix: "humanymous_platform-",
            expected,
        });
    }
    if case == "cli-watchdog" {
        return Some(Profile {
            selector: "final_delivery_watchdog_with_retained_unread_pipes".into(),
            executable_prefix: "watchdog_boundary-",
            expected: ExpectedOutcome::ParentPass,
        });
    }
    let (_, name) = CASES.iter().find(|(id, _)| *id == case)?;
    Some(Profile {
        selector: format!("testing::process::capture_attempt::actual_tests::{name}"),
        executable_prefix: "humanymous_platform-",
        expected: ExpectedOutcome::ParentPass,
    })
}
fn executable_name(name: &str, profile: &Profile) -> bool {
    name.strip_prefix(profile.executable_prefix)
        .and_then(|tail| tail.strip_suffix(".exe"))
        .is_some_and(|hash| {
            hash.len() == 16
                && hash
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
}
// Preserve unresolved native owners until the controller's OS boundary exits.
// Job Drop requests kill-on-close; it never manufactures convergence evidence.
thread_local! {
    // Stay on the owning controller thread; no unsafe Send for native HANDLEs.
    static RECOVERY: RefCell<Vec<OwnedController>> = const { RefCell::new(Vec::new()) };
}

struct Options {
    exe: PathBuf,
    digest: String,
    case: String,
    work: PathBuf,
}
fn options(mut args: impl Iterator<Item = String>) -> io::Result<Options> {
    let mut values = std::collections::BTreeMap::new();
    while let Some(flag) = args.next() {
        if !["--test-exe", "--expected-sha256", "--case", "--work"].contains(&flag.as_str())
            || values.contains_key(&flag)
        {
            return Err(refused());
        }
        values.insert(flag, args.next().ok_or_else(refused)?);
    }
    let mut take = |flag: &str| values.remove(flag).ok_or_else(refused);
    let result = Options {
        exe: take("--test-exe")?.into(),
        digest: take("--expected-sha256")?,
        case: take("--case")?,
        work: take("--work")?.into(),
    };
    if !result.exe.is_absolute()
        || !result.work.is_absolute()
        || profile(&result.case).is_none()
        || result.digest.len() != 64
        || !result
            .digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(refused());
    }
    let name = result
        .exe
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(refused)?;
    if !executable_name(name, &profile(&result.case).ok_or_else(refused)?) {
        return Err(refused());
    }
    Ok(result)
}
fn refused() -> io::Error {
    io::Error::other("fixed controlled fixture contract refused")
}
fn new_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

struct OwnedController {
    child: Child,
    job: ControlledFixtureJob,
    assigned: bool,
    exit: Option<ExitStatus>,
}
#[derive(Default)]
struct CleanupFacts {
    termination_requested: bool,
    termination_failed: bool,
    parent_exit_observed: bool,
    job_empty_observed: bool,
    within_deadline: bool,
    observation_failed: bool,
    active_before_termination: Option<u32>,
}
impl OwnedController {
    fn observe(&mut self) -> io::Result<()> {
        if self.exit.is_none() {
            self.exit = self.child.try_wait()?;
        }
        Ok(())
    }
    fn contain(&mut self, cleanup: Instant) -> CleanupFacts {
        let mut facts = CleanupFacts {
            parent_exit_observed: self.exit.is_some(),
            ..CleanupFacts::default()
        };
        if Instant::now() >= cleanup {
            return facts;
        }
        if self.assigned {
            match self.job.active() {
                Ok(active) => facts.active_before_termination = Some(active),
                Err(_) => facts.observation_failed = true,
            }
        }
        facts.termination_requested = true;
        facts.termination_failed = if self.assigned {
            self.job.terminate().is_err()
        } else {
            self.child.kill().is_err()
        };
        while Instant::now() < cleanup {
            if self.observe().is_err() {
                facts.observation_failed = true;
                return facts;
            }
            facts.parent_exit_observed = self.exit.is_some();
            // Never reuse an earlier empty observation if this query fails.
            facts.job_empty_observed = false;
            match self.job.active() {
                Ok(active) => facts.job_empty_observed = active == 0,
                Err(_) => {
                    facts.observation_failed = true;
                    return facts;
                }
            }
            if facts.parent_exit_observed && facts.job_empty_observed {
                facts.within_deadline = Instant::now() < cleanup;
                return facts;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        facts
    }
}

fn probe_marker(path: &Path, expected: &str) -> io::Result<bool> {
    const LIMIT: u64 = 64 * 1024;
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > LIMIT {
        return Err(refused());
    }
    let mut bytes = Vec::new();
    File::open(path)?.take(LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT {
        return Err(refused());
    }
    Ok(std::str::from_utf8(&bytes)
        .map_err(|_| refused())?
        .lines()
        .any(|line| line == expected))
}

pub(crate) fn run(args: impl Iterator<Item = String>) -> io::Result<i32> {
    let options = options(args)?;
    let metadata = fs::symlink_metadata(&options.exe)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > 256 * 1024 * 1024
    {
        return Err(refused());
    }
    let mut hasher = Sha256::new();
    let mut input = File::open(&options.exe)?.take(256 * 1024 * 1024 + 1);
    let mut buffer = [0_u8; 65536];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    if format!("{:x}", hasher.finalize()) != options.digest {
        return Err(refused());
    }
    let profile = profile(&options.case).ok_or_else(refused)?;
    let selector = profile.selector;
    let expected = profile.expected;
    fs::create_dir(&options.work)?;
    let stdout = new_file(&options.work.join("parent.stdout.log"))?;
    let stderr = new_file(&options.work.join("parent.stderr.log"))?;
    let mut random = [0_u8; 16];
    getrandom::fill(&mut random).map_err(io::Error::other)?;
    let nonce: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let grant = options.work.join("grant.json");
    let job = ControlledFixtureJob::create()?;
    // Both original thresholds precede the actual process creation. OS spawn,
    // assignment and queries still rely on a responsive kernel, not hard syscall cancellation.
    let original = Instant::now();
    let run_until = original + Duration::from_secs(10);
    let cleanup_until = original + Duration::from_secs(15);
    let child = Command::new(&options.exe)
        .args(["--exact", &selector, "--ignored", "--nocapture"])
        .env("HUMANYMOUS_CAPTURE_OUTER_GRANT", &grant)
        .env("HUMANYMOUS_CAPTURE_OUTER_NONCE", &nonce)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()?;
    let mut owner = OwnedController {
        child,
        job,
        assigned: false,
        exit: None,
    };
    let pid = owner.child.id();
    let mut granted = false;
    let mut run_expired = false;
    let mut observation_error = false;
    let mut live_at_run_deadline = false;
    let setup = (|| -> io::Result<()> {
        owner.job.assign(&owner.child)?;
        owner.assigned = true;
        if owner.job.active()? == 0 || Instant::now() >= run_until {
            return Err(refused());
        }
        if expected == ExpectedOutcome::MissingGrantRefusal {
            return Ok(());
        }
        let mut file = new_file(&options.work.join("grant.pending"))?;
        let bytes =
            humanymous_platform::testing::process::encode_unverified_controlled_fixture_grant(
                &owner.child,
                &selector,
                &nonce,
            )?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(options.work.join("grant.pending"), &grant)?;
        granted = true;
        Ok(())
    })();
    if setup.is_ok() {
        loop {
            if owner.observe().is_err() {
                observation_error = true;
                break;
            }
            if owner.exit.is_some() {
                break;
            }
            if Instant::now() >= run_until {
                run_expired = true;
                live_at_run_deadline = true; // Last actual try_wait returned None.
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    run_expired |= Instant::now() >= run_until;
    let natural = owner.exit.is_some() && !run_expired && !observation_error && setup.is_ok();
    let cleanup = owner.contain(cleanup_until);
    let cleanup_known = cleanup.parent_exit_observed && cleanup.job_empty_observed;
    let actual_exit = owner.exit.and_then(|status| status.code());
    // Read diagnostic output only AFTER known native convergence. Unknown owners
    // are retained below before any fallible report IO, never dropped by a read error.
    let marker = match expected {
        ExpectedOutcome::ParentPass => None,
        ExpectedOutcome::MissingGrantRefusal => {
            Some(humanymous_platform::testing::process::CONTROLLED_MISSING_GRANT_MARKER)
        }
        ExpectedOutcome::LiveDeadlineTermination => {
            Some(humanymous_platform::testing::process::CONTROLLED_LIVE_PARENT_MARKER)
        }
    };
    let mut marker_read_error = false;
    let marker_observed = if cleanup_known {
        marker.and_then(
            |text| match probe_marker(&options.work.join("parent.stdout.log"), text) {
                Ok(observed) => Some(observed),
                Err(_) => {
                    marker_read_error = true;
                    None
                }
            },
        )
    } else {
        None
    };
    let probe_timely = Instant::now() < cleanup_until;
    let elapsed_ms = original.elapsed().as_millis();
    let classification = classify(
        expected,
        Facts {
            setup_ok: setup.is_ok(),
            assigned: owner.assigned,
            granted,
            natural,
            run_expired,
            live_at_deadline: live_at_run_deadline,
            exit_code: actual_exit,
            parent_exited: cleanup.parent_exit_observed,
            job_empty: cleanup.job_empty_observed,
            cleanup_timely: cleanup.within_deadline,
            termination_requested: cleanup.termination_requested,
            termination_failed: cleanup.termination_failed,
            observation_failed: observation_error
                || cleanup.observation_failed
                || marker_read_error,
            active_before_termination: cleanup.active_before_termination,
            probe_marker: marker_observed,
            probe_timely,
        },
    );
    let report = json!({"schema":"humanymous.controlled-capture-outer/1","case":options.case,
        "selector":selector,"test_exe_sha256":options.digest,"actual_parent_pid":pid,
        "actual_controller_pid":std::process::id(),"execution_token":"inherited-current-process",
        "run_budget_ms":10000,"cleanup_budget_ms":15000,"elapsed_ms":elapsed_ms,
        "actual_job_assigned":owner.assigned,"job_assigned_before_grant":owner.assigned && granted,
        "cooperative_grant_written":granted,"grant_intentionally_withheld":expected == ExpectedOutcome::MissingGrantRefusal,
        "expected_outcome":expected.label(),"case_qualified":classification.case_qualified,
        "parent_live_observed_at_original_deadline":live_at_run_deadline,
        "active_job_processes_before_termination":cleanup.active_before_termination,
        "probe_marker_observed":marker_observed,"probe_marker_read_error":marker_read_error,
        "probe_observation_within_original_cleanup_budget":probe_timely,
        "natural_parent_exit_observed":natural,"actual_parent_exit_code":actual_exit,
        "run_expired":run_expired,"setup_failed":setup.is_err(),"observation_error":observation_error,
        "job_cleanup_termination_requested":cleanup.termination_requested && owner.assigned,
        "direct_parent_stop_requested":cleanup.termination_requested && !owner.assigned,
        "cleanup_termination_failed":cleanup.termination_failed,
        "cleanup_observation_error":cleanup.observation_failed,
        "actual_job_empty_observed":cleanup.job_empty_observed,
        "actual_parent_exit_observed":cleanup.parent_exit_observed,
        "cleanup_within_original_deadline":cleanup.within_deadline,
        "cleanup_scope":if owner.assigned {"assigned-job"} else {"unassigned-cooperative-parent-only"},
        "parent_exit_and_job_empty":cleanup_known,
        "outer_cleanup_unknown":!cleanup_known,"parent_test_passed":classification.parent_passed,
        "scope":"fixed-cooperative-test-only","race_free_untrusted_launch":false,
        "syscall_wallclock_guarantee":false,"account_scm_or_privilege_operations":false});
    // Retain first, BEFORE report IO can fail or panic.
    if !cleanup_known {
        RECOVERY.with(|recovery| recovery.borrow_mut().push(owner));
    }
    let mut output = new_file(&options.work.join("outer-result.json"))?;
    serde_json::to_writer(&mut output, &report)?;
    output.write_all(b"\n")?;
    output.sync_all()?;
    Ok(if classification.case_qualified { 0 } else { 1 })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_selectors_refuse_unknown_or_duplicate_arguments() {
        assert!(options(["--case", "arbitrary"].into_iter().map(str::to_owned)).is_err());
        assert!(
            options(
                ["--case", "normal", "--case", "normal"]
                    .into_iter()
                    .map(str::to_owned)
            )
            .is_err()
        );
        assert_eq!(CASES.len(), 5);
    }
    #[test]
    fn fixed_parent_requires_absolute_executable_digest_and_new_work_locator() {
        let valid = [
            "--test-exe",
            r"C:\fixture\humanymous_platform-0123456789abcdef.exe",
            "--expected-sha256",
            &"a".repeat(64),
            "--case",
            "normal",
            "--work",
            r"C:\private\new-case",
        ];
        assert!(options(valid.into_iter().map(str::to_owned)).is_ok());
        let invalid = [
            "--test-exe",
            r"C:\fixture\unrelated.exe",
            "--expected-sha256",
            &"a".repeat(64),
            "--case",
            "normal",
            "--work",
            r"C:\private\new-case",
        ];
        assert!(options(invalid.into_iter().map(str::to_owned)).is_err());
    }
    #[test]
    fn cli_profile_has_one_literal_selector_and_specific_artifact_prefix() {
        let cli = profile("cli-watchdog").unwrap();
        assert_eq!(
            cli.selector,
            "final_delivery_watchdog_with_retained_unread_pipes"
        );
        assert_eq!(cli.executable_prefix, "watchdog_boundary-");
        assert!(profile("CLI-watchdog").is_none());
        assert!(profile("cli-watchdog-extra").is_none());
        assert_eq!(
            profile("normal").unwrap().selector,
            "testing::process::capture_attempt::actual_tests::retained_normal_stdout_stderr_and_actual_joins"
        );
    }
    #[test]
    fn executable_hash_names_cannot_cross_profiles_or_escape_fixed_shape() {
        let cli = profile("cli-watchdog").unwrap();
        assert!(executable_name(
            "watchdog_boundary-0123456789abcdef.exe",
            &cli
        ));
        for name in [
            "humanymous_platform-0123456789abcdef.exe",
            "watchdog_boundary-0123.exe",
            "watchdog_boundary-0123456789ABCDEF.exe",
            "watchdog_boundary-0123456789abcdef.exe.extra",
            "watchdog_boundary-0123456789abcdef.dll",
        ] {
            assert!(!executable_name(name, &cli));
        }
        assert!(!executable_name(
            "watchdog_boundary-0123456789abcdef.exe",
            &profile("normal").unwrap()
        ));
    }
    #[test]
    fn fault_profiles_have_fixed_selectors_and_cannot_weaken_ordinary_outcomes() {
        let missing = profile("outer-missing-grant").unwrap();
        assert_eq!(
            missing.selector,
            "testing::process::controlled_barrier::actual_tests::missing_grant_refuses_before_workload"
        );
        assert_eq!(missing.expected, ExpectedOutcome::MissingGrantRefusal);
        let live = profile("outer-live-timeout").unwrap();
        assert_eq!(
            live.selector,
            "testing::process::controlled_barrier::actual_tests::live_parent_waits_for_original_outer_deadline"
        );
        assert_eq!(live.expected, ExpectedOutcome::LiveDeadlineTermination);
        assert_eq!(
            profile("normal").unwrap().expected,
            ExpectedOutcome::ParentPass
        );
        assert_eq!(
            profile("cli-watchdog").unwrap().expected,
            ExpectedOutcome::ParentPass
        );
        assert!(profile("outer-live-timeout-arbitrary").is_none());
    }
}
