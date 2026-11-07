//! Package graph, archive surfaces and dependency closure checks.
use crate::{Result, command, evidence, read_json, require, source, write_json};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub fn metadata(root: &Path) -> Result<Value> {
    serde_json::from_str(&command::text(
        root,
        "cargo",
        &["metadata", "--locked", "--no-deps", "--format-version", "1"],
    )?)
    .map_err(|e| e.to_string())
}
pub fn graph(metadata: &Value) -> Result<Vec<Vec<Value>>> {
    let members: BTreeSet<_> = metadata["workspace_members"]
        .as_array()
        .ok_or("missing members")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let packages: BTreeMap<String, Value> = metadata["packages"]
        .as_array()
        .ok_or("missing packages")?
        .iter()
        .filter(|p| members.contains(p["id"].as_str().unwrap_or("")) && p["publish"] != json!([]))
        .map(|p| (p["name"].as_str().unwrap_or("").into(), p.clone()))
        .collect();
    require(
        !packages.is_empty(),
        "publish=false: no publishable packages; registry readiness remains blocked",
    )?;
    fn visit(
        name: &str,
        packages: &BTreeMap<String, Value>,
        visiting: &mut BTreeSet<String>,
        levels: &mut BTreeMap<String, usize>,
    ) -> Result<usize> {
        if let Some(level) = levels.get(name) {
            return Ok(*level);
        }
        require(
            visiting.insert(name.into()),
            format!("registry dependency cycle: {name}"),
        )?;
        let package = packages
            .get(name)
            .ok_or_else(|| format!("unpublished required path dependency: {name}"))?;
        let mut level = 0;
        for dependency in package["dependencies"]
            .as_array()
            .ok_or("missing dependencies")?
        {
            if dependency.get("path").is_some() {
                require(
                    dependency["req"].as_str().is_some_and(|req| req != "*"),
                    "registry version required beside path dependency",
                )?;
                let dep = dependency["name"]
                    .as_str()
                    .ok_or("missing dependency name")?;
                level = level.max(visit(dep, packages, visiting, levels)? + 1);
            }
        }
        visiting.remove(name);
        levels.insert(name.into(), level);
        Ok(level)
    }
    let mut levels = BTreeMap::new();
    for name in packages.keys() {
        visit(name, &packages, &mut BTreeSet::new(), &mut levels)?;
    }
    let count = levels.values().max().ok_or("empty graph")? + 1;
    require(
        count <= 8,
        "graph exceeds eight reviewed authentication waves; extend workflow before publishing",
    )?;
    let mut waves = vec![Vec::new(); count];
    for (name, level) in levels {
        waves[level].push(packages[&name].clone());
    }
    Ok(waves)
}
pub fn safe_files(files: &[String], rust: bool) -> Result<()> {
    require(!files.is_empty(), "empty package")?;
    for path in files {
        require(
            source::safe_relative(path),
            format!("unsafe package member: {path}"),
        )?;
        let parts: Vec<_> = path.split('/').collect();
        let forbidden = [
            ".spec",
            ".spec.local",
            ".agents",
            ".git",
            ".codex",
            ".claude",
            "node_modules",
            "bots",
            "redteam",
            "attacker",
            "wargame",
            "target",
        ];
        require(
            !parts.iter().any(|part| forbidden.contains(part)),
            format!("private/attacker package member: {path}"),
        )?;
        require(
            rust || !parts
                .iter()
                .any(|part| ["test", "tests", "fixtures"].contains(part)),
            format!("Node test surface leaked: {path}"),
        )?;
        require(
            ![".pem", ".key", ".env"]
                .iter()
                .any(|suffix| path.ends_with(suffix)),
            "credential-like package member",
        )?;
    }
    Ok(())
}
pub fn check(root: &Path) -> Result<()> {
    let before = source::snapshot(root)?;
    let waves = graph(&metadata(root)?)?;
    let names: Vec<String> = waves
        .into_iter()
        .flatten()
        .filter_map(|p| p["name"].as_str().map(str::to_owned))
        .collect();
    for name in &names {
        let files = command::text(
            root,
            "cargo",
            &["package", "--locked", "--list", "-p", name],
        )?;
        safe_files(&files.lines().map(str::to_owned).collect::<Vec<_>>(), true)?;
    }
    let mut args = vec!["package", "--locked"];
    for name in &names {
        args.extend(["-p", name]);
    }
    command::status(root, "cargo", &args)?;
    let after = source::snapshot(root)?;
    require(
        before["manifest_sha256"] == after["manifest_sha256"]
            && before["source_sha"] == after["source_sha"],
        "source changed while verifying Cargo archives",
    )?;
    let data = metadata(root)?;
    let mut packages = Vec::new();
    for package in graph(&data)?.into_iter().flatten() {
        let name = package["name"].as_str().ok_or("crate name missing")?;
        let version = package["version"].as_str().ok_or("crate version missing")?;
        let filename = format!("{name}-{version}.crate");
        packages.push(json!({"name":name,"version":version,"filename":filename,"sha256":source::file(&root.join("target/package").join(&filename))?}));
    }
    write_json(
        &root.join("target/delivery-crates/validated-archives.json"),
        &json!({"schema":"humanymous.crate-archives/1","source_sha":source::head(root)?,"source_manifest_sha256":before["manifest_sha256"],"cargo_lock_sha256":source::file(&root.join("Cargo.lock"))?,"documentation_review_sha256":source::file(&root.join("docs/reviews.json"))?,"packages":packages}),
    )
}
pub fn validated_crate(root: &Path, name: &str, version: &str) -> Result<(PathBuf, String)> {
    let proof = read_json(&root.join("target/delivery-crates/validated-archives.json"))?;
    require(
        proof["schema"] == "humanymous.crate-archives/1"
            && proof["source_sha"] == source::head(root)?
            && proof["cargo_lock_sha256"] == source::file(&root.join("Cargo.lock"))?
            && proof["documentation_review_sha256"]
                == source::file(&root.join("docs/reviews.json"))?,
        "validated Cargo archive authority is absent/stale",
    )?;
    let records = proof["packages"]
        .as_array()
        .ok_or("validated Cargo package records missing")?;
    let matching: Vec<_> = records
        .iter()
        .filter(|package| package["name"] == name && package["version"] == version)
        .collect();
    require(
        matching.len() == 1,
        "validated Cargo archive record missing/duplicate",
    )?;
    let record = matching[0];
    let expected = format!("{name}-{version}.crate");
    require(
        record["filename"] == expected
            && source::safe_relative(&expected)
            && !expected.contains('/'),
        "unsafe validated crate filename",
    )?;
    let archive = root.join("target/package").join(expected);
    let hash = source::file(&archive)?;
    require(
        record["sha256"] == hash,
        "validated Cargo archive bytes changed",
    )?;
    Ok((archive, hash))
}
pub fn image_sources(root: &Path) -> Result<()> {
    crate::image_candidates::shipping(root)
}
pub fn native_artifacts(root: &Path) -> Result<()> {
    // Actual names are canonical in runtime/CLI INTEGRATION. These are bounded
    // components, never invented server/gate aliases or release qualification.
    let names = ["humanymous", "humanymous-runtime"];
    let directory = root.join("target/delivery-artifacts");
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let head = source::head(root)?;
    let name = format!(
        "humanymous-components-{}-{}-{head}.tar.gz",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let file = fs::File::create(directory.join(&name)).map_err(|e| e.to_string())?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut archive = tar::Builder::new(encoder);
    for binary in names {
        let filename = format!("{binary}{}", std::env::consts::EXE_SUFFIX);
        archive
            .append_path_with_name(root.join("target/release").join(&filename), filename)
            .map_err(|e| e.to_string())?;
    }
    archive
        .append_path_with_name(root.join("LICENSE"), "LICENSE")
        .map_err(|e| e.to_string())?;
    let notices = crate::notices::append(root, &mut archive)?;
    archive
        .into_inner()
        .map_err(|e| e.to_string())?
        .finish()
        .map_err(|e| e.to_string())?;
    write_json(
        &directory.join(format!("{name}.manifest.json")),
        &json!({"schema":"humanymous.native-components/1","source":source::snapshot(root)?,"archive_sha256":source::file(&directory.join(&name))?,"known_distribution_notices":notices,"full_product_qualified":false,"release_ready":false}),
    )
}
pub fn node(root: &Path) -> Result<PathBuf> {
    let folder = root.join("adapters/node");
    require(
        folder.join("package-lock.json").is_file(),
        "Node legacy adapter lockfile missing",
    )?;
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    command::status(&folder, npm, &["ci", "--ignore-scripts"])?;
    for mode in ["build", "inspect"] {
        command::status(
            root,
            "cargo",
            &[
                "run",
                "--locked",
                "-p",
                "humanymous-node-adapter",
                "--bin",
                "humanymous-node-package",
                "--",
                mode,
            ],
        )?;
    }
    command::status(&folder, npm, &["audit", "--audit-level=low"])?;
    // The owner must deliver source-bound real-host/authority qualification.
    // Generated loaders or the old handwritten JS tests are not that evidence.
    node_receipt(root)?;
    let destination = root.join("target/delivery-node");
    fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
    let packed: Value = serde_json::from_str(&command::text(
        &folder,
        npm,
        &[
            "pack",
            "--json",
            "--ignore-scripts",
            "--pack-destination",
            destination.to_str().ok_or("non-UTF8 artifact path")?,
        ],
    )?)
    .map_err(|e| e.to_string())?;
    require(
        packed.as_array().is_some_and(|items| items.len() == 1),
        "exactly one npm archive required",
    )?;
    let files = packed[0]["files"]
        .as_array()
        .ok_or("npm pack files missing")?
        .iter()
        .map(|item| {
            item["path"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "invalid npm member".into())
        })
        .collect::<Result<Vec<_>>>()?;
    safe_files(&files, false)?;
    require(
        !files
            .iter()
            .any(|path| path.starts_with("src/") || path.ends_with("/pilot.node")),
        "legacy implementation or host pilot binary leaked into npm archive",
    )?;
    require(
        read_json(&folder.join("package.json"))?["private"] != true,
        "private package may not publish",
    )?;
    let filename = packed[0]["filename"]
        .as_str()
        .ok_or("npm archive filename missing")?;
    require(
        source::safe_relative(filename) && !filename.contains('/'),
        "unsafe npm archive filename",
    )?;
    let archive = destination.join(filename);
    write_json(
        &destination.join("validated-archive.json"),
        &json!({"schema":"humanymous.node-archive/1","source_sha":source::head(root)?,"package_manifest_sha256":source::file(&folder.join("package.json"))?,"provenance_sha256":source::file(&folder.join("native/provenance.json"))?,"qualification_receipt_sha256":source::file(&root.join("target/node-native/qualification.json"))?,"filename":filename,"archive_sha256":source::file(&archive)?}),
    )?;
    Ok(archive)
}
pub fn node_receipt(root: &Path) -> Result<()> {
    let receipt = read_json(&root.join("target/node-native/qualification.json"))?;
    require(
        receipt["schema"] == "humanymous.node-native-qualification/1"
            && receipt["full_native_qualified"] == true,
        "native Node qualification receipt missing/unqualified",
    )?;
    require(
        receipt["source_sha"] == source::head(root)?
            && receipt["package_manifest_sha256"]
                == source::file(&root.join("adapters/node/package.json"))?,
        "native npm receipt source/manifest stale",
    )?;
    require(
        receipt["provenance_sha256"]
            == source::file(&root.join("adapters/node/native/provenance.json"))?,
        "native generation provenance stale",
    )?;
    let manifest = read_json(&root.join("adapters/node/package.json"))?;
    require(
        receipt["package_version"] == manifest["version"],
        "native package version differs from receipt",
    )?;
    let hosts = receipt["host_runs"]
        .as_array()
        .ok_or("real Node/Bun host runs missing")?;
    for runtime in ["node", "bun"] {
        require(
            hosts.iter().any(|run| {
                run["runtime"] == runtime
                    && run["status"] == "passed"
                    && run["mock"] == false
                    && run["test_count"].as_u64().is_some_and(|count| count > 0)
                    && run["failed"].as_u64() == Some(0)
                    && run["ignored"].as_u64() == Some(0)
                    && run["authority_artifact_sha256"]
                        .as_str()
                        .is_some_and(|hash| {
                            hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
                        })
                    && run["evidence_profile"] == "production-application-http"
                    && run["verdict_counts"].as_object().is_some_and(|counts| {
                        ["ALLOW", "CHALLENGE", "DENY", "error"]
                            .iter()
                            .all(|key| counts.get(*key).and_then(Value::as_u64).is_some())
                    })
                    && run["origin_effect_count"].as_u64().is_some()
            }),
            format!("actual {runtime} Rust-authority qualification missing"),
        )?;
    }
    let docker = &receipt["authority_docker"];
    require(
        docker["status"] == "passed"
            && docker["source_sha"] == source::head(root)?
            && docker["image_digests"].as_array().is_some_and(|images| {
                !images.is_empty()
                    && images
                        .iter()
                        .all(|image| image.as_str().is_some_and(crate::supply::digest))
            }),
        "actual source-bound authority Docker receipt missing",
    )?;
    let advertised = receipt["advertised_tuples"]
        .as_array()
        .ok_or("native advertised tuple receipts missing")?;
    require(
        !advertised.is_empty()
            && advertised.iter().all(|tuple| {
                tuple["qualified"] == true
                    && tuple["os"].as_str().is_some()
                    && tuple["arch"].as_str().is_some()
            }),
        "unqualified native npm target advertised",
    )?;
    for tuple in advertised {
        for runtime in ["node", "bun"] {
            require(
                hosts.iter().any(|run| {
                    run["runtime"] == runtime
                        && run["os"] == tuple["os"]
                        && run["arch"] == tuple["arch"]
                        && run["version"]
                            .as_str()
                            .is_some_and(|version| !version.is_empty())
                        && run["status"] == "passed"
                        && run["evidence_profile"] == "production-application-http"
                }),
                "advertised native tuple lacks actual source-bound Node/Bun host execution",
            )?;
        }
    }
    let descriptor = read_json(&root.join("adapters/node/capabilities.json"))?;
    require(
        descriptor["releaseQualifiedTuples"] == receipt["advertised_tuples"],
        "package capability tuples differ from actual qualification receipts",
    )
}
pub fn audit_closure(root: &Path) -> Result<()> {
    let out = root.join("target/delivery-evidence");
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let raw = evidence::bundle(root)?;
    let data = metadata(root)?;
    let members: BTreeSet<_> = data["workspace_members"]
        .as_array()
        .ok_or("workspace inventory absent")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let member_directories: BTreeSet<_> = data["packages"]
        .as_array()
        .ok_or("workspace package inventory absent")?
        .iter()
        .filter(|package| {
            package["id"]
                .as_str()
                .is_some_and(|id| members.contains(id))
        })
        .filter_map(|package| package["manifest_path"].as_str())
        .map(|manifest| {
            Path::new(manifest)
                .parent()
                .ok_or("manifest parent missing")
                .and_then(|path| path.canonicalize().map_err(|_| "manifest parent missing"))
        })
        .collect::<std::result::Result<_, _>>()?;
    let listed = command::text(
        root,
        "git",
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "Cargo.lock",
            "**/Cargo.lock",
        ],
    )?;
    let locks: BTreeSet<_> = listed
        .lines()
        .filter(|path| {
            *path == "Cargo.lock"
                || [
                    "scripts/ci/",
                    "test/lab/",
                    "test/rust-wargame/",
                    "crates/",
                    "adapters/",
                    "docs/tooling/",
                ]
                .iter()
                .any(|prefix| path.starts_with(prefix))
        })
        .map(str::to_owned)
        .collect();
    require(
        locks.contains("Cargo.lock"),
        "root Cargo lock absent from audit closure",
    )?;
    let mut reports = Vec::new();
    let mut blocked = Vec::new();
    let mut retired = Vec::new();
    for (index, lock) in locks.iter().enumerate() {
        if !root.join(lock).is_file() {
            let parent = root
                .join(lock)
                .parent()
                .ok_or("retired lock parent missing")?
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if member_directories.contains(&parent) {
                retired.push(json!({"lockfile":lock,"state":"retired-into-common-lock","covered_by":"Cargo.lock"}));
                continue;
            }
            blocked.push(lock.to_string());
            reports.push(json!({"lockfile":lock,"status":"missing-required-lock"}));
            continue;
        }
        let path = raw.join(format!("cargo-audit-{index}.json"));
        let lock_before = source::file(&root.join(lock))?;
        let code = command::json_log(root, "cargo", &["audit", "--file", lock, "--json"], &path)?;
        let lock_after = source::file(&root.join(lock))?;
        let report = match read_json(&path) {
            Ok(report) => report,
            Err(error) => {
                blocked.push(lock.to_string());
                reports.push(json!({"lockfile":lock,"status":"scan-output-invalid","exit_code":code,"error":error,"stdout_sha256":source::file(&path)?}));
                continue;
            }
        };
        reports.push(json!({"lockfile":lock,"sha256":lock_before,"sha256_after":lock_after,"source_changed":lock_before!=lock_after,"exit_code":code,"dependency_count":report["lockfile"]["dependency-count"],"database":report["database"],"vulnerabilities":report["vulnerabilities"],"warnings":report["warnings"]}));
        if code != 0 || report["vulnerabilities"]["found"] != false || lock_before != lock_after {
            blocked.push(lock.to_string());
        }
    }
    let summary = json!({"schema":"humanymous.dependency-closure/1","source":source::snapshot(root)?,"reports":reports,"retired_locks":retired,"blocked":blocked,"scope":"actual maintained Cargo locks plus common-root coverage of explicitly retired child locks; image/Ruby/npm remain separate"});
    write_json(&out.join("advisory-closure.json"), &summary)?;
    write_json(&raw.join("advisory-closure.json"), &summary)?;
    evidence::register(&raw, "Rust dependency closure scan")?;
    evidence::emit_public_summary(
        "Source-bound Cargo audit closure",
        &crate::dependency_summary::format(&summary),
    )?;
    require(
        blocked.is_empty(),
        format!("dependency scans failed: {blocked:?}"),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn package(name: &str, deps: &[&str], publish: Value) -> Value {
        json!({"id":name,"name":name,"publish":publish,"dependencies":deps.iter().map(|name|json!({"name":name,"path":"crates/x","req":"^0.1.0"})).collect::<Vec<_>>()})
    }
    #[test]
    fn graph_orders_normal_build_and_dev_paths() {
        let data = json!({"workspace_members":["a","b","c"],"packages":[package("a",&["b"],Value::Null),package("b",&["c"],Value::Null),package("c",&[],Value::Null)]});
        let waves = graph(&data).unwrap();
        assert_eq!(waves[0][0]["name"], "c");
        assert_eq!(waves[2][0]["name"], "a");
    }
    #[test]
    fn false_and_cycles_never_publish() {
        assert!(
            graph(&json!({"workspace_members":["a"],"packages":[package("a",&[],json!([]))]}))
                .is_err()
        );
        assert!(
            graph(&json!({"workspace_members":["a"],"packages":[package("a",&["a"],Value::Null)]}))
                .is_err()
        );
    }
    #[test]
    fn paths_and_private_members_cannot_escape() {
        for path in [
            ".spec.local/raw.json",
            "attacker/a.rs",
            "a/key.pem",
            "../x",
            "C:/x",
            "/x",
            "test/a.js",
        ] {
            assert!(safe_files(&[path.into()], false).is_err());
        }
        assert!(safe_files(&["tests/required.rs".into()], true).is_ok());
    }
}
