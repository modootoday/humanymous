//! Closed Cargo diagnostics; rendered text and unreviewed locations stay private.
use humanymous_test_support::json::decode_strict_json;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};
mod bootstrap;
use bootstrap::outcome as bootstrap_outcome;
pub(super) fn bootstrap_selector() -> std::io::Result<String> {
    bootstrap::selector()
}
pub(super) fn verify_bootstrap_protocol(text: &str) -> std::io::Result<Value> {
    bootstrap::verify_protocol(text)
}

const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_LINE: usize = 256 * 1024;
const MAX_DIAGNOSTICS: usize = 64;

pub(super) fn text(path: &Path) -> std::io::Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(std::io::Error::other("regular private log required"));
    }
    let mut text = String::new();
    fs::File::open(path)?
        .take((MAX_BYTES + 1) as u64)
        .read_to_string(&mut text)?;
    if text.len() > MAX_BYTES {
        return Err(std::io::Error::other(
            "private test receipt exceeds read bound",
        ));
    }
    Ok(text)
}

fn location(file: &str, source: &Value) -> Option<String> {
    let file = file.replace('\\', "/");
    if file.len() > 256
        || !file.ends_with(".rs")
        || !file.starts_with("crates/")
        || !file
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'/' | b'_' | b'-' | b'.'))
        || file
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
        || source["files"].get(&file).is_none()
    {
        return None;
    }
    Some(file)
}
fn code(value: &Value) -> (Option<&str>, Option<&str>) {
    let Some(code) = value.as_str() else {
        return (None, None);
    };
    if code.len() == 5
        && code.starts_with('E')
        && code.as_bytes()[1..].iter().all(u8::is_ascii_digit)
    {
        (Some(code), None)
    } else if matches!(
        code,
        "unused_imports"
            | "dead_code"
            | "unused_variables"
            | "unused_mut"
            | "deprecated"
            | "unsafe_op_in_unsafe_fn"
            | "unexpected_cfgs"
            | "warnings"
    ) {
        (None, Some(code))
    } else {
        (None, None)
    }
}
fn project(bytes: &[u8], source: &Value, exit: Option<i32>, truncated: bool) -> Value {
    let mut diagnostics = Vec::new();
    let mut malformed = 0;
    let mut oversized = 0;
    let mut omitted = 0;
    for line in bytes.split(|byte| *byte == b'\n') {
        if line.len() > MAX_LINE {
            oversized += 1;
            continue;
        }
        let Ok(value) = decode_strict_json(line, MAX_LINE) else {
            if line.first() == Some(&b'{') {
                malformed += 1;
            }
            continue;
        };
        if value["reason"] != "compiler-message" {
            continue;
        }
        let message = &value["message"];
        let Some(level @ ("error" | "warning")) = message["level"].as_str() else {
            continue;
        };
        if diagnostics.len() == MAX_DIAGNOSTICS {
            omitted += 1;
            if level == "error" {
                if let Some(index) = diagnostics
                    .iter()
                    .position(|row: &Value| row["level"] == "warning")
                {
                    diagnostics.remove(index);
                } else {
                    continue;
                }
            } else {
                continue;
            }
        }
        let (error_code, lint) = code(&message["code"]["code"]);
        let locations: Vec<_> = message["spans"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|span| span["is_primary"] == true)
            .filter_map(|span| {
                let file = location(span["file_name"].as_str()?, source)?;
                let line = span["line_start"]
                    .as_u64()
                    .filter(|line| (1..=1_000_000).contains(line))?;
                Some(json!({"file":file,"line":line}))
            })
            .take(4)
            .collect();
        diagnostics
            .push(json!({"level":level,"error_code":error_code,"lint":lint,"locations":locations}));
    }
    json!({"status":if diagnostics.is_empty(){"unknown"}else{"observed"},"exit_code":exit,
        "diagnostics":diagnostics,"malformed_json_records":malformed,"oversized_records":oversized,
        "omitted_diagnostics":omitted,"diagnostic_read_truncated":truncated,
        "compiler_root_cause_verified":false,"rendered_messages_included":false})
}
pub(super) fn read(path: &Path, source: &Value, exit: Option<i32>, bootstrap: bool) -> Value {
    let bytes = (|| -> std::io::Result<Vec<u8>> {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(std::io::Error::other("regular private log required"));
        }
        let mut bytes = Vec::new();
        fs::File::open(path)?
            .take((MAX_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    })();
    match bytes {
        Ok(mut bytes) => {
            let truncated = bytes.len() > MAX_BYTES;
            bytes.truncate(MAX_BYTES);
            let observed_bytes = bytes.len();
            let digest = format!("{:x}", Sha256::digest(&bytes));
            if truncated {
                if let Some(last) = bytes.iter().rposition(|byte| *byte == b'\n') {
                    bytes.truncate(last);
                } else {
                    bytes.clear();
                }
            }
            let mut result = project(&bytes, source, exit, truncated);
            if bootstrap {
                result["test_outcome"] = bootstrap_outcome(&bytes, exit, truncated);
                result["bootstrap_observations"] = self::bootstrap::observations(&bytes, truncated);
            }
            result["private_log_prefix_sha256"] = json!(digest);
            result["private_log_prefix_bytes"] = json!(observed_bytes);
            result["private_log_sha256"] = if truncated {
                Value::Null
            } else {
                json!(digest)
            };
            result
        }
        Err(_) => {
            let mut result = json!({"status":"missing_or_unreadable","exit_code":exit,"diagnostics":[],"compiler_root_cause_verified":false,"rendered_messages_included":false});
            if bootstrap {
                result["test_outcome"] = bootstrap_outcome(&[], exit, false);
                result["bootstrap_observations"] = self::bootstrap::observations(&[], false);
            }
            result
        }
    }
}
fn native_public(value: &Value, source: &Value) -> Value {
    let status = value["status"]
        .as_str()
        .filter(|status| matches!(*status, "observed" | "unknown" | "missing_or_unreadable"))
        .unwrap_or("unknown");
    let diagnostics: Vec<_> = value["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .take(MAX_DIAGNOSTICS)
        .filter_map(|row| {
            let level = row["level"]
                .as_str()
                .filter(|level| matches!(*level, "error" | "warning"))?;
            let (error_code, _) = code(&row["error_code"]);
            let (_, lint) = code(&row["lint"]);
            let locations: Vec<_> = row["locations"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|item| {
                    let file = location(item["file"].as_str()?, source)?;
                    let line = item["line"]
                        .as_u64()
                        .filter(|line| (1..=1_000_000).contains(line))?;
                    Some(json!({"file":file,"line":line}))
                })
                .take(4)
                .collect();
            Some(json!({"level":level,"error_code":error_code,"lint":lint,"locations":locations}))
        })
        .collect();
    let mut safe = json!({"status":status,"exit_code":value["exit_code"].as_i64().and_then(|code| i32::try_from(code).ok()),
        "status_observed":value["status_observed"].as_bool(),
        "diagnostics":diagnostics,"compiler_root_cause_verified":false,"rendered_messages_included":false,
        "diagnostic_read_truncated":value["diagnostic_read_truncated"].as_bool()});
    for key in [
        "malformed_json_records",
        "oversized_records",
        "omitted_diagnostics",
        "private_log_prefix_bytes",
    ] {
        safe[key] = json!(value[key].as_u64());
    }
    for key in ["private_log_prefix_sha256", "private_log_sha256"] {
        safe[key] =
            json!(value[key].as_str().filter(
                |hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
            ));
    }
    if let Some(outcome) = value.get("test_outcome") {
        let status = outcome["required_case_status"]
            .as_str()
            .filter(|status| matches!(*status, "passed" | "failed" | "unverified"))
            .unwrap_or("unverified");
        let mut projected = json!({"required_case_status":status});
        for key in ["executed_tests", "passed", "failed", "ignored", "filtered"] {
            projected[key] = json!(outcome[key].as_u64().filter(|count| *count <= 1_000_000));
        }
        safe["test_outcome"] = projected;
    }
    if let Some(observations) = value.get("bootstrap_observations") {
        safe["bootstrap_observations"] = bootstrap::public_observations(observations);
    }
    safe
}
pub(super) fn public(report: &Value) -> Value {
    let mut commands = serde_json::Map::new();
    for stage in [
        "native-build",
        "desktop-protocol-build",
        "desktop-protocol-tests",
        "bootstrap-protocol-build",
        "bootstrap-protocol-tests",
        "bootstrap-rejection",
        "desktop-process-tests",
    ] {
        if let Some(value) = report["stage_commands"].get(stage) {
            commands.insert(stage.into(), native_public(value, &report["source"]));
        }
    }
    let stage = report["stage"]
        .as_str()
        .filter(|stage| {
            matches!(
                *stage,
                "source-preflight"
                    | "native-build"
                    | "desktop-protocol-build"
                    | "desktop-protocol-tests"
                    | "bootstrap-protocol-build"
                    | "bootstrap-protocol-tests"
                    | "bootstrap-rejection"
                    | "desktop-bundles"
                    | "desktop-process-tests"
                    | "complete"
            )
        })
        .unwrap_or("unknown");
    let sha = report["source"]["commit"]
        .as_str()
        .filter(|sha| sha.len() == 40 && sha.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let manifest = report["source"]["files"]
        .as_object()
        .and_then(|files| serde_json::to_vec(files).ok())
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)));
    json!({"schema":"humanymous.cef-safe-diagnostics/1","stage":stage,"source_sha":sha,
        "source_manifest_sha256":manifest,"native_build":report.get("native_build").map(|value| native_public(value, &report["source"])),
        "source_changed":report["source_changed"].as_bool(),
        "stage_commands":commands,
        "status":if report["status"]=="passed"{"passed"}else{"failed"},
        "protocol_preflight":if report["protocol_preflight"].is_object(){Some(json!({
            "required":report["protocol_preflight"]["required"].as_u64(),
            "passed":report["protocol_preflight"]["passed"].as_u64(),
            "failed":report["protocol_preflight"]["failed"].as_u64(),
            "ignored":report["protocol_preflight"]["ignored"].as_u64(),
            "filtered":report["protocol_preflight"]["filtered"].as_u64(),
            "initializes_cef":false,"starts_processes":false
        }))}else{None},
        "bootstrap_rejection_passed":report["bootstrap_rejection_passed"]==true,
        "bootstrap_protocol_preflight":if report["bootstrap_protocol_preflight"].is_object(){Some(json!({
            "required":report["bootstrap_protocol_preflight"]["required"].as_u64(),
            "passed":report["bootstrap_protocol_preflight"]["passed"].as_u64(),
            "failed":report["bootstrap_protocol_preflight"]["failed"].as_u64(),
            "ignored":report["bootstrap_protocol_preflight"]["ignored"].as_u64(),
            "filtered":report["bootstrap_protocol_preflight"]["filtered"].as_u64(),
            "initializes_cef":false,"starts_processes":false
        }))}else{None},
        "desktop_passed_tests":report["desktop_passed_tests"].as_u64(),
        "browser_ready":false,"full_runtime_qualified":false,"raw_contents_included":false})
}

#[cfg(test)]
mod tests;
