//! Read-only component receipt validation. Supplied process/source observations
//! remain data, never native admission, an original-clock proof or full R1/Ready.
use super::{catalog, receipt};
use crate::harness;
use humanymous_platform::testing::json::decode_strict_json;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
#[cfg(test)]
mod tests;
const REPORT_CAP: usize = 16 * 1024 * 1024;
const LOG_CAP: usize = 1024 * 1024;
struct Expected {
    sha: String,
    run: u64,
    case: String,
    exit: i32,
}
struct Logs {
    list: Vec<u8>,
    list_error: Vec<u8>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
fn refuse() -> io::Error {
    io::Error::other("service component evidence incomplete")
}
fn hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn run_id(value: &Value) -> Option<u64> {
    let text = value.as_str()?;
    if text.is_empty() || text.starts_with('0') || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}
fn bounded(path: &Path, cap: usize) -> io::Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > cap as u64 {
        return Err(refuse());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > cap {
        return Err(refuse());
    }
    Ok(bytes)
}
pub(crate) fn run(args: impl Iterator<Item = String>) -> io::Result<i32> {
    let mut args = args;
    let mut fields = BTreeMap::new();
    while let Some(key) = args.next() {
        if ![
            "--source",
            "--report",
            "--actor-case",
            "--expected-source-sha",
            "--expected-run-id",
            "--observed-exit-code",
        ]
        .contains(&key.as_str())
            || fields.contains_key(&key)
        {
            return Err(refuse());
        }
        fields.insert(key, args.next().ok_or_else(refuse)?);
    }
    let get = |name: &str| fields.get(name).ok_or_else(refuse);
    let input = PathBuf::from(get("--source")?);
    let output = PathBuf::from(get("--report")?);
    let expected = Expected {
        sha: get("--expected-source-sha")?.clone(),
        run: get("--expected-run-id")?.parse().map_err(|_| refuse())?,
        case: get("--actor-case")?.clone(),
        exit: get("--observed-exit-code")?.parse().map_err(|_| refuse())?,
    };
    if fields.len() != 6
        || !input.is_absolute()
        || !output.is_absolute()
        || input == output
        || expected.run == 0
        || expected.run.to_string() != *get("--expected-run-id")?
        || expected.exit.to_string() != *get("--observed-exit-code")?
        || !(hex(&expected.sha, 40) || hex(&expected.sha, 64))
    {
        return Err(refuse());
    }
    let raw = bounded(&input, REPORT_CAP)?;
    let report = decode_strict_json(&raw, REPORT_CAP).map_err(|_| refuse())?;
    let logs = if expected.case == "normal" {
        Some(
            catalog::CASES
                .iter()
                .map(|case| {
                    Ok(Logs {
                        list: bounded(
                            &input.with_extension(format!("{}-list-stdout.log", case.log_prefix)),
                            LOG_CAP,
                        )?,
                        list_error: bounded(
                            &input.with_extension(format!("{}-list-stderr.log", case.log_prefix)),
                            LOG_CAP,
                        )?,
                        stdout: bounded(
                            &input.with_extension(format!("{}-stdout.log", case.log_prefix)),
                            LOG_CAP,
                        )?,
                        stderr: bounded(
                            &input.with_extension(format!("{}-stderr.log", case.log_prefix)),
                            LOG_CAP,
                        )?,
                    })
                })
                .collect::<io::Result<Vec<_>>>()?,
        )
    } else {
        None
    };
    let mut summary = verify(&report, &expected, logs.as_deref())?;
    summary["raw_report_sha256"] = json!(digest(&raw));
    let mut bytes = serde_json::to_vec_pretty(&summary)?;
    bytes.push(b'\n');
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(0)
}
fn verify(report: &Value, expected: &Expected, logs: Option<&[Logs]>) -> io::Result<Value> {
    let source = &report["source"];
    let life = &report["standard_user_lifecycle"];
    let scm = &report["scm_delegation"];
    if report["schema"] != 1
        || report["kind"] != "native"
        || report["qualification"] != "component-only"
        || report["full_runtime_qualified"] != false
        || report["source_sha"] != expected.sha
        || source["source_sha"] != expected.sha
        || source["dirty"] != false
        || run_id(&source["github"]["GITHUB_RUN_ID"]) != Some(expected.run)
        || source["github"]["GITHUB_SHA"] != expected.sha
        || !source["files"].as_object().is_some_and(|files| {
            !files.is_empty()
                && files
                    .values()
                    .all(|v| v.as_str().is_some_and(|s| hex(s, 64)))
        })
        || report["skips"] != 0
        || life["actor_case"] != expected.case
        || life["account_deleted"] != true
        || life["scratch_deleted"] != true
        || life["scm_delegation_requested"] != true
        || life["provisioning"] != "native-Win32"
        || scm["rights"] != 3
        || scm["single_generated_sid"] != true
        || scm["restored"] != true
        || !scm["original_descriptor_sha256"]
            .as_str()
            .is_some_and(|s| hex(s, 64))
    {
        return Err(refuse());
    }
    let mut rows = Vec::new();
    match expected.case.as_str() {
        "normal" => {
            if expected.exit != 2
                || report["status"] != "blocked"
                || report["errors"] != 0
                || report.get("lifecycle_errors").is_some()
                || life["child_launch_attempted"] != true
                || life["child_exit_code"] != 2
                || life["job_empty_observed"] != true
                || life["job_kill_on_close"] != true
                || report["actor"]["elevated"] != false
                || report["actor"]["enabled_administrator"] != false
                || !life["sid"].as_str().is_some_and(|s| !s.is_empty())
                || report["actor"]["sid"] != life["sid"]
            {
                return Err(refuse());
            }
            let passed = report["cases"].as_array().ok_or_else(refuse)?;
            let logs = logs
                .filter(|logs| logs.len() == catalog::CASES.len())
                .ok_or_else(refuse)?;
            if passed.len() != catalog::CASES.len() {
                return Err(refuse());
            }
            for ((case, logs), passed) in catalog::CASES.iter().zip(logs).zip(passed) {
                if passed["id"] != case.id || passed["status"] != "passed" {
                    return Err(refuse());
                }
                rows.push(verify_case(&report[case.field], case, logs, expected)?);
            }
        }
        "after-grant" => {
            if expected.exit != 1
                || report["status"] != "failed"
                || report["errors"] != 1
                || life["child_launch_attempted"] != false
                || life.get("child_exit_code") != Some(&Value::Null)
                || life.get("job_empty_observed") != Some(&Value::Null)
                || life.get("job_kill_on_close") != Some(&Value::Null)
                || report["lifecycle_errors"]
                    != json!(["child execution: injected failure after SCM grant"])
                || report["cases"] != json!([])
                || catalog::CASES.iter().any(|c| report.get(c.field).is_some())
                || logs.is_some()
            {
                return Err(refuse());
            }
        }
        _ => return Err(refuse()),
    }
    Ok(
        json!({"schema":"humanymous.service-component-summary/1","verified":true,"source_sha":expected.sha,"run_id":expected.run,"actor_case":expected.case,"observed_exit_code":expected.exit,"qualification":"component-only","full_runtime_qualified":false,"product_qualified":false,"native_producer_qualified":false,"cases":rows,"cleanup_verified":true,"source_manifest_sha256":digest(&serde_json::to_vec(&source["files"])?),"selected_execution_identity":"catalog/inventory and caller invocation data; not independently established from nocapture text","scope":"validated supplied source-bound component records; not native admission or original-clock qualification"}),
    )
}
fn verify_case(
    row: &Value,
    case: &catalog::Case,
    logs: &Logs,
    expected: &Expected,
) -> io::Result<Value> {
    let list = std::str::from_utf8(&logs.list).map_err(|_| refuse())?;
    let names: Vec<_> = list
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .collect();
    let stdout = std::str::from_utf8(&logs.stdout).map_err(|_| refuse())?;
    let totals = harness::successful_totals(stdout).map_err(|_| refuse())?;
    if !logs.stderr.is_empty()
        || !logs.list_error.is_empty()
        || names.iter().filter(|name| **name == case.test).count() != 1
        || totals[0..4] != [1, 0, 0, 0]
        || totals[4] != names.len().checked_sub(1).ok_or_else(refuse)?
        || row["test"] != case.test
        || row["validated"] != true
        || row["exit_code"] != 0
        || row["passed"] != 1
        || row["failed"] != 0
        || row["ignored"] != 0
        || row["measured"] != 0
        || row["discovered"].as_u64() != Some(names.len() as u64)
        || row["nonselected_tests"].as_u64() != Some(totals[4] as u64)
        || row["stdout_sha256"] != digest(&logs.stdout)
    {
        return Err(refuse());
    }
    let mut output = json!({"id":case.id,"test":case.test,"passed":1,"failed":0,"ignored":0,"measured":0,"filtered":totals[4],"stdout_sha256":digest(&logs.stdout)});
    if case.windows_only {
        let validated = receipt::validate(stdout, &expected.sha, expected.run)?;
        if row["runtime_receipt"] != validated {
            return Err(refuse());
        }
        output["runtime_receipt"] = validated;
    }
    Ok(output)
}
