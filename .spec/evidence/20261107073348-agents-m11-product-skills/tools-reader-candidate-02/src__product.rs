//! Product skill packaging/links, distinct from internal developer metadata.
#[path = "product_selected.rs"]
pub mod selected;
use crate::inventory::{self, FileRecord};
use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[derive(Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub name: Option<String>,
    pub entrypoint_sha256: String,
    pub resources: Vec<FileRecord>,
    pub local_references: usize,
    pub errors: Vec<String>,
    pub runtime_qualified: bool,
}

pub struct Bundle {
    pub entry: String,
    pub resources: BTreeMap<String, String>,
}

fn local_target(source: &str, target: &str) -> Result<Option<String>, String> {
    if target.starts_with("https://") || target.starts_with('#') {
        return Ok(None);
    }
    if target.is_empty() || target.starts_with('/') || target.contains(['\\', ':', '%', '?']) {
        return Err(format!("unsafe or unsupported link target: {target}"));
    }
    let path = target.split('#').next().unwrap_or_default();
    if path
        .split('/')
        .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err(format!("reference escapes or has ambiguous path: {target}"));
    }
    let parent = source.rsplit_once('/').map_or("", |(p, _)| p);
    Ok(Some(if parent.is_empty() {
        path.into()
    } else {
        format!("{parent}/{path}")
    }))
}

