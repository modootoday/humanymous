use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};
mod bundles;
pub(super) use bundles::fresh_bundle;

fn snapshot_directory(
    directory: &Path,
    files: &mut std::collections::BTreeMap<String, String>,
) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(io::Error::other("linked source input rejected"));
        }
        if kind.is_dir() {
            snapshot_directory(&entry.path(), files)?;
        } else if kind.is_file() {
            let path = entry.path();
            files.insert(path.to_string_lossy().replace('\\', "/"), digest(&path)?);
        }
    }
    Ok(())
}

fn metadata() -> io::Result<Value> {
    let result = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--locked"])
        .output()?;
    if !result.status.success() {
        return Err(io::Error::other("root locked metadata failed"));
    }
    Ok(serde_json::from_slice(&result.stdout)?)
}
fn cef_manifest(metadata: &Value) -> io::Result<PathBuf> {
    let cef = metadata["packages"]
        .as_array()
        .and_then(|packages| packages.iter().find(|p| p["name"] == "cef"))
        .ok_or_else(|| io::Error::other("pinned CEF dependency absent"))?;
    if cef["version"] != "156.3.0+156.0.15"
        || cef["source"]
            != "git+https://github.com/tauri-apps/cef-rs.git?rev=224a785c7b41758b51821cbb7395ea57ba493a63#224a785c7b41758b51821cbb7395ea57ba493a63"
    {
        return Err(io::Error::other("unexpected CEF revision/version"));
    }
    let manifest = Path::new(
        cef["manifest_path"]
            .as_str()
            .ok_or_else(|| io::Error::other("CEF manifest path absent"))?,
    );
    Ok(manifest
        .parent()
        .ok_or_else(|| io::Error::other("CEF directory absent"))?
        .join("src/build_util/win/cef-app.exe.manifest"))
}
pub(super) fn digest(path: &Path) -> io::Result<String> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}
pub(super) fn source_snapshot() -> io::Result<Value> {
    let head = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .output()?;
    if !head.status.success() || !status.status.success() {
        return Err(io::Error::other("source revision observation failed"));
    }
    let sha = String::from_utf8(head.stdout)
        .map_err(io::Error::other)?
        .trim()
        .to_owned();
    let dirty = !status.stdout.is_empty();
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
        && (dirty || std::env::var("GITHUB_SHA").as_deref() != Ok(sha.as_str()))
    {
        return Err(io::Error::other(
            "remote native pilot requires clean exact workflow source",
        ));
    }
    let mut files = std::collections::BTreeMap::new();
    for file in [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "scripts/ci/cef-pilot/Cargo.toml",
        "scripts/ci/cef-pilot/Cargo.lock",
        "crates/browser/native/bootstrap/Cargo.toml",
        "crates/browser/native/desktop/Cargo.toml",
        "crates/browser/native/desktop/pilot-contract.json",
        "crates/browser/Cargo.toml",
        "crates/test-support/Cargo.toml",
    ] {
        files.insert(file.to_owned(), digest(Path::new(file))?);
    }
    for directory in [
        "crates/browser/native/bootstrap/src",
        "crates/browser/native/desktop/src",
        "crates/browser/native/shared",
        "crates/browser/src",
        "crates/test-support/src",
        "scripts/ci/cef-pilot/src",
    ] {
        snapshot_directory(Path::new(directory), &mut files)?;
    }
    Ok(serde_json::json!({"commit":sha,"dirty":dirty,"files":files}))
}
fn copy_verified(source: &Path, destination: &Path) -> io::Result<()> {
    if fs::symlink_metadata(source)?.file_type().is_symlink() {
        return Err(io::Error::other("linked source rejected"));
    }
    if destination.try_exists()? {
        if fs::symlink_metadata(destination)?.file_type().is_symlink()
            || digest(source)? != digest(destination)?
        {
            return Err(io::Error::other(
                "existing pilot artifact differs or is linked",
            ));
        }
    } else {
        let mut input = fs::File::open(source)?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?;
        io::copy(&mut input, &mut output)?;
        output.sync_all()?;
    }
    Ok(())
}
pub(super) fn prepare() -> io::Result<PathBuf> {
    let meta = metadata()?;
    let target = Path::new(
        meta["target_directory"]
            .as_str()
            .ok_or_else(|| io::Error::other("Cargo target absent"))?,
    )
    .canonicalize()?;
    let bundle = target.join("debug");
    if fs::symlink_metadata(&bundle)?.file_type().is_symlink()
        || bundle.canonicalize()?.parent() != Some(target.as_path())
    {
        return Err(io::Error::other("pilot directory escapes Cargo target"));
    }
    let archive: Value = serde_json::from_slice(&fs::read(bundle.join("archive.json"))?)?;
    if archive["name"]
        != "cef_binary_156.0.15+gcf671c0+chromium-156.0.8078.80_windows64_minimal.tar.bz2"
        || archive["sha1"] != "b690e035689ee20ba9d2500ac8953815bc7ed46a"
        || archive["type"] != "minimal"
    {
        return Err(io::Error::other("unexpected upstream CEF archive metadata"));
    }
    for name in [
        "humanymous_cef_bootstrap.dll",
        "humanymous_cef_desktop.dll",
        "libcef.dll",
    ] {
        if !bundle.join(name).is_file() {
            return Err(io::Error::other(format!("native artifact absent: {name}")));
        }
    }
    copy_verified(
        &bundle.join("bootstrapc.exe"),
        &bundle.join("humanymous_cef_bootstrap.exe"),
    )?;
    copy_verified(
        &cef_manifest(&meta)?,
        &bundle.join("humanymous_cef_bootstrap.exe.manifest"),
    )?;
    Ok(bundle)
}
fn tool_directories() -> io::Result<Vec<PathBuf>> {
    let program_files = std::env::var_os("ProgramFiles(x86)")
        .ok_or_else(|| io::Error::other("Visual Studio environment absent"))?;
    let locator = Path::new(&program_files).join("Microsoft Visual Studio/Installer/vswhere.exe");
    let result = Command::new(locator)
        .args(["-latest", "-products", "*", "-property", "installationPath"])
        .output()?;
    let root = String::from_utf8(result.stdout).map_err(io::Error::other)?;
    if !result.status.success() || root.trim().is_empty() {
        return Err(io::Error::other("Visual Studio installation absent"));
    }
    let base = Path::new(root.trim()).join("Common7/IDE/CommonExtensions/Microsoft/CMake");
    let paths = vec![base.join("CMake/bin"), base.join("Ninja")];
    if paths.iter().any(|p| !p.is_dir()) {
        return Err(io::Error::other("Visual Studio CMake/Ninja absent"));
    }
    Ok(paths)
}
pub(super) fn tool_path() -> io::Result<OsString> {
    let current = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = tool_directories()?;
    paths.extend(std::env::split_paths(&current));
    std::env::join_paths(paths).map_err(io::Error::other)
}

