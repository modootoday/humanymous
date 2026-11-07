//! Provision and execute the explicit native CEF rejection pilot, never GUI qualification.
mod provisioning;
use serde_json::json;
use std::{
    fs, io,
    path::Path,
    process::{Command, Stdio},
};

const CASE: &str =
    "tests::native_bootstrap_rejects_security_overrides_before_ui_or_profile_creation";

fn logged(command: &mut Command, path: &Path) -> io::Result<()> {
    let file = fs::File::create(path)?;
    let status = command
        .stdout(Stdio::from(file.try_clone()?))
        .stderr(Stdio::from(file))
        .status()?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "command failed ({status}); inspect {}",
            path.display()
        )));
    }
    Ok(())
}

fn verified_case(log: &str) -> bool {
    let summaries: Vec<_> = log
        .lines()
        .filter(|line| line.starts_with("test result:"))
        .collect();
    log.lines()
        .filter(|line| *line == format!("test {CASE} ... ok"))
        .count()
        == 1
        && summaries.len() == 1
        && summaries[0].starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;")
}

fn run(output: &Path) -> io::Result<serde_json::Value> {
    if !cfg!(all(windows, target_arch = "x86_64")) {
        return Err(io::Error::other(
            "Windows x64 native pilot required; other tuples are unsupported",
        ));
    }
    let source = provisioning::source_snapshot()?;
    let tool_path = provisioning::tool_path()?;
    logged(
        Command::new("cargo").env("PATH", &tool_path).args([
            "build",
            "-p",
            "humanymous-cef-bootstrap",
            "-p",
            "humanymous-cef-desktop",
            "--locked",
        ]),
        &output.join("native-build.log"),
    )?;
    let bundle = provisioning::prepare()?;
    let log = output.join("native-negative-pilot.log");
    logged(
        Command::new("cargo")
            .env("PATH", &tool_path)
            .env("HUMANYMOUS_CEF_PILOT_BUNDLE", &bundle)
            .args([
                "test",
                "-p",
                "humanymous-cef-bootstrap",
                "--locked",
                "--features",
                "native-pilot-tests",
                "--",
                CASE,
                "--exact",
                "--nocapture",
            ]),
        &log,
    )?;
    if !verified_case(&fs::read_to_string(log)?) {
        return Err(io::Error::other(
            "required provisioned case was not executed exactly once without ignore",
        ));
    }
    if source["files"] != provisioning::source_snapshot()?["files"] {
        return Err(io::Error::other("native source changed during build/pilot"));
    }
    let artifacts = [
        "humanymous_cef_bootstrap.exe",
        "humanymous_cef_bootstrap.dll",
        "humanymous_cef_desktop.dll",
        "libcef.dll",
    ];
    let mut hashes = serde_json::Map::new();
    for name in artifacts {
        hashes.insert(
            name.into(),
            provisioning::digest(&bundle.join(name))?.into(),
        );
    }
    Ok(
        json!({"status":"passed", "source":source, "scope":"Windows DLL build and explicit security-override rejection pilot", "required_test":CASE,
        "passed_tests":1, "ignored_tests":0, "rejected_override_cases":4, "artifacts":hashes,
        "browser_ready":false, "full_runtime_qualified":false,
        "gui_keyboard_ime_ax":"not tested by this runner", "sandbox_policy":"not qualified by rejecting flags"}),
    )
}

fn main() -> io::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let output = args
        .next()
        .ok_or_else(|| io::Error::other("usage: humanymous-cef-pilot OUTPUT_DIRECTORY"))?;
    if args.next().is_some() {
        return Err(io::Error::other("unexpected argument"));
    }
    if output == "--configure-toolchain" {
        return provisioning::configure_github_path();
    }
    if output.to_string_lossy().starts_with('-') {
        return Err(io::Error::other("unknown runner option"));
    }
    let output = Path::new(&output);
    fs::create_dir_all(output)?;
    match run(output) {
        Ok(report) => fs::write(
            output.join("report.json"),
            serde_json::to_vec_pretty(&report)?,
        ),
        Err(error) => {
            fs::write(
                output.join("report.json"),
                serde_json::to_vec_pretty(
                    &json!({"status":"failed", "error":error.to_string(), "full_runtime_qualified":false, "browser_ready":false}),
                )?,
            )?;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_or_ignored_harness_cannot_qualify() {
        assert!(!verified_case(
            "test result: ok. 0 passed; 0 failed; 0 ignored;"
        ));
        assert!(!verified_case(&format!(
            "test {CASE} ... ignored\ntest result: ok. 0 passed; 0 failed; 1 ignored;"
        )));
        assert!(!verified_case(&format!(
            "test {CASE} ... ok\ntest result: FAILED"
        )));
        assert!(verified_case(&format!(
            "test {CASE} ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out;"
        )));
        assert!(!verified_case(&format!(
            "test {CASE} ... ok\ntest {CASE} ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored;"
        )));
    }
}