/// Pure validation of the portable bundle's standard inline Markdown links.
/// Not a full Markdown parser or a semantic/model-performance assessment.
pub fn validate(entry: &str, resources: &BTreeMap<String, String>) -> Report {
    let mut result = Report {
        schema: "humanymous.product-skill.v1",
        name: None,
        entrypoint_sha256: format!("{:x}", Sha256::digest(entry.as_bytes())),
        resources: Vec::new(),
        local_references: 0,
        errors: Vec::new(),
        runtime_qualified: false,
    };
    match inventory::description(entry) {
        Ok((name, _)) => result.name = Some(name),
        Err(e) => result.errors.push(e),
    }
    if entry.len() > 256 * 1024
        || entry.len() + resources.values().map(String::len).sum::<usize>() > 8 * 1024 * 1024
    {
        result
            .errors
            .push("product bundle exceeds text budget".into());
        return result;
    }
    let links = Regex::new(r"\[[^\]]*\]\(([^)\s]+)\)").expect("fixed link pattern");
    let all = std::iter::once(("SKILL.md", entry))
        .chain(resources.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (path, text) in all {
        if text.contains('\r') {
            result.errors.push(format!("CR bytes in {path}"));
        }
        if path != "SKILL.md" {
            if !path.starts_with("references/")
                || path.contains("..")
                || path.contains(['\\', ':', '%'])
            {
                result
                    .errors
                    .push(format!("invalid product resource path: {path}"));
            }
            result.resources.push(FileRecord {
                path: path.into(),
                bytes: text.len(),
                sha256: format!("{:x}", Sha256::digest(text.as_bytes())),
            });
        }
        for link in links.captures_iter(text) {
            match local_target(path, &link[1]) {
                Ok(Some(target)) => {
                    result.local_references += 1;
                    if !resources.contains_key(&target) {
                        result
                            .errors
                            .push(format!("{path}: missing resource {target}"));
                    }
                    graph.entry(path.into()).or_default().push(target);
                }
                Err(e) => result.errors.push(format!("{path}: {e}")),
                Ok(None) => {}
            }
        }
    }
    let mut reachable = BTreeSet::new();
    let mut pending = vec!["SKILL.md".to_owned()];
    while let Some(path) = pending.pop() {
        if !reachable.insert(path.clone()) {
            continue;
        }
        if let Some(targets) = graph.get(&path) {
            pending.extend(targets.iter().cloned());
        }
    }
    for path in resources.keys() {
        if !reachable.contains(path) {
            result
                .errors
                .push(format!("unreferenced product resource: {path}"));
        }
    }
    result
}

/// Read only the product entrypoint and reference tree, never the maintenance target directory.
pub fn load(root: &Path) -> Result<Bundle, Box<dyn std::error::Error>> {
    for path in [root.to_path_buf(), root.join("SKILL.md")] {
        if fs::symlink_metadata(path)?.file_type().is_symlink() {
            return Err("product root/entrypoint link refused".into());
        }
    }
    let mut records = Vec::new();
    let mut errors = Vec::new();
    let refs = root.join("references");
    if refs.exists() {
        if fs::symlink_metadata(&refs)?.file_type().is_symlink() {
            return Err("references root link refused".into());
        }
        inventory::files(root, &refs, &mut records, &mut errors)?;
    }
    if !errors.is_empty() {
        return Err(errors.join("; ").into());
    }
    let resources = records
        .into_iter()
        .map(|r| Ok((r.path.clone(), fs::read_to_string(root.join(r.path))?)))
        .collect::<Result<BTreeMap<_, _>, std::io::Error>>()?;
    if fs::metadata(root.join("SKILL.md"))?.len() > 256 * 1024 {
        return Err("oversized product entrypoint".into());
    }
    let entry = fs::read_to_string(root.join("SKILL.md"))?;
    Ok(Bundle { entry, resources })
}

pub fn inspect(root: &Path) -> Result<Report, Box<dyn std::error::Error>> {
    let bundle = load(root)?;
    Ok(validate(&bundle.entry, &bundle.resources))
}

#[cfg(test)]
mod tests {
    use super::*;
    const HEADER: &str = "---\nname: humanymous\ndescription: Operate an owned deployment.\n---\n";
    #[test]
    fn repository_product_bundle_has_resolved_references() {
        let entry = format!("{HEADER}[install](references/installation.md)");
        let resources = BTreeMap::from([
            ("references/installation.md".into(), "[operate](operations.md)".into()),
            ("references/operations.md".into(), "[adapt](adapters.md)".into()),
            ("references/adapters.md".into(), "[qualify](qualification.md)".into()),
            ("references/qualification.md".into(), "Synthetic reference endpoint.".into()),
        ]);
        let report = validate(&entry, &resources);
        assert!(report.errors.is_empty());
        assert_eq!(report.resources.len(), 4);
        assert!(!report.runtime_qualified);
    }
    #[test]
    fn accepts_portable_bundle_without_runtime_claim() {
        let r = validate(
            &format!("{HEADER}[install](references/install.md)"),
            &BTreeMap::from([(
                "references/install.md".into(),
                "[official](https://example.com)".into(),
            )]),
        );
        assert!(r.errors.is_empty());
        assert!(!r.runtime_qualified);
        assert_eq!(r.local_references, 1);
    }
    #[test]
    fn refuses_missing_and_escape_links() {
        for path in [
            "references/missing.md",
            "../private.md",
            "/etc/passwd",
            "file:///profile",
            "references/%2e%2e/secret",
            "javascript:alert(1)",
        ] {
            assert!(
                !validate(&format!("{HEADER}[x]({path})"), &BTreeMap::new())
                    .errors
                    .is_empty(),
                "{path}"
            );
        }
    }
    #[test]
    fn duplicate_metadata_is_not_last_wins() {
        assert!(
            !validate(
                &HEADER.replace("name: humanymous", "name: humanymous\nname: forged"),
                &BTreeMap::new()
            )
            .errors
            .is_empty()
        );
    }
    #[test]
    fn validates_transitive_resources_and_rejects_orphans() {
        let resources = BTreeMap::from([
            ("references/install.md".into(), "[next](detail.md)".into()),
            ("references/detail.md".into(), "details".into()),
        ]);
        assert!(
            validate(&format!("{HEADER}[i](references/install.md)"), &resources)
                .errors
                .is_empty()
        );
        assert!(!validate(HEADER, &resources).errors.is_empty());
    }
    #[test]
    fn orphan_cycles_do_not_count_as_reachable() {
        let resources = BTreeMap::from([
            ("references/a.md".into(), "[b](b.md)".into()),
            ("references/b.md".into(), "[a](a.md)".into()),
        ]);
        assert_eq!(validate(HEADER, &resources).errors.len(), 2);
    }
}
