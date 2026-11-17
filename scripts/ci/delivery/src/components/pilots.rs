//! Execute the declared ignored native pilots for this runner and emit a
//! source-bound receipt. The workspace probe accepts a declared ignored test
//! only when this receipt shows its exact name passed on the same source.
use crate::{Result, command, require, source, write_json};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub const DECLARATIONS: &str = include_str!("../../../native-pilots.json");
pub const RECEIPT_SCHEMA: &str = "humanymous.native-pilots-receipt/1";

pub fn declarations() -> Result<Value> {
    let value: Value = serde_json::from_str(DECLARATIONS).map_err(|e| e.to_string())?;
    validate(&value)?;
    Ok(value)
}

/// Closed declaration shape: exact names, a known executor and no duplicates.
pub fn validate(value: &Value) -> Result<()> {
    require(
        value["schema"] == "humanymous.native-pilots/1"
            && value["receipt"] == "target/native-pilots/receipt.json",
        "native pilot declaration schema mismatch",
    )?;
    let pilots = value["pilots"].as_array().ok_or("native pilots absent")?;
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for pilot in pilots {
        let id = pilot["id"].as_str().ok_or("pilot id absent")?;
        require(ids.insert(id.to_owned()), "duplicate pilot id")?;
        for key in ["package", "target", "kind"] {
            require(
                pilot[key].as_str().is_some_and(|s| !s.is_empty()),
                "pilot identity incomplete",
            )?;
        }
        let os = pilot["os"].as_array().ok_or("pilot os absent")?;
        require(
            !os.is_empty()
                && os
                    .iter()
                    .all(|os| matches!(os.as_str(), Some("linux" | "macos" | "windows"))),
            "pilot os unsupported",
        )?;
        let tests = strings(&pilot["tests"])?;
        let helpers = strings(&pilot["helpers"])?;
        require(!tests.is_empty(), "pilot declares no tests")?;
        for name in tests.iter().chain(&helpers) {
            require(
                !name.is_empty()
                    && name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
                    && names.insert(format!("{}#{}#{name}", pilot["package"], pilot["target"])),
                "pilot test name invalid or duplicated",
            )?;
        }
        match pilot["executor"].as_str() {
            Some("native-pilots") => {
                let cargo = strings(&pilot["cargo"])?;
                require(
                    matches!(
                        cargo
                            .iter()
                            .map(String::as_str)
                            .collect::<Vec<_>>()
                            .as_slice(),
                        ["--lib"] | ["--lib", "--features", _] | ["--test", _] | ["--bin", _]
                    ) && matches!(pilot["privilege"].as_str(), Some("user" | "root")),
                    "pilot cargo selection or privilege invalid",
                )?;
                require(
                    pilot.get("outer_grant").is_none()
                        || (pilot["outer_grant"] == true && pilot["privilege"] == "user"),
                    "outer-grant pilots run unprivileged with an explicit true flag",
                )?;
            }
            Some("job") => require(
                pilot["job"]["workflow"]
                    .as_str()
                    .is_some_and(|w| w.starts_with(".github/workflows/") && w.ends_with(".yml"))
                    && pilot["job"]["name"].as_str().is_some_and(|n| !n.is_empty())
                    && pilot["job"]["run"].as_str().is_some_and(|r| !r.is_empty())
                    && helpers.is_empty(),
                "job-executed pilot must name an exact workflow job step",
            )?,
            _ => return Err("unknown pilot executor".into()),
        }
    }
    Ok(())
}

fn strings(value: &Value) -> Result<Vec<String>> {
    value
        .as_array()
        .ok_or("pilot list absent")?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or("pilot entry not a string".into())
        })
        .collect()
}

/// A pilot applies when its OS (and optional architecture) match this runner.
pub fn applies(pilot: &Value, os: &str, arch: &str) -> bool {
    pilot["os"]
        .as_array()
        .is_some_and(|list| list.iter().any(|v| v == os))
        && pilot["arch"].as_str().is_none_or(|a| a == arch)
}

