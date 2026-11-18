//! Derived product claim: a capability is available only when every one of its
//! required cases was evaluated in this run and passed. Nothing is hard-coded
//! as available, and absence is never counted as success.
use serde_json::{Value, json};

/// Required product capability -> case IDs that must each pass in this run.
/// Pilot-required IDs (scripts/ci/delivery pilot docker/browser kinds) are all
/// covered here, so a lab report can never claim more than the pilot requires.
pub(super) const CAPABILITIES: &[(&str, &[&str])] = &[
    ("full Gate TLS/H2/capture", &["gate", "raw-tls-h2"]),
    (
        "session-bound real browser collector",
        &["collector-parity", "missing-signals-closed"],
    ),
    ("Pass recovery", &["pass"]),
    (
        "human/keyboard/assistive controls",
        &["human-control", "challenge-keyboard", "challenge-assistive"],
    ),
    (
        "runtime binary authenticated witness transport",
        &["runtime-binary-witness-transport"],
    ),
    (
        "cross-namespace origin/audit effect product binding",
        &["origin-isolation", "audit-restart"],
    ),
    (
        "signed remote target attestation",
        &["installation-target-binding", "unknown-target-refused"],
    ),
    ("native OS qualification", &["native-os-qualification"]),
    ("provider deployment", &["deployment"]),
    (
        "product attack catalog and swarm",
        &["attack-catalog", "swarm"],
    ),
    ("coherent browser ceiling", &["coherent-browser-ceiling"]),
    ("documentation examples", &["documentation-examples"]),
];

pub(super) struct Qualification {
    /// Capability labels with any required case not evaluated-and-passed.
    pub required_unavailable: Vec<&'static str>,
    /// Per required case: passed, failed or not_evaluated.
    pub required_cases: Vec<Value>,
    /// Required cases with no evaluation at all in this run.
    pub not_evaluated: usize,
    /// Failed lab cases (required or not).
    pub errors: usize,
    /// True only with every required case passed, no failure and owned cleanup.
    pub full_product_qualified: bool,
}

fn status(cases: &[Value], id: &str) -> &'static str {
    let mut seen = false;
    for case in cases.iter().filter(|case| case["id"] == id) {
        if case["status"] != "passed" {
            return "failed";
        }
        seen = true;
    }
    if seen { "passed" } else { "not_evaluated" }
}

pub(super) fn assess(cases: &[Value], cleanup_verified: bool) -> Qualification {
    let mut required_unavailable = Vec::new();
    let mut required_cases = Vec::new();
    let mut not_evaluated = 0;
    for (capability, ids) in CAPABILITIES {
        let mut available = true;
        for id in *ids {
            let state = status(cases, id);
            available &= state == "passed";
            not_evaluated += usize::from(state == "not_evaluated");
            required_cases.push(json!({"id":id,"capability":capability,"status":state}));
        }
        if !available {
            required_unavailable.push(*capability);
        }
    }
    let errors = cases
        .iter()
        .filter(|case| case["status"] != "passed")
        .count();
    let full_product_qualified = required_unavailable.is_empty() && errors == 0 && cleanup_verified;
    Qualification {
        required_unavailable,
        required_cases,
        not_evaluated,
        errors,
        full_product_qualified,
    }
}

impl Qualification {
    /// Top-level fields consumed by the delivery pilot.
    pub(super) fn report_fields(&self) -> Value {
        json!({
            "status": if self.errors > 0 { "failed" } else if self.full_product_qualified { "passed" } else { "blocked" },
            "qualification": if self.full_product_qualified { "full-product" } else { "component-only" },
            "errors": self.errors,
            "skips": self.not_evaluated,
            "required_unavailable": self.required_unavailable,
            "required_cases": self.required_cases,
            "full_product_qualified": self.full_product_qualified,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_required(status: &str) -> Vec<Value> {
        CAPABILITIES
            .iter()
            .flat_map(|(_, ids)| ids.iter())
            .map(|id| json!({"id":id,"status":status}))
            .collect()
    }

    #[test]
    fn todays_lab_cannot_qualify_and_lists_every_missing_capability() {
        let cases = vec![
            json!({"id":"installation-target-binding","status":"passed"}),
            json!({"id":"unknown-target-refused","status":"passed"}),
            json!({"id":"actual-runtime-process","status":"passed"}),
        ];
        let result = assess(&cases, true);
        assert!(!result.full_product_qualified);
        assert_eq!(result.required_unavailable.len(), CAPABILITIES.len() - 1);
        assert!(
            !result
                .required_unavailable
                .contains(&"signed remote target attestation")
        );
        assert!(
            result
                .required_unavailable
                .contains(&"human/keyboard/assistive controls")
        );
        let fields = result.report_fields();
        assert_eq!(fields["status"], "blocked");
        assert_eq!(fields["qualification"], "component-only");
        assert_eq!(fields["errors"], 0);
        assert_eq!(
            fields["skips"],
            CAPABILITIES.iter().map(|(_, ids)| ids.len()).sum::<usize>() - 2
        );
        assert_eq!(fields["full_product_qualified"], false);
    }

    #[test]
    fn partial_failed_or_duplicated_failure_never_counts_as_available() {
        let mut cases = vec![json!({"id":"installation-target-binding","status":"passed"})];
        let result = assess(&cases, true);
        assert!(
            result
                .required_unavailable
                .contains(&"signed remote target attestation")
        );
        cases.push(json!({"id":"unknown-target-refused","status":"passed"}));
        cases.push(json!({"id":"unknown-target-refused","status":"failed"}));
        let result = assess(&cases, true);
        assert!(
            result
                .required_unavailable
                .contains(&"signed remote target attestation")
        );
        assert_eq!(result.report_fields()["status"], "failed");
        assert_eq!(result.errors, 1);
    }

    #[test]
    fn full_claim_needs_every_case_no_failure_and_cleanup() {
        let complete = all_required("passed");
        let result = assess(&complete, true);
        assert!(result.full_product_qualified);
        assert!(result.required_unavailable.is_empty());
        assert_eq!(result.report_fields()["qualification"], "full-product");
        assert_eq!(result.report_fields()["skips"], 0);
        assert!(!assess(&complete, false).full_product_qualified);
        let mut failed = complete.clone();
        failed.push(json!({"id":"cleanup","status":"failed"}));
        assert!(!assess(&failed, true).full_product_qualified);
        let mut missing = complete;
        missing.retain(|case| case["id"] != "human-control");
        let result = assess(&missing, true);
        assert!(!result.full_product_qualified);
        assert_eq!(
            result.required_unavailable,
            vec!["human/keyboard/assistive controls"]
        );
        assert!(!assess(&all_required("blocked"), true).full_product_qualified);
    }

    #[test]
    fn every_pilot_required_case_is_a_required_lab_case() {
        let ids: std::collections::BTreeSet<_> = CAPABILITIES
            .iter()
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect();
        for id in [
            "attack-catalog",
            "gate",
            "pass",
            "swarm",
            "deployment",
            "human-control",
            "raw-tls-h2",
            "audit-restart",
            "origin-isolation",
            "documentation-examples",
            "collector-parity",
            "challenge-keyboard",
            "challenge-assistive",
            "missing-signals-closed",
            "coherent-browser-ceiling",
        ] {
            assert!(ids.contains(id), "{id}");
        }
    }
}
