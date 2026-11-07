//! Provisioned hosted-only CEF cases. Wire behind the explicit native pilot feature.
//! The shared supervisor reaps only its direct child; descendant cleanup is not attested.
use crate::pilot_assertions::{self, Case};
use humanymous_test_support::process::capture_with_stderr;
use std::{fs, io::Read, path::PathBuf, process::Command, time::Duration};

fn execute(case: Case, variable: &str) -> Result<(), &'static str> {
    let bundle = PathBuf::from(std::env::var_os(variable).ok_or("provisioned CEF bundle missing")?);
    if !bundle.is_absolute() {
        return Err("provisioned CEF bundle must be absolute");
    }
    let executable = bundle.join("humanymous_cef_bootstrap.exe");
    if !executable.is_file() {
        return Err("provisioned CEF executable missing");
    }
    let report_path = bundle.join("native-report.json");
    match fs::symlink_metadata(&report_path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        _ => return Err("fresh dedicated bundle required; report already exists or is unreadable"),
    }
    let mut command = Command::new(executable);
    command
        .current_dir(&bundle)
        .arg("--native-pilot-seconds=45")
        .arg("--native-internal-probe");
    if matches!(case, Case::Cancelled) {
        command.arg("--native-cancel-before-created");
    }
    let captured = capture_with_stderr(&mut command, b"", Duration::from_secs(90), 1024 * 1024)
        .map_err(|_| "native process capture failed; descendant cleanup remains unknown")?;
    // Do not dump private output in assertion errors or public summaries.
    let metadata = fs::symlink_metadata(&report_path).map_err(|_| "native report missing")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("native report is not an owned regular file");
    }
    const LIMIT: usize = 8 * 1024 * 1024;
    let mut bytes = Vec::new();
    fs::File::open(report_path)
        .map_err(|_| "native report open failed")?
        .take(LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "native report read failed")?;
    if bytes.len() > LIMIT {
        return Err("native report exceeds byte budget");
    }
    let report = serde_json::from_slice(&bytes).map_err(|_| "native report JSON invalid")?;
    let summary = pilot_assertions::verify(&report, captured.pid, captured.status.code(), case)?;
    let summary = serde_json::to_string(&summary).map_err(|_| "safe native summary failed")?;
    println!("{summary}");
    Ok(())
}

#[test]
fn provisioned_positive_internal_routes_and_cleanup() -> Result<(), &'static str> {
    execute(Case::Positive, "HUMANYMOUS_CEF_POSITIVE_BUNDLE")
}

#[test]
fn provisioned_cancellation_rejects_incomplete_probe_after_cleanup() -> Result<(), &'static str> {
    execute(Case::Cancelled, "HUMANYMOUS_CEF_CANCELLED_BUNDLE")
}
