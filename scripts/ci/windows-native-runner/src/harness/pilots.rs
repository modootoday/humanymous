//! Declared ignored native pilots (scripts/ci/native-pilots.json).
//!
//! An ignored test is accepted only when its exact name is declared for this
//! harness and OS, and either the same-job pilot receipt shows it passed on the
//! same source or a named step of another required job executes it. An
//! owner-declared unsupported tuple names the exact ignored tests that do not
//! run on that OS; they are reported as unsupported, never as passed. Anything
//! else stays a coverage-accounting failure; nothing here is a waiver.
use super::packages::Packages;
use serde_json::{Value, json};
use sha2::Digest;
use std::{collections::BTreeSet, io};

const DECLARATIONS: &str = include_str!("../../../native-pilots.json");
const RECEIPT_SCHEMA: &str = "humanymous.native-pilots-receipt/1";

#[derive(Debug, Default)]
pub(super) struct Declared {
    pub(super) ids: Vec<String>,
    /// Every declared ignored name (tests and subprocess helpers).
    pub(super) ignored: Vec<String>,
    pub(super) helpers: Vec<String>,
    pub(super) receipt_tests: Vec<(String, String)>,
    pub(super) jobs: Vec<Value>,
    /// Owner-declared unsupported tuple ids and their exact not-run tests.
    pub(super) unsupported_ids: Vec<String>,
    pub(super) unsupported_tests: Vec<String>,
}
impl Declared {
    pub(super) fn is_empty(&self) -> bool {
        self.ignored.is_empty()
    }
    pub(super) fn row(&self, verified: bool) -> Value {
        let tests: Vec<_> = self
            .ignored
            .iter()
            .filter(|name| !self.helpers.contains(name) && !self.unsupported_tests.contains(name))
            .collect();
        let scope = if !self.unsupported_ids.is_empty() {
            "unsupported"
        } else if self.jobs.is_empty() {
            "native-pilots"
        } else {
            "job"
        };
        json!({"required_not_run":tests,"declared_subprocess_helpers":self.helpers,"pilot_ids":self.ids,"required_execution_scope":scope,"pilot_job_executors":self.jobs,"pilot_receipt_verified":verified,"unsupported_ids":self.unsupported_ids,"unsupported_not_run":self.unsupported_tests})
    }
}

fn declarations() -> io::Result<Value> {
    let value: Value = serde_json::from_str(DECLARATIONS).map_err(io::Error::other)?;
    if value["schema"] != "humanymous.native-pilots/1" || !value["pilots"].is_array() {
        return Err(io::Error::other("native pilot declaration schema mismatch"));
    }
    Ok(value)
}

/// Declared pilot ids, for the closed public vocabulary.
pub(super) fn known_ids() -> BTreeSet<String> {
    declarations()
        .ok()
        .and_then(|value| value["pilots"].as_array().cloned())
        .into_iter()
        .flatten()
        .filter_map(|pilot| pilot["id"].as_str().map(str::to_owned))
        .collect()
}
/// Declared unsupported tuple ids, for the closed public vocabulary.
pub(super) fn known_unsupported_ids() -> BTreeSet<String> {
    known_unsupported("id")
}
/// Declared unsupported test names, for the closed public vocabulary.
pub(super) fn known_unsupported_tests() -> BTreeSet<String> {
    known_unsupported("tests")
}
fn known_unsupported(key: &str) -> BTreeSet<String> {
    declarations()
        .ok()
        .and_then(|value| value["unsupported"].as_array().cloned())
        .into_iter()
        .flatten()
        .flat_map(|entry| match &entry[key] {
            Value::Array(list) => list.clone(),
            other => vec![other.clone()],
        })
        .filter_map(|name| name.as_str().map(str::to_owned))
        .collect()
}
/// Declared helper names, for the closed public vocabulary.
pub(super) fn known_helpers() -> BTreeSet<String> {
    declarations()
        .ok()
        .and_then(|value| value["pilots"].as_array().cloned())
        .into_iter()
        .flatten()
        .flat_map(|pilot| pilot["helpers"].as_array().cloned().unwrap_or_default())
        .filter_map(|name| name.as_str().map(str::to_owned))
        .collect()
}

