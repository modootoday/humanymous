//! Linux backend of the `standard-user-dotnet-prebuilt` custody provider (SoT
//! dotnet-protected-host-launch L1/L2). The non-root runner user builds and
//! verifies the artifacts, then creates each harness inside a transient
//! systemd scope (`sudo -n systemd-run --scope --uid`) with a piped creator
//! channel. Only the scope's terminal facts make a run clean. Same-uid peers
//! are trusted hosted infrastructure; no same-uid adversary claim is made.
use crate::{Result, require, source};
use humanymous_platform::testing::{
    linux_tree_probe::{Availability, inspect},
    parent_context::{
        ManagedLineage, NativeLineage, PreflightEnvironment, UnverifiedBudget,
        UnverifiedProvenance,
        admission::{AdmissionProfile, UnverifiedEnvelope, encode_envelope},
        encode_provenance,
        host::{
            ActorIdentity, UnverifiedHostBinding, UnverifiedHostContext, encode_host,
            encode_host_binding,
        },
    },
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// Exactly the two row-24 tests this provider runs, each alone with --exact.
pub const TESTS: [&str; 2] = [
    "actual_kestrel_engine_audit_body_gc_and_shutdown",
    "generic::actual_generic_abi_only_process",
];
/// Rows 72/73: the package-matrix controller, itself admitted, which hands
/// each mode's harness a derived envelope (SoT 3.5).
pub const PACKAGE_TEST: &str = "sdk_package_and_real_host_matrix";
const RUN_MS: u64 = 180_000;
const CLEANUP_MS: u64 = 60_000;
const OUTPUT_LIMIT: u64 = 1_048_576;
/// The runtime's Unix authority listener refuses an endpoint longer than this
/// (`authority/transport/unix.rs`), below the kernel's 108-byte `sun_path`.
const SOCKET_ENDPOINT_LIMIT: usize = 100;
/// Longest endpoint the Kestrel case binds under the admitted audit root.
const SOCKET_SUFFIX: &str = "/audit/production/decision.sock";

/// Verified inputs handed from the build phase to the launch phase.
pub struct Prepared {
    pub harness: PathBuf,
    /// Package-matrix controller test executable.
    pub controller: PathBuf,
    /// Release native export the package controller packs and measures.
    pub release_native: PathBuf,
    /// Native provenance the controller compares against actual digests.
    pub provenance: PathBuf,
    /// Committed source checkout the controller verifies clean and exact.
    pub source_root: PathBuf,
    pub app_root: PathBuf,
    pub runtime_root: PathBuf,
    pub dotnet: PathBuf,
    pub source_sha: String,
    pub manifest_sha256: String,
    pub cargo_lock_sha256: String,
    pub hashes: Value,
    pub work: PathBuf,
}

fn now_ms() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis()
        .try_into()
        .map_err(|_| "wall clock out of range".to_owned())
}
fn nonce() -> Result<String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    let text: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    require(text.bytes().any(|b| b != b'0'), "zero run nonce")?;
    Ok(text)
}
fn text(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "non-UTF-8 provider path".to_owned())
}
fn private_dir(path: &Path) -> Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(path)
        .map_err(|e| format!("private directory: {e}"))
}

/// Whether the authority socket under this private work root fits the
/// listener's endpoint bound. A longer root fails the Kestrel case at bind.
pub fn socket_endpoint_fits(private_work: &Path) -> bool {
    private_work
        .to_str()
        .is_some_and(|root| root.len() + SOCKET_SUFFIX.len() <= SOCKET_ENDPOINT_LIMIT)
}

/// Hosted scratch: a fresh owner-only directory under `RUNNER_TEMP` when it is
/// an absolute directory, else under the canonical temp root, never under the
/// evidence bundle. The name stays short because the package matrix nests each
/// mode's audit root below the private work and the authority socket under it
/// must fit the listener's endpoint bound.
fn hosted_scratch() -> Result<PathBuf> {
    let base = std::env::var_os("RUNNER_TEMP")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.is_dir())
        .unwrap_or_else(std::env::temp_dir)
        .canonicalize()
        .map_err(|e| format!("temp root: {e}"))?;
    let scratch = base.join(format!("hm{}", &nonce()?[..6]));
    private_dir(&scratch)?;
    Ok(scratch)
}

/// Remove hosted scratch after launch; its deletion is the last provider fact.
fn remove_scratch(scratch: &Path, report: &mut Value, key: &str) -> bool {
    let removed = std::fs::remove_dir_all(scratch).is_ok() && !scratch.exists();
    report[key] = json!(removed);
    removed
}

/// L2 prerequisites (systemd PID 1, cgroup v2, system bus) must be Observed.
pub fn backend_observed() -> Result<()> {
    let probe = inspect();
    require(
        [
            probe.systemd_pid1,
            probe.unified_cgroup,
            probe.system_bus_socket,
        ]
        .iter()
        .all(|value| *value == Availability::Observed),
        "Linux scope prerequisites not observed; dotnet host launch Unsupported",
    )
}

/// cgroup v2 mount point from mountinfo (pure v2 or hybrid `unified`).
fn cgroup2_root() -> Result<PathBuf> {
    let info = std::fs::read_to_string("/proc/self/mountinfo").map_err(|e| e.to_string())?;
    info.lines()
        .find(|line| line.contains(" - cgroup2 "))
        .and_then(|line| line.split(' ').nth(4))
        .map(PathBuf::from)
        .ok_or_else(|| "cgroup v2 mount absent".to_owned())
}

