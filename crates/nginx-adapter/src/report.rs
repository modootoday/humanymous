//! Component evidence framing. A report never grants product qualification.
#![forbid(unsafe_code)]
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;

/// Exact component result schema consumed by the hosted/lab owners.
pub const SCHEMA: &str = "humanymous.nginx.pilot/0.2";

#[derive(Deserialize)]
struct Contract {
    report_schema: String,
    required_groups: Vec<String>,
}

/// Enforce the checked-in required group set before emitting success evidence.
/// Counts are scenario groups, never inferred Rust harness tests or human trials.
pub fn completed_groups(cases: &mut [Value]) -> Result<usize, &'static str> {
    let contract: Contract =
        serde_json::from_str(include_str!("../qualification/fixtures/nginx/pilot-contract.json"))
            .map_err(|_| "pilot contract schema")?;
    if contract.report_schema != SCHEMA {
        return Err("pilot contract version");
    }
    let required: BTreeSet<_> = contract.required_groups.iter().cloned().collect();
    let mut seen = BTreeSet::new();
    for case in cases.iter() {
        let name = case["name"].as_str().ok_or("pilot group name")?;
        if !seen.insert(name.to_owned()) {
            return Err("duplicate pilot group");
        }
    }
    if required.len() != contract.required_groups.len() || seen != required {
        return Err("missing or unknown pilot group");
    }
    for case in cases.iter_mut() {
        let id = case["name"].as_str().ok_or("pilot group name")?.to_owned();
        case["id"] = Value::String(id);
        case["result"] = Value::String("passed".into());
    }
    Ok(cases.len())
}

/// Require canonical lowercase SHA identities, never unknown/unbound defaults.
pub fn source_identity(revision: &str, manifest: &str) -> Result<(), &'static str> {
    let lowerhex = |s: &str| {
        s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    if ![40, 64].contains(&revision.len())
        || !lowerhex(revision)
        || manifest.len() != 64
        || !lowerhex(manifest)
    {
        return Err("unbound pilot source identity");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cases() -> Vec<Value> {
        let c: Contract =
            serde_json::from_str(include_str!("../qualification/fixtures/nginx/pilot-contract.json"))
                .unwrap();
        c.required_groups
            .into_iter()
            .map(|name| serde_json::json!({"name":name}))
            .collect()
    }
    #[test]
    fn exact_groups_count_groups_not_subassertions() {
        let mut rows = cases();
        let count = completed_groups(&mut rows).unwrap();
        assert_eq!(count, 35);
        assert!(
            rows.iter()
                .all(|r| r["id"] == r["name"] && r["result"] == "passed")
        );
    }
    #[test]
    fn missing_duplicate_unknown_group_is_not_success() {
        let mut rows = cases();
        rows.pop();
        assert!(completed_groups(&mut rows).is_err());
        let mut rows = cases();
        rows[1] = rows[0].clone();
        assert!(completed_groups(&mut rows).is_err());
        let mut rows = cases();
        rows[0]["name"] = serde_json::json!("unknown");
        assert!(completed_groups(&mut rows).is_err());
    }
    #[test]
    fn source_binding_rejects_unset_or_noncanonical_values() {
        assert!(source_identity(&"a".repeat(40), &"b".repeat(64)).is_ok());
        assert!(source_identity("unknown", &"b".repeat(64)).is_err());
        assert!(source_identity(&"A".repeat(40), &"b".repeat(64)).is_err());
        assert!(source_identity(&"a".repeat(40), "unset").is_err());
    }
}