/// Every job-executed pilot must name a step that exists in that job.
pub fn verify_job_executors(root: &Path, declarations: &Value) -> Result<()> {
    for pilot in declarations["pilots"]
        .as_array()
        .ok_or("native pilots absent")?
    {
        if pilot["executor"] != "job" {
            continue;
        }
        let workflow = pilot["job"]["workflow"].as_str().ok_or("workflow absent")?;
        let text = fs::read_to_string(root.join(workflow)).map_err(|e| e.to_string())?;
        let parsed = yaml_rust2::YamlLoader::load_from_str(&text).map_err(|e| e.to_string())?;
        let job = &parsed[0]["jobs"][pilot["job"]["name"].as_str().unwrap_or_default()];
        let found = job["steps"].as_vec().is_some_and(|steps| {
            steps
                .iter()
                .any(|step| step["run"].as_str() == pilot["job"]["run"].as_str())
        });
        require(
            found,
            "declared pilot job step is absent from its workflow job",
        )?;
    }
    Ok(())
}

fn executable(root: &Path, dir: &Path, pilot: &Value) -> Result<PathBuf> {
    let package = pilot["package"].as_str().ok_or("package absent")?;
    let mut args = vec![
        "test",
        "--locked",
        "--no-run",
        "--message-format=json-render-diagnostics",
        "-p",
        package,
    ];
    let cargo = strings(&pilot["cargo"])?;
    args.extend(cargo.iter().map(String::as_str));
    let log = dir.join(format!(
        "pilot-{}-build.log",
        pilot["id"].as_str().unwrap_or("x")
    ));
    let code = command::json_log(root, "cargo", &args, &log)?;
    require(code == 0, "pilot test build failed")?;
    let text = fs::read_to_string(&log).map_err(|e| e.to_string())?;
    let target = pilot["target"].as_str().unwrap_or_default();
    let kind = pilot["kind"].as_str().unwrap_or_default();
    let found: Vec<PathBuf> = text
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|row| {
            row["reason"] == "compiler-artifact"
                && row["target"]["name"] == target
                && row["target"]["kind"] == json!([kind])
                && row["profile"]["test"] == true
        })
        .filter_map(|row| row["executable"].as_str().map(PathBuf::from))
        .collect();
    require(
        found.len() == 1,
        "pilot test executable not uniquely identified",
    )?;
    Ok(found[0].clone())
}

/// The exact single-test success footer for one selected ignored test.
pub fn passed(text: &str, name: &str) -> bool {
    let summaries: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("test result: "))
        .collect();
    summaries.len() == 1
        && summaries[0].starts_with("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured;")
        && text
            .lines()
            .filter(|line| *line == format!("test {name} ... ok"))
            .count()
            == 1
}

