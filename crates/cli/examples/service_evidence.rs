//! Read-only source provenance for the bounded service pilots, never qualification.
use sha2::{Digest, Sha256};
use std::{
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
            visit(base, &entry?.path(), rows)?;
        }
    } else {
        let bytes = fs::read(path)?;
        rows.push(serde_json::json!({"path":path.strip_prefix(base).map_err(std::io::Error::other)?.to_string_lossy().replace('\\',"/"),"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(&bytes)),"lf_only":!bytes.contains(&b'\r')}));
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = crate_root
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root")?;
    let mut rows = Vec::new();
    for relative in [
        "crates/cli/src",
        "crates/cli/tests",
        "crates/cli/examples",
        "crates/cli/Cargo.toml",
        "crates/native-security/src",
        "crates/native-security/Cargo.toml",
        "Cargo.toml",
        "Cargo.lock",
    ] {
        visit(root, &root.join(relative), &mut rows)?;
    }
    rows.sort_by_key(|r| r["path"].as_str().unwrap_or("").to_string());
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
            .args(["show", &format!("HEAD:{path}")])
            .current_dir(root)
            .output()?;
        canonical.push(serde_json::json!({"path":path,"present":output.status.success(),"matches_working":output.status.success()&&format!("{:x}",Sha256::digest(&output.stdout))==row["sha256"].as_str().unwrap_or(""),"sha256":if output.status.success(){Some(format!("{:x}",Sha256::digest(&output.stdout)))}else{None}}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"schema":"humanymous.cli-service-source.v1","source_kind":"validated-working-source","head":head,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"rust":"1.99.1","working_manifest_sha256":source_hash,"files":rows,"canonical_git_files":canonical,"product_qualified":false})
        )?
    );
    Ok(())
}
