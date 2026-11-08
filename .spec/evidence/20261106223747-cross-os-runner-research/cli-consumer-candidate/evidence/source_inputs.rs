//! Required manifest and compiled-data custody shared by both inventory modes.
use super::{BTreeMap, Path, digest, fs, io};

const REQUIRED_INPUTS: [&str; 11] = [
    "crates/sdk/humanymous-core/Cargo.toml",
    "crates/sdk/humanymous-core/src/signals/wire/unicode.json",
    "crates/sdk/humanymous-core/src/signals/observation/registry.json",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "crates/sdk/humanymous-runtime/Cargo.toml",
    "crates/integrations/humanymous-platform/Cargo.toml",
    "crates/apps/humanymous-cli/Cargo.toml",
    "crates/apps/humanymous-cli/src/bin/native_runner/main.rs",
    "crates/apps/humanymous-cli/src/qualification/native_runner/mod.rs",
];

pub(super) fn require_coverage(
    files: &serde_json::Map<String, serde_json::Value>,
) -> io::Result<()> {
    if REQUIRED_INPUTS
        .iter()
        .any(|path| !files.contains_key(*path))
    {
        return Err(io::Error::other(
            "required current source input missing from snapshot",
        ));
    }
    Ok(())
}

fn insert(root: &Path, path: &Path, files: &mut BTreeMap<String, String>) -> io::Result<()> {
    let key = path
        .strip_prefix(root)
        .map_err(io::Error::other)?
        .to_string_lossy()
        .replace('\\', "/");
    files.insert(key, digest(path)?);
    Ok(())
}

fn manifests(
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<String, String>,
) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(io::Error::other("linked manifest inventory entry"));
        }
        if kind.is_dir() {
            if !["target", ".git", ".spec.local"]
                .iter()
                .any(|name| entry.file_name() == *name)
            {
                manifests(root, &entry.path(), files)?;
            }
        } else if entry.file_name() == "Cargo.toml" {
            insert(root, &entry.path(), files)?;
        }
    }
    Ok(())
}

pub(super) fn collect(root: &Path, files: &mut BTreeMap<String, String>) -> io::Result<()> {
    manifests(root, &root.join("crates"), files)?;
    for relative in REQUIRED_INPUTS {
        let path = root.join(relative);
        // Do not follow a linked required input or silently omit a missing file.
        if !fs::symlink_metadata(&path)?.file_type().is_file() {
            return Err(io::Error::other(
                "required compiled input is not a regular file",
            ));
        }
        insert(root, &path, files)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(root: &Path) {
        for relative in REQUIRED_INPUTS.into_iter().chain([
            "crates/other/Cargo.toml",
            "crates/other/target/ignored/Cargo.toml",
            "crates/other/.spec.local/ignored/Cargo.toml",
        ]) {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"original").unwrap();
        }
    }

    #[test]
    fn nested_manifest_and_each_compiled_input_mutation_changes_inventory() {
        let directory = crate::native_runner::fixture::Directory::new().unwrap();
        let root = directory.path();
        fixture(root);
        let mut original = BTreeMap::new();
        collect(root, &mut original).unwrap();
        assert_eq!(original.len(), REQUIRED_INPUTS.len() + 1);
        // Same verifier with an explicit fixture root; never change process cwd.
        let snapshot = serde_json::json!({"files": original});
        super::super::verify_snapshot_at(&snapshot, root).unwrap();
        assert!(
            !original
                .keys()
                .any(|key| key.contains("target/") || key.contains(".spec.local/"))
        );
        for relative in REQUIRED_INPUTS {
            fs::write(root.join(relative), b"mutated").unwrap();
            let mut changed = BTreeMap::new();
            collect(root, &mut changed).unwrap();
            assert_ne!(original[relative], changed[relative]);
            assert_ne!(original, changed);
            assert!(super::super::verify_snapshot_at(&snapshot, root).is_err());
            fs::write(root.join(relative), b"original").unwrap();
            super::super::verify_snapshot_at(&snapshot, root).unwrap();
        }
    }

    #[test]
    fn each_missing_required_input_refuses_inventory() {
        let directory = crate::native_runner::fixture::Directory::new().unwrap();
        fixture(directory.path());
        for relative in REQUIRED_INPUTS {
            fs::remove_file(directory.path().join(relative)).unwrap();
            assert!(collect(directory.path(), &mut BTreeMap::new()).is_err());
            fs::write(directory.path().join(relative), b"original").unwrap();
        }
    }

    #[test]
    fn each_omitted_required_snapshot_entry_refuses_verification() {
        let directory = crate::native_runner::fixture::Directory::new().unwrap();
        fixture(directory.path());
        let mut files = BTreeMap::new();
        collect(directory.path(), &mut files).unwrap();
        for relative in REQUIRED_INPUTS {
            let mut snapshot = serde_json::json!({"files": files});
            snapshot["files"].as_object_mut().unwrap().remove(relative);
            assert!(super::super::verify_snapshot_at(&snapshot, directory.path()).is_err());
        }
    }
}
