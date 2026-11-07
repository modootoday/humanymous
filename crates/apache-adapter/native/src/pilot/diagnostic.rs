//! Separate legacy CLI observation; never one of the required 45 case pairs.
use super::{
    Error, Server, authority, ensure, origin, pin_httpd, request, startup, write_configuration,
};
use rustix::process::{Signal, pidfd_send_signal};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
use std::process::{Command, ExitStatus};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

const LIMIT: u64 = 65_536;
struct Capture {
    status: ExitStatus,
    timed_out: bool,
    limited: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
fn bounded(path: &Path) -> Result<(Vec<u8>, bool), Error> {
    let mut bytes = Vec::new();
    File::open(path)?.take(LIMIT + 1).read_to_end(&mut bytes)?;
    let limited = bytes.len() as u64 > LIMIT;
    bytes.truncate(LIMIT as usize);
    Ok((bytes, limited))
}
fn capture(server: &Server, base: &Path) -> Result<Capture, Error> {
    let stdout_path = base.join("legacy-cli-stdout.raw");
    let stderr_path = base.join("legacy-cli-stderr.raw");
    let stdout = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&stdout_path)?;
    let stderr = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&stderr_path)?;
    let child = Command::new("/opt/httpd/bin/httpd")
        .process_group(0)
        .arg("-f")
        .arg(&server.config)
        .args(["-k", "stop"])
        .stdout(stdout)
        .stderr(stderr)
        .spawn()?;
    let (child, pidfd) = pin_httpd(child)?;
    // Reuse the exact instance-owner RAII for this short-lived helper. No
    // generation/HTTP methods are called on it, and no numeric signal is used.
    let mut helper = Server {
        child,
        pidfd,
        config: server.config.clone(),
        stopped: false,
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut timed_out = false;
    let mut limited = false;
    let status = loop {
        if let Some(status) = helper.child.try_wait()? {
            helper.stopped = true;
            break status;
        }
        limited = std::fs::metadata(&stdout_path)?.len() > LIMIT
            || std::fs::metadata(&stderr_path)?.len() > LIMIT;
        timed_out = Instant::now() >= deadline;
        if limited || timed_out {
            pidfd_send_signal(&helper.pidfd, Signal::KILL).map_err(std::io::Error::from)?;
            let status = helper.child.wait()?;
            helper.stopped = true;
            break status;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let (stdout, out_limit) = bounded(&stdout_path)?;
    let (stderr, err_limit) = bounded(&stderr_path)?;
    Ok(Capture {
        status,
        timed_out,
        limited: limited || out_limit || err_limit,
        stdout,
        stderr,
    })
}
fn pid_file_matches(server: &Server, base: &Path) -> Option<bool> {
    let mut bytes = Vec::new();
    File::open(base.join("httpd.pid"))
        .ok()?
        .take(65)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > 64 {
        return Some(false);
    }
    let pid = std::str::from_utf8(&bytes)
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()?;
    Some(pid == server.child.id())
}
pub(super) fn run() -> Result<bool, Error> {
    let base = std::env::temp_dir().join(format!(
        "h-apache-cli-{}",
        humanymous_framework_adapter::uuid()?
    ));
    std::fs::create_dir(&base)?;
    std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o700))?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    let mut service = runtime.block_on(authority::start(&base))?;
    let origin = origin::Origin::start()?;
    let private = write_configuration(&base, &service.credential)?;
    let (mut server, port) = startup(&base, "prefork", origin.port, &private, false, true)?;
    let before = origin.count.load(Ordering::Acquire);
    let nested = request(port, "/probe-subrequest", b"", "").is_ok_and(|(status, _, headers)| {
        status != 200
            && headers
                .to_ascii_lowercase()
                .contains("x-pilot-probe-ran: subrequest")
            && origin.count.load(Ordering::Acquire) == before
    });
    let prior = server.child.try_wait()?;
    if prior.is_some() {
        server.stopped = true;
    }
    ensure(prior.is_none(), "isolated diagnostic master exited")?;
    let pid_match = pid_file_matches(&server, &base);
    // A CLI may read only this reserved, unreaped master's private PID file.
    // A missing/mismatched proof is recorded as not-run, never signalled.
    let captured = if pid_match == Some(true) {
        Some(capture(&server, &base)?)
    } else {
        None
    };
    let after = server.child.try_wait()?;
    if after.is_some() {
        server.stopped = true;
    }
    let after_alive = after.is_none();
    let mut final_status = after;
    let mut forced = false;
    let cleanup = if let Some(status) = after {
        status.success()
    } else {
        let result = server.stop();
        if result.is_err() && !server.stopped {
            forced = true;
            let _ = pidfd_send_signal(&server.pidfd, Signal::KILL);
            final_status = server.child.wait().ok();
            server.stopped = final_status.is_some();
        } else {
            final_status = server.child.try_wait()?;
        }
        result.is_ok() && final_status.is_some_and(|status| status.success())
    };
    let empty = Vec::new();
    let stdout = captured.as_ref().map_or(&empty, |value| &value.stdout);
    let stderr = captured.as_ref().map_or(&empty, |value| &value.stderr);
    // These bounded byte carriers go only to the coordinator's private stderr
    // file. It decodes them privately and never copies them into public logs.
    eprintln!("APACHE_PRIVATE_CLI_STDOUT {}", hex::encode(stdout));
    eprintln!("APACHE_PRIVATE_CLI_STDERR {}", hex::encode(stderr));
    let mut code_bytes = stdout.clone();
    code_bytes.extend_from_slice(stderr);
    let timed_out = captured.as_ref().is_some_and(|value| value.timed_out);
    let limited = captured.as_ref().is_some_and(|value| value.limited);
    let cleanup_passed = cleanup && !forced && !timed_out && !limited;
    let observation = serde_json::json!({"schema":"humanymous.apache.cli-diagnostic/0.1",
        "mpm":"prefork","role":"isolated-cli-stop-observation","nestedProof":nested,
        "pidFileMatches":pid_match,"cliStarted":captured.is_some(),"cliReaped":captured.is_some(),
        "cliExit":captured.as_ref().and_then(|value| value.status.code()),
        "cliSignal":captured.as_ref().and_then(|value| value.status.signal()),
        "cliTimedOut":timed_out,"outputLimited":limited,"stdoutBytes":stdout.len(),"stderrBytes":stderr.len(),
        "apacheCodes":crate::coordinator::apache_error_codes(&code_bytes),"masterBeforeAlive":true,
        "masterAfterCliAlive":after_alive,"masterExit":final_status.and_then(|value| value.code()),
        "masterSignal":final_status.and_then(|value| value.signal()),"masterReaped":server.stopped,
        "forcedCleanup":forced,"cleanupPassed":cleanup_passed});
    eprintln!(
        "APACHE_CLI_DIAGNOSTIC {}",
        serde_json::to_string(&observation)?
    );
    runtime.block_on(service.stop());
    drop(service);
    drop(server);
    drop(origin);
    drop(runtime);
    Ok(cleanup_passed && nested && captured.is_some())
}