/// Pure classification of the scope's terminal observation.
#[derive(Debug, PartialEq, Eq)]
pub enum ScopeFact {
    /// `cgroup.events` read `populated 0`.
    Unpopulated,
    /// The kernel only removes an empty cgroup; the manager collected it.
    Collected,
    /// Live descendants remain inside the scope.
    Populated,
    /// Nothing established.
    Unknown,
}
pub fn classify_events(events: Option<&str>, unit_loaded: Option<bool>) -> ScopeFact {
    match events {
        Some(text) if text.lines().any(|line| line == "populated 0") => ScopeFact::Unpopulated,
        Some(text) if text.lines().any(|line| line == "populated 1") => ScopeFact::Populated,
        Some(_) => ScopeFact::Unknown,
        None if unit_loaded == Some(false) => ScopeFact::Collected,
        None => ScopeFact::Unknown,
    }
}

fn unit_loaded(unit: &str) -> Option<bool> {
    let output = Command::new("systemctl")
        .args(["show", "--property=LoadState", "--value", unit])
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let state = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    output
        .status
        .success()
        .then(|| state != "not-found")
        .filter(|_| !state.is_empty())
}

fn bounded_reader(
    stream: impl Read + Send + 'static,
) -> std::thread::JoinHandle<std::io::Result<Vec<u8>>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stream.take(OUTPUT_LIMIT + 1).read_to_end(&mut bytes)?;
        Ok(bytes)
    })
}

/// Footer and exact case line for one `--exact` test alone.
pub fn single_test_passed(output: &str, name: &str) -> bool {
    let footers: Vec<_> = output
        .lines()
        .filter(|line| line.starts_with("test result:"))
        .collect();
    footers.len() == 1
        && footers[0]
            .starts_with("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; ")
        // `--nocapture` output may split the case line between its start and
        // its `ok`; the exact single-case footer above carries the outcome.
        && output
            .lines()
            .filter(|line| line.starts_with(&format!("test {name} ... ")))
            .count()
            == 1
}

/// The Kestrel test emits exactly one clean observation per profile.
pub fn host_observations(output: &str) -> Result<Value> {
    // Under `--nocapture` the first row can share the `test <case> ... ` line.
    let prefix = format!("test {} ... ", TESTS[0]);
    let rows: Vec<Value> = output
        .lines()
        .map(|line| line.strip_prefix(&prefix).unwrap_or(line))
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|row| row["schema"] == "humanymous.dotnet-host-observation/1")
        .collect();
    let profiles: Vec<_> = rows.iter().map(|row| row["profile"].clone()).collect();
    require(
        rows.len() == 2
            && profiles
                == [
                    json!("production-application-http"),
                    json!("real-engine-audit-synthetic-evidence"),
                ]
            && rows.iter().all(|row| {
                row["failed"] == 0 && row["ignored"] == 0 && row["full_product"] == false
            }),
        "dotnet host observations incomplete",
    )?;
    Ok(json!(rows))
}

struct Attempt<'a> {
    harness: &'a Path,
    uid: u32,
    gid: u32,
    name: &'a str,
    stage: &'a str,
    env: Vec<(String, String)>,
    envelope: Option<Vec<u8>>,
    nonce: String,
    issued: Instant,
}

