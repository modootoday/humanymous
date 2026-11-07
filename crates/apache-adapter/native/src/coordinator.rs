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
const CASE_LABELS: &[(&str, &str)] = &[
    ("binary replay/effect", "binary-replay-effect"),
    ("concurrent request", "concurrent-request"),
    ("concurrent worker panic", "concurrent-worker-panic"),
    ("concurrent body isolation", "concurrent-body-isolation"),
    (
        "exact concurrent origin effects",
        "concurrent-origin-effects",
    ),
    ("spoof stripping", "spoof-stripping"),
    ("actual rejected nested request", "nested-request-rejection"),
    ("cache graph must fail closed", "cache-graph-rejection"),
    ("denied origin effect", "denied-origin-effect"),
    ("cancelled origin effect", "cancelled-origin-effect"),
    (
        "authority observed inflight request before reload",
        "inflight-not-observed",
    ),
    (
        "old generation must not reach origin",
        "old-generation-origin-effect",
    ),
    ("post-reload module request", "post-reload-request"),
    ("expired origin effect", "expired-origin-effect"),
    ("production missing capability", "missing-capability"),
    ("outage origin effect", "outage-origin-effect"),
    (
        "new MPM generation not observed",
        "new-generation-not-observed",
    ),
    ("httpd startup failed", "httpd-startup"),
    ("selected MPM observed", "mpm-selection"),
    ("cache module graph observed", "cache-module-selection"),
    ("httpd stop command", "httpd-stop"),
    ("MPM children required", "mpm-children"),
    ("graceful reload command", "graceful-reload-command"),
    ("owned cleanup path", "cleanup-path"),
];
const IO_LABELS: &[(std::io::ErrorKind, &str)] = &[
    (std::io::ErrorKind::TimedOut, "io-timeout"),
    (std::io::ErrorKind::WouldBlock, "io-would-block"),
    (
        std::io::ErrorKind::ConnectionRefused,
        "io-connection-refused",
    ),
    (std::io::ErrorKind::ConnectionReset, "io-connection-reset"),
    (
        std::io::ErrorKind::ConnectionAborted,
        "io-connection-aborted",
    ),
    (std::io::ErrorKind::UnexpectedEof, "io-unexpected-eof"),
    (std::io::ErrorKind::BrokenPipe, "io-broken-pipe"),
    (std::io::ErrorKind::NotFound, "io-not-found"),
    (std::io::ErrorKind::PermissionDenied, "io-permission-denied"),
    (std::io::ErrorKind::OutOfMemory, "io-memory"),
];
fn case_io_failure_label(kind: std::io::ErrorKind) -> &'static str {
    IO_LABELS
        .iter()
        .find_map(|(known, label)| (*known == kind).then_some(*label))
        .unwrap_or("operation-failed")
}
pub fn case_failure_label(message: &str) -> &'static str {
    CASE_LABELS
        .iter()
        .find_map(|(text, label)| (*text == message).then_some(*label))
        .unwrap_or("operation-failed")
}
pub fn case_error_label(error: &(dyn std::error::Error + 'static)) -> &'static str {
    if let Some(error) = error.downcast_ref::<std::io::Error>() {
        case_io_failure_label(error.kind())
    } else {
        case_failure_label(&error.to_string())
    }
}
pub fn case_diagnostics(value: &Value, bytes: &[u8]) -> Value {
    let Ok(c) = contract() else {
        return Value::Null;
    };
    let mut allowed = strings(&c["required_case_ids"]).unwrap_or_default();
    allowed.extend(strings(&c["allowed_supplementary_case_ids"]).unwrap_or_default());
    let mut labels = BTreeMap::new();
    if let Some(cases) = value["cases"].as_array() {
        for case in cases.iter().take(48) {
            if let Some(id) = case["id"].as_str()
                && allowed.contains(id)
                && case["result"] == "failed"
            {
                labels.insert(id.to_owned(), "operation-failed".to_owned());
            }
        }
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        for line in text.lines().take(1000) {
            let Some(tail) = line.strip_prefix("APACHE_CASE_FAILURE ") else {
                continue;
            };
            let parts: Vec<_> = tail.split_ascii_whitespace().collect();
            if parts.len() == 2
                && labels.contains_key(parts[0])
                && (parts[1] == "operation-failed"
                    || CASE_LABELS.iter().any(|(_, label)| *label == parts[1])
                    || IO_LABELS.iter().any(|(_, label)| *label == parts[1]))
            {
                labels.insert(parts[0].to_owned(), parts[1].to_owned());
            }
        }
    }
    serde_json::json!(labels)
}
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
const APACHE_CODES: &[&str] = &["AH00057", "AH00058", "AH00059", "AH00174"];
pub fn apache_error_codes(bytes: &[u8]) -> Vec<String> {
    APACHE_CODES
        .iter()
        .filter(|code| {
            bytes
                .windows(code.len())
                .any(|part| part == code.as_bytes())
        })
        .map(|code| (*code).to_owned())
        .collect()
}
fn diagnostic_frame<'a>(bytes: &'a [u8], prefix: &str) -> Result<&'a str> {
    let text = std::str::from_utf8(bytes).map_err(|_| "diagnostic-encoding")?;
    let mut frames = text.lines().filter_map(|line| line.strip_prefix(prefix));
    let frame = frames.next().ok_or("diagnostic-missing")?;
    if frames.next().is_some() {
        return Err("diagnostic-duplicate");
    }
    Ok(frame)
}
pub fn private_cli_stream(bytes: &[u8], stdout: bool) -> Result<Vec<u8>> {
    let frame = diagnostic_frame(
        bytes,
        if stdout {
            "APACHE_PRIVATE_CLI_STDOUT "
        } else {
            "APACHE_PRIVATE_CLI_STDERR "
        },
    )?;
    if frame.len() > 131_072 || !frame.len().is_multiple_of(2) || !hex(frame, frame.len()) {
        return Err("diagnostic-private-bound");
    }
    ::hex::decode(frame).map_err(|_| "diagnostic-private-encoding")
}
pub fn cli_diagnostic(bytes: &[u8]) -> Result<Value> {
    let frame = diagnostic_frame(bytes, "APACHE_CLI_DIAGNOSTIC ")?;
    if frame.len() > 4096 {
        return Err("diagnostic-bound");
    }
    let value = parse(frame.as_bytes())?;
    let allowed = "schema mpm role nestedProof pidFileMatches cliStarted cliReaped cliExit cliSignal cliTimedOut outputLimited stdoutBytes stderrBytes apacheCodes masterBeforeAlive masterAfterCliAlive masterExit masterSignal masterReaped forcedCleanup cleanupPassed"
        .split_ascii_whitespace().map(str::to_owned).collect();
    fields(&value, &allowed)?;
    if value["schema"] != "humanymous.apache.cli-diagnostic/0.1"
        || value["mpm"] != "prefork"
        || value["role"] != "isolated-cli-stop-observation"
    {
        return Err("diagnostic-role");
    }
    for key in [
        "nestedProof",
        "cliStarted",
        "cliReaped",
        "cliTimedOut",
        "outputLimited",
        "masterBeforeAlive",
        "masterAfterCliAlive",
        "masterReaped",
        "forcedCleanup",
        "cleanupPassed",
    ] {
        if !value[key].is_boolean() {
            return Err("diagnostic-boolean");
        }
    }
    if !(value["pidFileMatches"].is_null() || value["pidFileMatches"].is_boolean()) {
        return Err("diagnostic-pid-proof");
    }
    for key in ["stdoutBytes", "stderrBytes"] {
        if !value[key].as_u64().is_some_and(|size| size <= 65_536) {
            return Err("diagnostic-output-bound");
        }
    }
    for (exit, signal, reaped) in [
        ("cliExit", "cliSignal", "cliReaped"),
        ("masterExit", "masterSignal", "masterReaped"),
    ] {
        let code = value[exit].as_u64();
        let sig = value[signal].as_u64();
        if (!value[exit].is_null() && !code.is_some_and(|c| c <= 255))
            || (!value[signal].is_null() && !sig.is_some_and(|s| (1..=64).contains(&s)))
            || (code.is_some() && sig.is_some())
            || (value[reaped] == true) != (code.is_some() || sig.is_some())
        {
            return Err("diagnostic-exit");
        }
    }
    let codes = value["apacheCodes"].as_array().ok_or("diagnostic-codes")?;
    let mut seen = BTreeSet::new();
    for code in codes {
        let code = code.as_str().ok_or("diagnostic-code")?;
        if !APACHE_CODES.contains(&code) || !seen.insert(code) {
            return Err("diagnostic-code");
        }
    }
    if value["cliStarted"] != value["cliReaped"]
        || value["masterBeforeAlive"] != true
        || (value["cleanupPassed"] == true
            && (value["forcedCleanup"] != false
                || value["cliTimedOut"] != false
                || value["outputLimited"] != false
                || value["masterReaped"] != true
                || value["masterExit"] != 0))
    {
        return Err("diagnostic-consistency");
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
    fn cli_fixture() -> Value {
        serde_json::json!({"schema":"humanymous.apache.cli-diagnostic/0.1","mpm":"prefork","role":"isolated-cli-stop-observation","nestedProof":true,"pidFileMatches":true,"cliStarted":true,"cliReaped":true,"cliExit":1,"cliSignal":null,"cliTimedOut":false,"outputLimited":false,"stdoutBytes":0,"stderrBytes":7,"apacheCodes":["AH00058"],"masterBeforeAlive":true,"masterAfterCliAlive":true,"masterExit":0,"masterSignal":null,"masterReaped":true,"forcedCleanup":false,"cleanupPassed":true})
    }
    #[test]
    fn cli_projection_rejects_private_fields_and_contradictory_facts() {
        let frame = |value: &Value| format!("APACHE_CLI_DIAGNOSTIC {value}\n");
        let valid = cli_fixture();
        assert!(cli_diagnostic(frame(&valid).as_bytes()).is_ok());
        for (key, replacement) in [
            ("privatePath", serde_json::json!("/secret")),
            ("role", serde_json::json!("/secret")),
            ("apacheCodes", serde_json::json!(["secret"])),
            ("stdoutBytes", serde_json::json!(65537)),
            ("cliSignal", serde_json::json!(9)),
            ("masterReaped", serde_json::json!(false)),
            ("forcedCleanup", serde_json::json!(true)),
        ] {
            let mut bad = valid.clone();
            bad[key] = replacement;
            assert!(cli_diagnostic(frame(&bad).as_bytes()).is_err(), "{key}");
        }
        assert!(cli_diagnostic(frame(&valid).repeat(2).as_bytes()).is_err());
        assert_eq!(
            apache_error_codes(b"private AH00058 AH00058 AH12345"),
            ["AH00058"]
        );
    }
    #[test]
    fn private_cli_bytes_remain_bounded_and_unambiguous() {
        assert_eq!(
            private_cli_stream(b"APACHE_PRIVATE_CLI_STDOUT 00ff\n", true).unwrap(),
            [0, 255]
        );
        for bytes in [
            "APACHE_PRIVATE_CLI_STDOUT gg\n".to_owned(),
            "APACHE_PRIVATE_CLI_STDOUT 0\n".to_owned(),
            "APACHE_PRIVATE_CLI_STDOUT 00\n".repeat(2),
            format!("APACHE_PRIVATE_CLI_STDOUT {}\n", "00".repeat(65537)),
        ] {
            assert!(private_cli_stream(bytes.as_bytes(), true).is_err());
        }
        assert!(private_cli_stream(b"APACHE_PRIVATE_CLI_STDERR 00\n", true).is_err());
    }
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
    #[test]
    fn case_diagnostics_bind_failed_inventory_and_never_project_arbitrary_text() {
        let (mut value, _) = fixture();
        value["cases"][0]["result"] = "failed".into();
        value["passed"] = 44.into();
        value["failed"] = 1.into();
        let failed = value["cases"][0]["id"].as_str().unwrap().to_owned();
        let passed = value["cases"][1]["id"].as_str().unwrap().to_owned();
        value["cases"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"id":"private/secret","result":"failed"}));
        let bytes = format!(
            "secret=private\nAPACHE_CASE_FAILURE {failed} binary-replay-effect\nAPACHE_CASE_FAILURE {failed} private-body\nAPACHE_CASE_FAILURE {passed} spoof-stripping\nAPACHE_CASE_FAILURE private/secret binary-replay-effect\n"
        );
        let result = case_diagnostics(&value, bytes.as_bytes());
        assert_eq!(result[failed.as_str()], "binary-replay-effect");
        assert_eq!(result.as_object().unwrap().len(), 1);
        assert!(!result.to_string().contains("private"));
        assert_eq!(case_failure_label("credential=private"), "operation-failed");
        let private_io =
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "credential=private");
        assert_eq!(case_error_label(&private_io), "io-connection-refused");
        assert_eq!(
            case_io_failure_label(std::io::ErrorKind::ConnectionRefused),
            "io-connection-refused"
        );
        assert_eq!(
            case_io_failure_label(std::io::ErrorKind::Other),
            "operation-failed"
        );
        let io = case_diagnostics(
            &value,
            format!("APACHE_CASE_FAILURE {failed} io-connection-refused\n").as_bytes(),
        );
        assert_eq!(io[failed.as_str()], "io-connection-refused");
        assert_eq!(
            case_failure_label("binary replay/effect"),
            "binary-replay-effect"
        );
    }
}
