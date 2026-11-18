//! Cargo IDs are opaque; only same-source workspace metadata assigns roles.
use crate::evidence;
use humanymous_platform::testing::process::capture_with_stderr;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

pub(crate) struct Packages {
    root: PathBuf,
    entries: BTreeMap<String, Value>,
    source_sha: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    PlatformLibrary,
    RuntimeLibrary,
    GatewayNative,
    AuthorityNative,
    AuditDurableWitness,
    AuditKeyBackups,
    AuditKeyCustody,
    CliService,
    CliRuntimeService,
    GatewayBinary,
    AuthorityBinary,
    CliBinary,
}
impl Packages {
    /// Runs `cargo metadata` and keeps its private output beside `base`.
    /// The provisioning parent calls this before launching the standard-user
    /// actor, whose PATH holds only System32 and therefore no Cargo.
    pub(crate) fn capture_metadata(base: &Path) -> io::Result<Vec<u8>> {
        let output = capture_with_stderr(
            Command::new("cargo").args([
                "metadata",
                "--locked",
                "--offline",
                "--no-deps",
                "--format-version",
                "1",
            ]),
            b"",
            Duration::from_secs(30),
            16 * 1024 * 1024,
        )?;
        std::fs::write(base.with_extension("package-metadata.json"), &output.stdout)?;
        std::fs::write(
            base.with_extension("package-metadata.stderr.log"),
            &output.stderr,
        )?;
        if !output.status.success() {
            return Err(io::Error::other("workspace package metadata failed"));
        }
        Ok(output.stdout)
    }
    pub(crate) fn load(source: &Value, base: &Path, captured: Option<&Path>) -> io::Result<Self> {
        evidence::verify_snapshot(source)?;
        // Keep Cargo's logical absolute spelling; Windows canonicalize adds a
        // verbatim prefix that metadata workspace_root does not necessarily use.
        let root = logical(std::env::current_dir()?);
        let stdout = match captured {
            Some(path) => std::fs::read(path)?,
            None => Self::capture_metadata(base)?,
        };
        let metadata: Value =
            humanymous_platform::testing::json::decode_strict_json(&stdout, 16 * 1024 * 1024)
                .map_err(io::Error::other)?;
        let mut packages = Self::from_metadata(&metadata, root)?;
        packages.source_sha = source["source_sha"].as_str().map(str::to_owned);
        let files = source["files"]
            .as_object()
            .ok_or_else(|| io::Error::other("source files absent"))?;
        for package in packages.entries.values() {
            if !matches!(
                package["name"].as_str(),
                Some("humanymous-platform" | "humanymous-runtime" | "humanymous-cli")
            ) {
                continue;
            }
            let manifest = Path::new(package["manifest_path"].as_str().unwrap());
            let relative = manifest
                .strip_prefix(&packages.root)
                .map_err(io::Error::other)?
                .to_string_lossy()
                .replace('\\', "/");
            if files.get(&relative).and_then(Value::as_str)
                != Some(evidence::digest(manifest)?.as_str())
            {
                return Err(io::Error::other(
                    "metadata manifest outside observed source custody",
                ));
            }
        }
        evidence::verify_snapshot(source)?;
        Ok(packages)
    }

    fn from_metadata(metadata: &Value, root: PathBuf) -> io::Result<Self> {
        if metadata["workspace_root"].as_str().map(Path::new) != Some(root.as_path()) {
            return Err(io::Error::other("metadata workspace root mismatch"));
        }
        let members = metadata["workspace_members"]
            .as_array()
            .ok_or_else(|| io::Error::other("workspace member IDs absent"))?;
        let records = metadata["packages"]
            .as_array()
            .ok_or_else(|| io::Error::other("package records absent"))?;
        if members.is_empty() {
            return Err(io::Error::other("empty workspace package map"));
        }
        let mut entries = BTreeMap::new();
        for member in members {
            let id = member
                .as_str()
                .filter(|id| !id.is_empty())
                .ok_or_else(|| io::Error::other("invalid workspace member ID"))?;
            let found: Vec<_> = records
                .iter()
                .filter(|package| package["id"] == *member)
                .collect();
            if found.len() != 1 || entries.contains_key(id) {
                return Err(io::Error::other("ambiguous workspace package ID"));
            }
            let package = found[0];
            if package.get("source") != Some(&Value::Null)
                || package["name"].as_str().is_none_or(str::is_empty)
                || package["version"].as_str().is_none_or(str::is_empty)
                || package["manifest_path"]
                    .as_str()
                    .map(Path::new)
                    .is_none_or(|path| !path.is_absolute() || path.strip_prefix(&root).is_err())
                || !package["targets"].is_array()
            {
                return Err(io::Error::other("invalid local workspace package record"));
            }
            entries.insert(id.to_owned(), package.clone());
        }
        Ok(Self {
            root,
            entries,
            source_sha: None,
        })
    }
}

mod roles;
#[cfg(test)]
mod tests;

impl Packages {
    /// Cargo package name for a workspace manifest entry, if it is a member.
    pub(crate) fn name(&self, entry: &Value) -> Option<&str> {
        entry["package_id"]
            .as_str()
            .and_then(|id| self.entries.get(id))
            .and_then(|package| package["name"].as_str())
    }
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
    /// Observed source revision; absent for metadata-only fixtures.
    pub(crate) fn source_sha(&self) -> Option<&str> {
        self.source_sha.as_deref()
    }
}

/// Drops a `\\?\` verbatim prefix in front of a drive letter, which a child
/// launched in a canonicalized directory can observe as its current directory.
fn logical(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|text| text.strip_prefix(r"\\?\")) {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path,
    }
}
