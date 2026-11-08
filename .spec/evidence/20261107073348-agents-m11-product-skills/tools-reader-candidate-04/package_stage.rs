//! Bounded explicit skill/eval staging; no ambient inventory or source authentication.
#[path = "package_selection.rs"]
mod selection;
use crate::maintenance::{
    inventory, package::{self, Entry, Receipt, SourceReceipt}, product::Bundle,
    selected_inputs::{Clock, Limits, SystemClock, capture_source_file, checked_path},
};
use selection::{Kind, MAX_INPUTS, MAX_REQUEST, MAX_TEXT};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::{Component, Path, PathBuf}};

fn hash(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }

fn output_path(repo: &Path, output: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if !output.is_absolute() || output.components().any(|c| matches!(c, Component::CurDir | Component::ParentDir))
        || output.to_str().is_none_or(|s| s.len() > 4096 || s.chars().any(char::is_control))
        || output.components().any(|c| matches!(c, Component::Normal(name) if [".agents", ".codex", ".claude", ".grok", ".gemini"].iter().any(|blocked| name.to_string_lossy().eq_ignore_ascii_case(blocked))))
    { return Err("stage_output_path".into()); }
    let parent = output.parent().ok_or("stage_output_parent")?;
    checked_path(repo, false)?;
    checked_path(parent, false)?;
    let source = fs::canonicalize(repo)?;
    let selected = fs::canonicalize(parent)?.join(output.file_name().ok_or("stage_output_name")?);
    if selected.starts_with(&source) || source.starts_with(&selected) {
        return Err("stage_output_overlaps_source".into());
    }
    match fs::symlink_metadata(&selected) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(selected),
        _ => Err("stage_output_already_exists_or_unknown".into()),
    }
}

/// Reads only explicitly pinned files. All validation completes before output creation.
pub fn stage_request_file(request_path: &Path, output: &Path) -> Result<Receipt, Box<dyn std::error::Error>> {
    let mut clock = SystemClock::default();
    let (request_bytes, request_read) = capture_source_file(request_path, MAX_REQUEST, &mut clock)?;
    let request = selection::decode(&request_bytes)?;
    let output = output_path(&request.repo_root, output)?;
    let mut actual = request_read.actual_read_bytes;
    let mut texts = BTreeMap::new();
    let mut inputs = Vec::with_capacity(request.files.len());
    for pin in &request.files {
        // Declared length tightens the existing reader, including growth probes.
        let (captured, read) = capture_source_file(
            &request.repo_root.join(&pin.path), pin.bytes.max(1), &mut clock,
        )?;
        actual = actual.checked_add(read.actual_read_bytes).ok_or("stage_read_overflow")?;
        if actual > MAX_REQUEST + MAX_TEXT + MAX_INPUTS as u64 + 1 {
            return Err("stage_actual_read_cap".into());
        }
        let sha256 = hash(&captured);
        if captured.len() as u64 != pin.bytes || sha256 != pin.sha256 {
            return Err("stage_captured_source_mismatch".into());
        }
        let text = String::from_utf8(captured).map_err(|_| "stage_input_utf8")?;
        if text.contains('\r') { return Err("stage_input_non_lf".into()); }
        inputs.push(Entry { path: pin.path.clone(), bytes: text.len(), sha256 });
        texts.insert(pin.path.clone(), text);
    }
    let license = texts.remove("LICENSE").ok_or("stage_missing_license")?;
    let mut files = match request.kind {
        Kind::Product => {
            let entry = texts.remove("skills/SKILL.md").ok_or("stage_missing_product")?;
            let (name, _) = inventory::description(&entry).map_err(|_| "stage_product_metadata")?;
            if request.expected_skill_names.first().map(String::as_str) != Some(name.as_str()) {
                return Err("stage_product_name".into());
            }
            let resources = texts.iter().filter_map(|(path, text)| {
                path.strip_prefix("skills/").map(|p| (p.to_owned(), text.clone()))
            }).collect();
            package::plan_product(&Bundle { entry, resources }, &license)?
        }
        Kind::Developer => {
            let skills = texts.iter().filter_map(|(path, text)| {
                path.strip_prefix(".agents/skills/").map(|p| (p.to_owned(), text.clone()))
            }).collect::<BTreeMap<_, _>>();
            package::plan_developer(&skills, &license)?
        }
    };
    let eval_prefix = format!(".agents/evals/{}/", request.kind.name());
    for (path, text) in &texts {
        if let Some(relative) = path.strip_prefix(&eval_prefix) {
            files.insert(format!("plugins/{}/evals/{relative}", request.kind.plugin()), text.clone());
        }
    }
    let case_count = files.keys().filter(|path| {
        path.starts_with(&format!("plugins/{}/evals/", request.kind.plugin())) && path.ends_with("/prompt.md")
    }).count();
    if case_count != request.kind.cases().len() { return Err("stage_eval_case_count".into()); }
    if clock.elapsed() >= Limits::default().max_elapsed { return Err("stage_deadline_before_write".into()); }
    let source = SourceReceipt {
        repo_root: request.repo_root,
        declared_source_commit: request.source_commit,
        request_sha256: hash(&request_bytes), inputs,
        actual_read_bytes: actual, elapsed_micros_before_write: clock.elapsed().as_micros(),
        input_selection_declared_complete: true, selected_content_matches: true,
        canonical_tree_complete: false, source_commit_verified: false,
        provider_identity_qualified: false, qualified_complete: false,
        identity_status: "unsupported_source_identity; cooperative selected-content staging only",
    };
    package::stage_prepared(&output, request.kind.name(), files, source, &mut clock)
}
