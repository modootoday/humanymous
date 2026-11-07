//! One Actions run handle, one exact source; terminal evidence is never blended.
use crate::{Result, command, read_json, require, source, write_json};
use serde_json::{Value, json};
use std::path::Path;
pub fn observe(root: &Path, id: &str, sha: &str, directory: &Path) -> Result<()> {
    require(
        id.bytes().all(|b| b.is_ascii_digit()) && !id.is_empty(),
        "numeric run handle required",
    )?;
    require(
        sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit()),
        "full expected source SHA required",
    )?;
    let output = command::text(
        root,
        "gh",
        &[
            "run",
            "view",
            id,
            "--repo",
            "modootoday/humanymous",
            "--json",
            "databaseId,headSha,status,conclusion,jobs,url,createdAt,updatedAt,attempt,startedAt",
        ],
    )?;
    let run: Value = serde_json::from_str(&output).map_err(|e| e.to_string())?;
    require(
        run["databaseId"]
            .as_u64()
            .is_some_and(|v| v.to_string() == id)
            && run["headSha"] == sha,
        "run handle/source mismatch; refusing unrelated success",
    )?;
    let attempt = run["attempt"]
        .as_u64()
        .filter(|v| *v > 0)
        .ok_or("run attempt absent")?;
    let attempt_text = attempt.to_string();
    let directory = directory.join(format!("attempt-{attempt}"));
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let directory_metadata = std::fs::symlink_metadata(&directory).map_err(|e| e.to_string())?;
    require(
        directory_metadata.is_dir() && !directory_metadata.file_type().is_symlink(),
        "attempt directory must not be a symlink",
    )?;
    let marker = directory.join("terminal-capture.json");
    let marker_present = match std::fs::symlink_metadata(&marker) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.to_string()),
    };
    if marker_present {
        retained_file(&marker)?;
        let captured = read_json(&marker)?;
        return validate_capture(&directory, &captured, &run, id, sha, attempt);
    }
    write_json(&directory.join("run.json"), &run)?;
    let jobs:Vec<_>=run["jobs"].as_array().ok_or("run jobs absent")?.iter().map(|job|json!({"id":job["databaseId"],"name":job["name"],"status":job["status"],"conclusion":job["conclusion"],"url":job["url"],"failed_steps":job["steps"].as_array().into_iter().flatten().filter(|step|step["conclusion"]=="failure").map(|step|step["name"].clone()).collect::<Vec<_>>()})).collect();
    let terminal = run["status"] == "completed";
    write_json(
        &directory.join("summary.json"),
        &json!({"schema":"humanymous.actions-observation/2","run_id":id,"attempt":attempt,"source_sha":sha,"status":run["status"],"conclusion":run["conclusion"],"terminal":terminal,"jobs":jobs,"full_product_qualified":false,"source_bound":true}),
    )?;
    println!(
        "run {id} source {sha} status {} conclusion {}",
        run["status"], run["conclusion"]
    );
    if !terminal {
        return Ok(());
    }
    let logs_required = execution_logs_required(&run)?;
    let log_hash = if logs_required {
        require(
            command::log(
                root,
                "gh",
                &[
                    "run",
                    "view",
                    id,
                    "--repo",
                    "modootoday/humanymous",
                    "--attempt",
                    &attempt_text,
                    "--log",
                ],
                &directory.join("run.log"),
            )? == 0,
            "terminal log retrieval failed",
        )?;
        Some(source::file(&directory.join("run.log"))?)
    } else {
        None
    };
    let artifacts = directory.join("artifacts");
    std::fs::create_dir_all(&artifacts).map_err(|e| e.to_string())?;
    let endpoint = format!("repos/modootoday/humanymous/actions/runs/{id}/artifacts?per_page=100");
    let inventory: Value = serde_json::from_str(&command::text(root, "gh", &["api", &endpoint])?)
        .map_err(|e| e.to_string())?;
    let entries = inventory["artifacts"]
        .as_array()
        .ok_or("artifact inventory absent")?;
    require(
        inventory["total_count"].as_u64() == Some(entries.len() as u64),
        "artifact inventory requires pagination; refusing incomplete capture",
    )?;
    require(
        logs_required || entries.is_empty(),
        "zero-job startup failure unexpectedly has artifacts",
    )?;
    write_json(&directory.join("artifact-inventory.json"), &inventory)?;
    let mut captured_artifacts = Vec::new();
    for artifact in entries {
        if !artifact_in_attempt(artifact, &run)? {
            continue;
        }
        let artifact_id = artifact["id"].as_u64().ok_or("artifact ID absent")?;
        let endpoint = format!("repos/modootoday/humanymous/actions/artifacts/{artifact_id}/zip");
        let archive = artifacts.join(format!("{artifact_id}.zip"));
        require(
            command::json_log(root, "gh", &["api", &endpoint], &archive)? == 0,
            "artifact retrieval failed",
        )?;
        captured_artifacts.push(json!({"id":artifact_id,"sha256":source::file(&archive)?,"bytes":std::fs::metadata(&archive).map_err(|e| e.to_string())?.len()}));
    }
    let mut attachments = Vec::new();
    for name in ["run.json", "summary.json", "artifact-inventory.json"] {
        attachments.push(attachment(&directory, name, "decoded-json")?);
    }
    if logs_required {
        attachments.push(attachment(&directory, "run.log", "decoded-cli-text")?);
    }
    for artifact in &captured_artifacts {
        let name = format!("artifacts/{}.zip", artifact["id"]);
        attachments.push(attachment(&directory, &name, "original-artifact-api-body")?);
    }
    let captured = json!({"schema":"humanymous.actions-terminal-custody/1","run_id":id,"attempt":attempt,"source_sha":sha,"log_sha256":log_hash,"execution_log_status":if logs_required {"captured"} else {"not-created-workflow-startup"},"artifacts":captured_artifacts,"attachments":attachments,"conclusion":run["conclusion"]});
    validate_capture(&directory, &captured, &run, id, sha, attempt)?;
    write_json(&marker, &captured)
}

