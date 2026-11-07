//! Provision and execute the explicit native CEF rejection pilot, never GUI qualification.
mod desktop;
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
fn hosted(actions: Option<&str>, environment: Option<&str>, act_present: bool) -> bool {
    actions == Some("true") && environment == Some("github-hosted") && !act_present
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

fn run(output: &Path, report: &mut serde_json::Value) -> io::Result<()> {
    if !cfg!(all(windows, target_arch = "x86_64")) {
        return Err(io::Error::other(
            "Windows x64 native pilot required; other tuples are unsupported",
        ));
    }
    if !hosted(
        std::env::var("GITHUB_ACTIONS").ok().as_deref(),
        std::env::var("RUNNER_ENVIRONMENT").ok().as_deref(),
        std::env::var_os("ACT").is_some(),
    ) {
        return Err(io::Error::other(
            "standard hosted runner required for actual native pilot",
        ));
    }
    let source = provisioning::source_snapshot()?;
    report["source"] = source.clone();
    report["stage"] = "native-build".into();
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
    report["stage"] = "desktop-protocol-build".into();
    logged(
        Command::new("cargo").env("PATH", &tool_path).args([
            "test",
            "--locked",
            "-p",
            "humanymous-cef-desktop",
            "--features",
            "native-pilot-tests",
            "--lib",
            "--no-run",
        ]),
        &output.join("desktop-protocol-build.log"),
    )?;
    report["stage"] = "desktop-protocol-tests".into();
    let selector = desktop::protocol_selector()?;
    let protocol_log = output.join("desktop-protocol-tests.log");
    logged(
        Command::new("cargo").env("PATH", &tool_path).args([
            "test",
            "--locked",
            "-p",
            "humanymous-cef-desktop",
            "--features",
            "native-pilot-tests",
            "--lib",
            &selector,
            "--",
            "--test-threads=1",
        ]),
        &protocol_log,
    )?;
    report["protocol_preflight"] = desktop::verify_protocol(&fs::read_to_string(protocol_log)?)?;
    report["stage"] = "bootstrap-rejection".into();
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
    report["passed_tests"] = 1.into();
    report["bootstrap_ignored_tests"] = 0.into();
    report["rejected_override_cases"] = 4.into();
    report["bootstrap_rejection_passed"] = true.into();
    report["stage"] = "desktop-bundles".into();
    let private = output
        .canonicalize()?
        .parent()
        .ok_or_else(|| io::Error::other("private bundle parent absent"))?
        .join("cef-native-private");
    fs::create_dir(&private)?;
    let positive = provisioning::fresh_bundle(&bundle, &private, "cef-positive")?;
    let cancelled = provisioning::fresh_bundle(&bundle, &private, "cef-cancelled")?;
    report["stage"] = "desktop-process-tests".into();
    let desktop_log = output.join("desktop-process-pilot.log");
    logged(
        Command::new("cargo")
            .env("PATH", &tool_path)
            .env("HUMANYMOUS_CEF_POSITIVE_BUNDLE", positive)
            .env("HUMANYMOUS_CEF_CANCELLED_BUNDLE", cancelled)
            .args([
                "test",
                "--locked",
                "-p",
                "humanymous-cef-desktop",
                "--lib",
                "--features",
                "native-pilot-tests",
                "pilot_process::",
                "--",
                "--test-threads=1",
                "--nocapture",
            ]),
        &desktop_log,
    )?;
    report["desktop_cases"] = desktop::verify(&fs::read_to_string(&desktop_log)?)?;
    report["passed_tests"] = 3.into();
    report["desktop_passed_tests"] = 2.into();
    report["ignored_tests"] = 0.into();
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
    report["artifacts"] = hashes.into();
    report["status"] = "passed".into();
    report["stage"] = "complete".into();
    Ok(())
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
    let mut report = json!({"status":"failed","scope":"separate synthetic protocol preflight, bootstrap rejection and two provisioned desktop process cases","required_test":CASE,"passed_tests":null,"ignored_tests":null,"desktop_passed_tests":null,"protocol_preflight":null,"bootstrap_rejection_passed":false,"required_override_cases":4,"rejected_override_cases":null,"stage":"source-preflight","browser_ready":false,"full_runtime_qualified":false,"gui_keyboard_ime_ax":"not qualified by these bounded probes","descendant_cleanup_attested":false});
    match run(output, &mut report) {
        Ok(()) => fs::write(
            output.join("report.json"),
            serde_json::to_vec_pretty(&report)?,
        ),
        Err(error) => {
            report["status"] = "failed".into();
            report["error_code"] = "native_pilot_failed".into();
            fs::write(
                output.join("report.json"),
                serde_json::to_vec_pretty(&report)?,
            )?;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_act_labels_cannot_start_native_browser_pilots() {
        assert!(hosted(Some("true"), Some("github-hosted"), false));
        assert!(!hosted(Some("true"), Some("github-hosted"), true));
        assert!(!hosted(Some("true"), Some("self-hosted"), false));
        assert!(!hosted(None, Some("github-hosted"), false));
    }
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
