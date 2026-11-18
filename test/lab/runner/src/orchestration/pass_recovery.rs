//! Roadmap 2e Pass recovery, executed in its own hardened network-none
//! container: the shipped runtime binary serves an `attest_floor` route and the
//! legacy Pass surface, and an in-container Rust client is challenged, solves
//! the Pass, redeems the step-up receipt and is admitted. Refusals are asserted
//! on the wire and on the owned origin's contact counter.
use super::Lab;
use crate::{docker::Result, snapshot};
use serde_json::{Value, json};

/// Exact in-container ledger (test/lab/runtime/src/pass.rs); unknown or
/// missing rows are not a pass.
const RECOVERY: &[&str] = &[
    "attested-route-challenged",
    "pass-instance-issued",
    "proof-of-work-accepted",
    "solve-accepted-with-receipt",
    "receipt-redeemed",
    "recovered-session-admitted",
    "positive-control-after-negatives",
];
const REFUSED: &[&str] = &[
    "missing-pow-refused",
    "missing-receipt-refused",
    "forged-receipt-refused",
    "wrong-session-receipt-refused",
    "lifted-proof-refused",
    "wrong-session-proof-refused",
    "scoring-challenge-not-overridden",
    "replayed-solve-refused",
    "expired-receipt-refused",
];

fn complete(report: &Value) -> bool {
    report["cases"].as_array().is_some_and(|rows| {
        rows.len() == RECOVERY.len() + REFUSED.len()
            && RECOVERY
                .iter()
                .chain(REFUSED)
                .all(|id| rows.iter().filter(|row| row["id"] == *id).count() == 1)
    })
}

fn passed(report: &Value, ids: &[&str]) -> bool {
    complete(report)
        && report["schema"] == "humanymous.pass-recovery-lab/1"
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
    lab.compose(&["create", "pass-pilot"], true)?;
    let container = lab.inspect("pass-pilot")?;
    lab.case(
        "pass-network-none",
        container["HostConfig"]["NetworkMode"] == "none",
        json!(
            "runtime binary, owned loopback origin and Rust client share one network-none container"
        ),
    )?;
    let id = container["Id"]
        .as_str()
        .ok_or("pass pilot ID absent")?
        .to_owned();
    let result = lab
        .docker
        .invoke(&["start".into(), "-a".into(), id], &[], false)?;
    let report = humanymous_platform::testing::json::decode_strict_json(
        result.stdout.as_bytes(),
        1_048_576,
    )?;
    snapshot::write(&lab.evidence, "pass-recovery-report.json", &report)?;
    lab.components["pass_recovery"] = report.clone();
    let status = result.success && report["status"] == "passed";
    lab.case(
        "pass",
        status && passed(&report, RECOVERY),
        json!("attest_floor route challenged; Pass instance, proof of work and keyboard-lane solve; receipt redeemed for a socket-bound proof; recovered session admitted with one origin effect; positive control after negatives"),
    )?;
    lab.case(
        "pass-forgery-refused",
        status && passed(&report, REFUSED),
        json!("missing proof of work, missing/forged/wrong-session/expired receipt, lifted and wrong-session proof, replayed solve refused; a proof never overrides a scoring CHALLENGE"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> Value {
        let rows: Vec<_> = RECOVERY
            .iter()
            .chain(REFUSED)
            .map(|id| json!({"id":id,"status":"passed"}))
            .collect();
        json!({"schema":"humanymous.pass-recovery-lab/1","status":"passed","cases":rows,
            "full_product_qualified":false})
    }

    #[test]
    fn complete_exact_ledger_is_required() {
        let good = report();
        assert!(passed(&good, RECOVERY) && passed(&good, REFUSED));
        let mut missing = good.clone();
        missing["cases"].as_array_mut().unwrap().pop();
        assert!(!passed(&missing, RECOVERY));
        let mut duplicate = good.clone();
        let first = duplicate["cases"][0].clone();
        duplicate["cases"].as_array_mut().unwrap()[1] = first;
        assert!(!passed(&duplicate, REFUSED));
        let mut failed = good.clone();
        failed["cases"][RECOVERY.len()]["status"] = json!("failed");
        assert!(!passed(&failed, REFUSED));
        assert!(passed(&failed, RECOVERY));
        let mut extra = good.clone();
        extra["cases"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"unknown","status":"passed"}));
        assert!(!passed(&extra, RECOVERY));
        for (field, value) in [
            ("full_product_qualified", json!(true)),
            ("schema", json!("other")),
        ] {
            let mut bad = good.clone();
            bad[field] = value;
            assert!(!passed(&bad, RECOVERY));
        }
    }

    /// The runner ledger is the runtime pilot ledger, verbatim and in order.
    #[test]
    fn ledger_matches_the_runtime_pilot_source() {
        let source = include_str!("../../../runtime/src/pass.rs");
        for (name, ids) in [("RECOVERY", RECOVERY), ("REFUSED", REFUSED)] {
            let start = source
                .find(&format!("pub const {name}: &[&str] = &["))
                .unwrap();
            let block = &source[start..];
            let block = &block[block.find('[').unwrap()..block.find("];").unwrap()];
            let declared: Vec<&str> = block
                .lines()
                .filter_map(|line| line.trim().strip_prefix('"')?.strip_suffix("\","))
                .collect();
            assert_eq!(declared, ids, "{name}");
        }
    }
}