fn retained_file(path: &Path) -> Result<u64> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    require(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "retained attachment must be a regular non-symlink file",
    )?;
    Ok(metadata.len())
}

fn attachment(directory: &Path, name: &str, representation: &str) -> Result<Value> {
    let path = directory.join(name);
    let bytes = retained_file(&path)?;
    Ok(
        json!({"path":name,"representation":representation,"sha256":source::file(&path)?,"bytes":bytes}),
    )
}

fn validate_capture(
    directory: &Path,
    captured: &Value,
    run: &Value,
    id: &str,
    sha: &str,
    attempt: u64,
) -> Result<()> {
    require(
        captured["schema"] == "humanymous.actions-terminal-custody/1",
        "legacy or unknown terminal marker lacks attachment custody; preserve without promotion",
    )?;
    require(
        captured["run_id"] == id
            && captured["source_sha"] == sha
            && captured["attempt"] == attempt
            && captured["conclusion"] == run["conclusion"],
        "terminal marker belongs to another run, source, attempt or conclusion",
    )?;
    let logs_required = execution_logs_required(run)?;
    require(
        captured["execution_log_status"]
            == if logs_required {
                "captured"
            } else {
                "not-created-workflow-startup"
            },
        "terminal log disposition mismatch",
    )?;
    let mut expected = Vec::new();
    for name in ["run.json", "summary.json", "artifact-inventory.json"] {
        expected.push(attachment(directory, name, "decoded-json")?);
    }
    let retained_run = read_json(&directory.join("run.json"))?;
    for key in [
        "databaseId",
        "headSha",
        "attempt",
        "status",
        "conclusion",
        "startedAt",
        "updatedAt",
        "jobs",
    ] {
        require(
            retained_run[key] == run[key],
            format!("retained run identity differs: {key}"),
        )?;
    }
    let summary = read_json(&directory.join("summary.json"))?;
    require(
        summary["schema"] == "humanymous.actions-observation/2"
            && summary["run_id"] == id
            && summary["source_sha"] == sha
            && summary["attempt"] == attempt
            && summary["terminal"] == true
            && summary["status"] == "completed"
            && summary["conclusion"] == run["conclusion"]
            && summary["source_bound"] == true
            && summary["full_product_qualified"] == false,
        "retained summary identity or scope differs",
    )?;
    if logs_required {
        let log = attachment(directory, "run.log", "decoded-cli-text")?;
        require(
            captured["log_sha256"] == log["sha256"],
            "retained decoded log hash differs",
        )?;
        expected.push(log);
    } else {
        require(
            captured["log_sha256"].is_null()
                && !directory
                    .join("run.log")
                    .try_exists()
                    .map_err(|e| e.to_string())?,
            "startup failure cannot claim execution logs",
        )?;
    }
    let inventory = read_json(&directory.join("artifact-inventory.json"))?;
    let entries = inventory["artifacts"]
        .as_array()
        .ok_or("retained artifact inventory absent")?;
    require(
        inventory["total_count"].as_u64() == Some(entries.len() as u64)
            && (logs_required || entries.is_empty()),
        "retained artifact inventory incomplete or invalid for startup",
    )?;
    let mut ids = std::collections::BTreeSet::new();
    let mut artifacts = Vec::new();
    for entry in entries {
        let artifact_id = entry["id"]
            .as_u64()
            .filter(|v| *v > 0)
            .ok_or("positive artifact ID required")?;
        require(ids.insert(artifact_id), "duplicate artifact ID")?;
        if !artifact_in_attempt(entry, &retained_run)? {
            continue;
        }
        let parent =
            std::fs::symlink_metadata(directory.join("artifacts")).map_err(|e| e.to_string())?;
        require(
            parent.is_dir() && !parent.file_type().is_symlink(),
            "artifact directory must not be a symlink",
        )?;
        let name = format!("artifacts/{artifact_id}.zip");
        let record = attachment(directory, &name, "original-artifact-api-body")?;
        artifacts.push(json!({"id":artifact_id,"sha256":record["sha256"],"bytes":record["bytes"]}));
        expected.push(record);
    }
    require(
        captured["artifacts"] == json!(artifacts),
        "retained artifact bodies differ from marker",
    )?;
    require(
        captured["attachments"] == json!(expected),
        "terminal attachment presence, hash, size or representation differs",
    )
}

