//! Strict component receipt validation and the single hosted coordinator entry.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[path = "coordinator/engine.rs"]
mod engine;

#[cfg(target_os = "linux")]
#[path = "coordinator/host.rs"]
mod host;

type Result<T> = std::result::Result<T, &'static str>;
pub struct Expected {
    pub source: String,
    pub manifest: String,
    pub image: String,
    pub artifacts: BTreeMap<String, String>,
}
#[derive(Clone, Copy)]
pub struct Counts {
    pub passed: u64,
    pub failed: u64,
    pub not_run: u64,
}
pub fn contract() -> Result<Value> {
    serde_json::from_str(include_str!(
        "../../../../adapters/apache/pilot-contract.json"
    ))
    .map_err(|_| "invalid-embedded-contract")
}
fn strings(value: &Value) -> Result<BTreeSet<String>> {
    let values = value.as_array().ok_or("invalid-contract-inventory")?;
    let mut result = BTreeSet::new();
    for value in values {
        if !result.insert(value.as_str().ok_or("invalid-contract-item")?.to_owned()) {
            return Err("duplicate-contract-item");
        }
    }
    Ok(result)
}
fn fields(value: &Value, allowed: &BTreeSet<String>) -> Result<()> {
    let object = value.as_object().ok_or("object-required")?;
    if object.keys().any(|key| !allowed.contains(key)) {
        return Err("unexpected-report-field");
    }
    if allowed.iter().any(|key| !object.contains_key(key)) {
        return Err("missing-report-field");
    }
    Ok(())
}
fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
pub fn parse(bytes: &[u8]) -> Result<Value> {
    if bytes.len() > 1_048_576 {
        return Err("report-byte-bound");
    }
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let value: Value = serde_json::from_slice(bytes).map_err(|_| "invalid-report-json")?;
    // The Rust producer emits compact JSON. Requiring its exact serialization
    // also rejects duplicate object keys, alternate counters and hidden suffixes.
    if serde_json::to_vec(&value).map_err(|_| "invalid-report-json")? != bytes {
        return Err("noncanonical-report-json");
    }
    Ok(value)
}
pub fn diagnostics(bytes: &[u8]) -> Value {
    let mut codes = BTreeSet::new();
    let mut locations = BTreeSet::new();
    let mut labels = BTreeSet::new();
    if let Ok(text) = std::str::from_utf8(bytes) {
        for line in text.lines().take(100_000) {
            if let Some((_, tail)) = line.split_once("error[E")
                && let Some(code) = tail.get(..5).filter(|code| {
                    code.as_bytes()[..4].iter().all(u8::is_ascii_digit) && code.ends_with(']')
                })
            {
                codes.insert(format!("E{}", &code[..4]));
            }
            if let Some((_, tail)) = line.split_once("--> ") {
                let parts: Vec<_> = tail.trim().rsplitn(3, ':').collect();
                if parts.len() == 3
                    && parts[0].parse::<u32>().is_ok()
                    && parts[1].parse::<u32>().is_ok()
                {
                    let path = parts[2].strip_prefix("/src/").unwrap_or(parts[2]);
                    if path.starts_with("crates/")
                        && path.ends_with(".rs")
                        && path.len() < 256
                        && !path.contains("..")
                        && path
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b))
                    {
                        locations.insert(format!("{path}:{}:{}", parts[1], parts[0]));
                    }
                }
            }
            for (needle, label) in [
                ("pinned header missing", "missing-pinned-header"),
                (
                    "host header differs from pinned release",
                    "pinned-header-mismatch",
                ),
                (
                    "recorded libclang was not observed loaded",
                    "libclang-identity-mismatch",
                ),
                ("unqualified Apache target", "unsupported-target"),
            ] {
                if line.contains(needle) {
                    labels.insert(label);
                }
            }
            if codes.len() >= 20 || locations.len() >= 20 {
                break;
            }
        }
    }
    json_diagnostics(
        codes.into_iter().take(20).collect(),
        locations.into_iter().take(20).collect(),
        labels.into_iter().collect(),
    )
}
fn json_diagnostics(codes: Vec<String>, locations: Vec<String>, labels: Vec<&str>) -> Value {
    serde_json::json!({"errorCodes":codes,"sourceLocations":locations,"labels":labels})
}
pub fn inspect(value: &Value, expected: &Expected) -> Result<Counts> {
    let contract = contract()?;
    fields(value, &strings(&contract["report_fields"])?)?;
    if value["schema"] != contract["report_schema"]
        || value["fullProductSuccess"] != false
        || value["requiredScenarioCount"] != 45
        || !["passed", "incomplete"]
            .contains(&value["status"].as_str().ok_or("invalid-report-status")?)
    {
        return Err("invalid-report-scope");
    }
    if value["qualification"]
        != "actual-httpd/shared-authority-WAL; synthetic frozen ALLOW fixture; component only"
    {
        return Err("invalid-qualification-scope");
    }
    if !hex(&expected.source, 40)
        || !hex(&expected.manifest, 64)
        || !expected
            .image
            .strip_prefix("sha256:")
            .is_some_and(|value| hex(value, 64))
        || value["sourceCommit"] != expected.source
        || value["sourceManifestSha256"] != expected.manifest
        || value["imageDigest"] != expected.image
    {
        return Err("source-image-binding");
    }
    for name in [
        "moduleSha256",
        "probeModuleSha256",
        "httpdSha256",
        "abiReceiptSha256",
        "lockSha256",
    ] {
        let actual = expected
            .artifacts
            .get(name)
            .ok_or("missing-external-artifact-hash")?;
        if !hex(actual, 64) || value[name] != *actual {
            return Err("artifact-binding");
        }
    }
    let engine = value["engineVersionObservation"]
        .as_str()
        .ok_or("missing-engine-observation")?;
    if engine.len() > 16384 || !engine.contains("Apache/2.4.70") {
        return Err("engine-version-binding");
    }
    let required = strings(&contract["required_case_ids"])?;
    let supplemental = strings(&contract["allowed_supplementary_case_ids"])?;
    if required.len() != 45 {
        return Err("invalid-required-denominator");
    }
    let cases = value["cases"].as_array().ok_or("missing-cases")?;
    if !(45..=48).contains(&cases.len()) {
        return Err("case-count-bound");
    }
    let allowed_fields = strings(&contract["case_fields"])?;
    let mut seen = BTreeSet::new();
    let mut counts = Counts {
        passed: 0,
        failed: 0,
        not_run: 0,
    };
    for case in cases {
        fields(case, &allowed_fields)?;
        let id = case["id"].as_str().ok_or("missing-case-id")?;
        if !seen.insert(id.to_owned()) {
            return Err("duplicate-case-id");
        }
        let (mpm, name) = id.split_once('/').ok_or("invalid-case-id")?;
        if case["mpm"] != mpm {
            return Err("case-mpm-binding");
        }
        let supplementary = supplemental.contains(id);
        if (!required.contains(id) && !supplementary)
            || (!supplementary && case["name"] != name)
            || (supplementary
                && (case["name"] != "suite-infrastructure" || case["result"] != "failed"))
        {
            return Err("unexpected-case");
        }
        match case["result"].as_str() {
            Some("passed") => counts.passed += 1,
            Some("failed") => counts.failed += 1,
            Some("not_run") => counts.not_run += 1,
            _ => return Err("unknown-case-outcome"),
        }
    }
    if !required.is_subset(&seen) {
        return Err("missing-required-case");
    }
    if value["passed"].as_u64() != Some(counts.passed)
        || value["failed"].as_u64() != Some(counts.failed)
        || value["not_run"].as_u64() != Some(counts.not_run)
    {
        return Err("counter-mismatch");
    }
    if value["cleanupObserved"].as_bool().is_none() {
        return Err("missing-cleanup-status");
    }
    Ok(counts)
}
pub fn validate(value: &Value, expected: &Expected, exit: i32) -> Result<Counts> {
    let counts = inspect(value, expected)?;
    if exit != 0
        || value["status"] != "passed"
        || counts.passed != 45
        || counts.failed != 0
        || counts.not_run != 0
        || value["cleanupObserved"] != true
    {
        return Err("pilot-not-passed");
    }
    Ok(counts)
}
pub fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        host::run()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("Apache Docker coordinator requires an allocated Linux runner".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Value, Expected) {
        let c = contract().unwrap();
        let cases: Vec<_> = c["required_case_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| {
                let id = id.as_str().unwrap();
                let (mpm, name) = id.split_once('/').unwrap();
                serde_json::json!({"id":id,"mpm":mpm,"name":name,"result":"passed"})
            })
            .collect();
        let artifacts = [
            "moduleSha256",
            "probeModuleSha256",
            "httpdSha256",
            "abiReceiptSha256",
            "lockSha256",
        ]
        .into_iter()
        .map(|key| (key.to_owned(), "b".repeat(64)))
        .collect();
        let expected = Expected {
            source: "a".repeat(40),
            manifest: "c".repeat(64),
            image: format!("sha256:{}", "d".repeat(64)),
            artifacts,
        };
        let mut value = serde_json::json!({"schema":"humanymous.apache.pilot/0.1","status":"passed","qualification":"actual-httpd/shared-authority-WAL; synthetic frozen ALLOW fixture; component only", "sourceCommit":expected.source,"sourceManifestSha256":expected.manifest,"imageDigest":expected.image,"engineVersionObservation":"Server version: Apache/2.4.70","cases":cases,"requiredScenarioCount":45,"passed":45,"failed":0,"not_run":0,"cleanupObserved":true,"fullProductSuccess":false});
        for (key, hash) in &expected.artifacts {
            value[key] = hash.clone().into();
        }
        (value, expected)
    }
    #[test]
    fn accepts_exact_45_and_independent_artifact_binding() {
        let (v, e) = fixture();
        assert_eq!(validate(&v, &e, 0).unwrap().passed, 45);
    }
    #[test]
    fn rejects_duplicate_ids_even_when_totals_look_correct() {
        let (mut v, e) = fixture();
        v["cases"][1] = v["cases"][0].clone();
        assert!(validate(&v, &e, 0).is_err());
    }
    #[test]
    fn rejects_missing_ids_and_forged_counts() {
        let (mut v, e) = fixture();
        v["cases"].as_array_mut().unwrap().pop();
        v["passed"] = 44.into();
        assert!(validate(&v, &e, 0).is_err());
        let (mut v, e) = fixture();
        v["failed"] = 1.into();
        assert!(validate(&v, &e, 0).is_err());
    }
    #[test]
    fn rejects_every_unbound_hash_source_and_scope() {
        for key in [
            "sourceCommit",
            "sourceManifestSha256",
            "imageDigest",
            "moduleSha256",
            "probeModuleSha256",
            "httpdSha256",
            "abiReceiptSha256",
            "lockSha256",
        ] {
            let (mut v, e) = fixture();
            v[key] = "forged".into();
            assert!(validate(&v, &e, 0).is_err(), "{key}");
        }
        let (mut v, e) = fixture();
        v["fullProductSuccess"] = true.into();
        assert!(validate(&v, &e, 0).is_err());
    }
    #[test]
    fn failed_process_cleanup_and_notrun_cannot_pass() {
        let (v, e) = fixture();
        assert!(validate(&v, &e, 1).is_err());
        let (mut v, e) = fixture();
        v["cleanupObserved"] = false.into();
        assert!(validate(&v, &e, 0).is_err());
        let (mut v, e) = fixture();
        v["cases"][0]["result"] = "not_run".into();
        v["passed"] = 44.into();
        v["not_run"] = 1.into();
        assert!(inspect(&v, &e).is_ok());
        assert!(validate(&v, &e, 0).is_err());
    }
    #[test]
    fn supplemental_success_unknown_fields_and_duplicate_json_fail() {
        let (mut v, e) = fixture();
        v["cases"].as_array_mut().unwrap().push(serde_json::json!({"id":"event/infrastructure","mpm":"event","name":"suite-infrastructure","result":"passed"}));
        v["passed"] = 46.into();
        assert!(validate(&v, &e, 0).is_err());
        let (mut v, e) = fixture();
        v["body"] = "private".into();
        assert!(validate(&v, &e, 0).is_err());
        let bytes = br#"{"a":1,"a":2}"#;
        assert!(parse(bytes).is_err());
        let (v, _) = fixture();
        assert!(parse(&serde_json::to_vec(&v).unwrap()).is_ok());
    }
    #[test]
    fn diagnostics_project_only_codes_and_owned_source_locations() {
        let value=diagnostics(b"secret=never-upload\nerror[E0425]: private diagnostic\n --> crates/apache-adapter/native/src/host.rs:11:8\n --> /home/runner/private.rs:8:1\n");
        assert_eq!(value["errorCodes"][0], "E0425");
        assert_eq!(value["sourceLocations"].as_array().unwrap().len(), 1);
        assert!(!value.to_string().contains("never-upload"));
        assert!(!value.to_string().contains("private diagnostic"));
    }
}
