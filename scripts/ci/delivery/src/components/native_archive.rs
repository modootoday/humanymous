//! Actual release artifacts, bound to the owning Cargo receipt and source window.
use crate::{Result, require, source, write_json};
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::Read, path::Path};

fn records(bytes: &[u8], root: &Path) -> Result<BTreeMap<String, bool>> {
    let manifest: toml::Value = toml::from_str(
        &std::fs::read_to_string(root.join("Cargo.toml")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let version = manifest["workspace"]["package"]["version"]
        .as_str()
        .ok_or("workspace version absent")?;
    require(
        bytes.len() <= 64 * 1024 * 1024,
        "native Cargo receipt oversized",
    )?;
    let mut binaries = BTreeMap::new();
    let mut finished = 0;
    let mut last_finished = false;
    for line in bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let row = humanymous_test_support::json::decode_strict_json(line, 1024 * 1024)
            .map_err(|_| "invalid native Cargo JSON receipt")?;
        last_finished = row["reason"] == "build-finished";
        if last_finished {
            finished += 1;
            require(row["success"] == true, "native Cargo build failed")?;
        } else if row["reason"] == "compiler-artifact" && !row["executable"].is_null() {
            let name = row["target"]["name"]
                .as_str()
                .ok_or("binary target name absent")?;
            let package = match name {
                "humanymous" => "cli",
                "humanymous-runtime" => "runtime",
                _ => return Err("unexpected native executable in Cargo receipt".into()),
            };
            let expected = root.join("target/release").join(name);
            let manifest = root.join("crates").join(package).join("Cargo.toml");
            let entry = root.join("crates").join(package).join("src/main.rs");
            require(
                row["target"]["kind"] == json!(["bin"])
                    && row["package_id"].as_str().is_some_and(|id| {
                        (id.starts_with("path+file://") || id.starts_with("file://"))
                            && id.rsplit_once('#').is_some_and(|(_, suffix)| {
                                suffix == format!("humanymous-{package}@{version}")
                            })
                    })
                    && row["target"]["crate_types"] == json!(["bin"])
                    && row["target"]["edition"] == "2024"
                    && row["manifest_path"]
                        .as_str()
                        .is_some_and(|path| Path::new(path) == manifest)
                    && row["target"]["src_path"]
                        .as_str()
                        .is_some_and(|path| Path::new(path) == entry)
                    && row["executable"]
                        .as_str()
                        .is_some_and(|path| Path::new(path) == expected)
                    && row["profile"]["opt_level"] == "3"
                    && row["profile"]["debug_assertions"] == false
                    && row["profile"]["test"] == false,
                "native executable identity/profile/path mismatch",
            )?;
            let fresh = row["fresh"]
                .as_bool()
                .ok_or("native artifact freshness absent")?;
            require(
                binaries.insert(name.to_owned(), fresh).is_none(),
                "native artifact duplicated",
            )?;
        }
    }
    require(
        finished == 1 && last_finished && binaries.len() == 2,
        "native build completion or exact executable inventory absent",
    )?;
    Ok(binaries)
}
pub(super) fn run(root: &Path, dir: &Path, report: &mut Value, before: &Value) -> Result<()> {
    super::invoke(
        root,
        dir,
        report,
        "native-release-build",
        "cargo",
        &[
            "build",
            "--release",
            "--locked",
            "-p",
            "humanymous-cli",
            "-p",
            "humanymous-runtime",
            "--bin",
            "humanymous",
            "--bin",
            "humanymous-runtime",
            "--message-format=json",
            "-j1",
        ],
        "1500s",
    )?;
    require(
        source::snapshot(root)? == *before,
        "native build source changed",
    )?;
    let path = dir.join("native-release-build.log");
    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .map_err(|e| e.to_string())?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let parsed = records(&bytes, root)?;
    let mut bindings = serde_json::Map::new();
    for (name, fresh) in parsed {
        let path = root.join("target/release").join(&name);
        let meta = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        require(
            meta.is_file() && !meta.file_type().is_symlink() && meta.len() > 0,
            "native artifact not a regular nonempty file",
        )?;
        bindings.insert(
            name,
            json!({"sha256":source::file(&path)?,"bytes":meta.len(),"cargo_fresh":fresh}),
        );
    }
    let receipt = json!({"schema":"humanymous.native-build-receipt/1","source_sha":before["source_sha"],
        "source_manifest_sha256":before["manifest_sha256"],"cargo_json_sha256":source::file(&path)?,
        "build_finished":true,"binaries":bindings});
    write_json(&dir.join("native-build-receipt.json"), &receipt)?;
    report["native_build"] = receipt.clone();
    report["stage"] = json!("native-archive");
    report["native_archive"] = crate::package::native_artifacts_bound(root, &receipt)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(root: &Path) -> Vec<Value> {
        let mut rows = Vec::new();
        for (name, package) in [("humanymous", "cli"), ("humanymous-runtime", "runtime")] {
            rows.push(json!({"reason":"compiler-artifact","package_id":format!("path+file:///fixture#humanymous-{package}@0.1.0"),"manifest_path":root.join("crates").join(package).join("Cargo.toml"),
                "target":{"name":name,"kind":["bin"],"crate_types":["bin"],"edition":"2024","src_path":root.join("crates").join(package).join("src/main.rs")},
                "executable":root.join("target/release").join(name),"profile":{"opt_level":"3","debug_assertions":false,"test":false},"fresh":false}));
        }
        rows.push(json!({"reason":"build-finished","success":true}));
        rows
    }
    fn encode(rows: &[Value]) -> Vec<u8> {
        rows.iter()
            .map(|row| format!("{row}\n"))
            .collect::<String>()
            .into_bytes()
    }
    #[test]
    fn native_build_requires_exact_paths_release_profile_and_final_success() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let rows = fixture(&root);
        assert_eq!(records(&encode(&rows), &root).unwrap().len(), 2);
        for mutation in 0..7 {
            let mut changed = rows.clone();
            match mutation {
                0 => changed[0]["executable"] = json!(root.join("target/debug/humanymous")),
                1 => changed[0]["profile"]["debug_assertions"] = json!(true),
                2 => {
                    changed.remove(1);
                }
                3 => changed.push(changed[0].clone()),
                4 => changed[2]["success"] = json!(false),
                5 => changed.insert(1, changed[0].clone()),
                _ => {
                    changed[0]["package_id"] =
                        json!("registry+https://example.invalid#unrelated@0.1.0")
                }
            }
            assert!(records(&encode(&changed), &root).is_err());
        }
    }
}