fn execution_logs_required(run: &Value) -> Result<bool> {
    require(
        run["status"] == "completed",
        "log disposition requires terminal run",
    )?;
    let jobs = run["jobs"].as_array().ok_or("run jobs absent")?;
    if run["conclusion"] == "startup_failure" {
        require(
            jobs.is_empty(),
            "startup failure unexpectedly has execution jobs",
        )?;
        return Ok(false);
    }
    require(
        !jobs.is_empty(),
        "terminal execution jobs missing; logs cannot be waived",
    )?;
    Ok(true)
}

fn artifact_in_attempt(artifact: &Value, run: &Value) -> Result<bool> {
    require(
        artifact["workflow_run"]["id"] == run["databaseId"]
            && artifact["workflow_run"]["head_sha"] == run["headSha"],
        "artifact run/source mismatch",
    )?;
    let timestamp = |value: &Value| -> Result<String> {
        let text = value.as_str().ok_or("timestamp absent")?;
        require(
            text.len() == 20 && text.ends_with('Z') && text.as_bytes()[10] == b'T',
            "canonical UTC timestamp required",
        )?;
        Ok(text.to_owned())
    };
    let created = timestamp(&artifact["created_at"])?;
    Ok(created >= timestamp(&run["startedAt"])? && created <= timestamp(&run["updatedAt"])?)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Capture(std::path::PathBuf);
    impl Drop for Capture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn capture(startup: bool) -> (Capture, Value, Value) {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "humanymous-remote-custody-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::create_dir(directory.join("artifacts")).unwrap();
        let run = json!({"databaseId":7,"headSha":"abc","attempt":1,"status":"completed","conclusion":if startup {"startup_failure"} else {"failure"},"jobs":if startup {json!([])} else {json!([{}])},"startedAt":"2026-11-07T01:00:00Z","updatedAt":"2026-11-07T02:00:00Z"});
        write_json(&directory.join("run.json"), &run).unwrap();
        write_json(&directory.join("summary.json"), &json!({"schema":"humanymous.actions-observation/2","run_id":"7","source_sha":"abc","attempt":1,"terminal":true,"status":"completed","conclusion":run["conclusion"],"source_bound":true,"full_product_qualified":false})).unwrap();
        let entries = if startup {
            json!([])
        } else {
            json!([{"id":9,"workflow_run":{"id":7,"head_sha":"abc"},"created_at":"2026-11-07T01:30:00Z"}])
        };
        write_json(
            &directory.join("artifact-inventory.json"),
            &json!({"total_count":if startup {0} else {1},"artifacts":entries}),
        )
        .unwrap();
        let mut attachments: Vec<_> = ["run.json", "summary.json", "artifact-inventory.json"]
            .iter()
            .map(|name| attachment(&directory, name, "decoded-json").unwrap())
            .collect();
        let mut artifacts = Vec::new();
        let mut log_hash = Value::Null;
        if !startup {
            std::fs::write(directory.join("run.log"), b"decoded CLI output\n").unwrap();
            let log = attachment(&directory, "run.log", "decoded-cli-text").unwrap();
            log_hash = log["sha256"].clone();
            attachments.push(log);
            std::fs::write(directory.join("artifacts/9.zip"), b"original API body").unwrap();
            let archive =
                attachment(&directory, "artifacts/9.zip", "original-artifact-api-body").unwrap();
            artifacts.push(json!({"id":9,"sha256":archive["sha256"],"bytes":archive["bytes"]}));
            attachments.push(archive);
        }
        let marker = json!({"schema":"humanymous.actions-terminal-custody/1","run_id":"7","source_sha":"abc","attempt":1,"conclusion":run["conclusion"],"execution_log_status":if startup {"not-created-workflow-startup"} else {"captured"},"log_sha256":log_hash,"artifacts":artifacts,"attachments":attachments});
        (Capture(directory), run, marker)
    }
    #[test]
    fn terminal_receipt_rechecks_every_retained_attachment() {
        for name in [
            "run.json",
            "summary.json",
            "artifact-inventory.json",
            "run.log",
            "artifacts/9.zip",
        ] {
            let (directory, run, marker) = capture(false);
            assert!(validate_capture(&directory.0, &marker, &run, "7", "abc", 1).is_ok());
            let path = directory.0.join(name);
            let original = std::fs::read(&path).unwrap();
            std::fs::remove_file(&path).unwrap();
            assert!(
                validate_capture(&directory.0, &marker, &run, "7", "abc", 1).is_err(),
                "missing {name}"
            );
            let mut changed = original.clone();
            changed[0] ^= 1;
            std::fs::write(&path, changed).unwrap();
            assert!(
                validate_capture(&directory.0, &marker, &run, "7", "abc", 1).is_err(),
                "same-size tamper {name}"
            );
            std::fs::write(&path, original).unwrap();
            assert!(validate_capture(&directory.0, &marker, &run, "7", "abc", 1).is_ok());
        }
    }
    #[test]
    fn terminal_receipt_refuses_foreign_legacy_and_unmeasured_claims() {
        let (directory, run, marker) = capture(false);
        for (key, value) in [
            ("run_id", json!("8")),
            ("source_sha", json!("foreign")),
            ("attempt", json!(2)),
            ("conclusion", json!("success")),
            ("schema", Value::Null),
            (
                "execution_log_status",
                json!("not-created-workflow-startup"),
            ),
        ] {
            let mut changed = marker.clone();
            changed[key] = value;
            assert!(
                validate_capture(&directory.0, &changed, &run, "7", "abc", 1).is_err(),
                "{key}"
            );
        }
        for (key, value) in [
            ("bytes", json!(0)),
            ("sha256", json!("foreign")),
            ("representation", json!("original-run-log-zip")),
            ("path", json!("../run.json")),
        ] {
            let mut changed = marker.clone();
            changed["attachments"][0][key] = value;
            assert!(
                validate_capture(&directory.0, &changed, &run, "7", "abc", 1).is_err(),
                "{key}"
            );
        }
        let mut changed = marker.clone();
        changed["artifacts"][0]["id"] = json!(10);
        assert!(validate_capture(&directory.0, &changed, &run, "7", "abc", 1).is_err());
        let mut changed_run = run.clone();
        changed_run["jobs"] = json!([]);
        assert!(validate_capture(&directory.0, &marker, &changed_run, "7", "abc", 1).is_err());
    }
    #[test]
    fn zero_job_startup_receipt_requires_metadata_and_no_execution_log() {
        let (directory, run, marker) = capture(true);
        assert!(validate_capture(&directory.0, &marker, &run, "7", "abc", 1).is_ok());
        std::fs::write(directory.0.join("run.log"), b"unexpected").unwrap();
        assert!(validate_capture(&directory.0, &marker, &run, "7", "abc", 1).is_err());
    }
    #[test]
    fn only_zero_job_startup_failure_has_no_execution_log() {
        assert!(
            !execution_logs_required(
                &json!({"status":"completed","conclusion":"startup_failure","jobs":[]})
            )
            .unwrap()
        );
        for conclusion in ["success", "failure", "cancelled", ""] {
            assert!(
                execution_logs_required(
                    &json!({"status":"completed","conclusion":conclusion,"jobs":[]})
                )
                .is_err()
            );
            assert!(
                execution_logs_required(
                    &json!({"status":"completed","conclusion":conclusion,"jobs":[{}]})
                )
                .unwrap()
            );
        }
        assert!(
            execution_logs_required(
                &json!({"status":"in_progress","conclusion":"startup_failure","jobs":[]})
            )
            .is_err()
        );
        assert!(
            execution_logs_required(
                &json!({"status":"completed","conclusion":"startup_failure","jobs":[{}]})
            )
            .is_err()
        );
    }
    #[test]
    fn artifacts_are_bound_to_attempt_window_and_source() {
        let run = json!({"databaseId":7,"headSha":"abc","startedAt":"2026-11-07T01:00:00Z","updatedAt":"2026-11-07T02:00:00Z"});
        let mut artifact =
            json!({"workflow_run":{"id":7,"head_sha":"abc"},"created_at":"2026-11-07T01:30:00Z"});
        assert!(artifact_in_attempt(&artifact, &run).unwrap());
        artifact["created_at"] = json!("2026-11-07T00:30:00Z");
        assert!(!artifact_in_attempt(&artifact, &run).unwrap());
        artifact["workflow_run"]["head_sha"] = json!("other");
        assert!(artifact_in_attempt(&artifact, &run).is_err());
    }
}
