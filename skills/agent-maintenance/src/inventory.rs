//! Canonical inventory validation. Does not follow links or rewrite mirrors.
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, io, path::Path};

#[derive(Serialize)]
pub struct FileRecord {
    pub path: String,
    pub bytes: usize,
    pub sha256: String,
}
#[derive(Default, Serialize)]
pub struct Report {
    pub schema_version: u8,
    pub files: Vec<FileRecord>,
    pub skills: usize,
    pub errors: Vec<String>,
    pub mirror_drift: Vec<String>,
    pub role_pointers: usize,
    pub canonical_roles: usize,
    pub elapsed_micros: u128,
}

pub fn description(text: &str) -> Result<(String, String), String> {
    let front = text
        .strip_prefix("---\n")
        .and_then(|t| t.split_once("\n---\n"))
        .ok_or("missing LF frontmatter")?
        .0;
    let mut name = None;
    let mut description_seen = false;
    let mut desc = String::new();
    let mut folded = false;
    for line in front.lines() {
        if let Some(value) = line.strip_prefix("name:") {
            if name.is_some() {
                return Err("duplicate name".into());
            }
            name = Some(value.trim().to_owned());
        }
        if let Some(value) = line.strip_prefix("description:") {
            if description_seen {
                return Err("duplicate description".into());
            }
            description_seen = true;
            folded = value.trim() == ">" || value.trim() == "|";
            if !folded {
                desc = value.trim().trim_matches('"').to_owned();
            }
        } else if folded {
            if line.starts_with(' ') || line.starts_with('\t') {
                if !desc.is_empty() {
                    desc.push(' ');
                }
                desc.push_str(line.trim());
            } else {
                folded = false;
            }
        }
    }
    let name = name.filter(|n| !n.is_empty()).ok_or("missing name")?;
    if name.len() > 64
        || !name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err("name must be lowercase ASCII/digits/hyphens, at most64bytes".into());
    }
    if desc.is_empty() || desc.chars().count() > 1024 {
        return Err("description must contain 1..1024 characters".into());
    }
    Ok((name, desc))
}

pub(crate) fn files(
    root: &Path,
    base: &Path,
    records: &mut Vec<FileRecord>,
    errors: &mut Vec<String>,
) -> io::Result<()> {
    let mut entries = fs::read_dir(base)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            errors.push(format!("symlink refused: {}", path.display()));
            continue;
        }
        if kind.is_dir() {
            files(root, &path, records, errors)?;
        } else if kind.is_file() {
            if entry.metadata()?.len() > 2 * 1024 * 1024 {
                errors.push(format!("oversized canonical file: {}", path.display()));
                continue;
            }
            let data = fs::read(&path)?;
            let relative = path
                .strip_prefix(root)
                .expect("inside root")
                .to_string_lossy()
                .replace('\\', "/");
            if data.contains(&b'\r') {
                errors.push(format!("CR bytes: {relative}"));
            }
            records.push(FileRecord {
                path: relative,
                bytes: data.len(),
                sha256: format!("{:x}", Sha256::digest(&data)),
            });
        }
    }
    Ok(())
}

pub fn check(root: &Path) -> Result<Report, Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let mut result = Report {
        schema_version: 1,
        ..Report::default()
    };
    for folder in [".agents/skills", ".agents/rules", ".agents/agents"] {
        if fs::symlink_metadata(root.join(folder))?
            .file_type()
            .is_symlink()
        {
            return Err(format!("canonical root is a symlink: {folder}").into());
        }
        files(
            root,
            &root.join(folder),
            &mut result.files,
            &mut result.errors,
        )?;
    }
    let mut names = std::collections::BTreeSet::new();
    for record in &result.files {
        if record.path.ends_with("/SKILL.md") {
            let text = fs::read_to_string(root.join(&record.path))?;
            match description(&text) {
                Ok((name, _)) => {
                    if !names.insert(name.clone()) {
                        result.errors.push(format!("duplicate skill: {name}"));
                    }
                    if !record.path.ends_with(&format!("/{name}/SKILL.md")) {
                        result
                            .errors
                            .push(format!("name/path mismatch: {}", record.path));
                    }
                }
                Err(e) => result.errors.push(format!("{}: {e}", record.path)),
            }
            result.skills += 1;
        }
        if let Some(relative) = record.path.strip_prefix(".agents/skills/") {
            for vendor in [".codex", ".claude", ".grok", ".gemini"] {
                let mirror = root.join(vendor).join("skills").join(relative);
                if !mirror.exists() || fs::read(mirror)? != fs::read(root.join(&record.path))? {
                    result
                        .mirror_drift
                        .push(format!("{vendor}/skills/{relative}"));
                }
            }
        }
    }
    let agents = fs::read_to_string(root.join("AGENTS.md"))?;
    for term in [
        "1.99.1",
        "2024",
        "Detection freeze",
        "Fail-closed",
        "dual-control",
        "A11y",
        "T4",
        "Docker",
        "git-ops",
    ] {
        if !agents.contains(term) {
            result
                .errors
                .push(format!("AGENTS missing invariant: {term}"));
        }
    }
    for name in [
        "blue-architect",
        "red-attacker",
        "evaluator",
        "staff-synthesizer",
        "sre-ops",
        "compliance-dpo",
    ] {
        let canonical = root.join(".agents/agents").join(format!("{name}.md"));
        let pointer = root.join(".agents/personas").join(format!("{name}.md"));
        match fs::symlink_metadata(&canonical) {
            Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {
                result.canonical_roles += 1;
            }
            _ => result.errors.push(format!("missing/unsafe canonical role: {name}")),
        }
        match fs::symlink_metadata(&pointer) {
            Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {
                if fs::read_to_string(&pointer)?.contains(&format!("../agents/{name}.md")) {
                    result.role_pointers += 1;
                } else {
                    result.errors.push(format!("invalid compatibility role pointer: {name}"));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            _ => result.errors.push(format!("unsafe compatibility role pointer: {name}")),
        }
    }
    result.elapsed_micros = started.elapsed().as_micros();
    Ok(result)
}

/// Observe user skills as external inputs without editing their content/config.
pub fn external(path: &Path) -> Result<Vec<FileRecord>, Box<dyn std::error::Error>> {
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err("external root symlink refused".into());
    }
    let mut records = Vec::new();
    let mut errors = Vec::new();
    files(path, path, &mut records, &mut errors)?;
    if errors.iter().any(|e| e.starts_with("symlink")) {
        return Err("external tree contains links; select a concrete root".into());
    }
    Ok(records
        .into_iter()
        .filter(|r| r.path.ends_with("SKILL.md"))
        .collect())
}

