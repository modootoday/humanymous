//! Increment 5: the owned attack catalog run, and the required-case ledger.
//!
//! The ledger is never hard-coded: it is `orchestration::qualification::assess`
//! (the same derivation the `run` profile reports) over only the required case
//! IDs this run honestly evaluates. Today that is `attack-catalog`, from the
//! network-none owned process wargame, and it cannot pass while any catalog
//! entry is unsupported. Every other required case stays `not_evaluated`, with
//! the reason a related component measurement here does not satisfy it.
use super::{Lab, Result};
use crate::orchestration::qualification;
use serde_json::{Value, json};

pub(super) const WARGAME: &str = "humanymous-lab-product-e2e-wargame";

/// Required IDs this topology measures something near, and why that
/// measurement is not the required case. Unlisted IDs have no producer here.
const NOT_MAPPED: &[(&str, &str)] = &[
    (
        "gate",
        "the shipped-binary Gate runs the plaintext loopback listener in one shared namespace with origin and attacker; its TLS listener and a separate client namespace are not driven",
    ),
    (
        "raw-tls-h2",
        "no raw ClientHello or H2 frames reach any Gate in this topology",
    ),
    (
        "collector-parity",
        "the real WASM collector browser test is a component; no parity oracle or denominator is defined for it",
    ),
    (
        "missing-signals-closed",
        "the pilot defines this as a browser-kind case (session-bound real browser collector; client-collector SoT: pre-consumption 422 without ACK that closes the client epoch). This run measures 422 for sessionless synthetic HTTP requests to routes requiring raw TLS or collector capability, which is not that case",
    ),
    (
        "runtime-binary-witness-transport",
        "the witnessed and browser Gates are the runtime library in a lab supervisor, not the shipped binary over inherited witness pipes",
    ),
    (
        "origin-isolation",
        "origin, Gate and attacker share one loopback namespace by construction",
    ),
    (
        "audit-restart",
        "only the audit-outage half is measured (qualifies_product_case false); the restart/replay and cross-namespace halves are absent",
    ),
];

/// Run the owned catalog wargame from the built image with no network.
pub(super) fn wargame(lab: &mut Lab, compose: &[&str]) -> Result<Value> {
    let run = [
        compose,
        &[
            "run",
            "--no-deps",
            "--no-TTY",
            "--pull",
            "never",
            "--name",
            WARGAME,
            "wargame",
        ],
    ]
    .concat();
    let output = lab.docker(&run, 900, false)?;
    let container = lab.inspect(WARGAME)?;
    let report: Option<Value> = serde_json::from_str(output.stdout.trim()).ok();
    let receipts: Vec<Value> = output
        .stderr
        .lines()
        .filter_map(|line| line.strip_prefix("HM-WARGAME-VERIFICATION "))
        .filter_map(|text| serde_json::from_str(text).ok())
        .collect();
    let raw_sha256 = crate::snapshot::digest(output.stdout.as_bytes());
    Ok(judge(
        output.success,
        &container,
        report.as_ref(),
        &receipts,
        &raw_sha256,
    ))
}

/// Pure judgement of one wargame run over observed facts.
fn judge(
    exit_success: bool,
    container: &Value,
    report: Option<&Value>,
    receipts: &[Value],
    raw_sha256: &str,
) -> Value {
    let network_none = container["network_mode"] == "none";
    let empty = json!(null);
    let report = report.unwrap_or(&empty);
    let verified = match receipts {
        [receipt] => {
            receipt["schema"] == "humanymous.lab-wargame-byte-receipt/1"
                && receipt["raw_sha256"] == raw_sha256
                && receipt["verification"]["schema"] == "humanymous.wargame-verification.v1"
                && receipt["verification"]["mode"] == "owned_process_network"
                && receipt["verification"]["internally_consistent"] == true
                && receipt["verification"]["full_product_qualified"] == false
                && receipt["verification"]["scoped_passed"] == report["scoped_passed"]
                && receipt["verification"]["totals"] == report["totals"]
        }
        _ => false,
    };
    let ids = |status: &str| -> Vec<Value> {
        report["rows"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .filter(|row| row["status"] == status)
                    .map(|row| row["id"].clone())
                    .collect()
            })
            .unwrap_or_default()
    };
    let scoped_passed = report["scoped_passed"] == true;
    let pass = exit_success && network_none && verified && scoped_passed;
    json!({"container":{"network_mode":container["network_mode"],"image":container["image"],
            "user":container["user"],"read_only_rootfs":container["read_only_rootfs"],
            "state":container["state"],"exit_code":container["exit_code"]},
        "exit_success":exit_success,"network_none":network_none,
        "byte_verification_receipts":receipts.len(),"original_byte_verified":verified,
        "raw_sha256":raw_sha256,"report_schema":report["schema"],"mode":report["mode"],
        "scoped_passed":scoped_passed,"totals":report["totals"],
        "unsupported_ids":ids("unsupported"),"error_ids":ids("error"),"skipped_ids":ids("skipped"),
        "scope":"owned process network: the shipped runtime as a supervisor-owned child in a network-none container, not the product topology Gate",
        "pass":pass})
}

