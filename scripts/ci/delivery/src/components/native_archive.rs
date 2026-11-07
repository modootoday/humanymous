//! Actual release artifacts, bound to the owning Cargo receipt and source window.
use crate::{Result, require, source, write_json};
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::Read, path::Path};

fn records(bytes: &[u8], metadata: &Value, root: &Path) -> Result<BTreeMap<String, bool>> {
    let manifest: toml::Value = toml::from_str(
        &std::fs::read_to_string(root.join("Cargo.toml")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let version = manifest["workspace"]["package"]["version"]
        .as_str()
        .ok_or("workspace version absent")?;
    require(
        metadata["version"] == 1
            && metadata["workspace_root"] == json!(root)
            && metadata["target_directory"] == json!(root.join("target")),
        "native Cargo metadata workspace/target mismatch",
    )?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or("native Cargo metadata packages absent")?;
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("native Cargo metadata members absent")?;
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
        let row = humanymous_platform::testing::json::decode_strict_json(line, 1024 * 1024)
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
            let id = row["package_id"]
                .as_str()
                .filter(|id| !id.is_empty())
                .ok_or("native Cargo artifact package identity absent")?;
            let joined: Vec<_> = packages.iter().filter(|item| item["id"] == id).collect();
            require(
                joined.len() == 1
                    && members
                        .iter()
                        .filter(|item| item.as_str() == Some(id))
                        .count()
                        == 1,
                "native Cargo package join missing or ambiguous",
            )?;
            let joined = joined[0];
            let targets: Vec<_> = joined["targets"]
                .as_array()
                .ok_or("native Cargo package targets absent")?
                .iter()
                .filter(|target| target["name"] == name)
                .collect();
            require(targets.len() == 1, "native Cargo target join ambiguous")?;
            let target = targets[0];
            require(
                joined["name"] == format!("humanymous-{package}")
                    && joined["version"] == version
                    && joined.get("source") == Some(&Value::Null)
                    && joined["manifest_path"] == json!(manifest)
                    && target["kind"] == json!(["bin"])
                    && target["crate_types"] == json!(["bin"])
                    && target["edition"] == "2024"
                    && target["src_path"] == json!(entry)
                    && row["target"]["kind"] == json!(["bin"])
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
        "native-release-metadata",
        "cargo",
        &[
            "metadata",
            "--offline",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
        ],
        "30s",
    )?;
    let metadata_path = dir.join("native-release-metadata.log");
    let mut metadata_bytes = Vec::new();
    std::fs::File::open(&metadata_path)
        .map_err(|e| e.to_string())?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut metadata_bytes)
        .map_err(|e| e.to_string())?;
    let metadata =
        humanymous_platform::testing::json::decode_strict_json(&metadata_bytes, 16 * 1024 * 1024)
            .map_err(|_| "native Cargo metadata invalid or oversized")?;
    require(
        source::snapshot(root)? == *before,
        "native metadata source changed",
    )?;
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
    let metadata_sha256 = source::sha(&metadata_bytes);
    require(
        source::file(&metadata_path)? == metadata_sha256,
        "native Cargo metadata receipt changed",
    )?;
    let path = dir.join("native-release-build.log");
    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .map_err(|e| e.to_string())?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let parsed = records(&bytes, &metadata, root)?;
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
        "cargo_metadata_sha256":metadata_sha256,
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
    fn fixture(root: &Path) -> (Vec<Value>, Value) {
        let mut rows = Vec::new();
        let mut packages = Vec::new();
        for (name, package) in [("humanymous", "cli"), ("humanymous-runtime", "runtime")] {
            rows.push(json!({"reason":"compiler-artifact","package_id":format!("path+file:///fixture#humanymous-{package}@0.1.0"),"manifest_path":root.join("crates").join(package).join("Cargo.toml"),
                "target":{"name":name,"kind":["bin"],"crate_types":["bin"],"edition":"2024","src_path":root.join("crates").join(package).join("src/main.rs")},
                "executable":root.join("target/release").join(name),"profile":{"opt_level":"3","debug_assertions":false,"test":false},"fresh":false}));
            let artifact = rows.last().unwrap();
            packages.push(json!({"id":artifact["package_id"],"name":format!("humanymous-{package}"),"version":"0.1.0","source":null,"manifest_path":artifact["manifest_path"],"targets":[artifact["target"]]}));
        }
        rows.push(json!({"reason":"build-finished","success":true}));
        let members: Vec<_> = packages
            .iter()
            .map(|package| package["id"].clone())
            .collect();
        (
            rows,
            json!({"version":1,"workspace_root":root,"target_directory":root.join("target"),"packages":packages,"workspace_members":members}),
        )
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
        let (rows, metadata) = fixture(&root);
        assert_eq!(records(&encode(&rows), &metadata, &root).unwrap().len(), 2);
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
            assert!(records(&encode(&changed), &metadata, &root).is_err());
        }
    }
    #[test]
    fn opaque_native_ids_join_exact_metadata_without_suffix_grammar() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        for prefix in [
            "path+file:///fixture/humanymous-cli#0.1.0",
            "opaque-identity",
        ] {
            let (mut rows, mut metadata) = fixture(&root);
            rows[0]["package_id"] = json!(prefix);
            metadata["packages"][0]["id"] = json!(prefix);
            metadata["workspace_members"][0] = json!(prefix);
            assert_eq!(records(&encode(&rows), &metadata, &root).unwrap().len(), 2);
        }
    }
    #[test]
    fn native_metadata_join_refuses_foreign_ambiguous_and_drifted_roles() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let (rows, original) = fixture(&root);
        for mutation in 0..14 {
            let mut metadata = original.clone();
            match mutation {
                0 => metadata["workspace_root"] = json!(root.join("foreign")),
                1 => metadata["target_directory"] = json!(root.join("other-target")),
                2 => {
                    let duplicate = metadata["packages"][0].clone();
                    metadata["packages"].as_array_mut().unwrap().push(duplicate);
                }
                3 => {
                    metadata["workspace_members"]
                        .as_array_mut()
                        .unwrap()
                        .remove(0);
                }
                4 => metadata["packages"][0]["source"] = json!("registry+https://example.invalid"),
                5 => metadata["packages"][0]["name"] = json!("humanymous-unrelated"),
                6 => metadata["packages"][0]["version"] = json!("0.2.0"),
                7 => {
                    metadata["packages"][0]["manifest_path"] =
                        json!(root.join("foreign/Cargo.toml"))
                }
                8 => {
                    let duplicate = metadata["packages"][0]["targets"][0].clone();
                    metadata["packages"][0]["targets"]
                        .as_array_mut()
                        .unwrap()
                        .push(duplicate);
                }
                9 => {
                    metadata["packages"][0]["targets"][0]["src_path"] =
                        json!(root.join("foreign/main.rs"))
                }
                10 => metadata["packages"][0]["id"] = json!("different-opaque-id"),
                11 => {
                    let duplicate = metadata["workspace_members"][0].clone();
                    metadata["workspace_members"]
                        .as_array_mut()
                        .unwrap()
                        .push(duplicate);
                }
                12 => metadata["version"] = json!(2),
                _ => {
                    metadata["packages"][0]
                        .as_object_mut()
                        .unwrap()
                        .remove("source");
                }
            }
            assert!(records(&encode(&rows), &metadata, &root).is_err());
        }
    }
}
