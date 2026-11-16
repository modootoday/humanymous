//! Go↔Rust differential surface manifest: every recorded Go vector stays bound
//! to the frozen baseline, to the Rust test that replays it, and to the FM11
//! ledger package it covers; packages without vectors are named gaps that
//! carry a recorded owner acceptance.
use crate::{Result, command, require, source};
use serde_json::Value;
use std::{collections::BTreeSet, path::Path};

const MANIFEST: &str = "scripts/ci/parity-surfaces.json";
const SCHEMA: &str = "humanymous.parity-surfaces/1";
const BASELINE: &str = "981d25cf71a5a54855d7e2b4359e832a9d5998e5";
const PROVENANCE: &[&str] = &["go-recorded-differential", "go-regenerated-at-baseline"];

pub fn check(root: &Path) -> Result<()> {
    let manifest = crate::read_json(&root.join(MANIFEST))?;
    let ledger_path = manifest["package_source"]
        .as_str()
        .ok_or("parity manifest package_source missing")?;
    require(
        source::safe_relative(ledger_path),
        "parity package_source must be a safe relative path",
    )?;
    let ledger = crate::read_json(&root.join(ledger_path))?;
    let tracked = command::text(root, "git", &["ls-files", "crates"])?;
    validate(root, &manifest, &ledger, tracked.lines())
}

fn text<'a>(value: &'a Value, what: &str) -> Result<&'a str> {
    value
        .as_str()
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("parity {what} must be a non-empty string"))
}

fn items<'a>(value: &'a Value, what: &str) -> Result<&'a Vec<Value>> {
    value
        .as_array()
        .filter(|items| !items.is_empty())
        .ok_or_else(|| format!("parity {what} must be a non-empty array"))
}

fn has_test(body: &str, name: &str) -> bool {
    let needle = format!("fn {name}(");
    body.match_indices(&needle).any(|(at, _)| {
        body[..at]
            .trim_end()
            .rsplit('\n')
            .next()
            .is_some_and(|line| line.trim() == "#[test]")
    })
}

fn path<'a>(value: &'a Value, what: &str) -> Result<&'a str> {
    let path = text(value, what)?;
    require(
        source::safe_relative(path),
        format!("parity {what} {path} must be a safe relative path"),
    )?;
    Ok(path)
}