/// The required `attack-catalog` case from one wargame run. It passes only
/// when the run is verified and no scheduled entry is unsupported, skipped or
/// in error; anything else is recorded as not passed with the counts.
fn attack_catalog_case(wargame: &Value) -> Value {
    let count = |name: &str| wargame["totals"][name].as_u64();
    let complete = [count("unsupported"), count("errors"), count("skipped")]
        .iter()
        .all(|n| *n == Some(0));
    let passed = wargame["pass"] == true && complete;
    json!({"id":"attack-catalog","status":if passed { "passed" } else { "failed" },
        "detail":{"source":"product-e2e wargame profile","scheduled":count("scheduled"),
            "evaluated":count("evaluated"),"unsupported":count("unsupported"),
            "errors":count("errors"),"skipped":count("skipped"),
            "unsupported_ids":wargame["unsupported_ids"],"run_verified":wargame["pass"],
            "rule":"not passed while any catalog entry is unsupported, skipped or in error"}})
}

/// Required-case ledger for this receipt: the shared derivation over the
/// cases this run evaluated. Absent evidence is never success.
pub(super) fn ledger(receipt: &Value, cleanup_verified: bool) -> Value {
    let mut cases = Vec::new();
    if receipt["attack_catalog"].is_object() {
        cases.push(attack_catalog_case(&receipt["attack_catalog"]));
    }
    let assessed = qualification::assess(&cases, cleanup_verified);
    let mut fields = assessed.report_fields();
    fields["derivation"] = json!(
        "test/lab/runner/src/orchestration/qualification.rs assess over this run's mapped cases"
    );
    fields["mapped_cases"] = json!(cases);
    fields["cleanup_verified"] = json!(cleanup_verified);
    fields["not_mapped"] = NOT_MAPPED
        .iter()
        .map(|(id, reason)| json!({"id":id,"reason":reason}))
        .collect();
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(unsupported: u64) -> Value {
        json!({"schema":"humanymous.wargame.v1","mode":"owned_process_network","scoped_passed":true,
            "totals":{"scheduled":26,"evaluated":26-unsupported,"errors":0,"skipped":0,"unsupported":unsupported},
            "rows":[{"id":"real-browser-agent","status":"unsupported"},{"id":"go-http-client","status":"evaluated"}]})
    }
    fn receipt(report: &Value, sha: &str) -> Value {
        json!({"schema":"humanymous.lab-wargame-byte-receipt/1","raw_sha256":sha,
            "verification":{"schema":"humanymous.wargame-verification.v1","mode":"owned_process_network",
                "internally_consistent":true,"full_product_qualified":false,
                "scoped_passed":report["scoped_passed"],"totals":report["totals"]}})
    }
    fn none() -> Value {
        json!({"network_mode":"none"})
    }

    #[test]
    fn unsupported_catalog_entries_keep_attack_catalog_from_passing() {
        let partial = report(7);
        let run = judge(
            true,
            &none(),
            Some(&partial),
            &[receipt(&partial, "s")],
            "s",
        );
        assert_eq!(run["pass"], true);
        assert_eq!(run["unsupported_ids"], json!(["real-browser-agent"]));
        let case = attack_catalog_case(&run);
        assert_eq!(case["status"], "failed");
        assert_eq!(case["detail"]["unsupported"], 7);
        let complete = report(0);
        let run = judge(
            true,
            &none(),
            Some(&complete),
            &[receipt(&complete, "s")],
            "s",
        );
        assert_eq!(attack_catalog_case(&run)["status"], "passed");
    }

    #[test]
    fn unverified_or_networked_runs_never_pass() {
        let report = report(0);
        let good = receipt(&report, "s");
        assert_eq!(
            judge(
                true,
                &none(),
                Some(&report),
                std::slice::from_ref(&good),
                "other"
            )["pass"],
            false
        );
        assert_eq!(
            judge(
                true,
                &json!({"network_mode":"bridge"}),
                Some(&report),
                std::slice::from_ref(&good),
                "s"
            )["pass"],
            false
        );
        assert_eq!(
            judge(
                true,
                &none(),
                Some(&report),
                &[good.clone(), good.clone()],
                "s"
            )["pass"],
            false
        );
        assert_eq!(
            judge(false, &none(), Some(&report), &[good], "s")["pass"],
            false
        );
        let run = judge(true, &none(), None, &[], "s");
        assert_eq!(attack_catalog_case(&run)["status"], "failed");
    }

    #[test]
    fn ledger_maps_only_evaluated_cases_and_never_qualifies_partially() {
        let all: usize = qualification::CAPABILITIES
            .iter()
            .map(|(_, ids)| ids.len())
            .sum();
        let empty = ledger(&json!({}), true);
        assert_eq!(empty["skips"], all);
        assert_eq!(empty["full_product_qualified"], false);
        assert_eq!(empty["mapped_cases"], json!([]));
        let report = report(7);
        let run = judge(true, &none(), Some(&report), &[receipt(&report, "s")], "s");
        let with = ledger(&json!({"attack_catalog":run}), true);
        assert_eq!(with["skips"], all - 1);
        assert_eq!(with["errors"], 1);
        assert_eq!(with["full_product_qualified"], false);
        let rows = with["required_cases"].as_array().unwrap();
        assert_eq!(rows.len(), all);
        for row in rows {
            let expected = if row["id"] == "attack-catalog" {
                "failed"
            } else {
                "not_evaluated"
            };
            assert_eq!(row["status"], expected, "{}", row["id"]);
        }
        // Every unmapped reason names a real required case.
        for (id, _) in NOT_MAPPED {
            assert!(rows.iter().any(|row| row["id"] == *id), "{id}");
        }
    }
}
