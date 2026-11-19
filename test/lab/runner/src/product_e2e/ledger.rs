//! Increment 5: the owned attack catalog run, and the required-case ledger.
//!
//! The ledger is never hard-coded: it is `orchestration::qualification::assess`
//! (the same derivation the `run` profile reports) over only the required case
//! IDs this run honestly evaluates. Today that is `attack-catalog`, from the
//! network-none owned process wargame, and it cannot pass while any catalog
//! entry is unsupported. Every other required case stays `not_evaluated`, with
//! the reason a related component measurement here does not satisfy it.
//!
//! `gate` and `raw-tls-h2` are measured in the `tls` profile at a narrower
//! scope (shared loopback namespace) and reported as `scoped_cases`. A scoped
//! failure is a failure of the required case too, so it enters `assess` as
//! failed; a scoped success never passes it, because the required case needs
//! an original client in its own namespace (owner item B01 #1).
use super::{Lab, Result};
use crate::orchestration::qualification;
use serde_json::{Value, json};

pub(super) const WARGAME: &str = "humanymous-lab-product-e2e-wargame";

/// Why a shared-namespace TLS measurement cannot satisfy `gate` or `raw-tls-h2`.
const ORIGINAL_CLIENT_BLOCKER: &str = "the required case needs original-client TLS/H2 from a client in its own network namespace reaching the Gate's public edge listener; Configuration::validate requires a loopback listen address and the TLS accept loop drops non-loopback peers (crates/sdk/humanymous-runtime/src/gateway/configuration.rs, gateway/application/tls.rs), so the only client that can reach it shares the Gate namespace. Owner item B01 #1 (public edge listener) blocks it";

