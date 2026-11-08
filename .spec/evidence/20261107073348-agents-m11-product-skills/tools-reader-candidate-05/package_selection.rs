//! Fixed skill/evaluation staging input contract; no filesystem or source authority.
use crate::maintenance::selected_inputs::{FrozenSelection, Input, Limits, Root};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};

pub const MAX_REQUEST: u64 = 2 * 1024 * 1024;
pub const MAX_TEXT: u64 = 8 * 1024 * 1024;
pub const MAX_INPUTS: usize = 510; // Two generated metadata files keep output <=512.

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Product,
    Developer,
}
impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Product => "product",
            Self::Developer => "developer",
        }
    }
    pub fn plugin(self) -> &'static str {
        match self {
            Self::Product => "humanymous-product",
            Self::Developer => "humanymous-development",
        }
    }
    pub fn cases(self) -> &'static [&'static str] {
        match self {
            Self::Product => &["install-plan", "readiness", "unrelated-ui"],
            Self::Developer => &[
                "ci-identity",
                "ci-accounting",
                "release-prepare",
                "release-failed-claim",
                "unrelated-layout",
            ],
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub repo_root: PathBuf,
    pub source_commit: String,
    pub kind: Kind,
    pub input_selection_complete: bool,
    pub expected_skill_names: Vec<String>,
    pub files: Vec<Pin>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub path: String,
    pub owner: String,
    pub presence: String,
    pub bytes: u64,
    pub sha256: String,
}
fn hex(s: &str, length: usize) -> bool {
    s.len() == length
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn skill_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
pub fn cap(path: &str) -> u64 {
    if path == "skills/SKILL.md" {
        256 * 1024
    } else {
        2 * 1024 * 1024
    }
}

/// Compare raw bytes before any parsing or rewriting; shared by staging and
/// request materialization so changed legal bytes cannot be silently repinned.
pub fn verify_pin(pin: &Pin, captured: &[u8]) -> Result<String, &'static str> {
    use sha2::{Digest, Sha256};
    let sha256 = format!("{:x}", Sha256::digest(captured));
    if captured.len() as u64 != pin.bytes || sha256 != pin.sha256 {
        return Err("stage_captured_source_mismatch");
    }
    Ok(sha256)
}

pub fn decode(bytes: &[u8]) -> Result<Request, &'static str> {
    if bytes.len() as u64 > MAX_REQUEST {
        return Err("stage_request_size");
    }
    let request: Request = serde_json::from_slice(bytes).map_err(|_| "stage_request_json")?;
    if request.schema != "humanymous.stage-eval-source.v1"
        || !request.input_selection_complete
        || !(hex(&request.source_commit, 40) || hex(&request.source_commit, 64))
        || request.files.is_empty()
        || request.files.len() > MAX_INPUTS
        || request.expected_skill_names.is_empty()
        || request.expected_skill_names.len() > 128
        || request.expected_skill_names.iter().any(|n| !skill_name(n))
        || request
            .repo_root
            .to_str()
            .is_none_or(|p| p.contains(['*', '?', '[', ']']) || p.chars().any(char::is_control))
    {
        return Err("stage_request_contract");
    }
    let names: BTreeSet<_> = request.expected_skill_names.iter().cloned().collect();
    if names.len() != request.expected_skill_names.len()
        || (request.kind == Kind::Product && names.len() != 1)
    {
        return Err("stage_skill_set");
    }
    FrozenSelection::new(
        vec![Root {
            id: "repo".into(),
            path: request.repo_root.clone(),
        }],
        request
            .files
            .iter()
            .map(|p| Input {
                root: "repo".into(),
                path: p.path.clone(),
            })
            .collect(),
        true,
        &Limits::default(),
    )
    .map_err(|_| "stage_root_or_paths")?;
    let eval_root = format!(".agents/evals/{}/", request.kind.name());
    let mut required = BTreeSet::from(["LICENSE".to_owned(), format!("{eval_root}README.md")]);
    for case in request.kind.cases() {
        for tail in ["prompt.md", "graders/behavior.md", "graders/discovery.md"] {
            required.insert(format!("{eval_root}{case}/{tail}"));
        }
    }
    match request.kind {
        Kind::Product => {
            required.insert("skills/SKILL.md".into());
        }
        Kind::Developer => {
            for name in &names {
                required.insert(format!(".agents/skills/{name}/SKILL.md"));
            }
        }
    }
    let mut total = 0_u64;
    for pin in &request.files {
        let owner = if pin.path == "LICENSE" {
            "repository"
        } else {
            "agents-meta"
        };
        if pin.presence != "required"
            || pin.bytes > cap(&pin.path)
            || !hex(&pin.sha256, 64)
            || pin.path.contains(['*', '?', '[', ']'])
            || (pin.path != "LICENSE" && !pin.path.ends_with(".md"))
            || pin.owner != owner
        {
            return Err("stage_pin_contract");
        }
        total = total.checked_add(pin.bytes).ok_or("stage_size_overflow")?;
        if total > MAX_TEXT {
            return Err("stage_source_text_cap");
        }
        let allowed = if pin.path == "LICENSE" || required.contains(&pin.path) {
            true
        } else if request.kind == Kind::Product {
            pin.path.starts_with("skills/references/")
        } else {
            pin.path
                .strip_prefix(".agents/skills/")
                .and_then(|p| p.split_once('/'))
                .is_some_and(|(name, _)| names.contains(name))
        };
        if !allowed {
            return Err("stage_profile_exclusion");
        }
    }
    let selected: BTreeSet<_> = request.files.iter().map(|pin| pin.path.clone()).collect();
    if !required.is_subset(&selected) {
        return Err("stage_missing_required_input");
    }
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn product() -> serde_json::Value {
        let mut paths = vec![
            "LICENSE".to_owned(),
            "skills/SKILL.md".into(),
            ".agents/evals/product/README.md".into(),
        ];
        for case in Kind::Product.cases() {
            for tail in ["prompt.md", "graders/behavior.md", "graders/discovery.md"] {
                paths.push(format!(".agents/evals/product/{case}/{tail}"));
            }
        }
        serde_json::json!({"schema":"humanymous.stage-eval-source.v1","repo_root":std::env::temp_dir(),
            "source_commit":"a".repeat(40),"kind":"product","input_selection_complete":true,
            "expected_skill_names":["fixture"],"files":paths.iter().map(|p|serde_json::json!({
                "path":p,"owner":if p=="LICENSE" {"repository"} else {"agents-meta"},
                "presence":"required","bytes":0,"sha256":"0".repeat(64)})).collect::<Vec<_>>()})
    }
    #[test]
    fn fixed_eval_case_and_grader_closure_is_required() {
        let mut value = product();
        assert!(decode(&serde_json::to_vec(&value).unwrap()).is_ok());
        value["files"].as_array_mut().unwrap().pop();
        assert!(decode(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    #[test]
    fn another_profile_or_executable_is_not_a_stage_resource() {
        for path in [
            ".agents/evals/developer/README.md",
            "scripts/agents/git-coord.ps1",
            "skills/../secret.md",
        ] {
            let mut value = product();
            value["files"][0]["path"] = path.into();
            assert!(decode(&serde_json::to_vec(&value).unwrap()).is_err());
        }
    }
    #[test]
    fn false_completeness_unknown_fields_and_aliases_refuse() {
        let mut value = product();
        value["input_selection_complete"] = false.into();
        assert!(decode(&serde_json::to_vec(&value).unwrap()).is_err());
        let mut value = product();
        value["ambient_fallback"] = true.into();
        assert!(decode(&serde_json::to_vec(&value).unwrap()).is_err());
        let mut value = product();
        let duplicate = value["files"][0].clone();
        value["files"].as_array_mut().unwrap().push(duplicate);
        assert!(decode(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    #[test]
    fn tighter_caps_and_duplicate_json_keys_remain_failures() {
        let mut value = product();
        value["files"][1]["bytes"] = (256 * 1024 + 1).into();
        assert!(decode(&serde_json::to_vec(&value).unwrap()).is_err());
        assert!(decode(br#"{"schema":"x","schema":"y"}"#).is_err());
    }
}