fn names(value: &Value) -> io::Result<Vec<String>> {
    value
        .as_array()
        .ok_or_else(|| io::Error::other("pilot list absent"))?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or_else(|| io::Error::other("pilot name not a string"))
        })
        .collect()
}

pub(super) fn declared(
    entry: &Value,
    packages: &Packages,
    os: &str,
    arch: &str,
) -> io::Result<Declared> {
    let mut found = Declared::default();
    let Some(name) = packages.name(entry) else {
        return Ok(found);
    };
    let declarations = declarations()?;
    let selects = |declared: &Value| {
        declared["os"]
            .as_array()
            .is_some_and(|list| list.iter().any(|v| v == os))
            && declared["arch"].as_str().is_none_or(|a| a == arch)
            && declared["package"] == name
            && entry["target"] == declared["target"]
            && entry["kind"] == json!([declared["kind"]])
    };
    for pilot in declarations["pilots"].as_array().into_iter().flatten() {
        if !selects(pilot) {
            continue;
        }
        let id = pilot["id"]
            .as_str()
            .ok_or_else(|| io::Error::other("pilot id absent"))?;
        let tests = names(&pilot["tests"])?;
        let helpers = names(&pilot["helpers"])?;
        found.ids.push(id.to_owned());
        match pilot["executor"].as_str() {
            Some("native-pilots") => found
                .receipt_tests
                .extend(tests.iter().map(|t| (id.to_owned(), t.clone()))),
            Some("job") if helpers.is_empty() => {
                found.jobs.push(json!({"pilot":id,"workflow":pilot["job"]["workflow"],"job":pilot["job"]["name"],"run":pilot["job"]["run"]}));
            }
            _ => return Err(io::Error::other("unknown pilot executor")),
        }
        found.ignored.extend(tests);
        found.ignored.extend(helpers.iter().cloned());
        found.helpers.extend(helpers);
    }
    let unsupported = declarations["unsupported"]
        .as_array()
        .ok_or_else(|| io::Error::other("unsupported declarations absent"))?;
    for declared in unsupported {
        if !selects(declared) {
            continue;
        }
        let id = declared["id"]
            .as_str()
            .ok_or_else(|| io::Error::other("unsupported id absent"))?;
        let tests = names(&declared["tests"])?;
        found.unsupported_ids.push(id.to_owned());
        found.ignored.extend(tests.iter().cloned());
        found.unsupported_tests.extend(tests);
    }
    // Fail closed: one harness tuple is either executed or unsupported.
    if !found.unsupported_ids.is_empty() && !found.ids.is_empty() {
        return Err(io::Error::other(
            "harness tuple declared both as pilot and unsupported",
        ));
    }
    Ok(found)
}

/// The observed ignored names must equal the declared names exactly.
pub(super) fn ignored_matches(log: &str, expected: &[String]) -> bool {
    let mut observed: Vec<&str> = log
        .lines()
        .filter_map(|line| {
            line.strip_prefix("test ")
                .and_then(|s| s.split_once(" ... ignored").map(|(name, _)| name))
        })
        .collect();
    let mut expected: Vec<&str> = expected.iter().map(String::as_str).collect();
    observed.sort_unstable();
    expected.sort_unstable();
    observed == expected
}