pub(super) fn configure_github_path() -> io::Result<()> {
    use std::io::Write;
    let path = std::env::var_os("GITHUB_PATH")
        .ok_or_else(|| io::Error::other("GitHub Actions GITHUB_PATH is required"))?;
    let mut output = fs::OpenOptions::new().append(true).open(path)?;
    for directory in tool_directories()? {
        let text = directory
            .to_str()
            .ok_or_else(|| io::Error::other("tool path is not Unicode"))?;
        if text.contains(['\r', '\n']) {
            return Err(io::Error::other("invalid tool path"));
        }
        writeln!(output, "{text}")?;
    }
    output.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_requires_exact_native_dependency_revision_and_version() {
        let mut m = serde_json::json!({"packages":[{"name":"cef","version":"156.3.0+156.0.15","source":"git+https://github.com/tauri-apps/cef-rs.git?rev=224a785c7b41758b51821cbb7395ea57ba493a63#224a785c7b41758b51821cbb7395ea57ba493a63","manifest_path":"cef/Cargo.toml"}]});
        assert!(cef_manifest(&m).is_ok());
        m["packages"][0]["source"] = "git+https://example.invalid#other".into();
        assert!(cef_manifest(&m).is_err());
        assert!(cef_manifest(&serde_json::json!({})).is_err());
    }

    #[test]
    fn provisioning_is_idempotent_but_never_overwrites_mismatched_artifacts() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "humanymous-cef-copy-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&dir).unwrap();
        let source = dir.join("source");
        let destination = dir.join("destination");
        fs::write(&source, b"pinned artifact").unwrap();
        copy_verified(&source, &destination).unwrap();
        copy_verified(&source, &destination).unwrap();
        fs::write(&destination, b"different artifact").unwrap();
        assert!(copy_verified(&source, &destination).is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"different artifact");
        assert_eq!(fs::read(&source).unwrap(), b"pinned artifact");
        fs::remove_file(source).unwrap();
        fs::remove_file(destination).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
