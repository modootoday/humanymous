//! Source-bound private panic locations become fixed diagnostic codes only.
const SOURCE: &[u8] =
    include_bytes!("../../../../../../crates/collector/src/browser/transport/runtime_pilot.rs");
const SOURCE_SHA256: &str = "c61b764905884233d7dc814cd6179e4e31e439921314d8a83a4475bbdadf4064";
const PATH: &str = "crates/collector/src/browser/transport/runtime_pilot.rs";
const NAME: &str = "browser::transport::runtime_pilot::actual_runtime_session_report_and_required_capability_fail_closed";

// Each line identifies one fixed assertion or fallible call, never its values.
const SITES: &[(usize, &str, &str)] = &[
    (11, "browser_panic_fixture_json", "serde_json::from_str"),
    (12, "browser_panic_fixture_scope", "let scope:"),
    (14, "browser_panic_location_host", "&get(&get("),
    (17, "browser_panic_location_text", ".unwrap()"),
    (18, "browser_panic_owned_origin", "assert_eq!(authority"),
    (
        22,
        "browser_panic_bootstrap",
        ".expect(\"actual runtime bootstrap\")",
    ),
    (
        32,
        "browser_panic_effect_ack",
        ".expect(\"actual consumed request",
    ),
    (
        33,
        "browser_panic_effect_status",
        "assert_eq!(response.status()",
    ),
    (34, "browser_panic_effect_json", "serde_json::from_slice"),
    (
        35,
        "browser_panic_effect_verdict",
        "assert_eq!(body[\"verdict\"]",
    ),
    (
        36,
        "browser_panic_effect_epoch",
        "assert!(!session.is_closed())",
    ),
    (
        37,
        "browser_panic_response_headers_json",
        "serde_json::from_str",
    ),
    (38, "browser_panic_response_headers_hidden", "assert!("),
    (46, "browser_panic_oversize_local_refusal", "assert!("),
    (
        57,
        "browser_panic_oversize_epoch",
        "assert!(!session.is_closed())",
    ),
    (
        69,
        "browser_panic_maximum_body_ack",
        ".expect(\"maximum supported body",
    ),
    (70, "browser_panic_maximum_body_status", "assert_eq!("),
    (74, "browser_panic_fixture_status", ".unwrap()"),
    (
        76,
        "browser_panic_maximum_body_json",
        "serde_json::from_slice",
    ),
    (77, "browser_panic_maximum_body_verdict", "assert_eq!("),
    (
        81,
        "browser_panic_maximum_body_epoch",
        "assert!(!session.is_closed())",
    ),
    (
        85,
        "browser_panic_collector_start",
        "BrowserCollector::new()",
    ),
    (
        86,
        "browser_panic_collector_snapshot",
        "collector.snapshot_json()",
    ),
    (87, "browser_panic_collector_stop", "collector.stop()"),
    (
        96,
        "browser_panic_collect_ack",
        ".expect(\"actual report ingestion",
    ),
    (
        97,
        "browser_panic_collect_status",
        "assert_eq!(collected.status()",
    ),
    (98, "browser_panic_collect_json", "serde_json::from_slice"),
    (
        99,
        "browser_panic_collect_verdict",
        "assert_eq!(body[\"verdict\"]",
    ),
    (
        100,
        "browser_panic_collect_epoch",
        "assert!(!session.is_closed())",
    ),
    (104, "browser_panic_required_refusal", "assert!("),
    (
        115,
        "browser_panic_required_epoch",
        "assert!(session.is_closed())",
    ),
    (
        116,
        "browser_panic_session_close",
        "session.close().unwrap()",
    ),
    (117, "browser_panic_closed_refusal", "assert!("),
];

fn canonical_number(value: &str) -> Option<usize> {
    if value.is_empty()
        || value.len() > 3
        || value.starts_with('0')
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    value.parse().ok()
}

fn from_source(output: &str, stderr: &str, name: &str, source: &[u8]) -> Option<&'static str> {
    if name != NAME || crate::snapshot::digest(source) != SOURCE_SHA256 {
        return None;
    }
    let text = std::str::from_utf8(source).ok()?;
    let prefix = format!("panicked at {PATH}:");
    let mut found = None;
    for line in output.lines().chain(stderr.lines()).map(str::trim) {
        if !line.starts_with("panicked at ") {
            continue;
        }
        let location = line.strip_prefix(&prefix)?.strip_suffix(':')?;
        let (line_number, column) = location.split_once(':')?;
        let line_number = canonical_number(line_number)?;
        let column = canonical_number(column)?;
        let (_, code, _) = SITES.iter().find(|(number, _, _)| *number == line_number)?;
        if column > text.lines().nth(line_number - 1)?.len() || found.is_some() {
            return None;
        }
        found = Some(*code);
    }
    found
}

pub(super) fn classify(output: &str, stderr: &str, name: &str) -> Option<&'static str> {
    from_source(output, stderr, name, SOURCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn source_hash_sites_and_public_vocabulary_are_one_pinned_contract() {
        assert_eq!(crate::snapshot::digest(SOURCE), SOURCE_SHA256);
        let source = std::str::from_utf8(SOURCE).unwrap();
        let contract: serde_json::Value =
            serde_json::from_str(include_str!("../../../safe-diagnostics.json")).unwrap();
        let codes = contract["error_codes"].as_array().unwrap();
        let mut lines = HashSet::new();
        let mut names = HashSet::new();
        for (line, code, fragment) in SITES {
            assert!(lines.insert(line));
            assert!(names.insert(code));
            assert!(source.lines().nth(line - 1).unwrap().contains(fragment));
            assert!(codes.iter().any(|value| value.as_str() == Some(*code)));
            let observation = format!("  panicked at {PATH}:{line}:5:");
            assert_eq!(classify(&observation, "", NAME), Some(*code));
        }
    }

    #[test]
    fn drift_payloads_unknown_and_duplicate_locations_do_not_refine() {
        let valid = format!("panicked at {PATH}:22:10:");
        let mut drift = SOURCE.to_vec();
        drift.push(b' ');
        assert_eq!(from_source(&valid, "", NAME, &drift), None);
        assert_eq!(classify(&valid, "", "foreign::test"), None);
        for bad in [
            format!("{valid} PRIVATE_BODY"),
            format!("panicked at /private/{PATH}:22:10:"),
            format!("panicked at {PATH}:022:10:"),
            format!("panicked at {PATH}:22:0:"),
            format!("panicked at {PATH}:22:999:"),
            format!("panicked at {PATH}:23:5:"),
            format!("panicked at {PATH}:22:10:extra:"),
            "panicked at PRIVATE_PATH".into(),
        ] {
            assert_eq!(classify(&bad, "", NAME), None);
            assert_eq!(classify(&valid, &bad, NAME), None);
        }
        assert_eq!(classify(&valid, &valid, NAME), None);
        assert_eq!(classify("PRIVATE_BODY", "", NAME), None);
    }
}
