//! G7 pinned-key fresh-nonce target attestation, executed in its own hardened
//! network-none container. The key is generated there by the shipped runtime
//! binary at lab start; nothing is committed, mounted or exported.
use super::Lab;
use crate::{docker::Result, snapshot};
use serde_json::{Value, json};

/// Exact in-container case ledger; unknown or missing rows are not a pass.
const BINDING: &[&str] = &[
    "enrollment-binds-configuration",
    "installation-target-binding",
    "stale-nonce-refused",
    "tampered-signature-refused",
    "changed-configuration-refused",
    "positive-control-after-negatives",
];
const UNKNOWN: &[&str] = &["unknown-key-refused", "mismatched-installation-refused"];

fn complete(report: &Value) -> bool {
    report["cases"].as_array().is_some_and(|rows| {
        rows.len() == BINDING.len() + UNKNOWN.len()
            && BINDING
                .iter()
                .chain(UNKNOWN)
                .all(|id| rows.iter().filter(|row| row["id"] == *id).count() == 1)
    })
}

fn passed(report: &Value, ids: &[&str]) -> bool {
    complete(report)
        && report["schema"] == "humanymous.target-attestation-lab/1"
        && report["private_key_exported"] == false
        && report["full_product_qualified"] == false
        && ids.iter().all(|id| {
            report["cases"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|row| row["id"] == *id && row["status"] == "passed")
        })
}

pub(super) fn run(lab: &mut Lab) -> Result<()> {
    lab.compose(&["create", "attestation-pilot"], true)?;
    let container = lab.inspect("attestation-pilot")?;
    lab.case(
        "attestation-network-none",
        container["HostConfig"]["NetworkMode"] == "none",
        json!("supervisor-owned runtime binary invocations only; no remote target arguments"),
    )?;
    let id = container["Id"]
        .as_str()
        .ok_or("attestation ID absent")?
        .to_owned();
    let result = lab
        .docker
        .invoke(&["start".into(), "-a".into(), id], &[], false)?;
    let report = humanymous_platform::testing::json::decode_strict_json(
        result.stdout.as_bytes(),
        1_048_576,
    )?;
    snapshot::write(&lab.evidence, "target-attestation-report.json", &report)?;
    lab.components["target_attestation"] = report.clone();
    let status = result.success && report["status"] == "passed";
    lab.case(
        "installation-target-binding",
        status && passed(&report, BINDING),
        json!("runtime-generated in-topology key pinned at enrollment; fresh nonce over installation, binary and configuration digests verified"),
    )?;
    lab.case(
        "unknown-target-refused",
        status && passed(&report, UNKNOWN),
        json!(
            "foreign in-topology key and mismatched installation refused by target and challenger"
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> Value {
        let rows: Vec<_> = BINDING
            .iter()
            .chain(UNKNOWN)
            .map(|id| json!({"id":id,"status":"passed"}))
            .collect();
        json!({"schema":"humanymous.target-attestation-lab/1","status":"passed","cases":rows,
            "private_key_exported":false,"full_product_qualified":false})
    }

    #[test]
    fn complete_exact_ledger_is_required() {
        let good = report();
        assert!(passed(&good, BINDING) && passed(&good, UNKNOWN));
        let mut missing = good.clone();
        missing["cases"].as_array_mut().unwrap().pop();
        assert!(!passed(&missing, BINDING));
        let mut duplicate = good.clone();
        let first = duplicate["cases"][0].clone();
        duplicate["cases"].as_array_mut().unwrap()[1] = first;
        assert!(!passed(&duplicate, BINDING));
        let mut failed = good.clone();
        failed["cases"][7]["status"] = json!("failed");
        assert!(!passed(&failed, UNKNOWN));
        assert!(passed(&failed, BINDING));
        for (field, value) in [
            ("private_key_exported", json!(true)),
            ("full_product_qualified", json!(true)),
            ("schema", json!("other")),
        ] {
            let mut bad = good.clone();
            bad[field] = value;
            assert!(!passed(&bad, BINDING));
        }
    }
}