fn private_dir(base: &Path, label: &str, mode: u32) -> Result<PathBuf> {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = base.join(format!("{label}-{}-{sequence}", std::process::id()));
    if dir.exists() {
        return Err("pilot private directory already exists".into());
    }
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, mode);
    #[cfg(not(unix))]
    let _ = mode;
    builder.create(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Cooperative controlled-capture grant, in the closed shape the platform
/// barrier accepts: the exact selector, a fresh 128-bit nonce and the PID of
/// the directly spawned test process. It grants no production capability.
#[cfg(any(unix, test))]
pub fn grant_bytes(nonce: &str, selector: &str, pid: u32) -> Result<Vec<u8>> {
    require(
        nonce.len() == 32
            && nonce
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && nonce.bytes().any(|b| b != b'0')
            && pid != 0,
        "controlled grant identity invalid",
    )?;
    serde_json::to_vec(&json!({"schema":"humanymous.controlled-capture-grant/1","nonce":nonce,"selector":selector,"pid":pid}))
        .map_err(|e| e.to_string())
}

/// OS entropy without a Unix-only device path, so the grant shape is also
/// exercised on Windows test runners.
#[cfg(any(unix, test))]
fn nonce() -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Outer supervisor for controlled-capture parents: spawns the test process
/// directly in its own process group (so the grant names the real test PID),
/// publishes the grant atomically and kills the whole group at the deadline.
#[cfg(unix)]
fn supervise(
    program: &Path,
    name: &str,
    root: &Path,
    private: &Path,
    log: &Path,
) -> Result<(Option<i32>, bool)> {
    use std::os::unix::process::CommandExt;
    let grant = private.join("grant.json");
    let nonce = nonce()?;
    let stdout = fs::File::create(log).map_err(|e| e.to_string())?;
    let stderr = fs::File::create(log.with_extension("stderr.log")).map_err(|e| e.to_string())?;
    let mut child = Command::new(program)
        .args(["--exact", name, "--ignored", "--test-threads=1"])
        .current_dir(root)
        .env("HUMANYMOUS_CAPTURE_OUTER_GRANT", &grant)
        .env("HUMANYMOUS_CAPTURE_OUTER_NONCE", &nonce)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .process_group(0)
        .spawn()
        .map_err(|e| e.to_string())?;
    let staged = private.join("grant.staged");
    let published = fs::write(&staged, grant_bytes(&nonce, name, child.id())?)
        .and_then(|()| fs::rename(&staged, &grant));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(900);
    let mut expired = published.is_err();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if expired || std::time::Instant::now() >= deadline {
            expired = true;
            let _ = Command::new("kill")
                .args(["-KILL", &format!("-{}", child.id())])
                .status();
            break child.wait().map_err(|e| e.to_string())?;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    published.map_err(|e| e.to_string())?;
    Ok((status.code(), !expired && status.success()))
}
/// Windows controlled-capture case for a declared pilot test, as the native
/// runner's fixed contract names it. Unknown names refuse.
pub fn windows_controlled_case(name: &str) -> Option<&'static str> {
    Some(match name.rsplit("::").next()? {
        "retained_normal_stdout_stderr_and_actual_joins" => "normal",
        "retained_synthetic_pending_stdin_expires_original_run" => "pending-input",
        "retained_stdout_cap_is_sticky" => "stdout-cap",
        "retained_stderr_cap_is_sticky" => "stderr-cap",
        "retained_stop_request_preserves_observed_exit_zero" => "stop-after-zero",
        "missing_grant_refuses_before_workload" => "outer-missing-grant",
        "parent_remains_running_until_original_outer_deadline" => "outer-live-timeout",
        _ => return None,
    })
}
/// On Windows the outer supervisor is the native runner's controlled-capture
/// mode: it owns the test process in a real Windows Job, writes (or, for the
/// missing-grant case, withholds) the grant and classifies the outcome. A test
/// counts only when that classification qualifies the case.
#[cfg(windows)]
fn supervise(
    program: &Path,
    name: &str,
    root: &Path,
    private: &Path,
    log: &Path,
) -> Result<(Option<i32>, bool)> {
    let case = windows_controlled_case(name).ok_or("undeclared Windows controlled case")?;
    let work = private.join("work");
    let digest = source::file(program)?;
    let stdout = fs::File::create(log).map_err(|e| e.to_string())?;
    let stderr = fs::File::create(log.with_extension("stderr.log")).map_err(|e| e.to_string())?;
    let status = Command::new("cargo")
        .args([
            "run",
            "--locked",
            "--quiet",
            "--manifest-path",
            "scripts/ci/windows-native-runner/Cargo.toml",
            "--",
            "controlled-capture",
            "--test-exe",
        ])
        .arg(program)
        .args(["--expected-sha256", &digest, "--case", case, "--work"])
        .arg(&work)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .status()
        .map_err(|e| e.to_string())?;
    let outer: Value = fs::read(work.join("outer-result.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(Value::Null);
    // The parent's own libtest output is the evidence for the pass cases; the
    // refusal cases are qualified by the outer classification alone.
    let parent = fs::read(work.join("parent.stdout.log")).unwrap_or_default();
    fs::write(log, &parent).map_err(|e| e.to_string())?;
    let qualified = status.success()
        && outer["schema"] == "humanymous.controlled-capture-outer/1"
        && outer["case"] == case
        && outer["test_exe_sha256"] == digest.as_str()
        && outer["case_qualified"] == true;
    Ok((status.code(), qualified))
}
#[cfg(not(any(unix, windows)))]
fn supervise(_: &Path, _: &str, _: &Path, _: &Path, _: &Path) -> Result<(Option<i32>, bool)> {
    Err("controlled outer-grant supervision is not implemented for this OS".into())
}

/// Run an unprivileged pilot in its own process group and kill the whole group
/// at the same 900 s deadline GNU timeout applies on Linux. A killed run keeps
/// its signal status, so it can never read as a pass.
#[cfg(unix)]
fn bounded_output(mut command: Command) -> Result<std::process::Output> {
    use std::{io::Read, os::unix::process::CommandExt};
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|e| e.to_string())?;
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut bytes);
            }
            bytes
        })
    };
    let stdout = drain(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let stderr = drain(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(900);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            let _ = Command::new("kill")
                .args(["-KILL", &format!("-{}", child.id())])
                .status();
            break child.wait().map_err(|e| e.to_string())?;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    Ok(std::process::Output {
        status,
        stdout: stdout.join().map_err(|_| "pilot stdout reader panicked")?,
        stderr: stderr.join().map_err(|_| "pilot stderr reader panicked")?,
    })
}
/// Windows: the same 900 s deadline on the direct test process. Declared
/// Windows pilots of this kind start no child processes of their own.
#[cfg(not(unix))]
fn bounded_output(mut command: Command) -> Result<std::process::Output> {
    use std::io::Read;
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut bytes);
            }
            bytes
        })
    };
    let stdout = drain(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let stderr = drain(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(900);
    let mut expired = false;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            expired = true;
            let _ = child.kill();
            break child.wait().map_err(|e| e.to_string())?;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    // A killed run can still report exit code 0 on some hosts; never let it pass.
    if expired {
        return Err("native pilot exceeded its 900 s deadline".into());
    }
    Ok(std::process::Output {
        status,
        stdout: stdout.join().map_err(|_| "pilot stdout reader panicked")?,
        stderr: stderr.join().map_err(|_| "pilot stderr reader panicked")?,
    })
}

fn execute(root: &Path, dir: &Path, pilot: &Value, exe: &Path, name: &str) -> Result<Value> {
    let id = pilot["id"].as_str().unwrap_or_default();
    let scratch = std::env::var_os("RUNNER_TEMP")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    if pilot["outer_grant"] == true {
        let log = dir.join(format!("pilot-{id}-{}.log", name.replace("::", "-")));
        let private = private_dir(&scratch, &format!("humanymous-{id}"), 0o700)?;
        let (code, success) = supervise(exe, name, root, &private, &log)?;
        let stdout = fs::read_to_string(&log).map_err(|e| e.to_string())?;
        // The Windows barrier cases expect a refusal or an outer kill, not a
        // libtest pass; the native runner's case classification decides them.
        let outer_only = cfg!(windows)
            && matches!(
                windows_controlled_case(name),
                Some("outer-missing-grant" | "outer-live-timeout")
            );
        let ok = success && (outer_only || passed(&stdout, name));
        if !ok {
            print!("{stdout}");
            eprint!(
                "{}",
                fs::read_to_string(log.with_extension("stderr.log")).unwrap_or_default()
            );
        }
        return Ok(
            json!({"pilot":id,"name":name,"status":if ok {"passed"} else {"failed"},"exit_code":code,"stdout_sha256":source::file(&log)?}),
        );
    }
    let mut program = exe.to_path_buf();
    let mut prefix: Vec<String> = Vec::new();
    if pilot["privilege"] == "root" {
        // The parent re-executes itself as an unprivileged user, so the binary
        // must sit in a directory every user can traverse and execute.
        let shared = private_dir(Path::new("/tmp"), &format!("humanymous-pilot-{id}"), 0o755)?;
        let copy = shared.join(exe.file_name().ok_or("executable name absent")?);
        fs::copy(exe, &copy).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        fs::set_permissions(&copy, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .map_err(|e| e.to_string())?;
        prefix = vec!["sudo".into(), "-n".into()];
        program = copy;
    }
    // GNU timeout bounds the Linux run, including the sudo-owned child. Other
    // Unix runners have no GNU timeout; there the pilot runs unprivileged in
    // its own process group and `bounded_output` enforces the same deadline.
    let linux = cfg!(target_os = "linux");
    if !linux && !prefix.is_empty() {
        return Err("privileged native pilots are declared for Linux runners only".into());
    }
    let mut command = if linux {
        let mut command = Command::new("timeout");
        command
            .args(["--signal=TERM", "--kill-after=15s", "900"])
            .args(&prefix)
            .arg(&program);
        command
    } else {
        Command::new(&program)
    };
    command
        .args(["--exact", name, "--ignored", "--test-threads=1"])
        .current_dir(root)
        .stdin(Stdio::null());
    if let Some(env) = pilot["private_parent_env"].as_str() {
        command.env(
            env,
            private_dir(&scratch, &format!("humanymous-{id}"), 0o700)?,
        );
    }
    if let Some(env) = pilot["source_root_env"].as_str() {
        command.env(env, root);
    }
    let output = if linux {
        command.output().map_err(|e| e.to_string())?
    } else {
        bounded_output(command)?
    };
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let log = dir.join(format!("pilot-{id}-{}.log", name.replace("::", "-")));
    fs::write(&log, &output.stdout).map_err(|e| e.to_string())?;
    fs::write(log.with_extension("stderr.log"), &output.stderr).map_err(|e| e.to_string())?;
    let ok = output.status.success() && passed(&stdout, name);
    if !ok {
        print!("{stdout}");
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(
        json!({"pilot":id,"name":name,"status":if ok {"passed"} else {"failed"},"exit_code":output.status.code(),"stdout_sha256":source::file(&log)?}),
    )
}

pub fn run(root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
    let declarations = declarations()?;
    verify_job_executors(root, &declarations)?;
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    let mut rows = Vec::new();
    for pilot in declarations["pilots"]
        .as_array()
        .ok_or("native pilots absent")?
    {
        if pilot["executor"] != "native-pilots" || !applies(pilot, os, arch) {
            continue;
        }
        report["stage"] = json!(format!("pilot-{}", pilot["id"].as_str().unwrap_or("")));
        let exe = executable(root, dir, pilot)?;
        for name in strings(&pilot["tests"])? {
            rows.push(execute(root, dir, pilot, &exe, &name)?);
        }
    }
    let failed = rows.iter().filter(|row| row["status"] != "passed").count();
    let receipt = json!({"schema":RECEIPT_SCHEMA,"source_sha":source::head(root)?,"os":os,"arch":arch,"declarations_sha256":source::sha(DECLARATIONS.as_bytes()),"tests":rows});
    let path = root.join(
        declarations["receipt"]
            .as_str()
            .ok_or("receipt path absent")?,
    );
    fs::create_dir_all(path.parent().ok_or("receipt parent absent")?).map_err(|e| e.to_string())?;
    write_json(&path, &receipt)?;
    report["test_counts"] =
        json!({"required":rows.len(),"passed":rows.len()-failed,"failed":failed});
    require(failed == 0, "declared native pilot failed")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_windows_outer_grant_test_maps_to_a_fixed_controlled_case() {
        let value = declarations().unwrap();
        let mut seen = 0;
        for pilot in value["pilots"].as_array().unwrap() {
            if pilot["outer_grant"] != true || !applies(pilot, "windows", "x86_64") {
                continue;
            }
            for name in strings(&pilot["tests"]).unwrap() {
                assert!(windows_controlled_case(&name).is_some(), "{name}");
                seen += 1;
            }
        }
        assert_eq!(seen, 7);
        assert_eq!(windows_controlled_case("arbitrary::test"), None);
    }
    #[test]
    fn checked_in_declarations_are_closed_and_valid() {
        let value = declarations().unwrap();
        assert!(value["pilots"].as_array().unwrap().len() >= 5);
    }
    #[test]
    fn declaration_rejects_duplicates_unknown_executors_and_open_names() {
        let mut value = declarations().unwrap();
        value["pilots"][0]["executor"] = json!("waiver");
        assert!(validate(&value).is_err());
        let mut value = declarations().unwrap();
        value["pilots"][1]["id"] = value["pilots"][0]["id"].clone();
        assert!(validate(&value).is_err());
        let mut value = declarations().unwrap();
        value["pilots"][0]["tests"] = json!(["bad name"]);
        assert!(validate(&value).is_err());
        let mut value = declarations().unwrap();
        value["pilots"][0]["tests"] = json!([]);
        assert!(validate(&value).is_err());
    }
    #[test]
    fn job_executors_exist_in_the_checked_in_workflow() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        verify_job_executors(&root, &declarations().unwrap()).unwrap();
        let mut value = declarations().unwrap();
        for pilot in value["pilots"].as_array_mut().unwrap() {
            if pilot["executor"] == "job" {
                pilot["job"]["run"] = json!("cargo run -- missing-step");
            }
        }
        assert!(verify_job_executors(&root, &value).is_err());
    }
    #[test]
    fn controlled_grant_is_closed_and_bound_to_identity() {
        let nonce = "0a".repeat(16);
        let bytes = grant_bytes(&nonce, "a::b", 7).unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 4);
        assert_eq!(value["schema"], "humanymous.controlled-capture-grant/1");
        assert_eq!(
            (value["selector"].clone(), value["pid"].clone()),
            (json!("a::b"), json!(7))
        );
        assert!(grant_bytes(&"0".repeat(32), "a", 7).is_err());
        assert!(grant_bytes(&"A".repeat(32), "a", 7).is_err());
        assert!(grant_bytes(&nonce, "a", 0).is_err());
        assert_eq!(super::nonce().unwrap().len(), 32);
    }
    #[test]
    fn outer_grant_requires_unprivileged_explicit_flag() {
        let mut value = declarations().unwrap();
        let index = value["pilots"]
            .as_array()
            .unwrap()
            .iter()
            .position(|p| p["outer_grant"] == true)
            .unwrap();
        value["pilots"][index]["privilege"] = json!("root");
        assert!(validate(&value).is_err());
        value["pilots"][index]["privilege"] = json!("user");
        value["pilots"][index]["outer_grant"] = json!("yes");
        assert!(validate(&value).is_err());
    }
    #[test]
    fn single_test_footer_must_be_exact() {
        let good = "running 1 test\ntest a::b ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.1s\n";
        assert!(passed(good, "a::b"));
        assert!(!passed(good, "a::c"));
        assert!(!passed(&good.replace("1 passed", "0 passed"), "a::b"));
        assert!(!passed(&good.replace("0 ignored", "1 ignored"), "a::b"));
        assert!(!passed(&format!("{good}{good}"), "a::b"));
    }
    #[test]
    fn applies_matches_os_and_optional_arch() {
        let pilot = json!({"os":["linux"],"arch":"x86_64"});
        assert!(applies(&pilot, "linux", "x86_64"));
        assert!(!applies(&pilot, "linux", "aarch64"));
        assert!(!applies(&pilot, "macos", "x86_64"));
        assert!(applies(&json!({"os":["linux"]}), "linux", "aarch64"));
    }
}
