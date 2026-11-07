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
}
impl Packages {
    pub(crate) fn load(source: &Value, base: &Path) -> io::Result<Self> {
        evidence::verify_snapshot(source)?;
        // Keep Cargo's logical absolute spelling; Windows canonicalize adds a
        // verbatim prefix that metadata workspace_root does not necessarily use.
        let root = std::env::current_dir()?;
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
        let metadata: Value = humanymous_platform::testing::json::decode_strict_json(
            &output.stdout,
            16 * 1024 * 1024,
        )
        .map_err(io::Error::other)?;
        let packages = Self::from_metadata(&metadata, root)?;
        let files = source["files"]
            .as_object()
            .ok_or_else(|| io::Error::other("source files absent"))?;
        for package in packages.entries.values() {
            if !matches!(
                package["name"].as_str(),
                Some(
                    "humanymous-platform"
                        | "humanymous-audit"
                        | "humanymous-authority"
                        | "humanymous-runtime"
                        | "humanymous-cli"
                )
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
        Ok(Self { root, entries })
    }

    pub(crate) fn matches(&self, entry: &Value, name: &str) -> bool {
        let manifest = match name {
            "humanymous-platform" => "crates/integrations/humanymous-platform/Cargo.toml",
            "humanymous-audit" => "crates/audit/Cargo.toml",
            "humanymous-authority" => "crates/authority/Cargo.toml",
            "humanymous-runtime" => "crates/runtime/Cargo.toml",
            "humanymous-cli" => "crates/cli/Cargo.toml",
            _ => return false,
        };
        let Some(package) = entry["package_id"]
            .as_str()
            .and_then(|id| self.entries.get(id))
        else {
            return false;
        };
        package["name"] == name
            && package["manifest_path"].as_str().map(Path::new)
                == Some(self.root.join(manifest).as_path())
            && package["targets"].as_array().is_some_and(|targets| {
                targets.iter().any(|target| {
                    target["name"] == entry["target"] && target["kind"] == entry["kind"]
                })
            })
    }

    #[cfg(test)]
    pub(crate) fn fixture() -> Self {
        let root = std::env::current_dir().unwrap();
        let mut records = Vec::new();
        for (name, relative, target, kind) in [
            (
                "humanymous-platform",
                "crates/integrations/humanymous-platform",
                "humanymous_platform",
                "lib",
            ),
            (
                "humanymous-audit",
                "crates/audit",
                "humanymous_audit",
                "lib",
            ),
            (
                "humanymous-authority",
                "crates/authority",
                "native_process",
                "test",
            ),
            ("humanymous-cli", "crates/cli", "service_process", "test"),
        ] {
            let id = if name == "humanymous-platform" {
                "path+file:///C:/workspaces/humanymous/crates/integrations/humanymous-platform#0.1.0".to_owned()
            } else {
                format!("opaque-{name}")
            };
            let mut targets = vec![serde_json::json!({"name":target,"kind":[kind]})];
            if name == "humanymous-audit" {
                for target in ["durable_witness", "key_backups", "key_custody"] {
                    targets.push(serde_json::json!({"name":target,"kind":["test"]}));
                }
            }
            if name == "humanymous-cli" {
                targets.push(serde_json::json!({"name":"runtime_service_process","kind":["test"]}));
            }
            records.push(serde_json::json!({"id":id,"name":name,"version":"0.1.0","manifest_path":root.join(relative).join("Cargo.toml"),"source":null,"targets":targets}));
        }
        let members: Vec<_> = records.iter().map(|record| record["id"].clone()).collect();
        Self::from_metadata(&serde_json::json!({"workspace_root":root,"workspace_members":members,"packages":records}), root).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn short_id_requires_exact_member_name_manifest_and_target_join() {
        let packages = Packages::fixture();
        let entry = json!({"package_id":"path+file:///C:/workspaces/humanymous/crates/integrations/humanymous-platform#0.1.0","target":"humanymous_platform","kind":["lib"]});
        assert!(packages.matches(&entry, "humanymous-platform"));
        for (field, value) in [
            ("package_id", json!("unknown#humanymous-platform@0.1.0")),
            ("package_id", json!("opaque-humanymous-audit")),
            ("target", json!("wrong")),
            ("kind", json!(["bin"])),
        ] {
            let mut wrong = entry.clone();
            wrong[field] = value;
            assert!(!packages.matches(&wrong, "humanymous-platform"));
        }
        let id = entry["package_id"].as_str().unwrap();
        for (field, value) in [
            ("name", json!("humanymous-platform-counterfeit")),
            (
                "manifest_path",
                json!(packages.root.join("counterfeit/Cargo.toml")),
            ),
        ] {
            let mut wrong = Packages::fixture();
            wrong.entries.get_mut(id).unwrap()[field] = value;
            assert!(!wrong.matches(&entry, "humanymous-platform"));
        }
        let mut nonmember = Packages::fixture();
        nonmember.entries.remove(id);
        assert!(!nonmember.matches(&entry, "humanymous-platform"));
    }

    #[test]
    fn metadata_nonmember_duplicate_registry_and_missing_version_cannot_assign_role() {
        let packages = Packages::fixture();
        let records: Vec<_> = packages.entries.values().cloned().collect();
        let members: Vec<_> = packages.entries.keys().cloned().collect();
        let metadata =
            json!({"workspace_root":packages.root,"workspace_members":members,"packages":records});
        let id =
            "path+file:///C:/workspaces/humanymous/crates/integrations/humanymous-platform#0.1.0";
        let entry = json!({"package_id":id,"target":"humanymous_platform","kind":["lib"]});
        let mut nonmember = metadata.clone();
        nonmember["workspace_members"]
            .as_array_mut()
            .unwrap()
            .retain(|member| member != id);
        assert!(
            !Packages::from_metadata(&nonmember, packages.root.clone())
                .unwrap()
                .matches(&entry, "humanymous-platform")
        );
        let index = records
            .iter()
            .position(|record| record["id"] == id)
            .unwrap();
        for (field, value) in [
            ("source", json!("registry+counterfeit")),
            ("version", json!("")),
            ("manifest_path", json!("relative/Cargo.toml")),
        ] {
            let mut wrong = metadata.clone();
            wrong["packages"][index][field] = value;
            assert!(Packages::from_metadata(&wrong, packages.root.clone()).is_err());
        }
        let mut duplicate = metadata.clone();
        duplicate["packages"]
            .as_array_mut()
            .unwrap()
            .push(records[index].clone());
        assert!(Packages::from_metadata(&duplicate, packages.root.clone()).is_err());
        let mut unknown = metadata;
        unknown["workspace_members"]
            .as_array_mut()
            .unwrap()
            .push(json!("absent opaque ID"));
        assert!(Packages::from_metadata(&unknown, packages.root).is_err());
    }
}