/// One scoped harness invocation and its terminal facts.
fn launch_one(dir: &Path, attempt: Attempt<'_>) -> Result<Value> {
    let (uid, gid) = (attempt.uid, attempt.gid);
    let unit = format!("hm-dotnet-{}.scope", attempt.nonce);
    let mut arguments = vec![
        "-n".to_owned(),
        "systemd-run".to_owned(),
        "--scope".to_owned(),
        "--quiet".to_owned(),
        format!("--unit={unit}"),
        format!("--uid={uid}"),
        format!("--gid={gid}"),
        "--".to_owned(),
        "/usr/bin/env".to_owned(),
        "-i".to_owned(),
    ];
    arguments.extend(attempt.env.iter().map(|(k, v)| format!("{k}={v}")));
    arguments.extend([
        text(attempt.harness)?,
        "--exact".to_owned(),
        attempt.name.to_owned(),
        "--ignored".to_owned(),
        "--test-threads=1".to_owned(),
        "--nocapture".to_owned(),
    ]);
    let mut child = Command::new("sudo")
        .args(&arguments)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .current_dir("/")
        .stdin(if attempt.envelope.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("provider spawn: {e}"))?;
    let elapsed_at_dispatch = attempt.issued.elapsed();
    // Creator channel: write the one frame, then close the only write end.
    let channel = match (attempt.envelope, child.stdin.take()) {
        (Some(frame), Some(mut stdin)) => stdin.write_all(&frame).map_err(|e| e.to_string()),
        (None, None) => Ok(()),
        _ => Err("creator channel mismatch".into()),
    };
    let stdout = bounded_reader(child.stdout.take().ok_or("stdout absent")?);
    let stderr = bounded_reader(child.stderr.take().ok_or("stderr absent")?);
    let scope_dir = cgroup2_root()?.join("system.slice").join(&unit);
    let run_until = attempt.issued + Duration::from_millis(RUN_MS);
    let cleanup_until = run_until + Duration::from_millis(CLEANUP_MS);
    let mut populated_seen = false;
    let mut deadline_kill = false;
    let status = loop {
        if !populated_seen {
            populated_seen = std::fs::read_to_string(scope_dir.join("cgroup.events"))
                .is_ok_and(|events| events.lines().any(|line| line == "populated 1"));
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break Some(status);
        }
        let now = Instant::now();
        if now >= run_until && !deadline_kill {
            deadline_kill = true;
            let _kill = Command::new("sudo")
                .args(["-n", "systemctl", "kill", "--signal=SIGKILL", &unit])
                .stdin(Stdio::null())
                .status();
        }
        if now >= cleanup_until {
            break None;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    // Terminal fact: read now; a populated scope is killed through the scope.
    let mut fact = classify_events(
        std::fs::read_to_string(scope_dir.join("cgroup.events"))
            .ok()
            .as_deref(),
        unit_loaded(&unit),
    );
    let residual = fact == ScopeFact::Populated;
    if residual {
        let _kill = Command::new("sudo")
            .args(["-n", "systemctl", "kill", "--signal=SIGKILL", &unit])
            .stdin(Stdio::null())
            .status();
        while Instant::now() < cleanup_until {
            fact = classify_events(
                std::fs::read_to_string(scope_dir.join("cgroup.events"))
                    .ok()
                    .as_deref(),
                unit_loaded(&unit),
            );
            if matches!(fact, ScopeFact::Unpopulated | ScopeFact::Collected) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    // Join both readers only inside the original cleanup deadline.
    let joined = |reader: std::thread::JoinHandle<std::io::Result<Vec<u8>>>| {
        while !reader.is_finished() && Instant::now() < cleanup_until {
            std::thread::sleep(Duration::from_millis(5));
        }
        reader
            .is_finished()
            .then(|| reader.join().ok().and_then(std::result::Result::ok))
            .flatten()
    };
    let (out, err) = (joined(stdout), joined(stderr));
    let readers_joined = out.is_some() && err.is_some();
    let output = out.unwrap_or_default();
    let error_output = err.unwrap_or_default();
    std::fs::write(dir.join(format!("{}.log", attempt.stage)), &output)
        .map_err(|e| e.to_string())?;
    std::fs::write(
        dir.join(format!("{}.stderr.log", attempt.stage)),
        &error_output,
    )
    .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&output).into_owned();
    Ok(json!({
        "test": attempt.name,
        "unit": unit,
        "creator_channel_written": channel.is_ok(),
        "dispatch_elapsed_ms": elapsed_at_dispatch.as_millis() as u64,
        "exit_code": status.and_then(|s| s.code()),
        "readers_joined": readers_joined && output.len() as u64 <= OUTPUT_LIMIT && error_output.len() as u64 <= OUTPUT_LIMIT,
        "scope_populated_observed": populated_seen,
        "scope_terminal": format!("{fact:?}"),
        "residual_killed": residual,
        "deadline_kill": deadline_kill,
        "passed": single_test_passed(&text, attempt.name),
        "stdout_sha256": source::sha(&output),
    }))
}

fn lineage(sha: &str, lock: &str, receipt: &str) -> NativeLineage {
    NativeLineage {
        source_sha: sha.into(),
        cargo_lock_sha256: lock.into(),
        receipt_sha256: receipt.into(),
    }
}

/// Which declared harness a provider run executes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Row 24: the two hosted tests, each in its own scope.
    Hosted,
    /// Rows 72/73: the package-matrix controller in one scope.
    Package,
}

/// Original `/2` context, binding and envelope for one scoped run. The
/// package controller later re-sends these exact bytes, narrowed per mode.
fn envelope(
    prepared: &Prepared,
    actor: u32,
    nonce: &str,
    issued: Instant,
    private_work: &Path,
    handoff: &Path,
) -> Result<Vec<u8>> {
    let issued_ms = now_ms()?;
    let provenance = UnverifiedProvenance {
        managed: ManagedLineage {
            source_sha: prepared.source_sha.clone(),
            required_source_sha: prepared.source_sha.clone(),
            receipt_sha256: prepared.hashes["managed_assembly"]
                .as_str()
                .ok_or("managed hash absent")?
                .into(),
        },
        native: lineage(
            &prepared.source_sha,
            &prepared.cargo_lock_sha256,
            prepared.hashes["native_library"]
                .as_str()
                .ok_or("native hash absent")?,
        ),
        harness: lineage(
            &prepared.source_sha,
            &prepared.cargo_lock_sha256,
            prepared.hashes["harness"]
                .as_str()
                .ok_or("harness hash absent")?,
        ),
    };
    let context = UnverifiedHostContext {
        schema: "humanymous.dotnet-host-launch/2".into(),
        apphost: text(&prepared.app_root.join("Humanymous.HostPilot"))?,
        app_root: text(&prepared.app_root)?,
        runtime_root: text(&prepared.runtime_root)?,
        system_root: None,
        private_work: text(private_work)?,
        audit_root: text(&private_work.join("audit"))?,
        handoff_root: text(handoff)?,
        actor: ActorIdentity::Linux { uid: actor },
        provenance,
        budget: UnverifiedBudget {
            issued_utc_ms: issued_ms,
            run_expires_utc_ms: issued_ms + RUN_MS,
            cleanup_expires_utc_ms: issued_ms + RUN_MS + CLEANUP_MS,
            parent_elapsed_ms_at_dispatch: issued.elapsed().as_millis() as u64,
        },
    };
    let context_bytes = encode_host(&context).map_err(|e| e.to_string())?;
    let binding = UnverifiedHostBinding {
        schema: "humanymous.parent-context-binding/2".into(),
        run_nonce: nonce.to_owned(),
        actor: context.actor.clone(),
        source_sha: prepared.source_sha.clone(),
        context_sha256: source::sha(&context_bytes),
        verified_manifest_sha256: prepared.manifest_sha256.clone(),
        provenance_sha256: source::sha(
            &encode_provenance(&context.provenance).map_err(|e| e.to_string())?,
        ),
        original_budget: UnverifiedBudget { ..context.budget },
        preflight_environment: PreflightEnvironment::GithubHosted,
    };
    let binding_bytes = encode_host_binding(&binding, &context).map_err(|e| e.to_string())?;
    encode_envelope(&UnverifiedEnvelope {
        profile: AdmissionProfile::StandardUserDotnetPrebuilt,
        run_nonce: nonce.to_owned(),
        derived: None,
        context: context_bytes,
        binding: binding_bytes,
    })
    .map_err(|e| e.to_string())
}

/// The declared tests a target runs, each alone with `--exact` in its scope.
pub fn tests(target: Target) -> &'static [&'static str] {
    match target {
        Target::Hosted => &TESTS,
        Target::Package => &[PACKAGE_TEST],
    }
}

/// Create the private layout, the `/2` context, binding and envelope, then run
/// each declared test inside its own scope. Every terminal fact must hold.
pub fn launch(
    prepared: &Prepared,
    target: Target,
    scratch: &Path,
    dir: &Path,
    venue: &str,
    report: &mut Value,
) -> Result<()> {
    backend_observed()?;
    let (real, effective) = humanymous_runtime::native_security::process::current_uids();
    require(
        real == effective && real != 0,
        "ordinary non-root runner actor required",
    )?;
    let mut results = Vec::new();
    for name in tests(target) {
        let nonce = nonce()?;
        let issued = Instant::now();
        let private_work = scratch.join(format!("run-{}", &nonce[..12]));
        let handoff = scratch.join(format!("handoff-{}", &nonce[..12]));
        require(
            socket_endpoint_fits(&private_work),
            "dotnet host scratch too long for the authority socket endpoint",
        )?;
        for path in [
            &private_work,
            &handoff,
            &private_work.join("temp"),
            &private_work.join("profile"),
            &private_work.join("audit"),
        ] {
            private_dir(path)?;
        }
        let gid = {
            use std::os::unix::fs::MetadataExt;
            std::fs::metadata(&private_work)
                .map_err(|e| e.to_string())?
                .gid()
        };
        let mut env = vec![
            ("HOME".to_owned(), text(&private_work.join("profile"))?),
            ("TMPDIR".to_owned(), text(&private_work.join("temp"))?),
            ("RUST_BACKTRACE".to_owned(), "0".to_owned()),
        ];
        match venue {
            "github-hosted" => env.extend([
                ("GITHUB_ACTIONS".to_owned(), "true".to_owned()),
                ("RUNNER_ENVIRONMENT".to_owned(), "github-hosted".to_owned()),
            ]),
            _ => env.push((
                "HUMANYMOUS_DOTNET_LOCAL_ALLOCATION".to_owned(),
                "root-approved-local".to_owned(),
            )),
        }
        let (harness, envelope) = if *name == TESTS[1] {
            let report_dir = private_work.join("generic-report");
            private_dir(&report_dir)?;
            env.extend([
                (
                    "HUMANYMOUS_DOTNET_GENERIC_INPUT".to_owned(),
                    text(&prepared.work.join("generic-input.json"))?,
                ),
                (
                    "HUMANYMOUS_DOTNET_GENERIC_REPORT_DIR".to_owned(),
                    text(&report_dir)?,
                ),
            ]);
            (&prepared.harness, None)
        } else {
            if *name == PACKAGE_TEST {
                // The controller runs the pinned SDK and git; it measures the
                // source, native, harness and provenance it is handed.
                let sdk = prepared.dotnet.parent().ok_or("dotnet root absent")?;
                env.extend([
                    ("PATH".to_owned(), format!("{}:/usr/bin:/bin", text(sdk)?)),
                    (
                        "HUMANYMOUS_DOTNET_SOURCE_ROOT".to_owned(),
                        text(&prepared.source_root)?,
                    ),
                    (
                        "HUMANYMOUS_DOTNET_NATIVE_LIBRARY".to_owned(),
                        text(&prepared.release_native)?,
                    ),
                    (
                        "HUMANYMOUS_DOTNET_HOST_HARNESS".to_owned(),
                        text(&prepared.harness)?,
                    ),
                    (
                        "HUMANYMOUS_DOTNET_NATIVE_PROVENANCE".to_owned(),
                        text(&prepared.provenance)?,
                    ),
                ]);
            }
            let frame = envelope(prepared, real, &nonce, issued, &private_work, &handoff)?;
            let harness = if *name == PACKAGE_TEST {
                &prepared.controller
            } else {
                &prepared.harness
            };
            (harness, Some(frame))
        };
        let stage = format!("dotnet-{}", name.replace("::", "-"));
        let mut result = launch_one(
            dir,
            Attempt {
                harness,
                uid: real,
                gid,
                name,
                stage: &stage,
                env,
                envelope,
                nonce,
                issued,
            },
        )?;
        // Private, then handoff and scratch cleanup are provider facts too.
        let handoff_files = std::fs::read_dir(&handoff).map(Iterator::count).ok();
        result["handoff_files"] = json!(handoff_files);
        result["private_removed"] =
            json!(std::fs::remove_dir_all(&private_work).is_ok() && !private_work.exists());
        result["handoff_removed"] =
            json!(std::fs::remove_dir_all(&handoff).is_ok() && !handoff.exists());
        let output =
            std::fs::read_to_string(dir.join(format!("{stage}.log"))).map_err(|e| e.to_string())?;
        if *name == TESTS[0] {
            result["observations"] = host_observations(&output).unwrap_or(Value::Null);
        }
        if *name == PACKAGE_TEST {
            result["package"] = package_projection(&output).unwrap_or(Value::Null);
        }
        results.push(result);
    }
    let key = match target {
        Target::Hosted => "dotnet_host",
        Target::Package => "dotnet_package",
    };
    report[key] = json!({
        "provider": "standard-user-dotnet-prebuilt",
        "backend": "linux-systemd-scope",
        "venue": venue,
        "actor_uid": real,
        "same_uid_adversary_defended": false,
        "account_deletion": "not-applicable: shared hosted runner uid",
        "results": results,
        "full_product": false,
    });
    let clean = results.iter().all(|result| {
        result["exit_code"] == 0
            && result["passed"] == true
            && result["creator_channel_written"] == true
            && result["readers_joined"] == true
            && result["scope_populated_observed"] == true
            && matches!(
                result["scope_terminal"].as_str(),
                Some("Unpopulated" | "Collected")
            )
            && result["residual_killed"] == false
            && result["deadline_kill"] == false
            && result["handoff_files"] == 0
            && result["private_removed"] == true
            && result["handoff_removed"] == true
    }) && match target {
        Target::Hosted => results[0]["observations"].is_array(),
        Target::Package => results[0]["package"].is_object(),
    };
    require(clean, "dotnet provider terminal facts incomplete")
}

/// The controller's one public projection line: four modes, complete host
/// contract, package install evidenced by the packed modes, no product claim.
pub fn package_projection(output: &str) -> Result<Value> {
    let prefix = format!("test {PACKAGE_TEST} ... ");
    let rows: Vec<Value> = output
        .lines()
        .map(|line| line.strip_prefix(&prefix).unwrap_or(line))
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|row| row.get("host_contract_complete").is_some())
        .collect();
    require(
        rows.len() == 1
            && rows[0]["host_contract_complete"] == true
            && rows[0]["full_product"] == false,
        "dotnet package projection incomplete",
    )?;
    Ok(rows[0].clone())
}

