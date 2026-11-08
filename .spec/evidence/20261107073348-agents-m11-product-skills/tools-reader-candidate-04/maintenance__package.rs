//! Fresh staged marketplace artifacts. Never writes active provider mirrors.
use crate::maintenance::{
    inventory,
    selected_inputs::{Clock, Limits, SystemClock},
    product::{self, Bundle},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

#[derive(Serialize)]
pub struct Entry {
    pub path: String,
    pub bytes: usize,
    pub sha256: String,
}
#[derive(Serialize)]
pub struct Receipt {
    pub schema: &'static str,
    pub kind: String,
    pub files: Vec<Entry>,
    pub runtime_qualified: bool,
    pub release_qualified: bool,
    pub source: SourceReceipt,
}

#[derive(Serialize)]
pub struct SourceReceipt {
    pub repo_root: std::path::PathBuf,
    pub declared_source_commit: String,
    pub request_sha256: String,
    pub inputs: Vec<Entry>,
    pub actual_read_bytes: u64,
    pub elapsed_micros_before_write: u128,
    pub input_selection_declared_complete: bool,
    pub selected_content_matches: bool,
    pub canonical_tree_complete: bool,
    pub source_commit_verified: bool,
    pub provider_identity_qualified: bool,
    pub qualified_complete: bool,
    pub identity_status: &'static str,
}

fn metadata(
    name: &str,
    files: &mut BTreeMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = serde_json::json!({"name":"humanymous-skills-eval","owner":{"name":"Humanymous"},"plugins":[{"name":name,"source":format!("./plugins/{name}")} ]});
    files.insert(
        ".claude-plugin/marketplace.json".into(),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    );
    let plugin = serde_json::json!({"name":name,"version":"0.1.0","description":"Source-bound experimental Humanymous skills evaluation artifact; not a registry release.","skills":"./skills/"});
    files.insert(
        format!("plugins/{name}/.claude-plugin/plugin.json"),
        format!("{}\n", serde_json::to_string_pretty(&plugin)?),
    );
    Ok(())
}

pub fn plan_product(
    bundle: &Bundle,
    license: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let report = product::validate(&bundle.entry, &bundle.resources);
    if !report.errors.is_empty() {
        return Err(report.errors.join("; ").into());
    }
    let name = report.name.ok_or("missing product name")?;
    let mut files = BTreeMap::new();
    metadata("humanymous-product", &mut files)?;
    let prefix = format!("plugins/humanymous-product/skills/{name}");
    files.insert(format!("{prefix}/SKILL.md"), bundle.entry.clone());
    for (path, text) in &bundle.resources {
        files.insert(format!("{prefix}/{path}"), text.clone());
    }
    files.insert("LICENSE".into(), license.into());
    Ok(files)
}

pub(crate) fn plan_developer(
    skills: &BTreeMap<String, String>,
    license: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    if skills.is_empty() { return Err("empty selected developer skill set".into()); }
    let mut files = BTreeMap::new();
    metadata("humanymous-development", &mut files)?;
    for (path, text) in skills {
        if path.ends_with("/SKILL.md") {
            let (name, _) = inventory::description(text)?;
            if path != &format!("{name}/SKILL.md") {
                return Err("developer skill name/path mismatch".into());
            }
        }
        files.insert(format!("plugins/humanymous-development/skills/{path}"), text.clone());
    }
    files.insert("LICENSE".into(), license.into());
    Ok(files)
}

/// Write one fresh artifact only. Partial output remains for diagnosis on any failure.
pub(crate) fn stage_prepared(
    output: &Path,
    kind: &str,
    files: BTreeMap<String, String>,
    source: SourceReceipt,
    clock: &mut SystemClock,
) -> Result<Receipt, Box<dyn std::error::Error>> {
    if !["product", "developer"].contains(&kind) {
        return Err("unknown staged skill kind".into());
    }
    if files.len() > 512 || files.values().map(String::len).sum::<usize>() > 8 * 1024 * 1024 {
        return Err("staged artifact exceeds budget".into());
    }
    for (path, text) in &files {
        super::sessions::model::scope(path)?;
        if path.contains('*') || text.contains('\r') {
            return Err("non-LF or wildcard artifact path".into());
        }
    }
    if clock.elapsed() >= Limits::default().max_elapsed { return Err("stage write deadline".into()); }
    fs::create_dir(output)?;
    let mut receipt = Receipt {
        schema: "humanymous.skill-package.v1",
        kind: kind.into(),
        files: Vec::new(),
        runtime_qualified: false,
        release_qualified: false,
        source,
    };
    for (path, text) in files {
        if clock.elapsed() >= Limits::default().max_elapsed { return Err("stage write deadline".into()); }
        let target = output.join(&path);
        fs::create_dir_all(target.parent().ok_or("artifact parent missing")?)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        receipt.files.push(Entry {
            path,
            bytes: text.len(),
            sha256: format!("{:x}", Sha256::digest(text.as_bytes())),
        });
    }
    if clock.elapsed() >= Limits::default().max_elapsed { return Err("stage receipt deadline".into()); }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("bundle-receipt.json"))?;
    file.write_all(serde_json::to_string_pretty(&receipt)?.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    if clock.elapsed() >= Limits::default().max_elapsed { return Err("stage final deadline".into()); }
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn product_plan_is_portable_and_has_no_developer_or_executable_glue() {
        let bundle=Bundle{entry:"---\nname: humanymous\ndescription: Operate owned deployments.\n---\n[x](references/x.md)\n".into(),resources:BTreeMap::from([("references/x.md".into(),"bounded guidance\n".into())])};
        let files = plan_product(&bundle, "license\n").unwrap();
        assert!(files.contains_key("plugins/humanymous-product/skills/humanymous/SKILL.md"));
        assert!(
            !files
                .keys()
                .any(|p| p.contains(".agents") || p.ends_with(".js") || p.ends_with(".ps1"))
        );
        let manifest: serde_json::Value =
            serde_json::from_str(&files[".claude-plugin/marketplace.json"]).unwrap();
        assert_eq!(
            manifest["plugins"][0]["source"],
            "./plugins/humanymous-product"
        );
    }
    #[test]
    fn invalid_product_cannot_be_packaged() {
        let bundle = Bundle {
            entry: "no metadata".into(),
            resources: BTreeMap::new(),
        };
        assert!(plan_product(&bundle, "license").is_err());
    }
}
