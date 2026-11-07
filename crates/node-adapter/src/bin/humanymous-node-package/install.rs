//! Fresh offline native npm archive installation; no authored JS entrypoint.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) fn execute(archive: &Path) -> Result<(), String> {
    if std::env::var("HUMANYMOUS_HOSTED_NATIVE_PILOT").as_deref() != Ok("1") {
        return Err("native npm installation requires explicit allocated pilot opt-in".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(archive)
        .map_err(|e| e.to_string())?
        .take(100 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() > 100 * 1024 * 1024 {
        return Err("native npm archive is empty or exceeds limit".into());
    }
    let sha256 = hex::encode(Sha256::digest(&bytes));
    let parent = super::root().join("target/node-native/install");
    fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let directory = parent.join(format!("{}-{stamp}", std::process::id()));
    fs::create_dir(&directory).map_err(|e| e.to_string())?;
    // npm consumes this owned copy of exactly the bytes whose hash is retained.
    let copied_archive = directory.join("input.tgz");
    fs::write(&copied_archive, &bytes).map_err(|e| e.to_string())?;
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    super::run(
        npm,
        &[
            "install",
            "--offline",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "--package-lock=false",
            copied_archive.to_str().ok_or("archive path encoding")?,
        ],
        &directory,
    )?;
    let installed = directory.join("node_modules/@humanymous/node");
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(installed.join("package.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if metadata["name"] != "@humanymous/node"
        || metadata["version"] != "0.1.0-experimental.1"
        || metadata["exports"]["."]["require"]["default"] != "./native/index.cjs"
        || metadata["exports"]["."]["import"]["default"] != "./native/index.mjs"
    {
        return Err("installed native npm package identity/exports mismatch".into());
    }
    for runtime in ["node", "bun"] {
        for loader in [
            "./node_modules/@humanymous/node/native/index.cjs",
            "./node_modules/@humanymous/node/native/index.mjs",
        ] {
            super::production_import(runtime, &[loader], &directory)?;
        }
    }
    // Closed stdin supplies no authored program; upstream Node resolves exports.
    super::production_import("node", &["--require", "@humanymous/node"], &directory)?;
    super::production_import("node", &["--import", "@humanymous/node"], &directory)?;
    println!(
        "{}",
        serde_json::json!({"schema":"humanymous.node-clean-install/1","archiveSha256":sha256,
        "status":"fresh-offline-install-and-native-import-succeeded","runtimeLoaderImports":4,"nodePublicExportLoads":2,
        "installationScriptsExecuted":false,"productionFixtureEnabled":false,"actualFrameworkQualified":false,
        "generationProvenanceVerified":false,"wholeProductQualified":false})
    );
    Ok(())
}