/// Resolve the `dotnet` muxer actually on PATH and its runtime root.
fn muxer() -> Result<(PathBuf, PathBuf)> {
    let path = std::env::var_os("PATH").ok_or("PATH absent")?;
    let dotnet = std::env::split_paths(&path)
        .map(|dir| dir.join("dotnet"))
        .find(|candidate| candidate.is_file())
        .ok_or("dotnet muxer absent")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let root = dotnet.parent().ok_or("dotnet root absent")?.to_path_buf();
    Ok((dotnet, root))
}

fn harness_executable(log: &str, name: &str) -> Result<PathBuf> {
    let paths: Vec<_> = log
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|row| {
            row["reason"] == "compiler-artifact"
                && row["target"]["name"] == name
                && row["target"]["kind"] == json!(["test"])
        })
        .filter_map(|row| row["executable"].as_str().map(PathBuf::from))
        .collect();
    require(paths.len() == 1, "exactly one test harness executable")?;
    Ok(paths[0].clone())
}

/// Build phase: native exports, harness and the FDD/JIT pilot app.
pub fn prepare(root: &Path, dir: &Path, report: &mut Value, work: &Path) -> Result<Prepared> {
    let step = |report: &mut Value, stage: &str, program: &str, args: &[&str], budget: &str| {
        super::invoke(root, dir, report, stage, program, args, budget)
    };
    step(
        report,
        "dotnet-native-build",
        "cargo",
        &[
            "build",
            "--locked",
            "-p",
            "humanymous-dotnet-adapter-export",
            "-p",
            "humanymous-native-abi-export",
        ],
        "900s",
    )?;
    step(
        report,
        "dotnet-harness-build",
        "cargo",
        &[
            "test",
            "--locked",
            "-p",
            "humanymous-dotnet-adapter",
            "--test",
            "hosted",
            "--no-run",
            "--message-format=json",
        ],
        "900s",
    )?;
    let harness = harness_executable(
        &std::fs::read_to_string(dir.join("dotnet-harness-build.log"))
            .map_err(|e| e.to_string())?,
        "hosted",
    )?;
    step(
        report,
        "dotnet-native-release-build",
        "cargo",
        &[
            "build",
            "--locked",
            "--release",
            "-p",
            "humanymous-dotnet-adapter-export",
        ],
        "900s",
    )?;
    step(
        report,
        "dotnet-controller-build",
        "cargo",
        &[
            "test",
            "--locked",
            "-p",
            "humanymous-dotnet-adapter",
            "--test",
            "package",
            "--no-run",
            "--message-format=json",
        ],
        "900s",
    )?;
    let controller = harness_executable(
        &std::fs::read_to_string(dir.join("dotnet-controller-build.log"))
            .map_err(|e| e.to_string())?,
        "package",
    )?;
    step(report, "rustc-version", "rustc", &["-V"], "30s")?;
    let rustc = std::fs::read_to_string(dir.join("rustc-version.log"))
        .map_err(|e| e.to_string())?
        .trim()
        .to_owned();
    require(
        rustc.starts_with("rustc 1.99.1 ") && !rustc.contains('\n'),
        "pinned rustc 1.99.1 required",
    )?;
    let (dotnet, runtime_root) = muxer()?;
    step(report, "dotnet-info", "dotnet", &["--info"], "60s")?;
    let info = std::fs::read_to_string(dir.join("dotnet-info.log")).map_err(|e| e.to_string())?;
    require(
        info.lines()
            .any(|line| line.trim() == "Version:           10.0.403")
            || info.contains("10.0.403"),
        "pinned .NET SDK 10.0.403 required",
    )?;
    let app_root = work.join("app");
    let build = work.join("dotnet-build");
    let target_root = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let target = target_root.join("debug");
    let release_native = target_root
        .join("release")
        .join("libhumanymous_dotnet_adapter_export.so");
    let (app, build_text) = (text(&app_root)?, text(&build)?);
    step(
        report,
        "dotnet-publish",
        "dotnet",
        &[
            "publish",
            "adapters/dotnet/pilot/Humanymous.HostPilot.csproj",
            "-c",
            "Release",
            "-r",
            "linux-x64",
            "--self-contained",
            "false",
            "-o",
            &app,
            "-m:1",
            "-nr:false",
            "-p:UseSharedCompilation=false",
            &format!("-p:HumanymousBuildRoot={build_text}"),
        ],
        "900s",
    )?;
    let mut hashes = json!({});
    for (label, name) in [
        ("native_library", "libhumanymous_dotnet_adapter_export.so"),
        ("generic_library", "libhumanymous_native_abi_export.so"),
    ] {
        let destination = app_root.join(name);
        std::fs::copy(target.join(name), &destination).map_err(|e| e.to_string())?;
        hashes[label] = json!(source::file(&destination)?);
    }
    hashes["managed_assembly"] = json!(source::file(&app_root.join("Humanymous.HostPilot.dll"))?);
    hashes["harness"] = json!(source::file(&harness)?);
    let artifact = |path: PathBuf| -> Result<Value> {
        Ok(json!({"sha256": source::file(&path)?, "path": text(&path)?}))
    };
    let generic = json!({
        "source_sha": report["source_sha"],
        "native_source_sha": report["source_sha"],
        "sdk": artifact(dotnet.clone())?,
        "assembly": artifact(app_root.join("Humanymous.HostPilot.dll"))?,
        "native_library": artifact(app_root.join("libhumanymous_native_abi_export.so"))?,
        "managed_source": artifact(root.join("adapters/dotnet/pilot/Program.cs"))?,
        "managed_support": [
            artifact(app_root.join("Humanymous.HostPilot.deps.json"))?,
            artifact(app_root.join("Humanymous.HostPilot.runtimeconfig.json"))?,
            artifact(app_root.join("Humanymous.Native.dll"))?,
        ],
    });
    std::fs::write(
        work.join("generic-input.json"),
        serde_json::to_vec(&generic).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    // Provenance the package controller re-measures; it is a claim, never proof.
    let provenance = work.join("package-provenance.json");
    std::fs::write(
        &provenance,
        serde_json::to_vec(&json!({
            "source_sha": report["source_sha"],
            "cargo_lock_sha256": source::file(&root.join("Cargo.lock"))?,
            "native_sha256": source::file(&release_native)?,
            "harness_sha256": hashes["harness"],
            "rustc": rustc,
            "target": "x86_64-unknown-linux-gnu",
            "native_profile": "release",
            "generic_sha256": null,
        }))
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    hashes["controller"] = json!(source::file(&controller)?);
    hashes["release_native"] = json!(source::file(&release_native)?);
    Ok(Prepared {
        controller,
        release_native,
        provenance,
        source_root: root.to_path_buf(),
        harness,
        app_root,
        runtime_root,
        dotnet,
        source_sha: report["source_sha"].as_str().ok_or("source absent")?.into(),
        manifest_sha256: report["source_manifest_sha256"]
            .as_str()
            .ok_or("manifest absent")?
            .into(),
        cargo_lock_sha256: source::file(&root.join("Cargo.lock"))?,
        hashes,
        work: work.to_path_buf(),
    })
}

/// Hosted component: one runner user builds, then launches under scopes.
pub(super) fn run(root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
    backend_observed()?;
    let work = dir.join("dotnet-host-work");
    private_dir(&work)?;
    let prepared = prepare(root, dir, report, &work)?;
    report["dotnet_sdk_muxer_sha256"] = json!(source::file(&prepared.dotnet)?);
    // Private work and handoff live in short provider scratch, not under the
    // evidence bundle; scratch deletion is the last provider fact.
    let scratch = hosted_scratch()?;
    let outcome = launch(
        &prepared,
        Target::Hosted,
        &scratch,
        dir,
        "github-hosted",
        report,
    );
    let scratch_removed = remove_scratch(&scratch, report, "dotnet_host_scratch_removed");
    report["full_native_qualified"] = json!(false);
    report["package_qualification_receipt_created"] = json!(false);
    outcome?;
    require(scratch_removed, "dotnet host provider scratch not removed")
}

/// Hosted package component: the same provider admits the package controller,
/// which derives one narrowing envelope per mode.
pub(super) fn run_package(root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
    backend_observed()?;
    let work = dir.join("dotnet-package-work");
    private_dir(&work)?;
    let prepared = prepare(root, dir, report, &work)?;
    report["dotnet_sdk_muxer_sha256"] = json!(source::file(&prepared.dotnet)?);
    let scratch = hosted_scratch()?;
    let outcome = launch(
        &prepared,
        Target::Package,
        &scratch,
        dir,
        "github-hosted",
        report,
    );
    let scratch_removed = remove_scratch(&scratch, report, "dotnet_package_scratch_removed");
    report["full_native_qualified"] = json!(false);
    outcome?;
    require(
        scratch_removed,
        "dotnet package provider scratch not removed",
    )
}

fn local_allocation() -> Result<()> {
    require(
        std::env::var("HUMANYMOUS_DOTNET_LOCAL_ALLOCATION").as_deref() == Ok("root-approved-local")
            && std::env::var_os("GITHUB_ACTIONS").is_none()
            && std::env::var_os("ACT").is_none(),
        "explicit root-approved local allocation required; never qualification",
    )
}
impl Prepared {
    fn to_json(&self) -> Result<Value> {
        Ok(json!({
            "harness": text(&self.harness)?, "app_root": text(&self.app_root)?,
            "controller": text(&self.controller)?, "release_native": text(&self.release_native)?,
            "provenance": text(&self.provenance)?, "source_root": text(&self.source_root)?,
            "runtime_root": text(&self.runtime_root)?, "dotnet": text(&self.dotnet)?,
            "source_sha": self.source_sha, "manifest_sha256": self.manifest_sha256,
            "cargo_lock_sha256": self.cargo_lock_sha256, "hashes": self.hashes,
            "work": text(&self.work)?,
        }))
    }
    /// Re-read and re-verify every artifact digest before any launch.
    fn from_json(value: &Value) -> Result<Self> {
        let path = |key: &str| -> Result<PathBuf> {
            value[key]
                .as_str()
                .map(PathBuf::from)
                .ok_or_else(|| format!("prepared {key} absent"))
        };
        let string = |key: &str| -> Result<String> {
            value[key]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("prepared {key} absent"))
        };
        let prepared = Self {
            controller: path("controller")?,
            release_native: path("release_native")?,
            provenance: path("provenance")?,
            source_root: path("source_root")?,
            harness: path("harness")?,
            app_root: path("app_root")?,
            runtime_root: path("runtime_root")?,
            dotnet: path("dotnet")?,
            source_sha: string("source_sha")?,
            manifest_sha256: string("manifest_sha256")?,
            cargo_lock_sha256: string("cargo_lock_sha256")?,
            hashes: value["hashes"].clone(),
            work: path("work")?,
        };
        for (label, file) in [
            ("harness", prepared.harness.clone()),
            ("controller", prepared.controller.clone()),
            ("release_native", prepared.release_native.clone()),
            (
                "native_library",
                prepared
                    .app_root
                    .join("libhumanymous_dotnet_adapter_export.so"),
            ),
            (
                "managed_assembly",
                prepared.app_root.join("Humanymous.HostPilot.dll"),
            ),
        ] {
            require(
                prepared.hashes[label] == json!(source::file(&file)?),
                "prepared artifact changed before launch",
            )?;
        }
        Ok(prepared)
    }
}

/// Root-allocated local run: `prepare <artifacts>` then, as the non-root
/// runner user, `launch <artifacts> <scratch>`. Same provider code; the
/// receipt says `root-allocated-local` and is never hosted qualification.
pub fn local(root: &Path, args: &[String]) -> Result<()> {
    local_allocation()?;
    let argument = |index: usize| {
        args.get(index)
            .map(PathBuf::from)
            .ok_or_else(|| "missing local provider argument".to_owned())
    };
    let work = argument(1)?;
    let snapshot = source::snapshot(root)?;
    let mut report = json!({"schema":"humanymous.hosted-component/1","component":"dotnet-host","venue":"root-allocated-local","qualification":false,"source_sha":snapshot["source_sha"],"source_manifest_sha256":snapshot["manifest_sha256"],"source_dirty":snapshot["dirty"],"commands":{},"status":"failed","full_product_qualified":false});
    match args.first().map(String::as_str) {
        Some("prepare") => {
            let dir = work.join("prepare-evidence");
            std::fs::create_dir(&dir).map_err(|e| e.to_string())?;
            let prepared = prepare(root, &dir, &mut report, &work)?;
            std::fs::write(
                work.join("prepared.json"),
                serde_json::to_vec_pretty(&prepared.to_json()?).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())
        }
        Some(mode @ ("launch" | "launch-package")) => {
            let target = if mode == "launch" {
                Target::Hosted
            } else {
                Target::Package
            };
            let scratch = argument(2)?;
            let dir = scratch.join("launch-evidence");
            private_dir(&dir)?;
            let prepared = Prepared::from_json(
                &serde_json::from_slice(
                    &std::fs::read(work.join("prepared.json")).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?,
            )?;
            let outcome = launch(
                &prepared,
                target,
                &scratch,
                &dir,
                "root-allocated-local",
                &mut report,
            );
            if outcome.is_ok() {
                report["status"] = json!("passed");
            }
            std::fs::write(
                dir.join("component.json"),
                serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string(&report).map_err(|e| e.to_string())?
            );
            outcome
        }
        _ => Err("local provider mode must be prepare, launch or launch-package".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scope_terminal_fact_needs_unpopulated_or_manager_collection() {
        assert_eq!(
            classify_events(Some("populated 0\nfrozen 0\n"), Some(true)),
            ScopeFact::Unpopulated
        );
        assert_eq!(
            classify_events(Some("populated 1\nfrozen 0\n"), Some(true)),
            ScopeFact::Populated
        );
        assert_eq!(classify_events(None, Some(false)), ScopeFact::Collected);
        assert_eq!(classify_events(None, Some(true)), ScopeFact::Unknown);
        assert_eq!(classify_events(None, None), ScopeFact::Unknown);
        assert_eq!(
            classify_events(Some("frozen 0\n"), Some(true)),
            ScopeFact::Unknown
        );
    }
    #[test]
    fn exact_single_test_footer_and_case_line() {
        let name = TESTS[0];
        let good = format!(
            "running 1 test\ntest {name} ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out; finished in 9.1s\n"
        );
        assert!(single_test_passed(&good, name));
        assert!(!single_test_passed(
            &good.replace("0 ignored", "1 ignored"),
            name
        ));
        assert!(!single_test_passed(
            &good.replace("1 passed", "2 passed"),
            name
        ));
        assert!(!single_test_passed(&format!("{good}{good}"), name));
        assert!(!single_test_passed(&good, TESTS[1]));
        let split = good.replace(" ... ok\n", " ... {\"row\":1}\nok\n");
        assert!(single_test_passed(&split, name));
        assert!(!single_test_passed(
            &split.replace("test result: ok.", "test result: FAILED."),
            name
        ));
    }
    #[test]
    fn observations_require_both_profiles_clean() {
        let row = |profile: &str| json!({"schema":"humanymous.dotnet-host-observation/1","profile":profile,"failed":0,"ignored":0,"full_product":false});
        let good = format!(
            "{}\n{}\n",
            row("production-application-http"),
            row("real-engine-audit-synthetic-evidence")
        );
        assert!(host_observations(&good).is_ok());
        assert!(host_observations(&good.replace("\"failed\":0", "\"failed\":1")).is_err());
        assert!(host_observations(&format!("{good}{good}")).is_err());
        assert!(
            host_observations(&good.replace("\"full_product\":false", "\"full_product\":true"))
                .is_err()
        );
        let shared = format!("test {} ... {good}", TESTS[0]);
        assert!(host_observations(&shared).is_ok());
        let other = format!("test {} ... {good}", TESTS[1]);
        assert!(host_observations(&other).is_err());
    }
    #[test]
    fn socket_endpoint_bound_refuses_the_evidence_bundle_root() {
        // The hosted run 134858913712 placed private work here; the Kestrel
        // case's authority socket could not bind under it.
        let bundle = Path::new(
            "/home/runner/work/humanymous/humanymous/.spec.local/root/evidence/20261118140959-delivery-pilot/dotnet-host-work/run-0123456789ab",
        );
        assert!(!socket_endpoint_fits(bundle));
        assert!(socket_endpoint_fits(Path::new(
            "/tmp/hm012345/run-0123456789ab"
        )));
        let edge = format!(
            "/{}",
            "a".repeat(SOCKET_ENDPOINT_LIMIT - SOCKET_SUFFIX.len() - 1)
        );
        assert!(socket_endpoint_fits(Path::new(&edge)));
        assert!(!socket_endpoint_fits(Path::new(&format!("{edge}a"))));
    }
    #[test]
    fn harness_executable_is_the_one_hosted_test_artifact() {
        let row = |name: &str, kind: &str, exe: &str| {
            json!({"reason":"compiler-artifact","target":{"name":name,"kind":[kind]},"executable":exe}).to_string()
        };
        let log = format!(
            "{}\n{}\n",
            row("hosted", "test", "/t/hosted-1"),
            row("package", "test", "/t/package-1")
        );
        assert_eq!(
            harness_executable(&log, "hosted").unwrap(),
            PathBuf::from("/t/hosted-1")
        );
        assert_eq!(
            harness_executable(&log, "package").unwrap(),
            PathBuf::from("/t/package-1")
        );
        assert!(harness_executable(&format!("{log}{log}"), "hosted").is_err());
        assert!(harness_executable("", "hosted").is_err());
    }
    #[test]
    fn package_projection_requires_one_complete_line_without_product_claim() {
        let line = json!({"schema":"humanymous.dotnet-component-summary/1","host_contract_complete":true,"full_product":false}).to_string();
        assert!(package_projection(&format!("{line}\n")).is_ok());
        assert!(package_projection(&format!("test {PACKAGE_TEST} ... {line}\nok\n")).is_ok());
        assert!(package_projection(&format!("{line}\n{line}\n")).is_err());
        assert!(
            package_projection(&line.replace(
                "\"host_contract_complete\":true",
                "\"host_contract_complete\":false"
            ))
            .is_err()
        );
        assert!(
            package_projection(&line.replace("\"full_product\":false", "\"full_product\":true"))
                .is_err()
        );
        assert!(package_projection("").is_err());
        assert_eq!(tests(Target::Package), [PACKAGE_TEST]);
        assert_eq!(tests(Target::Hosted), TESTS);
    }
}
