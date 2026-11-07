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
    let marker = directory.join("terminal-capture.json");
    if marker.is_file() {
        let captured = read_json(&marker)?;
        require(
            captured["run_id"] == id
                && captured["source_sha"] == sha
                && captured["attempt"] == attempt,
            "terminal marker belongs to another source or attempt",
        )?;
        return Ok(());
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
    write_json(
        &marker,
        &json!({"run_id":id,"attempt":attempt,"source_sha":sha,"log_sha256":log_hash,"execution_log_status":if logs_required {"captured"} else {"not-created-workflow-startup"},"artifacts":captured_artifacts,"conclusion":run["conclusion"]}),
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