pub fn validate<'a>(
    root: &Path,
    manifest: &Value,
    ledger: &Value,
    tracked: impl Iterator<Item = &'a str>,
) -> Result<()> {
    require(
        manifest["schema"] == SCHEMA,
        "parity manifest schema mismatch",
    )?;
    require(
        manifest["baseline"]["commit"] == BASELINE,
        "parity baseline must be the frozen Go commit",
    )?;
    text(&manifest["baseline"]["go"], "baseline go version")?;
    let packages: BTreeSet<&str> = ledger["by_package"]
        .as_object()
        .ok_or("ledger by_package missing")?
        .keys()
        .map(String::as_str)
        .collect();
    let vocabulary = manifest["provenance_vocabulary"]
        .as_object()
        .ok_or("parity provenance_vocabulary missing")?;
    require(
        vocabulary
            .keys()
            .map(String::as_str)
            .eq(PROVENANCE.iter().copied()),
        "parity provenance vocabulary must name exactly the allowed provenances",
    )?;
    let mut covered = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut referenced = BTreeSet::new();
    let mut vectors = BTreeSet::new();
    for surface in items(&manifest["surfaces"], "surfaces")? {
        let id = text(&surface["id"], "surface id")?;
        require(ids.insert(id), format!("duplicate parity surface {id}"))?;
        text(&surface["compares"], "surface compares")?;
        for package in items(&surface["go_packages"], "surface go_packages")? {
            let package = text(package, "go package")?;
            require(
                packages.contains(package),
                format!("parity surface {id} names {package} outside the FM11 ledger"),
            )?;
            require(
                covered.insert(package),
                format!("Go package {package} is covered more than once"),
            )?;
        }
        for test in items(&surface["tests"], "surface tests")? {
            let file = path(&test["file"], "test file")?;
            require(
                file.ends_with("_parity.rs"),
                format!("parity test {file} must be a *_parity.rs file"),
            )?;
            require(
                referenced.insert(file),
                format!("parity test {file} listed twice"),
            )?;
            let body =
                std::fs::read_to_string(root.join(file)).map_err(|e| format!("{file}: {e}"))?;
            for function in items(&test["functions"], "test functions")? {
                let function = text(function, "test function")?;
                require(
                    has_test(&body, function),
                    format!("{file} has no #[test] fn {function}"),
                )?;
            }
        }
        for vector in items(&surface["vectors"], "surface vectors")? {
            let file = path(&vector["path"], "vector path")?;
            require(
                vectors.insert(file),
                format!("parity vector {file} listed twice"),
            )?;
            require(
                source::file(&root.join(file))? == text(&vector["sha256"], "vector sha256")?,
                format!("parity vector {file} digest drifted from the recorded Go output"),
            )?;
            match text(&vector["provenance"], "vector provenance")? {
                "go-regenerated-at-baseline" => {
                    path(&vector["baseline_path"], "vector baseline_path")?;
                }
                "go-recorded-differential" => {
                    let record = text(&vector["record"], "vector record")?;
                    require(
                        record.starts_with("plan:"),
                        format!("parity vector {file} record must name a plan record"),
                    )?;
                }
                other => {
                    return Err(format!(
                        "parity vector {file} has unknown provenance {other}"
                    ));
                }
            }
        }
    }
    let mut gaps = BTreeSet::new();
    for gap in manifest["gaps"].as_array().ok_or("parity gaps missing")? {
        let package = text(&gap["go_package"], "gap package")?;
        text(&gap["reason"], "gap reason")?;
        text(&gap["acceptance"]["decision"], "gap acceptance decision")?;
        let record = text(&gap["acceptance"]["record"], "gap acceptance record")?;
        require(
            source::safe_relative(record) && root.join(record).is_file(),
            format!("parity gap {package} acceptance record {record} is missing"),
        )?;
        require(
            packages.contains(package),
            format!("parity gap {package} is outside the FM11 ledger"),
        )?;
        require(
            !covered.contains(package),
            format!("Go package {package} is both a surface and a gap"),
        )?;
        require(
            gaps.insert(package),
            format!("parity gap {package} listed twice"),
        )?;
    }
    let accounted: BTreeSet<&str> = covered.union(&gaps).copied().collect();
    require(
        accounted == packages,
        format!(
            "parity manifest leaves ledger packages unaccounted: {:?}",
            packages.difference(&accounted).collect::<Vec<_>>()
        ),
    )?;
    let unlisted: Vec<&str> = tracked
        .filter(|file| file.ends_with("_parity.rs") && !referenced.contains(file))
        .collect();
    require(
        unlisted.is_empty(),
        format!("parity tests missing from the manifest: {unlisted:?}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    type Mutation = Box<dyn Fn(&mut Value, &mut String)>;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
    }

    fn inputs() -> (Value, Value, String) {
        let root = root();
        let manifest = crate::read_json(&root.join(MANIFEST)).unwrap();
        let ledger =
            crate::read_json(&root.join(manifest["package_source"].as_str().unwrap())).unwrap();
        let tracked = command::text(&root, "git", &["ls-files", "crates"]).unwrap();
        (manifest, ledger, tracked)
    }

    fn run(manifest: &Value, ledger: &Value, tracked: &str) -> Result<()> {
        validate(&root(), manifest, ledger, tracked.lines())
    }

    #[test]
    fn committed_manifest_is_valid() {
        check(&root()).unwrap();
    }

    #[test]
    fn rejects_mutations() {
        let (manifest, ledger, tracked) = inputs();
        let cases: Vec<(&str, Mutation)> = vec![
            (
                "baseline",
                Box::new(|m, _| m["baseline"]["commit"] = "0".into()),
            ),
            (
                "digest",
                Box::new(|m, _| m["surfaces"][0]["vectors"][0]["sha256"] = "00".into()),
            ),
            (
                "provenance",
                Box::new(|m, _| m["surfaces"][0]["vectors"][0]["provenance"] = "guess".into()),
            ),
            (
                "function",
                Box::new(|m, _| m["surfaces"][0]["tests"][0]["functions"][0] = "absent".into()),
            ),
            (
                "outside",
                Box::new(|m, _| m["surfaces"][0]["go_packages"][0] = "internal/nowhere".into()),
            ),
            (
                "unaccounted",
                Box::new(|m, _| {
                    m["gaps"].as_array_mut().unwrap().pop();
                }),
            ),
            (
                "unaccepted",
                Box::new(|m, _| {
                    m["gaps"][0].as_object_mut().unwrap().remove("acceptance");
                }),
            ),
            (
                "record",
                Box::new(|m, _| m["gaps"][0]["acceptance"]["record"] = "absent.md".into()),
            ),
            (
                "double",
                Box::new(|m, _| {
                    let package = m["surfaces"][0]["go_packages"][0].clone();
                    let acceptance = m["gaps"][0]["acceptance"].clone();
                    m["gaps"].as_array_mut().unwrap().push(
                        serde_json::json!({"go_package": package, "reason": "x", "acceptance": acceptance}),
                    );
                }),
            ),
            (
                "unlisted",
                Box::new(|_, t| t.push_str("\ncrates/x/tests/new_parity.rs")),
            ),
            (
                "unsafe",
                Box::new(|m, _| m["surfaces"][0]["tests"][0]["file"] = "../x_parity.rs".into()),
            ),
        ];
        run(&manifest, &ledger, &tracked).unwrap();
        for (name, mutate) in cases {
            let (mut manifest, mut tracked) = (manifest.clone(), tracked.clone());
            mutate(&mut manifest, &mut tracked);
            assert!(
                run(&manifest, &ledger, &tracked).is_err(),
                "{name} accepted"
            );
        }
    }

    #[test]
    fn test_attribute_must_precede_function() {
        assert!(has_test("#[test]\nfn a() {}", "a"));
        assert!(has_test("    #[test]\n    fn a() {}", "a"));
        assert!(!has_test("fn a() {}", "a"));
        assert!(!has_test("#[test]\nfn ab() {}", "a"));
    }
}
