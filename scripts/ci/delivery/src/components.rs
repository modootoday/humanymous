//! Hosted component invocations; owner assertions remain the semantic authority.
use crate::{Result, command, evidence, require, source, write_json};
use serde_json::{Value, json};
use std::path::Path;

const CAPTURE_CASES: [&str; 5] = [
    "actual_tls_h1_never_claims_h2",
    "actual_h2_continuation_and_stream_binding",
    "actual_h2_wrong_stream_invalidates_all_capture",
    "actual_invalid_tls_never_creates_an_owner",
    "actual_supervisor_cancellation_revokes_the_connection_epoch",
];
fn capture_counts(text: &str) -> Result<Value> {
    let summaries: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("test result:"))
        .collect();
    require(
        summaries.len() == 1
            && summaries[0].starts_with(
                "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;",
            ),
        "capture exact five-case summary missing",
    )?;
    require(
        CAPTURE_CASES.iter().all(|name| {
            text.lines()
                .filter(|line| *line == format!("test {name} ... ok"))
                .count()
                == 1
        }),
        "capture required case missing or duplicated",
    )?;
    Ok(json!({"required":5,"passed":5,"failed":0,"ignored":0,"filtered":0}))
}
fn invoke(
    root: &Path,
    dir: &Path,
    report: &mut Value,
    stage: &str,
    program: &str,
    args: &[&str],
    budget: &str,
) -> Result<()> {
    report["stage"] = json!(stage);
    let mut fixed = vec!["--signal=TERM", "--kill-after=15s", budget, program];
    fixed.extend_from_slice(args);
    let log = dir.join(format!("{stage}.log"));
    let code = command::json_log(root, "timeout", &fixed, &log)?;
    report["commands"][stage] = json!({"exit_code":code,"stdout_sha256":source::file(&log)?,"stderr_sha256":source::file(&log.with_extension("stderr.log"))?});
    require(code == 0, "owned component command failed")
}
fn capture(root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
    invoke(
        root,
        dir,
        report,
        "capture-tests",
        "cargo",
        &[
            "test",
            "--locked",
            "-p",
            "humanymous-capture",
            "--features",
            "hosted-pilot",
            "--test",
            "hosted_transport",
            "--",
            "--ignored",
            "--test-threads=1",
        ],
        "180s",
    )?;
    report["test_counts"] = capture_counts(
        &std::fs::read_to_string(dir.join("capture-tests.log")).map_err(|e| e.to_string())?,
    )?;
    Ok(())
}
fn header(root: &Path, dir: &Path, report: &mut Value) -> Result<()> {
    let mut inputs = serde_json::Map::new();
    for path in [
        "adapters/native/cbindgen.toml",
        "crates/native-abi/export/examples/header_conformance.rs",
        "crates/native-abi/src/lib.rs",
        "crates/native-abi/export/src/lib.rs",
        "crates/native-abi/export/Cargo.toml",
    ] {
        inputs.insert(path.into(), json!(source::file(&root.join(path))?));
    }
    report["header_inputs_sha256"] = json!(inputs);
    let stamp = dir
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.get(..14))
        .ok_or("component evidence identifier absent")?;
    require(
        stamp.bytes().all(|c| c.is_ascii_digit()),
        "component evidence identifier invalid",
    )?;
    let tool_base = root
        .join("target/private-ci-tools")
        .join(format!("cbindgen-{stamp}"));
    std::fs::create_dir_all(tool_base.parent().ok_or("tool parent absent")?)
        .map_err(|e| e.to_string())?;
    std::fs::create_dir(&tool_base).map_err(|e| e.to_string())?;
    let install = tool_base.join("install");
    let target = tool_base.join("target");
    invoke(
        root,
        dir,
        report,
        "cbindgen-install",
        "cargo",
        &[
            "install",
            "--locked",
            "--version",
            "0.29.4",
            "cbindgen",
            "--root",
            install.to_str().ok_or("tool path invalid")?,
            "--target-dir",
            target.to_str().ok_or("tool target invalid")?,
        ],
        "900s",
    )?;
    let tool = install.join("bin/cbindgen");
    let tool_text = tool.to_str().ok_or("tool path invalid")?;
    invoke(
        root,
        dir,
        report,
        "cbindgen-version",
        tool_text,
        &["--version"],
        "15s",
    )?;
    require(
        std::fs::read_to_string(dir.join("cbindgen-version.log"))
            .map_err(|e| e.to_string())?
            .trim()
            == "cbindgen 0.29.4",
        "unexpected header generator version",
    )?;
    let generated = dir.join("native-abi-v1.h");
    invoke(
        root,
        dir,
        report,
        "header-generation",
        tool_text,
        &[
            "--config",
            "adapters/native/cbindgen.toml",
            "--crate",
            "humanymous-native-abi-export",
            "--output",
            generated.to_str().ok_or("header path invalid")?,
        ],
        "120s",
    )?;
    let compiler = dir.join("header-compiler");
    invoke(
        root,
        dir,
        report,
        "header-conformance",
        "cargo",
        &[
            "run",
            "--locked",
            "-p",
            "humanymous-native-abi-export",
            "--example",
            "header_conformance",
            "--",
            generated.to_str().ok_or("header path invalid")?,
            compiler.to_str().ok_or("compiler evidence path invalid")?,
        ],
        "180s",
    )?;
    let mut hashes = serde_json::Map::new();
    for label in ["c", "cpp"] {
        for suffix in [
            "version",
            "version-stderr",
            "target",
            "target-stderr",
            "stdout",
            "stderr",
        ] {
            let name = format!("{label}-{suffix}.txt");
            let metadata =
                std::fs::symlink_metadata(compiler.join(&name)).map_err(|e| e.to_string())?;
            require(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "compiler evidence must be an owned regular file",
            )?;
            hashes.insert(name.clone(), json!(source::file(&compiler.join(name))?));
        }
    }
    hashes.insert(
        "scope.txt".into(),
        json!(source::file(&compiler.join("scope.txt"))?),
    );
    let scope = std::fs::symlink_metadata(compiler.join("scope.txt")).map_err(|e| e.to_string())?;
    require(
        scope.is_file() && !scope.file_type().is_symlink(),
        "compiler scope must be an owned regular file",
    )?;
    require(std::fs::read(compiler.join("scope.txt")).map_err(|e|e.to_string())?==b"C11 and C++11 header syntax and record layouts match Rust. Actual dynamic loading, RAII, exceptions, allocator host glue and product e2e: not_run.\n","header owner scope receipt mismatch")?;
    require(
        std::fs::read_to_string(dir.join("header-conformance.log"))
            .map_err(|e| e.to_string())?
            .trim()
            == "C/C++ generated header layout conformance passed; actual dynamic ABI not_run",
        "header owner success marker mismatch",
    )?;
    require(
        std::fs::read_dir(&compiler)
            .map_err(|e| e.to_string())?
            .count()
            == 13,
        "unexpected header compiler evidence inventory",
    )?;
    report["header"] = json!({"generator_version":"0.29.4","generator_sha256":source::file(&tool)?,"header_sha256":source::file(&generated)?,"compiler_evidence_sha256":hashes,"compiler_profiles":["C11","C++11"],"layout_verifier":"owning header_conformance example","dynamic_abi_qualified":false});
    Ok(())
}
pub fn run(root: &Path, kind: &str) -> Result<()> {
    require(
        matches!(kind, "capture" | "header"),
        "unknown hosted component",
    )?;
    require(
        cfg!(all(target_os = "linux", target_arch = "x86_64"))
            && std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
            && std::env::var("RUNNER_ENVIRONMENT").as_deref() == Ok("github-hosted"),
        "component requires standard hosted Linux x64",
    )?;
    let before = source::snapshot(root)?;
    require(
        before["dirty"] == false
            && before["source_sha"] == std::env::var("GITHUB_SHA").unwrap_or_default(),
        "component requires clean exact workflow source",
    )?;
    let dir = evidence::bundle(root)?;
    write_json(&dir.join("source-before.json"), &before)?;
    let mut report = json!({"schema":"humanymous.hosted-component/1","component":kind,"source_sha":before["source_sha"],"source_manifest_sha256":before["manifest_sha256"],"cargo_lock_sha256":source::file(&root.join("Cargo.lock"))?,"toolchain_sha256":source::file(&root.join("rust-toolchain.toml"))?,"runner_os":std::env::consts::OS,"runner_arch":std::env::consts::ARCH,"runner_image":std::env::var("ImageVersion").ok(),"status":"failed","stage":"preflight","commands":{},"test_counts":null,"full_product_qualified":false});
    let result = match kind {
        "capture" => capture(root, &dir, &mut report),
        _ => header(root, &dir, &mut report),
    };
    let after = source::snapshot(root)?;
    write_json(&dir.join("source-after.json"), &after)?;
    let changed = before["source_sha"] != after["source_sha"]
        || before["manifest_sha256"] != after["manifest_sha256"];
    report["source_changed"] = json!(changed);
    if result.is_ok() && !changed {
        report["status"] = json!("passed");
    }
    write_json(&dir.join("component.json"), &report)?;
    evidence::register(&dir, "Source-bound hosted component")?;
    evidence::emit_public_summary("Source-bound hosted component", &report)?;
    result?;
    require(!changed, "component source changed during execution")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_requires_all_five_actual_tests_without_ignore_or_extra_summary() {
        let cases = CAPTURE_CASES
            .iter()
            .map(|name| format!("test {name} ... ok\n"))
            .collect::<String>();
        let log = format!(
            "{cases}test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;"
        );
        assert!(capture_counts(&log).is_ok());
        assert!(capture_counts(&log.replace("5 passed", "0 passed")).is_err());
        assert!(capture_counts(&log.replace("0 ignored", "5 ignored")).is_err());
        assert!(capture_counts(&format!("{log}\n{log}")).is_err());
        assert!(capture_counts(&log.replace(CAPTURE_CASES[0], "unknown")).is_err());
    }
}
