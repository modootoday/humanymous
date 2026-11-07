//! Fixed version commands only; bounded capture and owned-child termination.
use serde::Serialize;
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const CAP: u64 = 16 * 1024;
#[derive(Debug, Serialize)]
pub struct Version {
    pub status: &'static str,
    pub text: Option<String>,
    pub expected_profile: bool,
}
impl Version {
    pub fn absent(status: &'static str) -> Self {
        Self {
            status,
            text: None,
            expected_profile: false,
        }
    }
}
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
pub(super) fn expected(name: &str, text: &str) -> bool {
    match name {
        "rustc" => {
            text.lines().any(|line| line.starts_with("rustc 1.99.1 "))
                && text
                    .lines()
                    .any(|line| line == "host: x86_64-pc-windows-msvc")
        }
        "cargo" => text.starts_with("cargo 1.99.1 "),
        "python312" => text.starts_with("Python 3.12."),
        "node" => text.starts_with("v24."),
        // Observed interpreter profile, not extension ABI compatibility.
        "ruby" => text.starts_with("ruby 3.3.") && text.contains("x64-mingw-ucrt"),
        "docker" => text.starts_with("Docker version "),
        "cmake" => text.starts_with("cmake version "),
        "ninja" => text.lines().next().is_some_and(|s| s.starts_with("1.")),
        _ => false,
    }
}
fn invoke(path: &Path, name: &str, timeout: Duration) -> io::Result<Version> {
    let dir = std::env::temp_dir().join(format!(
        "hmn-doctor-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos()
    ));
    fs::create_dir(&dir)?;
    let scratch = Scratch(dir);
    let out = scratch.0.join("stdout");
    let err = scratch.0.join("stderr");
    let stdout = fs::File::options()
        .write(true)
        .create_new(true)
        .open(&out)?;
    let stderr = fs::File::options()
        .write(true)
        .create_new(true)
        .open(&err)?;
    let args: &[&str] = if name == "rustc" {
        &["-Vv"]
    } else {
        &["--version"]
    };
    let mut command = Command::new(path);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn()?;
    let start = Instant::now();
    loop {
        let oversized = [&out, &err]
            .iter()
            .any(|p| fs::metadata(p).map_or(true, |m| m.len() > CAP));
        if oversized || start.elapsed() >= timeout {
            let _ = child.kill();
            child.wait()?;
            return Ok(Version::absent(if oversized {
                "output_limit"
            } else {
                "timeout"
            }));
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Ok(Version::absent("exit_failure"));
                }
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        }
    }
    let mut bytes = Vec::new();
    for file in [&out, &err] {
        fs::File::open(file)?
            .take(CAP + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > CAP {
            return Ok(Version::absent("output_limit"));
        }
    }
    let Ok(text) = String::from_utf8(bytes) else {
        return Ok(Version::absent("invalid_utf8"));
    };
    let text = text.trim().to_owned();
    let accepted = expected(name, &text);
    Ok(Version {
        status: "observed",
        text: Some(text),
        expected_profile: accepted,
    })
}
pub fn run(path: &Path, name: &str, timeout: Duration) -> Version {
    if timeout.is_zero() {
        return Version::absent("command_deadline");
    }
    invoke(path, name, timeout).unwrap_or_else(|_| Version::absent("io_failure"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_profiles_reject_wrong_python_host_and_ruby_abi() {
        assert!(expected("python312", "Python 3.12.10"));
        assert!(!expected("python312", "Python 3.13.0"));
        assert!(expected(
            "rustc",
            "rustc 1.99.1 (test)\nhost: x86_64-pc-windows-msvc"
        ));
        assert!(!expected(
            "rustc",
            "rustc 1.99.1 (test)\nhost: x86_64-pc-windows-gnu"
        ));
        assert!(expected("ruby", "ruby 3.3.12 [x64-mingw-ucrt]"));
        assert!(!expected("ruby", "ruby 3.3.12 [x64-mswin64]"));
        assert!(!expected("unknown", "1.0"));
    }
    #[test]
    fn fixed_probe_reports_missing_executable_and_expired_budget() {
        let absent = std::env::temp_dir().join("hmn-doctor-no-such-executable.exe");
        assert_eq!(
            run(&absent, "python312", Duration::ZERO).status,
            "command_deadline"
        );
        assert_eq!(
            run(&absent, "python312", Duration::from_secs(1)).status,
            "io_failure"
        );
    }
    #[test]
    fn actual_child_failure_is_not_a_version_success() {
        // The Rust test harness rejects --version; no shell or foreign fixture.
        let result = run(
            &std::env::current_exe().unwrap(),
            "python312",
            Duration::from_secs(3),
        );
        assert_eq!(result.status, "exit_failure");
        assert!(!result.expected_profile);
        assert!(result.text.is_none());
    }
}
