//! SoT dotnet-protected-host-launch N1 fixture (owner decision D5(a), issue 75
//! comment 6998447599). It proves on a hosted runner that the secondary-logon
//! service delivers the std handles of the NoProfile actor launch: the actor
//! child sees a pipe on stdin (N2), reads the exact frame and then EOF, and its
//! stdout and stderr reach the parent's bounded readers. EOF in the child is
//! only observable once every write end of the creator pipe is closed, so the
//! parent's dropped write end leaves no live copy in the child or elsewhere.
//! The child is this runner binary in mode `dotnet-stdio-child`, inside the
//! kill-on-close Job; it runs no product code.
use super::{
    CleanupTarget, StartFailure, account, acl, generated_actor, profile_absent,
    start_prebuilt_actor,
};
use crate::evidence;
use serde_json::{Value, json};
use sha2::Digest;
use std::{
    ffi::{OsStr, OsString},
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc,
    time::{Duration, Instant},
};

pub(crate) const CHILD_MODE: &str = "dotnet-stdio-child";
const CHILD_SCHEMA: &str = "humanymous.dotnet-n1-child/1";
const REPORT_SCHEMA: &str = "humanymous.dotnet-n1-fixture/1";
const STDERR_MARKER: &str = "humanymous.dotnet-n1-stderr/1 ok\n";
/// Larger than one default pipe buffer, so delivery needs concurrent reads.
const FRAME_BYTES: usize = 96 * 1024;
const STREAM_LIMIT: usize = 64 * 1024;
const RUN_BUDGET: Duration = Duration::from_secs(120);
const CLEANUP_BUDGET: Duration = Duration::from_secs(30);
const CHANNEL_BUDGET: Duration = Duration::from_secs(30);

fn bounded_reader(mut stream: io::PipeReader, limit: usize) -> mpsc::Receiver<io::Result<Vec<u8>>> {
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = (&mut stream)
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = sender.send(result);
    });
    receiver
}

fn hex(bytes: &[u8]) -> String {
    format!("{:x}", sha2::Sha256::digest(bytes))
}

/// The child's own observations. Every field is a native fact of the child.
pub(crate) fn child(args: &mut impl Iterator<Item = String>) -> io::Result<i32> {
    let (Some(flag), Some(expected), None) = (args.next(), args.next(), args.next()) else {
        return Err(io::Error::other("--expected-sid <sid> required"));
    };
    if flag != "--expected-sid" {
        return Err(io::Error::other("--expected-sid <sid> required"));
    }
    let pipe = humanymous_platform::windows_object::stdin_is_pipe()
        .map_err(|_| io::Error::other("stdin type query failed"))?;
    let identity = humanymous_runtime::native_security::process::current_identity()?;
    let (sender, receiver) = mpsc::sync_channel(1);
    if pipe {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = io::stdin()
                .lock()
                .take(FRAME_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .map(|_| bytes);
            let _ = sender.send(result);
        });
    }
    let frame = if pipe {
        match receiver.recv_timeout(CHANNEL_BUDGET) {
            Ok(Ok(bytes)) => Some(bytes),
            _ => None,
        }
    } else {
        None
    };
    let eof = frame
        .as_ref()
        .is_some_and(|bytes| bytes.len() <= FRAME_BYTES);
    let line = json!({
        "schema": CHILD_SCHEMA,
        "stdin_pipe": pipe,
        "eof_observed": eof,
        "bytes": frame.as_ref().map(Vec::len),
        "sha256": frame.as_deref().map(hex),
        "sid_matches": identity.sid() == expected,
        "elevated": identity.elevated(),
        "enabled_administrator": identity.enabled_administrator(),
    });
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{line}")?;
    stdout.flush()?;
    let mut stderr = io::stderr().lock();
    stderr.write_all(STDERR_MARKER.as_bytes())?;
    stderr.flush()?;
    Ok(
        if pipe && eof && identity.sid() == expected && !identity.elevated() {
            0
        } else {
            3
        },
    )
}

/// Exact expected child line; any other content fails the fixture.
pub(crate) fn child_line_ok(stdout: &[u8], frame: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(stdout) else {
        return false;
    };
    let Some(line) = text.strip_suffix('\n') else {
        return false;
    };
    let line = line.strip_suffix('\r').unwrap_or(line);
    if line.contains('\n') {
        return false;
    }
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return false;
    };
    value
        == json!({
            "schema": CHILD_SCHEMA,
            "stdin_pipe": true,
            "eof_observed": true,
            "bytes": frame.len(),
            "sha256": hex(frame),
            "sid_matches": true,
            "elevated": false,
            "enabled_administrator": false,
        })
}

