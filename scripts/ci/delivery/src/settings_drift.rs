//! `Settings drift` comparison (common automation rules C2.6–C2.11, v1.2).
//!
//! Compares the live rulesets and labels with `.github/rulesets/*.json` and
//! `.github/labels.json`, using only the keys present in the files. It changes
//! no setting. With `--issue` it opens or updates one `needs-owner` issue when
//! they differ. Applying the files stays an owner task on the owner's computer.
use crate::{Result, command};
use serde_json::Value;
use std::path::Path;

const ISSUE_TITLE: &str = "Settings drift: live repository settings differ from .github files";

/// True when every key present in `file` has an equal value in `live`.
/// Arrays must have the same length and each file element must match a live
/// element.
fn subset(file: &Value, live: &Value) -> bool {
    match (file, live) {
        (Value::Object(want), Value::Object(have)) => want
            .iter()
            .all(|(key, value)| have.get(key).is_some_and(|actual| subset(value, actual))),
        (Value::Array(want), Value::Array(have)) => {
            want.len() == have.len()
                && want
                    .iter()
                    .all(|value| have.iter().any(|actual| subset(value, actual)))
        }
        _ => file == live,
    }
}

/// Compares one label list; returns human-readable differences.
fn label_drift(file: &Value, live: &Value) -> Result<Vec<String>> {
    let mut drift = Vec::new();
    for label in file
        .as_array()
        .ok_or(".github/labels.json must be an array")?
    {
        let name = label["name"].as_str().ok_or("label name absent")?;
        match live
            .as_array()
            .into_iter()
            .flatten()
            .find(|actual| actual["name"].as_str() == Some(name))
        {
            None => drift.push(format!("label {name}: not present")),
            Some(actual) if !subset(label, actual) => {
                drift.push(format!("label {name}: color or description differs"))
            }
            Some(_) => {}
        }
    }
    Ok(drift)
}

fn gh_json(root: &Path, path: &str) -> Result<Value> {
    serde_json::from_str(&command::text(root, "gh", &["api", path])?).map_err(|e| e.to_string())
}

fn repository() -> Result<String> {
    let repo = std::env::var("GITHUB_REPOSITORY").map_err(|_| "GITHUB_REPOSITORY is not set")?;
    if repo.split('/').count() != 2 || repo.starts_with('-') {
        return Err("GITHUB_REPOSITORY must be owner/name".into());
    }
    Ok(repo)
}

fn compare(root: &Path, repo: &str) -> Result<Vec<String>> {
    let mut drift = Vec::new();
    let live_list = gh_json(root, &format!("repos/{repo}/rulesets?per_page=100"))?;
    let mut files: Vec<_> = std::fs::read_dir(root.join(".github/rulesets"))
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    for path in files {
        let file = crate::read_json(&path)?;
        let name = file["name"].as_str().ok_or("ruleset name absent")?;
        let found = live_list
            .as_array()
            .into_iter()
            .flatten()
            .find(|ruleset| ruleset["name"].as_str() == Some(name));
        match found.and_then(|ruleset| ruleset["id"].as_u64()) {
            None => drift.push(format!("ruleset {name}: not applied")),
            Some(id) => {
                let live = gh_json(root, &format!("repos/{repo}/rulesets/{id}"))?;
                if !subset(&file, &live) {
                    drift.push(format!("ruleset {name}: live rules differ from the file"));
                }
            }
        }
    }
    let labels = crate::read_json(&root.join(".github/labels.json"))?;
    let live_labels = gh_json(root, &format!("repos/{repo}/labels?per_page=100"))?;
    drift.extend(label_drift(&labels, &live_labels)?);
    Ok(drift)
}