/// Required IDs this topology measures something near, and why that
/// measurement is not the required case. Unlisted IDs have no producer here.
const NOT_MAPPED: &[(&str, &str)] = &[
    (
        "gate",
        "measured only at shared-loopback-namespace scope (scoped_cases: the shipped binary's TLS listener in profile tls); the required case needs original-client TLS/H2 from its own namespace, blocked by owner item B01 #1 (public edge listener)",
    ),
    (
        "raw-tls-h2",
        "measured only at shared-loopback-namespace scope (scoped_cases: real TLS/H2 sessions, raw ClientHello parrots and malformed H2 frames from the Gate's own namespace); original-client raw capture across namespaces is blocked by owner item B01 #1 (public edge listener)",
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
    // Distinct static refusal reasons of rows in error, e.g. an isolation
    // precondition the venue cannot satisfy; never request or key material.
    let error_reasons: std::collections::BTreeSet<&str> = report["rows"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter(|row| row["status"] == "error")
                .filter_map(|row| row["reason"].as_str())
                .collect()
        })
        .unwrap_or_default();
    // The IPv6 isolation evidence each owned child recorded (decision #152-2):
    // loopback_routes_only or stack_absent, from the byte-verified report.
    let process_ipv6: std::collections::BTreeSet<&str> = report["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row["processes"].as_array())
        .flatten()
        .filter_map(|process| process["ipv6"].as_str())
        .collect();
    let scoped_passed = report["scoped_passed"] == true;
    let pass = exit_success && network_none && verified && scoped_passed;
    json!({"container":{"network_mode":container["network_mode"],"image":container["image"],
            "user":container["user"],"read_only_rootfs":container["read_only_rootfs"],
            "state":container["state"],"exit_code":container["exit_code"]},
        "exit_success":exit_success,"network_none":network_none,
        "byte_verification_receipts":receipts.len(),"original_byte_verified":verified,
        "raw_sha256":raw_sha256,"report_schema":report["schema"],"evidence_type":report["evidence_type"],
        "scoped_passed":scoped_passed,"totals":report["totals"],
        "unsupported_ids":ids("unsupported"),"error_ids":ids("error"),"skipped_ids":ids("skipped"),
        "error_reasons":error_reasons,"process_ipv6":process_ipv6,
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

/// TLS driver cases each scoped component judgement requires, all passing.
const SCOPED: &[(&str, &[&str])] = &[
    (
        "gate",
        &[
            "tls_client_refuses_other_root",
            "tls_client_refuses_foreign_name",
            "tls_h1_signed_effect",
            "tls_h2_signed_effect",
            "tls_h2_no_session_protected_route",
            "tls_gate_serves_after_refused_inputs",
        ],
    ),
    (
        "raw-tls-h2",
        &[
            "tls_alpn_h2_selected",
            "tls_alpn_http11_only",
            "tls_alpn_not_offered",
            "tls_h1_raw_tls_route_admitted",
            "tls_h1_original_h2_route_refused",
            "tls_h2_raw_tls_route_admitted",
            "tls_h2_original_h2_route_admitted",
            "plaintext_http_on_tls_listener",
            "raw_parrot_clienthello_chrome",
            "raw_parrot_clienthello_chrome100",
            "raw_parrot_clienthello_firefox",
            "raw_parrot_clienthello_stock_go",
            "malformed_clienthello_truncated",
            "malformed_clienthello_wrong_handshake_type",
            "malformed_record_length_over_limit",
            "h2_bad_preface",
            "h2_frame_over_budget",
            "h2_request_on_stream_3",
            "h2_settings_on_stream_1",
            "h2_continuation_without_headers",
            "h2_truncated_before_end_stream",
        ],
    ),
];

/// Shared-loopback-namespace judgement of `gate` and `raw-tls-h2` from the
/// `tls` profile. `measured` is passed only with every listed driver case,
/// the TLS topology/readiness assertions and the driver run passing; the
/// required status is then still `not_evaluated`, never `passed`.
fn scoped_cases(tls: &Value) -> Vec<Value> {
    let cases = tls["driver"]["cases"].as_array();
    let topology: Vec<&Value> = tls["topology"]["assertions"]
        .as_array()
        .map(|rows| rows.iter().collect())
        .unwrap_or_default();
    let topology_pass = !topology.is_empty() && topology.iter().all(|row| row["pass"] == true);
    let driver_ran = tls["driver_exit_success"] == true && tls["driver"]["driver_error"].is_null();
    SCOPED
        .iter()
        .map(|(id, required)| {
            let rows: Vec<Value> = required
                .iter()
                .map(|case| {
                    let found = cases.and_then(|all| all.iter().find(|row| row["id"] == *case));
                    json!({"id":case,"observed":found.is_some(),
                        "pass":found.is_some_and(|row| row["pass"] == true)})
                })
                .collect();
            let all = rows.iter().all(|row| row["pass"] == true);
            let measured = if all && topology_pass && driver_ran {
                "passed"
            } else {
                "failed"
            };
            json!({"id":id,"scope":"shared loopback namespace","measured":measured,
                "required_case_status":if measured == "passed" { "not_evaluated" } else { "failed" },
                "blocked_by":"owner item B01 #1 (public edge listener)","reason":ORIGINAL_CLIENT_BLOCKER,
                "evidence":{"source":"product-e2e profile tls (receipt.tls)","driver_cases":rows,
                    "topology_and_readiness_assertions_pass":topology_pass,"driver_ran":driver_ran}})
        })
        .collect()
}

/// Required-case ledger for this receipt: the shared derivation over the
/// cases this run evaluated. Absent evidence is never success.
pub(super) fn ledger(receipt: &Value, cleanup_verified: bool) -> Value {
    let mut cases = Vec::new();
    if receipt["attack_catalog"].is_object() {
        cases.push(attack_catalog_case(&receipt["attack_catalog"]));
    }
    let scoped = if receipt["tls"].is_object() {
        scoped_cases(&receipt["tls"])
    } else {
        Vec::new()
    };
    // A failure at the narrower scope fails the required case; a scoped
    // success leaves it unevaluated.
    for case in scoped.iter().filter(|case| case["measured"] != "passed") {
        cases.push(json!({"id":case["id"],"status":"failed",
            "detail":{"source":"product-e2e profile tls","scope":"shared loopback namespace",
                "rule":"a failed shared-namespace measurement fails the required case"}}));
    }
    let mut assessed = qualification::assess(&cases, cleanup_verified);
    // This profile runs only on the explicit cloud-container venue.
    assessed.non_qualifying_venue();
    let mut fields = assessed.report_fields();
    fields["derivation"] = json!(
        "test/lab/runner/src/orchestration/qualification.rs assess over this run's mapped cases"
    );
    fields["not_mapped"] = NOT_MAPPED
        .iter()
        .filter(|(id, _)| !cases.iter().any(|case| case["id"] == *id))
        .map(|(id, reason)| json!({"id":id,"reason":reason}))
        .collect();
    fields["mapped_cases"] = json!(cases);
    fields["scoped_cases"] = json!(scoped);
    fields["cleanup_verified"] = json!(cleanup_verified);
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
        assert_eq!(run["process_ipv6"], json!([]));
        let mut absent = partial.clone();
        absent["rows"][1]["processes"] = json!([{"ipv6":"stack_absent"},{"ipv6":"stack_absent"}]);
        let run_absent = judge(true, &none(), Some(&absent), &[receipt(&absent, "s")], "s");
        assert_eq!(run_absent["process_ipv6"], json!(["stack_absent"]));
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
        // A venue that cannot show namespace evidence errors every program;
        // the distinct reason is kept and the case is not passed.
        let mut failed = report.clone();
        failed["scoped_passed"] = json!(false);
        failed["totals"]["errors"] = json!(1);
        failed["rows"] = json!([{"id":"a","status":"error","reason":"namespace evidence unavailable"},
            {"id":"b","status":"error","reason":"namespace evidence unavailable"}]);
        let run = judge(false, &none(), Some(&failed), &[receipt(&failed, "s")], "s");
        assert_eq!(
            run["error_reasons"],
            json!(["namespace evidence unavailable"])
        );
        assert_eq!(run["error_ids"], json!(["a", "b"]));
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
        assert!(
            empty["required_unavailable"]
                .as_array()
                .unwrap()
                .contains(&json!(qualification::QUALIFYING_VENUE))
        );
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

    fn tls(pass: bool) -> Value {
        let cases: Vec<Value> = SCOPED
            .iter()
            .flat_map(|(_, ids)| ids.iter())
            .map(|id| json!({"id":id,"pass":pass}))
            .collect();
        json!({"driver_exit_success":true,"driver":{"cases":cases,"driver_error":null},
            "topology":{"assertions":[{"id":"tls_listener_operator_certificate","pass":true}]}})
    }

    #[test]
    fn shared_namespace_tls_success_never_passes_the_required_cases() {
        let ledger = ledger(&json!({"tls":tls(true)}), true);
        let scoped = ledger["scoped_cases"].as_array().unwrap();
        assert_eq!(scoped.len(), 2);
        for case in scoped {
            assert_eq!(case["measured"], "passed");
            assert_eq!(case["required_case_status"], "not_evaluated");
            assert_eq!(case["scope"], "shared loopback namespace");
        }
        for row in ledger["required_cases"].as_array().unwrap() {
            if row["id"] == "gate" || row["id"] == "raw-tls-h2" {
                assert_eq!(row["status"], "not_evaluated");
            }
        }
        assert_eq!(ledger["mapped_cases"], json!([]));
        assert_eq!(ledger["full_product_qualified"], false);
        assert!(
            ledger["required_unavailable"]
                .as_array()
                .unwrap()
                .contains(&json!("full Gate TLS/H2/capture"))
        );
        assert!(
            ledger["not_mapped"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["id"] == "raw-tls-h2")
        );
    }

    #[test]
    fn a_scoped_tls_failure_fails_the_required_case() {
        let mut one = tls(true);
        one["driver"]["cases"][0]["pass"] = json!(false);
        let ledger = ledger(&json!({"tls":one}), true);
        let status = |id: &str| {
            ledger["required_cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["id"] == id)
                .unwrap()["status"]
                .clone()
        };
        assert_eq!(status("gate"), "failed");
        assert_eq!(status("raw-tls-h2"), "not_evaluated");
        assert_eq!(ledger["errors"], 1);
        assert!(
            !ledger["not_mapped"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["id"] == "gate")
        );
        // A missing driver case, a failed topology assertion or an aborted
        // driver run each fail the scoped measurement.
        let mut missing = tls(true);
        missing["driver"]["cases"] = json!([]);
        assert!(
            scoped_cases(&missing)
                .iter()
                .all(|case| case["measured"] == "failed")
        );
        let mut topology = tls(true);
        topology["topology"]["assertions"][0]["pass"] = json!(false);
        assert!(
            scoped_cases(&topology)
                .iter()
                .all(|case| case["measured"] == "failed")
        );
        let mut aborted = tls(true);
        aborted["driver"]["driver_error"] = json!("boom");
        assert!(
            scoped_cases(&aborted)
                .iter()
                .all(|case| case["measured"] == "failed")
        );
        assert!(
            scoped_cases(&json!({"pass":false,"error":"x"}))
                .iter()
                .all(|c| c["measured"] == "failed")
        );
    }
}
