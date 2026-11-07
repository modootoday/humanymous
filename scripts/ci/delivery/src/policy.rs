//! Workflow structure, pinned entrypoints and the aggregate authorization boundary.
use crate::{Result, command, require, source};
use serde_json::{Map, Value, json};
use std::{collections::BTreeSet, path::Path};
use yaml_rust2::{Yaml, YamlLoader};
const REQUIRED: &[&str] = &[
    "policy",
    "native",
    "msrv",
    "quality",
    "browser",
    "docker",
    "images",
    "packages",
    "node",
    "advisories",
    "documentation",
    "pages",
];

fn convert(value: &Yaml) -> Result<Value> {
    Ok(match value {
        Yaml::String(text) => json!(text),
        Yaml::Integer(number) => json!(number),
        Yaml::Boolean(boolean) => json!(boolean),
        Yaml::Null => Value::Null,
        Yaml::Real(number) => serde_json::from_str(number).map_err(|e| e.to_string())?,
        Yaml::Array(items) => Value::Array(items.iter().map(convert).collect::<Result<_>>()?),
        Yaml::Hash(items) => {
            let mut result = Map::new();
            for (key, value) in items {
                result.insert(
                    key.as_str().ok_or("non-string YAML key")?.into(),
                    convert(value)?,
                );
            }
            Value::Object(result)
        }
        _ => return Err("unsupported YAML node".into()),
    })
}
pub fn yaml(text: &str) -> Result<Value> {
    let documents = YamlLoader::load_from_str(text).map_err(|e| e.to_string())?;
    require(documents.len() == 1, "one YAML document required")?;
    convert(&documents[0])
}
fn workflows(root: &Path) -> Result<Vec<(String, Value)>> {
    let mut result = Vec::new();
    for entry in std::fs::read_dir(root.join(".github/workflows")).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if matches!(
            path.extension().and_then(|v| v.to_str()),
            Some("yml" | "yaml")
        ) {
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            result.push((
                path.file_name()
                    .ok_or("missing workflow name")?
                    .to_string_lossy()
                    .into_owned(),
                yaml(&text)?,
            ));
        }
    }
    result.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(result)
}
fn strings(value: &Value) -> BTreeSet<String> {
    if let Some(text) = value.as_str() {
        [text.into()].into()
    } else {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect()
    }
}
fn pinned(action: &str) -> bool {
    if action.starts_with("./") {
        return true;
    }
    action.rsplit_once('@').is_some_and(|(name, sha)| {
        name.split('/').count() >= 2
            && sha.len() == 40
            && sha
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
pub fn document(name: &str, doc: &Value) -> Result<()> {
    fn registry_env(value: &Value) -> Result<()> {
        for key in [
            "NODE_AUTH_TOKEN",
            "CARGO_REGISTRY_TOKEN",
            "CARGO_REGISTRIES_CRATES_IO_TOKEN",
            "NPM_TOKEN",
        ] {
            if let Some(value) = value[key].as_str() {
                require(
                    !value.contains("secrets."),
                    "registry secret-token fallback forbidden",
                )?;
            }
        }
        Ok(())
    }
    registry_env(&doc["env"])?;
    require(
        doc["permissions"] == json!({"contents":"read"}),
        format!("{name}: global permissions expanded"),
    )?;
    require(
        doc["on"].get("pull_request_target").is_none(),
        "privileged untrusted PR trigger forbidden",
    )?;
    let jobs = doc["jobs"].as_object().ok_or("missing workflow jobs")?;
    for (id, job) in jobs {
        registry_env(&job["env"])?;
        let permissions = &job["permissions"];
        for (permission, allowed) in [
            ("contents", name == "release.yml" && id == "gh-release"),
            ("packages", name == "release.yml" && id == "images"),
            ("pages", name == "pages.yml" && id == "deploy"),
        ] {
            require(
                permissions[permission] != "write" || allowed,
                format!("unexpected {permission} write: {name}/{id}"),
            )?;
        }
        if permissions["id-token"] == "write" {
            require(
                (name == "release.yml" && ["crates", "npm", "images"].contains(&id.as_str()))
                    || (name == "pages.yml" && id == "deploy"),
                "unexpected OIDC privilege",
            )?;
        }
        for step in job["steps"].as_array().into_iter().flatten() {
            registry_env(&step["env"])?;
            require(
                step["continue-on-error"] != true,
                "validation step failure may not be tolerated",
            )?;
            if let Some(action) = step["uses"].as_str() {
                require(pinned(action), format!("unpinned action {action}"))?;
            }
            for key in ["NODE_AUTH_TOKEN", "CARGO_REGISTRY_TOKEN", "NPM_TOKEN"] {
                if let Some(value) = step["env"][key].as_str() {
                    require(
                        !value.contains("secrets."),
                        "registry secret-token fallback forbidden",
                    )?;
                }
            }
            if let Some(run) = step["run"].as_str() {
                require(
                    !run.contains("delivery.py")
                        && !run.contains("check_action_pins.py")
                        && !run.contains("test_delivery.py"),
                    "authored delivery validator still uses a legacy script",
                )?;
            }
        }
    }
    if name == "rust-validation.yml" {
        require(
            strings(&doc["jobs"]["ready"]["needs"])
                == REQUIRED.iter().map(|id| (*id).into()).collect(),
            "required aggregate coverage changed",
        )?;
        for id in REQUIRED {
            let job = &jobs[*id];
            require(
                job.get("if").is_none() && job.get("continue-on-error").is_none(),
                format!("required job may not be conditional/tolerated: {id}"),
            )?;
        }
        for (id, required) in [
            ("native", "pilot native"),
            ("browser", "pilot browser"),
            ("docker", "pilot docker"),
            ("packages", " -- package"),
            ("node", " -- node"),
            ("advisories", "audit-closure"),
        ] {
            require(
                jobs[id]["steps"].as_array().is_some_and(|steps| {
                    steps.iter().any(|step| {
                        step["run"].as_str().is_some_and(|run| {
                            run.contains("cargo run --locked")
                                && run.contains("scripts/ci/delivery/Cargo.toml")
                                && run.contains(required)
                        }) && step.get("if").is_none()
                    })
                }),
                format!("required Rust authority step missing/conditional: {id}"),
            )?;
        }
    }
    if name == "release.yml" {
        require(
            doc["concurrency"]["cancel-in-progress"] == false,
            "release may not cancel",
        )?;
        require(
            doc["on"].get("workflow_dispatch").is_none(),
            "manual publication forbidden",
        )?;
        require(
            jobs["validate"]["uses"] == "./.github/workflows/rust-validation.yml",
            "release must rerun exact-source validation",
        )?;
        require(
            strings(&jobs["images"]["needs"]) == ["readiness".into()].into()
                && strings(&jobs["readiness"]["needs"]) == ["validate".into()].into(),
            "image authorization bypass",
        )?;
        let images: BTreeSet<_> = jobs["images"]["strategy"]["matrix"]["include"]
            .as_array()
            .ok_or("missing image matrix")?
            .iter()
            .filter_map(|v| v["name"].as_str())
            .collect();
        require(
            images == ["core", "gate"].into(),
            "only defender core/gate may publish",
        )?;
        require(
            jobs["images"]["strategy"]["matrix"]["include"]
                .as_array()
                .is_some_and(|images| {
                    images.iter().all(|image| {
                        matches!(
                            (image["name"].as_str(), image["dockerfile"].as_str()),
                            (Some("core"), Some("build/core.Dockerfile"))
                                | (Some("gate"), Some("build/gate.Dockerfile"))
                        )
                    })
                }),
            "defender image name may not alias an attacker Dockerfile",
        )?;
        for id in ["crates", "npm"] {
            require(
                strings(&jobs[id]["needs"]).contains("readiness"),
                "registry readiness bypass",
            )?;
        }
    }
    Ok(())
}
pub fn check(root: &Path) -> Result<()> {
    for (name, doc) in workflows(root)? {
        document(&name, &doc)?;
    }
    // Only newly authored Rust/workflow sources are owned here; legacy oracles
    // remain distinct. Enforce actual bytes, not a post-hash newline rewrite.
    let paths = command::text(
        root,
        "git",
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "scripts/ci/delivery",
            ".github/workflows",
        ],
    )?;
    source::lf_files(
        root,
        paths
            .lines()
            .filter(|p| p.ends_with(".rs") || p.ends_with(".yml") || p.ends_with(".toml"))
            .map(str::to_owned),
    )?;
    println!("Rust workflow authorization/LF policy passed");
    Ok(())
}
pub fn pins(root: &Path) -> Result<()> {
    let mut actions = BTreeSet::new();
    for (_, doc) in workflows(root)? {
        for job in doc["jobs"].as_object().ok_or("missing jobs")?.values() {
            for step in job["steps"].as_array().into_iter().flatten() {
                if let Some(action) = step["uses"].as_str()
                    && !action.starts_with("./")
                {
                    require(pinned(action), "unpinned action")?;
                    actions.insert(action.to_owned());
                }
            }
        }
    }
    let mut reports = Vec::new();
    for action in actions {
        let (name, sha) = action.rsplit_once('@').ok_or("bad action")?;
        let parts: Vec<_> = name.split('/').collect();
        let commit: Value = serde_json::from_str(&command::text(
            root,
            "gh",
            &[
                "api",
                &format!("repos/{}/{}/commits/{sha}", parts[0], parts[1]),
            ],
        )?)
        .map_err(|e| e.to_string())?;
        require(commit["sha"] == sha, "action commit identity mismatch")?;
        let suffix = if parts.len() > 2 {
            format!("/{}/", parts[2..].join("/"))
        } else {
            "/".into()
        };
        let prefix = format!(
            "https://raw.githubusercontent.com/{}/{}/{sha}{suffix}",
            parts[0], parts[1]
        );
        let content = match ureq::get(&format!("{prefix}action.yml")).call() {
            Ok(mut response) => response
                .body_mut()
                .read_to_string()
                .map_err(|e| e.to_string())?,
            Err(ureq::Error::StatusCode(404)) => ureq::get(&format!("{prefix}action.yaml"))
                .call()
                .map_err(|e| e.to_string())?
                .body_mut()
                .read_to_string()
                .map_err(|e| e.to_string())?,
            Err(error) => return Err(error.to_string()),
        };
        require(
            yaml(&content)?.get("runs").is_some(),
            "action entrypoint missing",
        )?;
        reports.push(json!({"action":action,"commit":sha,"entrypoint":"verified"}));
    }
    println!("{}", json!(reports));
    Ok(())
}
pub fn jobs(raw: &str) -> Result<()> {
    let value: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let jobs = value.as_object().ok_or("aggregate must be an object")?;
    require(
        !jobs.is_empty() && jobs.values().all(|v| v["result"] == "success"),
        "failed/skipped/cancelled validation never authorizes release",
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn yaml_boolean_keys_follow_yaml12() {
        assert!(
            yaml("on:\n  workflow_call:\npermissions:\n  contents: read\n")
                .unwrap()
                .get("on")
                .is_some()
        );
    }
    #[test]
    fn moving_action_refs_fail() {
        for action in [
            "actions/checkout@v4",
            "actions/checkout@main",
            "actions/checkout@123",
        ] {
            assert!(!pinned(action));
        }
        assert!(pinned(
            "actions/checkout@11d5960a326750d5838078e36cf38b85af677262"
        ));
    }
    #[test]
    fn every_non_success_aggregate_fails() {
        for result in ["skipped", "cancelled", "failure", "neutral", ""] {
            assert!(jobs(&json!({"docker":{"result":result}}).to_string()).is_err());
        }
        assert!(jobs("{}").is_err());
        assert!(jobs(r#"{"docker":{"result":"success"}}"#).is_ok());
    }
    #[test]
    fn broad_global_permissions_fail() {
        assert!(
            document(
                "ci.yml",
                &json!({"permissions":{"contents":"write"},"jobs":{}})
            )
            .is_err()
        );
    }
    #[test]
    fn required_docker_aggregate_and_execution_cannot_be_removed_or_skipped() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../.github/workflows/rust-validation.yml");
        let original = yaml(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert!(document("rust-validation.yml", &original).is_ok());
        for mutation in 0..4 {
            let mut changed = original.clone();
            match mutation {
                0 => changed["jobs"]["ready"]["needs"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|value| value != "docker"),
                1 => changed["jobs"]["docker"]["if"] = json!("false"),
                2 => changed["jobs"]["docker"]["steps"] = json!([]),
                _ => changed["jobs"]["docker"]["steps"][0]["continue-on-error"] = json!(true),
            }
            assert!(document("rust-validation.yml", &changed).is_err());
        }
    }
    #[test]
    fn registry_tokens_at_job_scope_cannot_bypass_step_policy() {
        let workflow = json!({"permissions":{"contents":"read"},"on":{"push":null},"jobs":{"x":{"env":{"NPM_TOKEN":"${{ secrets.NPM_TOKEN }}"}}}});
        assert!(document("ci.yml", &workflow).is_err());
    }
}
