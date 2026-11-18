use super::*;
use humanymous_runtime::audit::{
    log::{Config, Log},
    storage::{Wal, WalOptions},
};

const WRITER: [u8; 32] = [0x11; 32];
const WITNESS: [u8; 32] = [0x22; 32];
const WRONG: [u8; 32] = [0x33; 32];
const HMAC: [u8; 32] = [0x44; 32];

fn keys() -> Keys {
    Keys {
        writer_public: public(&WRITER),
        witness_public: public(&WITNESS),
        wrong_public: public(&WRONG),
        hmac: HMAC,
    }
}

/// A real co-signed WAL: collect, ALLOW decision, then two more decisions.
fn wal(witness_seed: [u8; 32]) -> (Vec<Record>, Vec<Checkpoint>) {
    let directory = std::env::temp_dir().join(format!(
        "wtx-runner-test-{}-{}",
        std::process::id(),
        hex(&random::<8>().unwrap())
    ));
    std::fs::create_dir(&directory).unwrap();
    let result = {
        let mut log = Log::open(
            Wal::open(&directory, WalOptions::default()).unwrap(),
            Config {
                node_id: NODE.into(),
                hmac_key: HMAC.to_vec(),
                signing_seed: WRITER,
                checkpoint_every: 1,
                witness: Some(Witness::from_seed(
                    &witness_seed,
                    public(&WRITER),
                    NODE.into(),
                )),
            },
            None,
            true,
        )
        .unwrap();
        for (event, verdict) in [
            ("runtime.collect", "ALLOW"),
            ("runtime.decision", "ALLOW"),
            ("runtime.decision", "ALLOW"),
            ("runtime.decision", "ALLOW"),
        ] {
            log.append(Record {
                event_type: event.into(),
                verdict: verdict.into(),
                ..Default::default()
            })
            .unwrap();
        }
        (log.records().unwrap(), log.checkpoints().unwrap())
    };
    std::fs::remove_dir_all(&directory).unwrap();
    result
}

fn input(pinned: &[u8; 32]) -> Value {
    json!({"read_fd":5,"write_fd":4,"witness_public":pinned,"writer_public":public(&WRITER),
        "durability":"file_and_directory","require_external_witness":true})
}

fn exchange(name: &str, status: u16, body: &str) -> Value {
    json!({"name":name,"response":{"status":status,"body":body}})
}

/// The supervisor relays a session response redacted: no session or key.
fn session(status: u16, issued: bool, error: Option<&str>) -> Value {
    let body = error.map(|code| json!({ "error": code }).to_string());
    json!({"name":"session","response":{"status":status,"issued":issued,"body":body}})
}

