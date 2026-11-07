//! Closed Cargo diagnostics; rendered text and unreviewed locations stay private.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};

const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_LINE: usize = 256 * 1024;
const MAX_DIAGNOSTICS: usize = 64;

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
        let Ok(value) = serde_json::from_slice::<Value>(line) else {
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
            continue;
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
pub(super) fn read(path: &Path, source: &Value, exit: Option<i32>) -> Value {
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
            json!({"status":"missing_or_unreadable","exit_code":exit,"diagnostics":[],"compiler_root_cause_verified":false,"rendered_messages_included":false})
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
    safe
}
pub(super) fn public(report: &Value) -> Value {
    let stage = report["stage"]
        .as_str()
        .filter(|stage| {
            matches!(
                *stage,
                "source-preflight"
                    | "native-build"
                    | "desktop-protocol-build"
                    | "desktop-protocol-tests"
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
        "desktop_passed_tests":report["desktop_passed_tests"].as_u64(),
        "browser_ready":false,"full_runtime_qualified":false,"raw_contents_included":false})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Value {
        json!({"files":{"crates/browser/native/bootstrap/src/lib.rs":"a"}})
    }
    fn message(file: &str) -> Value {
        json!({"reason":"compiler-message","message":{"level":"error","code":{"code":"E0308"},"rendered":"PRIVATE_CANARY","message":"PRIVATE_CANARY","spans":[{"is_primary":true,"file_name":file,"line_start":9,"text":[{"text":"PRIVATE_CANARY"}]}]}})
    }
    #[test]
    fn compiler_projection_keeps_only_codes_and_reviewed_locations() {
        let bytes = message("crates/browser/native/bootstrap/src/lib.rs").to_string();
        let value = project(bytes.as_bytes(), &source(), Some(101), false);
        assert_eq!(value["diagnostics"][0]["error_code"], "E0308");
        assert_eq!(value["diagnostics"][0]["locations"][0]["line"], 9);
        assert!(!value.to_string().contains("PRIVATE_CANARY"));
        for path in [
            "C:/PRIVATE_CANARY/lib.rs",
            "../PRIVATE_CANARY.rs",
            "crates/PRIVATE_CANARY.rs",
        ] {
            let value = project(
                message(path).to_string().as_bytes(),
                &source(),
                Some(101),
                false,
            );
            assert_eq!(value["diagnostics"][0]["locations"], json!([]));
            assert!(!value.to_string().contains("PRIVATE_CANARY"));
        }
    }
    #[test]
    fn malformed_and_unstructured_logs_cannot_invent_diagnostics() {
        let value = project(
            b"PRIVATE_CANARY\n{broken\n{\"reason\":\"build-script-executed\"}\n",
            &source(),
            Some(101),
            false,
        );
        assert_eq!(value["status"], "unknown");
        assert_eq!(value["malformed_json_records"], 1);
        assert!(!value.to_string().contains("PRIVATE_CANARY"));
    }
    #[test]
    fn long_and_excess_records_are_bounded_with_explicit_omissions() {
        let mut bytes = vec![b'x'; MAX_LINE + 1];
        bytes.push(b'\n');
        for _ in 0..70 {
            bytes.extend(
                message("crates/browser/native/bootstrap/src/lib.rs")
                    .to_string()
                    .bytes(),
            );
            bytes.push(b'\n');
        }
        let value = project(&bytes, &source(), None, true);
        assert_eq!(value["diagnostics"].as_array().unwrap().len(), 64);
        assert_eq!(value["oversized_records"], 1);
        assert_eq!(value["omitted_diagnostics"], 6);
        assert_eq!(value["diagnostic_read_truncated"], true);
    }
    #[test]
    fn arbitrary_codes_and_lints_remain_unknown() {
        assert_eq!(code(&json!("PRIVATE_CANARY")), (None, None));
        assert_eq!(code(&json!("E0000PRIVATE_CANARY")), (None, None));
        assert_eq!(
            code(&json!("unused_imports")),
            (None, Some("unused_imports"))
        );
        assert_eq!(
            read(Path::new("absent-private-compiler-log"), &source(), None)["status"],
            "missing_or_unreadable"
        );
    }
    #[test]
    fn public_stage_and_protocol_fields_cannot_echo_private_text() {
        let report = json!({"stage":"PRIVATE_CANARY","private_error":"PRIVATE_CANARY",
            "source":{"commit":"PRIVATE_CANARY","files":{}},
            "protocol_preflight":{"required":"PRIVATE_CANARY","raw":"PRIVATE_CANARY"},
            "native_build":{"status":"PRIVATE_CANARY","private_log_sha256":"PRIVATE_CANARY","raw":"PRIVATE_CANARY",
            "diagnostics":[{"level":"error","error_code":"PRIVATE_CANARY","lint":"PRIVATE_CANARY","locations":[{"file":"PRIVATE_CANARY","line":1}]}]}});
        let value = public(&report);
        assert_eq!(value["stage"], "unknown");
        assert!(value["source_sha"].is_null());
        assert!(value["protocol_preflight"]["required"].is_null());
        assert!(!value.to_string().contains("PRIVATE_CANARY"));
    }
}
