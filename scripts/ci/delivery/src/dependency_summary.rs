//! Small public Cargo audit accounting; raw scanner reports stay private.
use serde_json::{Value, json};

fn advisory_ids(values: impl Iterator<Item = Value>) -> Vec<Value> {
    values
        .filter_map(|value| {
            let id = value["advisory"]["id"].as_str()?;
            (id.starts_with("RUSTSEC-")
                && id.len() == 17
                && id.bytes().all(|c| {
                    c.is_ascii_digit()
                        || matches!(c, b'R' | b'U' | b'S' | b'T' | b'E' | b'C' | b'-')
                }))
            .then(|| json!(id))
        })
        .collect()
}
pub fn format(summary: &Value) -> Value {
    let reports: Vec<_> = summary["reports"].as_array().into_iter().flatten().map(|report| {
        let warning_count = report["warnings"].as_object().and_then(|warnings| warnings.values().try_fold(0usize, |count, rows| rows.as_array().map(|rows| count + rows.len())));
        let warning_ids = advisory_ids(report["warnings"].as_object().into_iter().flat_map(|warnings| warnings.values()).filter_map(Value::as_array).flatten().cloned());
        let vulnerability_ids = advisory_ids(report["vulnerabilities"]["list"].as_array().into_iter().flatten().cloned());
        json!({"lockfile":report["lockfile"],"sha256":report["sha256"],"sha256_after":report["sha256_after"],
            "source_changed":report["source_changed"],"exit_code":report["exit_code"],"status":report["status"],
            "dependency_count":report["dependency_count"],"vulnerabilities_found":report["vulnerabilities"]["found"],
            "vulnerability_count":report["vulnerabilities"]["count"],"vulnerability_ids":vulnerability_ids,
            "warning_count":warning_count,"warning_ids":warning_ids})
    }).collect();
    json!({"schema":"humanymous.dependency-closure-summary/1","source_sha":summary["source"]["source_sha"],
        "source_manifest_sha256":summary["source"]["manifest_sha256"],"reports":reports,
        "blocked":summary["blocked"],"retired_locks":summary["retired_locks"],"scope":"Cargo only; other ecosystems and images remain separate"})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_scanner_payloads_are_not_public_and_unknown_counts_stay_unknown() {
        let report = json!({"reports":[{"error":"private-canary","warnings":{"unmaintained":[{"advisory":{"id":"RUSTSEC-2026-0192","description":"private-canary"}}]}}]});
        let public = format(&report);
        assert!(!public.to_string().contains("private-canary"));
        assert_eq!(public["reports"][0]["warning_count"], 1);
        assert_eq!(public["reports"][0]["warning_ids"][0], "RUSTSEC-2026-0192");
        assert!(public["reports"][0]["dependency_count"].is_null());
    }
}