pub(super) fn icacls(path: &Path, args: &[&str]) -> io::Result<()> {
    let status = Command::new("icacls.exe")
        .arg(path)
        .args(args)
        .stdout(std::process::Stdio::null())
        .status()?;
    if !status.success() {
        return Err(io::Error::other("runner image ACL change failed"));
    }
    Ok(())
}

/// Elevated parent: provision one generated standard account, launch the
/// child through the approved N1/N5 variant and require every fact.
pub(crate) fn run(report: &Path) -> io::Result<i32> {
    if !report.is_absolute() {
        return Err(io::Error::other("absolute --report required"));
    }
    if !evidence::disposable_host() {
        return Err(io::Error::other("disposable GitHub-hosted actor required"));
    }
    let identity = humanymous_runtime::native_security::process::current_identity()?;
    if !identity.elevated() || !identity.enabled_administrator() {
        return Err(io::Error::other(
            "hosted provisioning administrator required",
        ));
    }
    let (name, password) = generated_actor()?;
    let mut owner = account(&name, password)?;
    let (sid, _, _) =
        windows_permissions::wrappers::LookupAccountName(Option::<&OsStr>::None, &name)?;
    let sid = sid.to_string();
    let system = std::env::var("SystemRoot").map_err(io::Error::other)?;
    let drive = std::env::var("SystemDrive").map_err(io::Error::other)?;
    let profile_directory = PathBuf::from(format!("{drive}\\Users\\{name}"));
    let base = PathBuf::from(
        std::env::var_os("ProgramData").ok_or_else(|| io::Error::other("ProgramData missing"))?,
    );
    let scratch = base.join(&name);
    fs::create_dir(&scratch)?;
    let exe = std::env::current_exe()?.canonicalize()?;
    let mut facts = json!({});
    let outcome = (|| -> io::Result<()> {
        acl(&scratch, &sid, identity.sid())?;
        icacls(&exe, &["/grant", &format!("*{sid}:(RX)"), "/Q", "/L"])?;
        icacls(
            &exe,
            &[
                "/deny",
                &format!("*{sid}:(WD,AD,WEA,WA,DE,DC,WDAC,WO)"),
                "/Q",
                "/L",
            ],
        )?;
        let environment = vec![
            ("SystemRoot".to_owned(), system.clone()),
            ("WINDIR".to_owned(), system.clone()),
            ("PATH".to_owned(), format!("{system}\\System32;{system}")),
        ];
        let arguments: Vec<OsString> = vec![
            CHILD_MODE.into(),
            "--expected-sid".into(),
            sid.clone().into(),
        ];
        facts["profile_absent_before"] = json!(profile_absent(&sid, &profile_directory)?);
        let started = Instant::now();
        let actor = match start_prebuilt_actor(
            &owner.account,
            &owner.password,
            (&arguments, &environment, &scratch),
            &sid,
            &profile_directory,
        ) {
            Ok(actor) => actor,
            Err(StartFailure::NotStarted(error)) => {
                facts["start"] = json!("not-started");
                return Err(error);
            }
            Err(StartFailure::Retained(mut failure)) => {
                facts["start"] = json!("retained-failure");
                let contained = failure.owner.contain(Instant::now() + CLEANUP_BUDGET);
                facts["retained_contained"] = json!(contained.is_ok());
                return Err(failure.error);
            }
        };
        facts["start"] = json!("resumed-in-job");
        let super::PrebuiltActor {
            mut owner,
            mut creator,
            output,
            error,
        } = actor;
        let stdout = bounded_reader(output, STREAM_LIMIT);
        let stderr = bounded_reader(error, STREAM_LIMIT);
        let mut frame = vec![0u8; FRAME_BYTES];
        getrandom::fill(&mut frame).map_err(io::Error::other)?;
        let written = creator.write_all(&frame).and_then(|()| creator.flush());
        // The parent's only write end closes here; the child can observe EOF
        // only if no other write end of this pipe is alive anywhere.
        drop(creator);
        facts["frame_written"] = json!(written.is_ok());
        let run = owner.wait_run(started + RUN_BUDGET);
        let proof = owner.contain(Instant::now() + CLEANUP_BUDGET);
        let (exit_code, natural, job_scope) = match (&run, &proof) {
            (Ok(exit), Ok(proof)) => (
                Some(exit.code),
                exit.natural && proof.exit.natural,
                proof.scope == CleanupTarget::OwnedJob,
            ),
            _ => (None, false, false),
        };
        drop(proof);
        facts["child_exit_code"] = json!(exit_code);
        facts["natural_exit"] = json!(natural);
        facts["job_empty_observed"] = json!(job_scope);
        let collect = |receiver: mpsc::Receiver<io::Result<Vec<u8>>>| {
            receiver
                .recv_timeout(CLEANUP_BUDGET)
                .ok()
                .and_then(Result::ok)
                .filter(|bytes| bytes.len() <= STREAM_LIMIT)
        };
        let (stdout, stderr) = (collect(stdout), collect(stderr));
        let stdout_ok = stdout
            .as_deref()
            .is_some_and(|bytes| child_line_ok(bytes, &frame));
        let stderr_ok = stderr.as_deref() == Some(STDERR_MARKER.as_bytes());
        facts["stdout_joined"] = json!(stdout.is_some());
        facts["stderr_joined"] = json!(stderr.is_some());
        facts["child_line_exact"] = json!(stdout_ok);
        facts["stderr_marker_exact"] = json!(stderr_ok);
        facts["frame_bytes"] = json!(FRAME_BYTES);
        if !stdout_ok {
            // Child facts only; no SID, path or frame bytes leave this report.
            facts["child_line"] = stdout
                .as_deref()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .and_then(|text| serde_json::from_str::<Value>(text.trim_end()).ok())
                .map(|mut value| {
                    value["sha256"] = json!(value["sha256"] == json!(hex(&frame)));
                    value
                })
                .unwrap_or(Value::Null);
        }
        drop(owner);
        written?;
        run?;
        if exit_code != Some(0) || !natural || !job_scope || !stdout_ok || !stderr_ok {
            return Err(io::Error::other("N1 std-handle delivery not observed"));
        }
        Ok(())
    })();
    let ungranted = icacls(&exe, &["/remove", &format!("*{sid}"), "/Q", "/L"]);
    let deletion = owner.delete_legacy_account();
    let profile_after = profile_absent(&sid, &profile_directory);
    let cleanup = if scratch.parent() == Some(base.as_path())
        && scratch.file_name() == Some(OsStr::new(&name))
    {
        fs::remove_dir_all(&scratch)
    } else {
        Err(io::Error::other("unsafe generated cleanup path"))
    };
    let mut failures = Vec::new();
    for (label, result) in [
        ("fixture", outcome.as_ref().map(|_| ())),
        ("exe grant removal", ungranted.as_ref().map(|_| ())),
        ("account deletion", deletion.as_ref().map(|_| ())),
        ("scratch cleanup", cleanup.as_ref().map(|_| ())),
    ] {
        if let Err(error) = result {
            failures.push(format!("{label}: {error}"));
        }
    }
    let absent_after = matches!(profile_after, Ok(true));
    if !absent_after {
        failures.push("profile present or unreadable after NoProfile launch".into());
    }
    if facts["profile_absent_before"] != true {
        failures.push("profile not absent before launch".into());
    }
    facts["profile_absent_after"] = json!(absent_after);
    facts["account_deleted"] = json!(deletion.is_ok());
    facts["scratch_deleted"] = json!(cleanup.is_ok());
    let passed = failures.is_empty();
    let value = json!({
        "schema": REPORT_SCHEMA,
        "sot": "sot:root/dotnet-protected-host-launch N1/N2/N5",
        "decision": "D5(a) issue 75 comment 6998447599",
        "status": if passed { "passed" } else { "failed" },
        "launch": "CreateProcessWithLogonW logonflags 0 STARTF_USESTDHANDLES, kill-on-close Job",
        "facts": facts,
        "failures": failures,
    });
    evidence::write(report, &value)?;
    Ok(if passed { 0 } else { 1 })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn child_line_must_be_exact() {
        let frame = b"frame".to_vec();
        let good = format!(
            "{}\n",
            json!({"schema":CHILD_SCHEMA,"stdin_pipe":true,"eof_observed":true,"bytes":5,"sha256":hex(&frame),"sid_matches":true,"elevated":false,"enabled_administrator":false})
        );
        assert!(child_line_ok(good.as_bytes(), &frame));
        assert!(child_line_ok(good.replace('\n', "\r\n").as_bytes(), &frame));
        assert!(!child_line_ok(good.trim_end().as_bytes(), &frame));
        assert!(!child_line_ok(format!("{good}{good}").as_bytes(), &frame));
        assert!(!child_line_ok(good.as_bytes(), b"other"));
        for (field, wrong) in [
            ("stdin_pipe", json!(false)),
            ("eof_observed", json!(false)),
            ("sid_matches", json!(false)),
            ("elevated", json!(true)),
            ("enabled_administrator", json!(true)),
            ("bytes", json!(4)),
        ] {
            let mut value: Value = serde_json::from_str(good.trim_end()).unwrap();
            value[field] = wrong;
            assert!(!child_line_ok(format!("{value}\n").as_bytes(), &frame));
        }
    }
}