/// Evidence of one complete, correct run; the last decision is the record
/// whose checkpoint the lost witness could not co-sign.
fn complete() -> Evidence {
    let (records, mut checkpoints) = wal(WITNESS);
    checkpoints.pop();
    let ready = json!({"listen":"127.0.0.1:41234","profile":{"audit_external_witness":true,
        "audit_witness_required_durability":"file_and_directory",
        "audit_witness_independence_qualified":false,"product_qualified":false}});
    let mut evidence = Evidence {
        supervisor_id: "abc123".into(),
        supervisor_name: "/lab-wtx-supervisor-1".into(),
        supervisor_network_mode: "none".into(),
        origin_network_mode: "container:abc123".into(),
        origin_networks: 0,
        origin_ready: true,
        origin_log_valid: true,
        ..Default::default()
    };
    let wrong = public(&WRONG);
    let witness = public(&WITNESS);
    for (step, reply, contacts) in [
        (
            "wrong_key",
            json!({"step":"wrong_key","configured":input(&wrong),"readiness_line":"",
                "exit_code":1,"audit_entries":0}),
            (0, 0),
        ),
        (
            "witnessed",
            json!({"step":"witnessed","configured":input(&witness),
                "readiness_line":format!("{ready}\n"),"witness_head_entries":1,
                "exchanges":[session(200, true, None),exchange("collect",200,"{}"),
                    exchange("effect",200,"ok")]}),
            (0, 1),
        ),
        (
            "witness_loss",
            json!({"step":"witness_loss","witnessed_session":true,"witness_exit_success":false,
                "exchanges":[exchange("effect",503,r#"{"error":"durable_audit"}"#),
                    exchange("effect_repeated",503,r#"{"error":"session_epoch"}"#),
                    session(503, false, Some("session_epoch"))],
                "runtime_alive":true}),
            (1, 1),
        ),
        (
            "evidence",
            json!({"step":"evidence","records":records,"checkpoints":checkpoints}),
            (1, 1),
        ),
    ] {
        evidence.steps.insert(step.into(), reply);
        evidence.contacts.insert(step.into(), contacts);
    }
    evidence
}

fn failing(evidence: &Evidence, keys: &Keys) -> Vec<&'static str> {
    evaluate(evidence, keys)
        .into_iter()
        .filter(|(_, ok)| !ok)
        .map(|(id, _)| id)
        .collect()
}

#[test]
fn complete_observed_run_passes_every_required_check() {
    let evidence = complete();
    let checks = evaluate(&evidence, &keys());
    assert_eq!(
        checks.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        CHECKS.to_vec()
    );
    assert!(failing(&evidence, &keys()).is_empty());
    assert!(passed(&checks, &evidence));
}

#[test]
fn missing_evidence_is_never_a_pass() {
    let empty = Evidence::default();
    let checks = evaluate(&empty, &keys());
    assert!(checks.iter().all(|(_, ok)| !ok));
    assert!(!passed(&checks, &empty));
    for step in STEPS {
        let mut evidence = complete();
        evidence.steps.remove(step);
        assert!(!passed(&evaluate(&evidence, &keys()), &evidence), "{step}");
        let mut evidence = complete();
        evidence.contacts.remove(step);
        assert!(!passed(&evaluate(&evidence, &keys()), &evidence), "{step}");
    }
    let mut evidence = complete();
    evidence.error = Some("supervisor step mismatch".into());
    assert!(!passed(&evaluate(&evidence, &keys()), &evidence));
    // A check list missing one required entry cannot pass either.
    let evidence = complete();
    let mut checks = evaluate(&evidence, &keys());
    checks.pop();
    assert!(!passed(&checks, &evidence));
}

#[test]
fn each_failure_path_fails_its_own_check() {
    type Mutation = fn(&mut Evidence);
    let cases: &[(&str, Mutation)] = &[
        ("topology-supervisor-network-none", |e| {
            e.supervisor_network_mode = "bridge".into()
        }),
        ("topology-origin-shares-supervisor-loopback-only", |e| {
            e.origin_network_mode = "container:other".into()
        }),
        ("topology-origin-shares-supervisor-loopback-only", |e| {
            e.origin_networks = 1
        }),
        ("origin-contact-log-owned", |e| e.origin_log_valid = false),
        ("origin-contact-log-owned", |e| e.origin_ready = false),
        ("wrong-key-pin-configured", |e| {
            e.steps.get_mut("wrong_key").unwrap()["configured"] = input(&public(&WITNESS))
        }),
        ("wrong-key-pin-configured", |e| {
            e.steps.get_mut("wrong_key").unwrap()["configured"]["read_fd"] = json!(2)
        }),
        ("wrong-key-refused-without-readiness", |e| {
            e.steps.get_mut("wrong_key").unwrap()["readiness_line"] = json!("{}\n")
        }),
        ("wrong-key-refused-without-readiness", |e| {
            e.steps.get_mut("wrong_key").unwrap()["exit_code"] = json!(0)
        }),
        ("wrong-key-refused-without-readiness", |e| {
            e.steps.get_mut("wrong_key").unwrap()["exit_code"] = Value::Null
        }),
        ("wrong-key-no-audit-state", |e| {
            e.steps.get_mut("wrong_key").unwrap()["audit_entries"] = json!(2)
        }),
        ("wrong-key-no-origin-contact", |e| {
            e.contacts.insert("wrong_key".into(), (0, 1));
        }),
        ("witnessed-pin-configured", |e| {
            e.steps.get_mut("witnessed").unwrap()["configured"]["require_external_witness"] =
                json!(false)
        }),
        ("witnessed-readiness", |e| {
            e.steps.get_mut("witnessed").unwrap()["readiness_line"] = json!("")
        }),
        ("witnessed-readiness", |e| {
            let ready = json!({"listen":"127.0.0.1:1","profile":{"audit_external_witness":false,
                "audit_witness_required_durability":null,
                "audit_witness_independence_qualified":false,"product_qualified":false}});
            e.steps.get_mut("witnessed").unwrap()["readiness_line"] = json!(ready.to_string())
        }),
        ("witnessed-readiness", |e| {
            let ready = json!({"listen":"127.0.0.1:1","profile":{"audit_external_witness":true,
                "audit_witness_required_durability":"file_and_directory",
                "audit_witness_independence_qualified":true,"product_qualified":false}});
            e.steps.get_mut("witnessed").unwrap()["readiness_line"] = json!(ready.to_string())
        }),
        ("witnessed-exchanges", |e| {
            e.steps.get_mut("witnessed").unwrap()["exchanges"][2] = exchange("effect", 403, "")
        }),
        ("witnessed-exchanges", |e| {
            e.steps.get_mut("witnessed").unwrap()["witness_head_entries"] = json!(0)
        }),
        ("witnessed-origin-contact", |e| {
            e.contacts.insert("witnessed".into(), (0, 0));
        }),
        ("witness-loss-fails-closed", |e| {
            e.steps.get_mut("witness_loss").unwrap()["exchanges"][0] = exchange("effect", 200, "ok")
        }),
        ("witness-loss-fails-closed", |e| {
            e.steps.get_mut("witness_loss").unwrap()["exchanges"][1] =
                exchange("effect_repeated", 503, r#"{"error":"origin_unavailable"}"#)
        }),
        ("witness-loss-fails-closed", |e| {
            e.steps.get_mut("witness_loss").unwrap()["exchanges"][2] = session(200, true, None)
        }),
        ("witness-loss-fails-closed", |e| {
            e.steps.get_mut("witness_loss").unwrap()["exchanges"][2] =
                session(503, false, Some("origin_unavailable"))
        }),
        ("witness-loss-fails-closed", |e| {
            e.steps.get_mut("witness_loss").unwrap()["exchanges"][0] =
                exchange("effect", 503, r#"{"error":"session_epoch"}"#)
        }),
        ("witnessed-exchanges", |e| {
            e.steps.get_mut("witnessed").unwrap()["exchanges"][0] = session(200, false, None)
        }),
        ("witness-loss-fails-closed", |e| {
            e.steps.insert(
                "witness_loss".into(),
                json!({"step":"witness_loss","witnessed_session":false}),
            );
        }),
        ("witness-loss-no-origin-contact", |e| {
            e.contacts.insert("witness_loss".into(), (1, 2));
        }),
        ("witness-loss-no-origin-contact", |e| {
            e.contacts.insert("evidence".into(), (1, 2));
        }),
        ("durable-audit-co-signed", |e| {
            e.steps.get_mut("evidence").unwrap()["checkpoints"][0]["witness_sig"] = json!("")
        }),
        ("durable-audit-co-signed", |e| {
            e.steps.get_mut("evidence").unwrap()["records"][1]["verdict"] = json!("DENY")
        }),
        ("durable-audit-co-signed", |e| {
            e.steps.get_mut("evidence").unwrap()["checkpoints"] = json!([])
        }),
        ("durable-audit-co-signed", |e| {
            e.steps.get_mut("evidence").unwrap()["records"] = json!("absent")
        }),
        ("poisoned-epoch-writes-nothing-further", |e| {
            let checkpoints = &mut e.steps.get_mut("evidence").unwrap()["checkpoints"];
            checkpoints.as_array_mut().unwrap().pop();
        }),
    ];
    for (check, mutate) in cases {
        let mut evidence = complete();
        mutate(&mut evidence);
        let failed = failing(&evidence, &keys());
        assert!(failed.contains(check), "{check}: {failed:?}");
        assert!(!passed(&evaluate(&evidence, &keys()), &evidence), "{check}");
    }
}

#[test]
fn checkpoints_signed_by_another_witness_are_not_co_signed() {
    let mut evidence = complete();
    let (records, mut checkpoints) = wal(WRONG);
    checkpoints.pop();
    evidence.steps.insert(
        "evidence".into(),
        json!({"step":"evidence","records":records,"checkpoints":checkpoints}),
    );
    assert!(failing(&evidence, &keys()).contains(&"durable-audit-co-signed"));
    // The wrong HMAC key also refuses records the runner did not provision.
    let mut other = keys();
    other.hmac = [0x55; 32];
    assert!(failing(&complete(), &other).contains(&"durable-audit-co-signed"));
}

#[test]
fn failures_project_only_static_check_ids_and_stage_markers() {
    let keys = keys();
    let checks = evaluate(&complete(), &keys);
    assert!(failure_rows(&checks, None).is_empty());

    // A drive that stopped while entering a step: every failed check is named,
    // plus the stage marker, and the safe projection keeps exactly those labels.
    let mut evidence = complete();
    evidence.steps.remove("witness_loss");
    evidence.steps.remove("evidence");
    evidence.contacts.remove("witness_loss");
    evidence.contacts.remove("evidence");
    evidence.error = Some("private detail: /tmp/witness-transport-secret".into());
    let rows = failure_rows(&evaluate(&evidence, &keys), Some("wtx_step_witness_loss"));
    let safe = crate::orchestration::safe_diagnostics(&rows);
    let ids: Vec<&str> = safe["failures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["case_id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"runtime-binary-witness-transport--witness-loss-fails-closed"));
    assert!(ids.contains(&"runtime-binary-witness-transport--durable-audit-co-signed"));
    assert!(!ids.contains(&"runtime-binary-witness-transport--witnessed-readiness"));
    assert_eq!(
        safe["failures"].as_array().unwrap().last().unwrap()["error_code"],
        "wtx_step_witness_loss"
    );
    assert!(!safe.to_string().contains("secret"));

    // Every check row ID fits the public case-ID bound and every stage marker
    // is in the owned vocabulary, so none degrades to an unclassified row.
    let contract: Value =
        serde_json::from_str(include_str!("../../../safe-diagnostics.json")).unwrap();
    for step in STEPS {
        assert!(PHASES.contains(&step_phase(step)));
    }
    let all = failure_rows(&[], Some("wtx_supervisor_exit"));
    assert_eq!(all.len(), CHECKS.len() + 1);
    for phase in PHASES
        .iter()
        .chain(&["witness_transport", "witness_transport_process"])
    {
        assert!(
            contract["error_codes"]
                .as_array()
                .unwrap()
                .contains(&json!(phase))
        );
    }
    for row in crate::orchestration::safe_diagnostics(&all)["failures"]
        .as_array()
        .unwrap()
    {
        assert_ne!(row["case_id"], "unclassified_case");
    }
}
