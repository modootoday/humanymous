//! Native tuple qualification and integration of the single owning actor/parser.
use crate::{Result, command, evidence, read_json, require, source, write_json};
use serde_json::json;
use std::path::Path;
fn tuple(os: &str, arch: &str) -> Result<()> {
    require(
        os == std::env::consts::OS,
        "requested OS differs from executing native process",
    )?;
    let observed = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        _ => return Err("unsupported native architecture".into()),
    };
    require(
        arch == observed,
        "requested architecture differs from executing native process",
    )
}
pub fn run(root: &Path, os: &str, arch: &str) -> Result<()> {
    tuple(os, arch)?;
    let directory = evidence::bundle(root)?;
    let raw = directory.join("workspace-owner.json");
    let output = root.join("target/runner-probe/report.json");
    let before = source::snapshot(root)?;
    write_json(&directory.join("source-before.json"), &before)?;
    let binaries = root.join("target/debug");
    let mode = if cfg!(windows) && std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
        "standard-user-workspace"
    } else {
        "workspace"
    };
    let code = command::log(
        root,
        "cargo",
        &[
            "run",
            "--locked",
            "--manifest-path",
            "scripts/ci/windows-native-runner/Cargo.toml",
            "--",
            mode,
            "--report",
            raw.to_str().ok_or("non-UTF8 report")?,
            "--binaries",
            binaries.to_str().ok_or("non-UTF8 artifacts")?,
        ],
        &directory.join("workspace-owner.log"),
    )?;
    let after = source::snapshot(root)?;
    write_json(&directory.join("source-after.json"), &after)?;
    if !raw.is_file() {
        write_json(
            &raw,
            &json!({"schema":"humanymous.workspace-probe/1","kind":"workspace","status":"failed","full_runtime_qualified":false,"errors":1,"reason":"owner report absent","source_sha":before["source_sha"]}),
        )?;
    }
    let report = read_json(&raw)?;
    std::fs::create_dir_all(output.parent().ok_or("summary parent missing")?)
        .map_err(|e| e.to_string())?;
    // The owner's formatter is the privacy boundary for its own report schema.
    let summarize = command::log(
        root,
        "cargo",
        &[
            "run",
            "--locked",
            "--manifest-path",
            "scripts/ci/windows-native-runner/Cargo.toml",
            "--",
            "summarize",
            "--source",
            raw.to_str().ok_or("non-UTF8 report")?,
            "--report",
            output.to_str().ok_or("non-UTF8 summary")?,
        ],
        &directory.join("workspace-summary.log"),
    )?;
    if summarize != 0 {
        write_json(
            &output,
            &json!({"schema":"humanymous.workspace-summary/1","status":"failed","source_sha":before["source_sha"],"raw_report_sha256":source::file(&raw)?,"full_runtime_qualified":false,"reason":"owning summary formatter rejected report"}),
        )?;
    }
    evidence::register(&directory, "Rust workspace component probe")?;
    // Owning formatter is the privacy boundary; publish before failure checks.
    evidence::emit_public_summary("Source-bound workspace probe", &read_json(&output)?)?;
    require(
        code == 0 && summarize == 0,
        "owning workspace execution/summary failed; no component success inferred",
    )?;
    require(
        report["schema"] == "humanymous.workspace-probe/1"
            && report["kind"] == "workspace"
            && report["status"] == "passed"
            && report["full_runtime_qualified"] == false
            && report["source_sha"] == before["source_sha"],
        "workspace probe identity/scope/status mismatch",
    )?;
    require(
        before["source_sha"] == after["source_sha"]
            && before["manifest_sha256"] == after["manifest_sha256"],
        "workspace input source changed during execution",
    )?;
    require(
        report["os"] == os && report["arch"] == std::env::consts::ARCH,
        "owner workspace native tuple mismatch",
    )?;
    println!(
        "workspace component report: {} (not full product qualification)",
        output.display()
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_cross_compiled_label_cannot_qualify_a_native_tuple() {
        assert!(tuple("unknown", "x64").is_err());
        assert!(tuple(std::env::consts::OS, "unknown").is_err());
    }
}