fn report(root: &Path, repo: &str, drift: &[String]) -> Result<()> {
    let body = format!(
        "The `Settings drift` job found live settings that differ from the files on the protected base branch.\n\n{}\n\nApplying the files is an owner task, run from the owner's computer (common automation rules C2.7). `.github/settings/repository.json` is not compared, because `GITHUB_TOKEN` cannot read those fields.\n\nRun: {}/{}/actions/runs/{}\n",
        drift
            .iter()
            .map(|line| format!("- {line}"))
            .collect::<Vec<_>>()
            .join("\n"),
        std::env::var("GITHUB_SERVER_URL").unwrap_or_else(|_| "https://github.com".into()),
        repo,
        std::env::var("GITHUB_RUN_ID").unwrap_or_default(),
    );
    let file = std::env::temp_dir().join("settings-drift-issue.md");
    std::fs::write(&file, body).map_err(|e| e.to_string())?;
    let file = file.to_string_lossy().into_owned();
    let open = command::text(
        root,
        "gh",
        &[
            "issue",
            "list",
            "--repo",
            repo,
            "--state",
            "open",
            "--search",
            "in:title \"Settings drift\"",
            "--json",
            "number,title",
            "--limit",
            "20",
        ],
    )?;
    let existing = serde_json::from_str::<Value>(&open)
        .map_err(|e| e.to_string())?
        .as_array()
        .into_iter()
        .flatten()
        .find(|issue| issue["title"] == ISSUE_TITLE)
        .and_then(|issue| issue["number"].as_u64());
    if let Some(number) = existing {
        let number = number.to_string();
        command::text(
            root,
            "gh",
            &[
                "issue",
                "edit",
                &number,
                "--repo",
                repo,
                "--body-file",
                &file,
            ],
        )?;
        println!("updated drift issue #{number}");
    } else {
        let labelled = command::output(
            root,
            "gh",
            &[
                "issue",
                "create",
                "--repo",
                repo,
                "--title",
                ISSUE_TITLE,
                "--body-file",
                &file,
                "--label",
                "needs-owner",
            ],
        )?;
        if !labelled.status.success() {
            // The label itself may be part of the drift (labels not applied yet).
            command::text(
                root,
                "gh",
                &[
                    "issue",
                    "create",
                    "--repo",
                    repo,
                    "--title",
                    ISSUE_TITLE,
                    "--body-file",
                    &file,
                ],
            )?;
        }
        println!("opened drift issue");
    }
    Ok(())
}

/// `delivery settings-drift [--issue]`.
pub fn check(root: &Path, issue: bool) -> Result<()> {
    let repo = repository()?;
    let drift = compare(root, &repo)?;
    if drift.is_empty() {
        println!("settings match the files for {repo}");
        return Ok(());
    }
    for line in &drift {
        println!("drift: {line}");
    }
    if issue {
        report(root, &repo, &drift)?;
    }
    Err(format!(
        "{} setting(s) differ from the files; an owner applies them (C2.7)",
        drift.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn subset_ignores_extra_live_keys_and_checks_present_ones() {
        let file = json!({"name":"x","rules":[{"type":"deletion"}]});
        let live = json!({"id":1,"name":"x","rules":[{"type":"deletion","extra":true}]});
        assert!(subset(&file, &live));
        assert!(!subset(
            &file,
            &json!({"name":"x","rules":[{"type":"update"}]})
        ));
        assert!(!subset(
            &file,
            &json!({"name":"x","rules":[{"type":"deletion"},{"type":"update"}]})
        ));
        assert!(!subset(&file, &json!({"name":"x"})));
    }
    #[test]
    fn labels_report_missing_and_changed_entries() {
        let file = json!([{"name":"bug","color":"d73a4a"},{"name":"needs-owner","color":"b60205"}]);
        let live = json!([{"name":"bug","color":"ffffff","id":1}]);
        assert_eq!(
            label_drift(&file, &live).unwrap(),
            [
                "label bug: color or description differs",
                "label needs-owner: not present"
            ]
        );
        assert!(label_drift(&file, &file).unwrap().is_empty());
    }
    #[test]
    fn committed_settings_files_parse() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        for name in ["main-protected.json", "v-tags.json"] {
            let file = crate::read_json(&root.join(".github/rulesets").join(name)).unwrap();
            assert!(file["name"].is_string() && file["rules"].is_array());
        }
        assert!(
            crate::read_json(&root.join(".github/labels.json"))
                .unwrap()
                .is_array()
        );
        assert!(
            crate::read_json(&root.join(".github/settings/repository.json"))
                .unwrap()
                .is_object()
        );
    }
}