/// Same-job receipt: exact source, OS, architecture, declarations and a single
/// passed row for every declared receipt test of this harness.
pub(super) fn verify_receipt(
    declared: &Declared,
    source_sha: Option<&str>,
    receipt: Option<&str>,
    os: &str,
    arch: &str,
) -> io::Result<()> {
    if declared.receipt_tests.is_empty() {
        return Ok(());
    }
    let refused = || io::Error::other("declared native pilot receipt absent or mismatched");
    let receipt: Value =
        serde_json::from_str(receipt.ok_or_else(refused)?).map_err(|_| refused())?;
    let digest = format!("{:x}", sha2::Sha256::digest(DECLARATIONS.as_bytes()));
    if receipt["schema"] != RECEIPT_SCHEMA
        || source_sha.is_none_or(|sha| receipt["source_sha"] != sha)
        || receipt["os"] != os
        || receipt["arch"] != arch
        || receipt["declarations_sha256"] != digest
    {
        return Err(refused());
    }
    let rows = receipt["tests"].as_array().ok_or_else(refused)?;
    for (pilot, name) in &declared.receipt_tests {
        let matching: Vec<_> = rows
            .iter()
            .filter(|row| row["pilot"] == pilot.as_str() && row["name"] == name.as_str())
            .collect();
        if matching.len() != 1 || matching[0]["status"] != "passed" {
            return Err(refused());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn runtime_entry() -> Value {
        json!({"package_id":"path+file:///C:/workspaces/humanymous/crates/sdk/humanymous-runtime#0.1.0","kind":["lib"],"target":"humanymous_runtime"})
    }
    #[test]
    fn declarations_select_only_the_matching_harness_and_os() {
        let packages = Packages::fixture();
        let found = declared(&runtime_entry(), &packages, "linux", "x86_64").unwrap();
        assert_eq!(found.ids, ["runtime-audit-native-source"]);
        assert_eq!(found.receipt_tests.len(), 6);
        assert!(
            declared(&runtime_entry(), &packages, "macos", "x86_64")
                .unwrap()
                .is_empty()
        );
        assert!(
            declared(&runtime_entry(), &packages, "linux", "aarch64")
                .unwrap()
                .is_empty()
        );
        let mut other = runtime_entry();
        other["target"] = json!("gateway-native-process");
        assert!(
            declared(&other, &packages, "linux", "x86_64")
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn dotnet_harnesses_are_unsupported_only_on_macos() {
        let packages = Packages::fixture();
        let hosted = json!({"package_id":"opaque-humanymous-dotnet-adapter","kind":["test"],"target":"hosted"});
        let found = declared(&hosted, &packages, "macos", "aarch64").unwrap();
        assert!(found.ids.is_empty() && found.receipt_tests.is_empty() && found.jobs.is_empty());
        assert_eq!(found.unsupported_ids, ["dotnet-hosted-macos-unsupported"]);
        assert_eq!(
            found.ignored,
            [
                "actual_kestrel_engine_audit_body_gc_and_shutdown",
                "generic::actual_generic_abi_only_process"
            ]
        );
        let row = found.row(true);
        assert_eq!(row["required_execution_scope"], "unsupported");
        assert_eq!(row["required_not_run"], json!([]));
        assert_eq!(row["unsupported_not_run"], json!(found.ignored));
        // No receipt is needed or consulted: nothing is claimed to have run.
        assert!(verify_receipt(&found, None, None, "macos", "aarch64").is_ok());
        let mut package = hosted.clone();
        package["target"] = json!("package");
        let found = declared(&package, &packages, "macos", "x86_64").unwrap();
        assert_eq!(found.unsupported_ids, ["dotnet-package-macos-unsupported"]);
        assert_eq!(found.ignored, ["sdk_package_and_real_host_matrix"]);
        // Linux hosted runs in the `dotnet` job (systemd-scope provider). Its
        // pilots are accounted there, never as unsupported.
        let linux = declared(&hosted, &packages, "linux", "x86_64").unwrap();
        assert_eq!(linux.jobs.len(), 1);
        assert_eq!(linux.jobs[0]["pilot"], "dotnet-hosted-linux");
        assert_eq!(linux.jobs[0]["job"], "dotnet");
        assert!(linux.unsupported_ids.is_empty());
        // Windows hosted and both package rows keep strict accounting: nothing
        // is declared, so those ignored pilots stay a coverage failure.
        assert!(
            declared(&hosted, &packages, "windows", "x86_64")
                .unwrap()
                .is_empty()
        );
        for os in ["linux", "windows"] {
            assert!(
                declared(&package, &packages, os, "x86_64")
                    .unwrap()
                    .is_empty()
            );
        }
        let mut other = hosted.clone();
        other["target"] = json!("managed_scope");
        assert!(
            declared(&other, &packages, "macos", "aarch64")
                .unwrap()
                .is_empty()
        );
        assert!(known_unsupported_ids().contains("dotnet-package-macos-unsupported"));
        assert!(known_unsupported_tests().contains("sdk_package_and_real_host_matrix"));
    }
    #[test]
    fn dotnet_sdk_download_is_accounted_by_the_dotnet_job_on_every_os() {
        let packages = Packages::fixture();
        let delivery = json!({"package_id":"opaque-humanymous-delivery","kind":["bin"],"target":"humanymous-delivery"});
        for (os, arch) in [
            ("linux", "x86_64"),
            ("macos", "aarch64"),
            ("windows", "x86_64"),
        ] {
            let found = declared(&delivery, &packages, os, arch).unwrap();
            assert_eq!(found.ids, ["dotnet-sdk-download"]);
            assert_eq!(found.jobs.len(), 1);
            assert_eq!(found.jobs[0]["job"], "dotnet");
            assert!(found.receipt_tests.is_empty() && found.unsupported_ids.is_empty());
            assert_eq!(
                found.ignored,
                ["components::dotnet_host::sdk::tests::actual_pinned_sdk_download_verifies"]
            );
            assert_eq!(found.row(true)["required_execution_scope"], "job");
        }
        // Only that exact ignored name is accepted for the delivery binary.
        let mut lib = delivery.clone();
        lib["kind"] = json!(["lib"]);
        assert!(
            declared(&lib, &packages, "linux", "x86_64")
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn authority_helper_is_declared_but_never_receipted() {
        let entry = json!({"package_id":"path+file:///C:/workspaces/humanymous/crates/sdk/humanymous-runtime#0.1.0","kind":["test"],"target":"authority-native-process"});
        let found = declared(&entry, &Packages::fixture(), "linux", "x86_64").unwrap();
        assert_eq!(found.helpers, ["unix_unprivileged_connect_probe"]);
        assert_eq!(found.receipt_tests.len(), 1);
        assert_eq!(
            found.row(true)["required_not_run"],
            json!(["unix_other_user_is_denied_by_actual_socket_acl"])
        );
    }
    #[test]
    fn ignored_names_must_match_exactly() {
        let expected = vec!["a::b".to_owned(), "c".to_owned()];
        assert!(ignored_matches(
            "test c ... ignored\ntest a::b ... ignored, reason\ntest d ... ok\n",
            &expected
        ));
        assert!(!ignored_matches("test c ... ignored\n", &expected));
        assert!(!ignored_matches(
            "test c ... ignored\ntest a::b ... ignored\ntest e ... ignored\n",
            &expected
        ));
        assert!(ignored_matches("", &[]));
    }
    #[test]
    fn receipt_must_bind_source_os_declarations_and_each_pass() {
        let found = declared(&runtime_entry(), &Packages::fixture(), "linux", "x86_64").unwrap();
        let digest = format!("{:x}", sha2::Sha256::digest(DECLARATIONS.as_bytes()));
        let rows: Vec<_> = found
            .receipt_tests
            .iter()
            .map(|(p, n)| json!({"pilot":p,"name":n,"status":"passed"}))
            .collect();
        let good = json!({"schema":RECEIPT_SCHEMA,"source_sha":"a".repeat(40),"os":"linux","arch":"x86_64","declarations_sha256":digest,"tests":rows});
        let check = |value: &Value, sha: Option<&str>| {
            verify_receipt(&found, sha, Some(&value.to_string()), "linux", "x86_64")
        };
        let sha = "a".repeat(40);
        assert!(check(&good, Some(&sha)).is_ok());
        assert!(check(&good, None).is_err());
        assert!(check(&good, Some(&"b".repeat(40))).is_err());
        assert!(verify_receipt(&found, Some(&sha), None, "linux", "x86_64").is_err());
        for (key, value) in [
            ("os", json!("macos")),
            ("arch", json!("aarch64")),
            ("declarations_sha256", json!("0")),
            ("schema", json!("x")),
        ] {
            let mut bad = good.clone();
            bad[key] = value;
            assert!(check(&bad, Some(&sha)).is_err());
        }
        let mut failed = good.clone();
        failed["tests"][0]["status"] = json!("failed");
        assert!(check(&failed, Some(&sha)).is_err());
        let mut missing = good.clone();
        missing["tests"].as_array_mut().unwrap().pop();
        assert!(check(&missing, Some(&sha)).is_err());
        let mut duplicated = good.clone();
        let first = duplicated["tests"][0].clone();
        duplicated["tests"].as_array_mut().unwrap().push(first);
        assert!(check(&duplicated, Some(&sha)).is_err());
    }
}
