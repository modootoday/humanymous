//! Read-only source provenance for the bounded service pilots, never qualification.
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn visit(base: &Path, path: &Path, rows: &mut Vec<serde_json::Value>) -> std::io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(std::io::Error::other("source link rejected"));
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if matches!(
                entry.file_name().to_str(),
                Some("target" | ".git" | "node_modules")
            ) {
                continue;
            }
            visit(base, &entry.path(), rows)?;
        }
    } else {
        let bytes = fs::read(path)?;
        rows.push(serde_json::json!({"path":path.strip_prefix(base).map_err(std::io::Error::other)?.to_string_lossy().replace('\\',"/"),"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(&bytes)),"lf_only":!bytes.contains(&b'\r')}));
    }
    Ok(())
}

fn dependency_sources(
    root: &Path,
) -> Result<(Vec<PathBuf>, serde_json::Value), Box<dyn std::error::Error>> {
    let version = Command::new("rustc").arg("-vV").output()?;
    if !version.status.success() {
        return Err("compiler host metadata unavailable".into());
    }
    let version = String::from_utf8(version.stdout)?;
    let host = version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or("compiler host triple")?;
    let metadata = Command::new("cargo")
        .args([
            "metadata",
            "--locked",
            "--offline",
            "--format-version=1",
            "--features",
            "humanymous-cli/runtime-service-pilot,humanymous-cli/service-pilot",
            "--filter-platform",
            host,
        ])
        .current_dir(root)
        .output()?;
    if !metadata.status.success() || metadata.stdout.len() > 32 * 1024 * 1024 {
        return Err("bounded locked dependency metadata unavailable".into());
    }
    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout)?;
    let packages = metadata["packages"].as_array().ok_or("metadata packages")?;
    let start = packages
        .iter()
        .find(|package| package["name"] == "humanymous-cli")
        .and_then(|package| package["id"].as_str())
        .ok_or("CLI package identity")?;
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("metadata resolved graph")?;
    let lookup: BTreeMap<_, _> = nodes
        .iter()
        .filter_map(|node| node["id"].as_str().map(|id| (id, node)))
        .collect();
    let mut pending = vec![start];
    let mut seen = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id) {
            continue;
        }
        let node = lookup.get(id).ok_or("resolved dependency node missing")?;
        for dependency in node["dependencies"]
            .as_array()
            .ok_or("resolved dependency edges")?
        {
            pending.push(dependency.as_str().ok_or("dependency identity")?);
        }
    }
    let mut roots = BTreeSet::new();
    let mut graph = Vec::new();
    for package in packages
        .iter()
        .filter(|package| package["id"].as_str().is_some_and(|id| seen.contains(id)))
    {
        let id = package["id"].as_str().ok_or("package identity")?;
        let node = lookup.get(id).ok_or("package graph node")?;
        graph.push(serde_json::json!({"id":id,"name":package["name"],"version":package["version"],"source":package["source"],"dependencies":node["dependencies"],"features":node["features"]}));
        if package["source"].is_null() {
            let manifest = PathBuf::from(
                package["manifest_path"]
                    .as_str()
                    .ok_or("local manifest path")?,
            );
            if !manifest.starts_with(root) {
                return Err("local dependency escapes source root".into());
            }
            roots.insert(manifest.parent().ok_or("local package root")?.to_owned());
        }
    }
    graph.sort_by_key(|row| row["id"].as_str().unwrap_or("").to_owned());
    Ok((
        roots.into_iter().collect(),
        serde_json::json!({"requested_features":["humanymous-cli/runtime-service-pilot","humanymous-cli/service-pilot"],"filter_platform":host,"compiler_verbose_version":version,"metadata_feature_superset":true,"registry_inputs_pinned_by_lock":true,"nodes":graph}),
    ))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = crate_root
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root")?;
    let mut rows = Vec::new();
    let (packages, dependency_graph) = dependency_sources(root)?;
    for package in packages {
        visit(root, &package, &mut rows)?;
    }
    for relative in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo"] {
        let path = root.join(relative);
        if path.exists() {
            visit(root, &path, &mut rows)?;
        }
    }
    rows.sort_by_key(|r| r["path"].as_str().unwrap_or("").to_string());
    rows.dedup_by(|left, right| left["path"] == right["path"]);
    let encoded = serde_json::to_vec(&rows)?;
    let source_hash = format!("{:x}", Sha256::digest(&encoded));
    let git = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()?;
    let head = if git.status.success() {
        String::from_utf8(git.stdout)?.trim().to_owned()
    } else {
        "unavailable".into()
    };
    let mut canonical = Vec::new();
    for row in &rows {
        let path = row["path"].as_str().ok_or("source path")?;
        let output = Command::new("git")
            .args(["show", &format!("{head}:{path}")])
            .current_dir(root)
            .output()?;
        canonical.push(serde_json::json!({"path":path,"present":output.status.success(),"matches_working":output.status.success()&&format!("{:x}",Sha256::digest(&output.stdout))==row["sha256"].as_str().unwrap_or(""),"sha256":if output.status.success(){Some(format!("{:x}",Sha256::digest(&output.stdout)))}else{None}}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"schema":"humanymous.cli-service-source.v2","source_kind":"observed-working-source","head":head,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"rust_expected":"1.99.1","working_manifest_sha256":source_hash,"files":rows,"dependency_graph":dependency_graph,"excluded_build_outputs":["target",".git","node_modules"],"canonical_git_files":canonical,"product_qualified":false})
        )?
    );
    Ok(())
}