fn lf(text: &str) -> Result<String, &'static str> {
    let normalized = text.replace("\r\n", "\n");
    if normalized.contains('\r') {
        return Err("bare CR refused");
    }
    Ok(normalized)
}

/// Normalize inventoried canonical UTF-8 text in a trusted checkout.
/// Not a hostile-directory race-resistant publication primitive.
pub fn normalize(root: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut records = Vec::new();
    let mut errors = Vec::new();
    for folder in [
        ".agents/skills",
        ".agents/rules",
        ".agents/agents",
        ".agents/personas",
        ".agents/evals",
        ".agents/retirement",
        "skills/agent-maintenance/src",
        "skills/agent-maintenance/tests",
    ] {
        if !root.join(folder).exists() {
            continue;
        }
        if fs::symlink_metadata(root.join(folder))?
            .file_type()
            .is_symlink()
        {
            return Err("symlink root refused".into());
        }
        files(root, &root.join(folder), &mut records, &mut errors)?;
    }
    if errors.iter().any(|e| !e.starts_with("CR bytes:")) {
        return Err("structural errors prevent normalization".into());
    }
    let mut pending = Vec::new();
    let mut paths = records.into_iter().map(|r| r.path).collect::<Vec<_>>();
    for path in [
        "AGENTS.md",
        ".agents/README.md",
        ".agents/manifest.yaml",
        ".agents/INTEGRATION.md",
        "skills/README.md",
        "skills/agent-maintenance/Cargo.toml",
        "skills/agent-maintenance/Cargo.lock",
        "skills/agent-maintenance/.gitignore",
    ] {
        if root.join(path).exists() {
            if fs::symlink_metadata(root.join(path))?
                .file_type()
                .is_symlink()
            {
                return Err("symlink file refused".into());
            }
            paths.push(path.into());
        }
    }
    for path in paths {
        let text = fs::read_to_string(root.join(&path))?;
        let normalized = lf(&text)?;
        if normalized != text {
            pending.push((path, normalized));
        }
    }
    let mut changes = Vec::new();
    for (path, text) in pending {
        fs::write(root.join(&path), text)?;
        changes.push(path);
    }
    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizes_crlf_but_refuses_bare_cr() {
        assert_eq!(lf("a\r\nb\n").unwrap(), "a\nb\n");
        assert!(lf("a\rb").is_err());
    }
    #[test]
    fn folded_description_stops_at_next_key() {
        assert_eq!(
            description(
                "---\nname: foo\ndescription: >\n  Use when x\n  and y.\nwhen_to_use: z\n---\n"
            )
            .unwrap(),
            ("foo".into(), "Use when x and y.".into())
        );
    }
    #[test]
    fn invalid_frontmatter_rejected() {
        for value in [
            "name: x",
            "---\nname: x\n---\n",
            "---\ndescription: x\n---\n",
            "---\r\nname: x\r\n---\r\n",
        ] {
            assert!(description(value).is_err());
        }
    }
    #[test]
    fn description_limit_is_unicode_characters() {
        let text = format!("---\nname: x\ndescription: {}\n---\n", "가".repeat(1024));
        assert!(description(&text).is_ok());
        assert!(description(&text.replace(&"가".repeat(1024), &"가".repeat(1025))).is_err());
    }
}
