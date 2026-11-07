use super::{catalog::Case, receipt};
use crate::{evidence, harness};
use humanymous_test_support::process::capture_with_stderr;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io, path::Path, process::Command, time::Duration};

fn metadata(source: &Value) -> io::Result<(&str, u64)> {
    let sha = source["source_sha"]
        .as_str()
        .filter(|sha| {
            matches!(sha.len(), 40 | 64)
                && sha
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
        .ok_or_else(|| io::Error::other("runtime pilot source SHA absent"))?;
    let run = source["github"]["GITHUB_RUN_ID"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|id| *id > 0)
        .ok_or_else(|| io::Error::other("runtime pilot run id absent"))?;
    Ok((sha, run))
}

pub(super) fn run(entry: &Value, case: &Case, base: &Path, source: &Value) -> io::Result<Value> {
    let executable = Path::new(
        entry["executable"]
            .as_str()
            .ok_or_else(|| io::Error::other("service executable absent"))?,
    );
    if evidence::digest(executable)? != entry["sha256"] {
        return Err(io::Error::other("service executable digest mismatch"));
    }
    let list = capture_with_stderr(
        Command::new(executable).args(["--list", "--format", "terse"]),
        b"",
        Duration::from_secs(30),
        1024 * 1024,
    )?;
    fs::write(
        base.with_extension(format!("{}-list-stdout.log", case.log_prefix)),
        &list.stdout,
    )?;
    fs::write(
        base.with_extension(format!("{}-list-stderr.log", case.log_prefix)),
        &list.stderr,
    )?;
    let inventory = std::str::from_utf8(&list.stdout).map_err(io::Error::other)?;
    let names: Vec<_> = inventory
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .collect();
    if !list.status.success()
        || !list.stderr.is_empty()
        || names.iter().filter(|name| **name == case.test).count() != 1
    {
        return Err(io::Error::other(
            "required exact service test inventory absent",
        ));
    }
    let mut command = Command::new(executable);
    command
        .args([
            "--ignored",
            "--exact",
            case.test,
            "--nocapture",
            "--test-threads",
            "1",
        ])
        .env("HUMANYMOUS_DISPOSABLE_SERVICE_ACTOR", "1");
    let context = if case.windows_only {
        Some(metadata(source)?)
    } else {
        None
    };
    if let Some((sha, run)) = context {
        command
            .env("HUMANYMOUS_SERVICE_PILOT_SOURCE_SHA", sha)
            .env("HUMANYMOUS_SERVICE_PILOT_RUN_ID", run.to_string());
    }
    let output = capture_with_stderr(
        &mut command,
        b"",
        Duration::from_secs(if case.windows_only { 300 } else { 180 }),
        1024 * 1024,
    )?;
    fs::write(
        base.with_extension(format!("{}-stdout.log", case.log_prefix)),
        &output.stdout,
    )?;
    fs::write(
        base.with_extension(format!("{}-stderr.log", case.log_prefix)),
        &output.stderr,
    )?;
    let text = std::str::from_utf8(&output.stdout).map_err(io::Error::other)?;
    let totals = harness::successful_totals(text).ok();
    let mut row = json!({"test":case.test,"exit_code":output.status.code(),"discovered":names.len(),"passed":totals.map(|t|t[0]),"failed":totals.map(|t|t[1]),"ignored":totals.map(|t|t[2]),"measured":totals.map(|t|t[3]),"nonselected_tests":totals.map(|t|t[4]),"stdout_sha256":format!("{:x}",Sha256::digest(&output.stdout)),"validated":false});
    let count_ok = totals.is_some_and(|t| t[0..4] == [1, 0, 0, 0] && t[4] == names.len() - 1);
    if output.status.success() && output.stderr.is_empty() && count_ok {
        if let Some((sha, run)) = context {
            match receipt::validate(text, sha, run) {
                Ok(value) => row["runtime_receipt"] = value,
                Err(_) => {
                    row["error_code"] = json!("runtime-receipt-incomplete");
                    return Ok(row);
                }
            }
        }
        row["validated"] = json!(true);
    }
    Ok(row)
}
