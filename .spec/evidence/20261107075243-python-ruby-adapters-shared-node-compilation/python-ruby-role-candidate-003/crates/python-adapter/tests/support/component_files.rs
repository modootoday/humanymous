//! Immutable source, dependency and copied-artifact closure helpers.
use humanymous_platform::testing::process::capture_with_stderr;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
type Result<T> = std::result::Result<T, String>;
pub(super) fn harness_path(
    raw: &[u8],
    package_id: &str,
    source: &Path,
    target_root: &Path,
) -> Result<PathBuf> {
    if package_id.is_empty() || !source.is_absolute() || !target_root.is_absolute() {
        return Err("exact package ID and absolute source/target roots required".into());
    }
    let text = std::str::from_utf8(raw).map_err(|_| "Cargo stream is not UTF-8")?;
    let mut paths = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let row: Value = serde_json::from_str(line).map_err(|_| "invalid Cargo JSON stream")?;
        if row["reason"] != "compiler-artifact" || row["target"]["name"] != "wsgi_server" {
            continue;
        }
        let mut features: Vec<&str> = row["features"]
            .as_array()
            .ok_or("artifact features missing")?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| "non-string artifact feature".to_owned())
            })
            .collect::<Result<_>>()?;
        features.sort_unstable();
        if row["package_id"].as_str() != Some(package_id)
            || row["target"]["src_path"].as_str() != source.to_str()
            || row["target"]["kind"] != serde_json::json!(["test"])
            || row["target"]["crate_types"] != serde_json::json!(["bin"])
            || row["profile"]["test"] != true
            || features != ["host-pilot", "native"]
        {
            return Err("Cargo harness role/source/package/features do not match".into());
        }
        let executable = PathBuf::from(
            row["executable"]
                .as_str()
                .ok_or("artifact executable missing")?,
        );
        if !executable.is_absolute()
            || !executable.starts_with(target_root)
            || executable
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("harness outside exact target root".into());
        }
        paths.push(executable);
    }
    if paths.len() != 1 {
        return Err("one exact Cargo WSGI executable required".into());
    }
    Ok(paths.remove(0))
}
pub(super) fn digest(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut block = [0; 65536];
    loop {
        let size = file.read(&mut block).map_err(|e| e.to_string())?;
        if size == 0 {
            break;
        }
        hash.update(&block[..size]);
    }
    Ok(hex::encode(hash.finalize()))
}
pub(super) fn directory_manifest(directory: &Path) -> Result<String> {
    fn visit(base: &Path, path: &Path, rows: &mut Vec<String>) -> Result<()> {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_dir() {
                visit(base, &entry.path(), rows)?;
            } else if kind.is_file() {
                rows.push(format!(
                    "{} {}",
                    entry.path().strip_prefix(base).unwrap().display(),
                    digest(&entry.path())?
                ));
            } else if kind.is_symlink() {
                let path = entry.path();
                let target = fs::read_link(&path).map_err(|e| e.to_string())?;
                let content = if path.is_file() {
                    digest(&path)?
                } else {
                    String::from("directory-link")
                };
                rows.push(format!(
                    "{} symlink:{} {content}",
                    path.strip_prefix(base).unwrap().display(),
                    target.display()
                ));
            } else {
                return Err("unexpected dependency filesystem entry".into());
            }
        }
        Ok(())
    }
    let mut rows = Vec::new();
    visit(directory, directory, &mut rows)?;
    rows.sort();
    Ok(hex::encode(Sha256::digest(rows.join("\n").as_bytes())))
}
pub(super) fn directory_manifest_excluding_venv(bundle: &Path) -> Result<String> {
    let mut rows = Vec::new();
    for name in [
        "module/humanymous_python.so",
        "humanymous-authority",
        "wsgi_server",
    ] {
        rows.push(format!("{name} {}", digest(&bundle.join(name))?));
    }
    Ok(hex::encode(Sha256::digest(rows.join("\n").as_bytes())))
}
pub(super) fn source_manifest(root: &Path) -> Result<String> {
    let mut command = Command::new("git");
    command
        .args(["status", "--porcelain", "--untracked-files=no"])
        .current_dir(root);
    let clean = capture_with_stderr(&mut command, b"", Duration::from_secs(15), 1 << 20)
        .map_err(|_| "source status failed")?;
    if !clean.status.success() || !clean.stdout.is_empty() {
        return Err("tracked source changed".into());
    }
    let mut command = Command::new("git");
    command.args(["ls-files", "-z"]).current_dir(root);
    let files = capture_with_stderr(&mut command, b"", Duration::from_secs(15), 4 << 20)
        .map_err(|_| "source inventory failed")?;
    if !files.status.success() {
        return Err("source inventory failed".into());
    }
    let mut rows = Vec::new();
    for name in files
        .stdout
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
    {
        let name = std::str::from_utf8(name).map_err(|_| "source path encoding")?;
        rows.push(format!("{name} {}", digest(&root.join(name))?));
    }
    rows.sort();
    Ok(hex::encode(Sha256::digest(rows.join("\n").as_bytes())))
}

#[cfg(test)]
mod artifact_tests {
    use super::*;
    #[test]
    fn matching_name_does_not_admit_wrong_package_role_or_source() {
        let root = std::env::current_dir().unwrap();
        let source = root.join("tests/wsgi_server.rs");
        let target = root.join("isolated-target");
        let row = serde_json::json!({"reason":"compiler-artifact","package_id":"exact-source-package-id",
            "target":{"name":"wsgi_server","kind":["test"],"crate_types":["bin"],"src_path":source},
            "profile":{"test":true},"features":["native","host-pilot"],"executable":target.join("debug/wsgi_server")});
        let select = |value: &Value| {
            harness_path(
                value.to_string().as_bytes(),
                "exact-source-package-id",
                &source,
                &target,
            )
        };
        assert!(select(&row).is_ok());
        for (pointer, value) in [
            (
                "/package_id",
                serde_json::json!("same-package-name-other-source"),
            ),
            ("/target/src_path", serde_json::json!(root.join("wrong.rs"))),
            ("/target/kind", serde_json::json!(["bin"])),
            (
                "/features",
                serde_json::json!(["native", "host-pilot", "package-tool"]),
            ),
            ("/profile/test", serde_json::json!(false)),
            (
                "/executable",
                serde_json::json!(root.join("outside-target")),
            ),
        ] {
            let mut wrong = row.clone();
            *wrong.pointer_mut(pointer).unwrap() = value;
            assert!(select(&wrong).is_err(), "{pointer}");
        }
        let raw = format!("{row}\n{row}\n");
        assert!(harness_path(raw.as_bytes(), "exact-source-package-id", &source, &target).is_err());
        assert!(harness_path(b"not-json", "exact-source-package-id", &source, &target).is_err());
        assert!(harness_path(b"", "exact-source-package-id", &source, &target).is_err());
    }
}
