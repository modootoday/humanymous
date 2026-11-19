//! Ports of legacy internal/gate/ws3_test.go (TestErasureHoldWindowAndReverseResolve,
//! TestErasureCancel) and wargame_escalation_r121_test.go (R132) at baseline
//! 981d25c through the served admin plane, with the erasure capability issued
//! by a real control ledger (`ControlAuthority::issue_erasure`) and shredded
//! through the scope-bound reference `Vault`. The Go-capture pins compare
//! against `tests/fixtures/gateway/admin-erasure-go-golden.json`.
use super::*;
use crate::audit::keys::vault::{Mode, Vault};
use crate::gateway::operations::erasure_hold::DEFAULT_HOLD;

const ROOT: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const WINDOW: Duration = Duration::from_secs(6 * 60);

fn golden() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/gateway/admin-erasure-go-golden.json"
    ))
    .unwrap()
}

fn scoped_vault() -> Arc<Vault> {
    Arc::new(Vault::new_scoped(Mode::Hmac, 64, scope()).unwrap())
}

async fn erasure_stack(vault: &Arc<Vault>) -> Stack {
    let vault = vault.clone();
    stack_with(move |plane| plane.with_erasure(vault, DEFAULT_HOLD).unwrap()).await
}

/// Request an erasure as the operator and commit it as the DPO; the commit reply.
async fn request_and_commit(s: &Stack, subject: &str, basis: &str) -> Reply {
    let body = format!(r#"{{"Subject":"{subject}","LegalBasis":"{basis}"}}"#);
    let req = s
        .admin_do(s.token("operator"), "POST", "erasure", &body)
        .await;
    assert_eq!(req.status, 200, "{}", req.body);
    let id = approval_id(&req);
    s.admin_do(s.token("dpo"), "POST", &format!("approvals/{id}"), "")
        .await
}

fn shred_id(reply: &Reply) -> String {
    reply.json()["shredId"].as_str().unwrap_or("").to_owned()
}

/// Go `TestErasureHoldWindowAndReverseResolve`: the erasure is requested by
/// the console handle, scheduled (not shredded) on the DPO commit, kept for
/// the hold window, then shredded with `erasure.completed` evidence.
#[tokio::test]
async fn erasure_hold_window_and_reverse_resolve() {
    let vault = scoped_vault();
    let s = erasure_stack(&vault).await;
    let mut ledger = Ledger::new(scope());
    // A subject exists with a linkage key and a console-visible handle.
    let p0 = vault.pseudonymize("subjX", "1.2.3.4").unwrap();
    vault.link("psn-handle-X", "subjX").unwrap();
    s.plane
        .retain_erasure_permit("subjX", ledger.erasure_permit("subjX"))
        .unwrap();

    // Operator requests erasure by the handle; a distinct DPO approves.
    let commit = request_and_commit(&s, "psn-handle-X", "GDPR Art.17").await;
    assert_eq!(commit.status, 200, "{}", commit.body);
    assert!(
        commit.body.contains("\"scheduled\":true"),
        "erasure should be scheduled, not immediate: {}",
        commit.body
    );

    // During the hold window the subject is not yet shredded.
    assert_eq!(vault.pseudonymize("subjX", "1.2.3.4").unwrap(), p0);
    assert_eq!(s.plane.run_due_shreds(s.now(), ("", 0)).unwrap(), 0);

    // After the window the shred executes and a certificate is sealed.
    assert_eq!(
        s.plane.run_due_shreds(s.now() + WINDOW, (ROOT, 8)).unwrap(),
        1
    );
    assert_ne!(
        vault.pseudonymize("subjX", "1.2.3.4").unwrap(),
        p0,
        "subject linkage must be severed after the shred"
    );
    assert_eq!(vault.resolve("psn-handle-X").unwrap(), None);
    assert!(
        s.journal
            .events()
            .iter()
            .any(|(kind, reason)| kind == "erasure.completed"
                && reason.contains("erasure-cert")
                && reason.contains("GDPR Art.17")),
        "erasure.completed audit evidence must be sealed on shred"
    );
}

/// Go `TestErasureCancel`: a scheduled erasure cancelled in the hold window
/// never executes and the subject keeps its linkage.
#[tokio::test]
async fn erasure_cancel() {
    let vault = scoped_vault();
    let s = erasure_stack(&vault).await;
    let mut ledger = Ledger::new(scope());
    let p0 = vault.pseudonymize("subjY", "9.9.9.9").unwrap();
    s.plane
        .retain_erasure_permit("subjY", ledger.erasure_permit("subjY"))
        .unwrap();
    let commit = request_and_commit(&s, "subjY", "PIPA").await;
    let id = shred_id(&commit);

    let c = s
        .admin_do(s.token("dpo"), "POST", &format!("erasures/{id}/cancel"), "")
        .await;
    assert_eq!(c.status, 200, "cancel failed: {}", c.body);
    assert_eq!(
        s.plane
            .run_due_shreds(s.now() + Duration::from_secs(10 * 60), ("", 0))
            .unwrap(),
        0,
        "cancelled erasure must not execute"
    );
    assert_eq!(vault.pseudonymize("subjY", "9.9.9.9").unwrap(), p0);
}

/// Go `TestWargameR132_ErasureCancelHoldWindow`: after a committed erasure,
/// cancelling an unknown id answers 404.
#[tokio::test]
async fn r132_erasure_cancel_hold_window() {
    let vault = scoped_vault();
    let s = erasure_stack(&vault).await;
    let mut ledger = Ledger::new(scope());
    s.plane
        .retain_erasure_permit("subj-r132", ledger.erasure_permit("subj-r132"))
        .unwrap();
    let w = request_and_commit(&s, "subj-r132", "art17").await;
    assert_eq!(w.status, 200, "dpo commit: {}", w.body);
    let cw = s
        .admin_do(s.token("operator"), "POST", "erasures/nope/cancel", "")
        .await;
    assert_eq!(cw.status, 404, "cancel unknown: {}", cw.body);
}

// --- Go capture pins ----------------------------------------------------------

/// One Go capture record without `route_class` (the plane files every record
/// under the `control` route class).
fn go_record(record: &Value) -> Value {
    let mut record = record.clone();
    record.as_object_mut().unwrap().remove("route_class");
    record
}

fn rust_record(record: &Record) -> Value {
    json!({
        "event_type": record.event_type,
        "actor_kind": record.actor.kind,
        "tenant_id": record.tenant_id,
        "verdict": record.verdict,
        "action": record.enforcement_action,
        "mode": record.enforcement_mode,
        "fail_reason": record.fail_reason,
        "key_id": record.key_id,
    })
}

/// Redact the Rust values the capture redacted.
fn redact(text: &str, substitutions: &[(&str, &str)]) -> String {
    let mut text = text.to_owned();
    for (value, placeholder) in substitutions {
        if !value.is_empty() {
            text = text.replace(value, placeholder);
        }
    }
    if let Some(at) = text.find("ticket=")
        && text.len() >= at + 19
    {
        text.replace_range(at + 7..at + 19, "<ticket12>");
    }
    text
}

impl Stack {
    fn records_since(&self, seen: &mut usize) -> Vec<Record> {
        let records = self.journal.records.lock().unwrap();
        let fresh = records[*seen..].to_vec();
        *seen = records.len();
        fresh
    }
}

/// Compare one exchange's records with the capture: `admin.access` by event,
/// action and reason (the plane also stamps it `enforce`), the rest by every
/// captured field except the route class.
fn assert_records(go: &Value, rust: &[Record], substitutions: &[(&str, &str)], case: &str) {
    let go = go.as_array().unwrap();
    assert_eq!(go.len(), rust.len(), "{case}: record count");
    for (go, rust) in go.iter().zip(rust) {
        let mut rust = rust_record(rust);
        rust["fail_reason"] = json!(redact(rust["fail_reason"].as_str().unwrap(), substitutions));
        if go["event_type"] == "admin.access" {
            for field in [
                "event_type",
                "action",
                "fail_reason",
                "actor_kind",
                "key_id",
            ] {
                assert_eq!(go[field], rust[field], "{case}: admin.access {field}");
            }
        } else {
            assert_eq!(go_record(go), rust, "{case}");
        }
    }
}

fn step<'a>(steps: &'a Value, case: &str) -> &'a Value {
    steps
        .as_array()
        .unwrap()
        .iter()
        .find(|step| step["case"] == case)
        .unwrap_or_else(|| panic!("capture step {case}"))
}

fn sorted_keys(body: &str) -> Value {
    match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(fields)) => json!(fields.keys().cloned().collect::<Vec<_>>()),
        _ => Value::Null,
    }
}

/// The hold-window flow of the capture: list, request by handle, commit,
/// list, shred and list again, byte-compared where Go was deterministic.
#[tokio::test]
async fn hold_window_flow_matches_go_capture() {
    let golden = golden();
    let capture = &golden["hold_window"];
    let steps = &capture["steps"];
    let vault = scoped_vault();
    let s = erasure_stack(&vault).await;
    let mut ledger = Ledger::new(scope());
    vault.pseudonymize("subjX", "1.2.3.4").unwrap();
    vault.link("psn-handle-X", "subjX").unwrap();
    s.plane
        .retain_erasure_permit("subjX", ledger.erasure_permit("subjX"))
        .unwrap();
    let mut seen = 0;

    let go = step(steps, "list_empty");
    let w = s.admin_do(s.token("auditor"), "GET", "erasures", "").await;
    assert_eq!(
        (w.status, w.content_type.as_str(), w.body.as_str()),
        (
            go["status"].as_u64().unwrap() as u16,
            go["content_type"].as_str().unwrap(),
            go["response"].as_str().unwrap()
        )
    );
    assert_records(
        &go["records"],
        &s.records_since(&mut seen),
        &[],
        "list_empty",
    );

    let go = step(steps, "request_by_handle");
    let req = s
        .admin_do(
            s.token("operator"),
            "POST",
            "erasure",
            go["body"].as_str().unwrap(),
        )
        .await;
    assert_eq!(req.status, 200);
    assert_eq!(sorted_keys(&req.body), go["response_keys"]);
    assert_eq!(req.json()["needsRole"], go["needs_role"]);
    let approval = approval_id(&req);
    assert_eq!(
        approval.len() as u64,
        go["approval_id_len"].as_u64().unwrap()
    );
    assert_records(
        &go["records"],
        &s.records_since(&mut seen),
        &[],
        "request_by_handle",
    );

    // The pending action carries the resolved subject, not the handle.
    let pending = s.plane.approvals.pending(s.now());
    let params: Vec<Value> = pending.iter().map(|action| json!(action.params)).collect();
    assert_eq!(
        json!(params),
        step(steps, "approvals_params")["params"],
        "pending params"
    );

    let go = step(steps, "dpo_commit");
    let before = crate::authority::now_ms().unwrap() / 1000;
    let commit = s
        .admin_do(s.token("dpo"), "POST", &format!("approvals/{approval}"), "")
        .await;
    let after = crate::authority::now_ms().unwrap() / 1000;
    assert_eq!(commit.status, 200, "{}", commit.body);
    assert_eq!(commit.content_type, go["content_type"].as_str().unwrap());
    assert_eq!(sorted_keys(&commit.body), go["response_keys"]);
    let body = commit.json();
    for field in ["committed", "ok", "scheduled"] {
        assert_eq!(body[field], go[field], "{field}");
    }
    // Go: 20 random hex digits; the queue's ids are sequential (deliberate).
    assert_eq!(go["shred_id_len"], 20);
    let id = shred_id(&commit);
    assert_eq!(id, "shred-0000000001");
    let hold = go["execute_at_minus_now_in"][0].as_u64().unwrap();
    assert_eq!(hold, DEFAULT_HOLD.as_secs());
    let at = body["executeAtUnix"].as_u64().unwrap();
    assert!(
        (before + hold..=after + hold).contains(&at),
        "{at} {before} {after}"
    );
    assert_records(
        &go["records"],
        &s.records_since(&mut seen),
        &[(&id, "<shredId>"), (&approval, "<approvalId>")],
        "dpo_commit",
    );

    let go = step(steps, "list_scheduled");
    let w = s.admin_do(s.token("auditor"), "GET", "erasures", "").await;
    assert_eq!(w.status, 200);
    assert_eq!(sorted_keys(&w.body), go["response_keys"]);
    let listed = w.json();
    assert_eq!(listed["count"], go["count"]);
    let entry = &listed["scheduled"][0];
    let go_entry = &go["scheduled"][0];
    assert_eq!(sorted_keys(&entry.to_string()), go_entry["_keys"]);
    assert_eq!(entry["id"], id.as_str());
    for field in ["approver", "legalBasis", "requester", "executesInSec"] {
        assert_eq!(entry[field], go_entry[field], "{field}");
    }
    assert_records(
        &go["records"],
        &s.records_since(&mut seen),
        &[],
        "list_scheduled",
    );

    assert!(capture["pseudonym_stable_during_hold"].as_bool().unwrap());
    assert_eq!(
        s.plane.run_due_shreds(s.now(), ("", 0)).unwrap() as u64,
        capture["run_due_before_window"].as_u64().unwrap()
    );
    assert_eq!(
        s.plane
            .run_due_shreds(
                s.now() + WINDOW,
                (ROOT, capture["checkpoint_tree_size"].as_u64().unwrap())
            )
            .unwrap() as u64,
        capture["run_due_after_window"].as_u64().unwrap()
    );
    assert_records(
        &capture["shred_records"],
        &s.records_since(&mut seen),
        &[(&ROOT[..12], "<root12>")],
        "shred_records",
    );
    assert_eq!(
        vault.resolve("psn-handle-X").unwrap().is_some(),
        capture["handle_resolvable_after"].as_bool().unwrap()
    );

    let go = step(steps, "list_after_shred");
    let w = s.admin_do(s.token("auditor"), "GET", "erasures", "").await;
    assert_eq!(
        (w.status, w.body.as_str()),
        (200, go["response"].as_str().unwrap())
    );
}

/// The cancel cases of the capture: role refusals, unknown, empty and bare
/// ids, a path without the suffix, the DPO cancel, a repeat, wrong methods.
#[tokio::test]
async fn cancel_cases_match_go_capture() {
    let golden = golden();
    let capture = &golden["cancel"];
    let vault = scoped_vault();
    let s = erasure_stack(&vault).await;
    let mut ledger = Ledger::new(scope());
    let p0 = vault.pseudonymize("subjY", "9.9.9.9").unwrap();
    s.plane
        .retain_erasure_permit("subjY", ledger.erasure_permit("subjY"))
        .unwrap();
    let req = s
        .admin_do(
            s.token("operator"),
            "POST",
            "erasure",
            r#"{"Subject":"subjY","LegalBasis":"PIPA"}"#,
        )
        .await;
    let pending = s.plane.approvals.pending(s.now());
    let params: Vec<Value> = pending.iter().map(|action| json!(action.params)).collect();
    assert_eq!(
        json!(params),
        step(&capture["steps"], "approvals_params_raw")["params"]
    );
    let commit = s
        .admin_do(
            s.token("dpo"),
            "POST",
            &format!("approvals/{}", approval_id(&req)),
            "",
        )
        .await;
    let id = shred_id(&commit);
    let mut seen = s.journal.records.lock().unwrap().len();

    for go in capture["steps"].as_array().unwrap().iter().skip(1) {
        let case = go["case"].as_str().unwrap();
        let sub = go["path"]
            .as_str()
            .unwrap()
            .strip_prefix("/__hmn/admin/")
            .unwrap()
            .replace("<shredId>", &id);
        let w = s
            .admin_do(
                s.token(go["role"].as_str().unwrap()),
                go["method"].as_str().unwrap(),
                &sub,
                "",
            )
            .await;
        assert_eq!(w.status as u64, go["status"].as_u64().unwrap(), "{case}");
        assert_eq!(
            w.content_type,
            go["content_type"].as_str().unwrap(),
            "{case}"
        );
        assert_eq!(
            w.body.replace(&id, "<shredId>"),
            go["response"].as_str().unwrap(),
            "{case}"
        );
        assert_records(
            &go["records"],
            &s.records_since(&mut seen),
            &[(&id, "<shredId>")],
            case,
        );
    }
    assert_eq!(
        s.plane
            .run_due_shreds(s.now() + Duration::from_secs(10 * 60), ("", 0))
            .unwrap() as u64,
        capture["run_due_after_cancel"].as_u64().unwrap()
    );
    assert_eq!(
        vault.pseudonymize("subjY", "9.9.9.9").unwrap() == p0,
        capture["linkage_kept"].as_bool().unwrap()
    );
}

/// The capture's erasure of a subject the vault never keyed: the shred still
/// tombstones it and records `erasure.no_subject` (no checkpoint yet).
#[tokio::test]
async fn no_subject_matches_go_capture() {
    let golden = golden();
    let capture = &golden["no_subject"];
    let vault = scoped_vault();
    let s = erasure_stack(&vault).await;
    let mut ledger = Ledger::new(scope());
    s.plane
        .retain_erasure_permit("ghost", ledger.erasure_permit("ghost"))
        .unwrap();
    let commit = request_and_commit(&s, "ghost", "art17").await;
    assert_eq!(commit.status, 200, "{}", commit.body);
    let mut seen = s.journal.records.lock().unwrap().len();
    assert_eq!(
        s.plane.run_due_shreds(s.now() + WINDOW, ("", 0)).unwrap() as u64,
        capture["run_due"].as_u64().unwrap()
    );
    assert_records(
        &capture["records"],
        &s.records_since(&mut seen),
        &[],
        "no_subject",
    );
}

// --- Rust fail-closed rules -----------------------------------------------------

/// Two bearer principals alone never erase: without a staged ledger permit
/// for that subject the commit answers 409 and nothing is scheduled.
#[tokio::test]
async fn commit_without_permit_is_refused() {
    let vault = scoped_vault();
    let s = erasure_stack(&vault).await;
    let mut ledger = Ledger::new(scope());
    s.plane
        .retain_erasure_permit("other", ledger.erasure_permit("other"))
        .unwrap();
    let commit = request_and_commit(&s, "subjZ", "art17").await;
    assert_eq!(
        (commit.status, commit.body.as_str()),
        (
            409,
            "erasure commit needs a ledger-issued erasure permit for this subject\n"
        )
    );
    let w = s.admin_do(s.token("auditor"), "GET", "erasures", "").await;
    assert_eq!(w.body, "{\"count\":0,\"scheduled\":[]}\n");
    assert!(
        !s.journal
            .events()
            .iter()
            .any(|(_, reason)| reason.contains("scheduled"))
    );
}

/// Staging refuses a permit for another subject or scope, and a plane
/// without an erasure hold refuses staging and keeps the 409 and 404 answers.
#[tokio::test]
async fn staging_binds_subject_and_scope() {
    let vault = scoped_vault();
    let s = erasure_stack(&vault).await;
    let mut ledger = Ledger::new(scope());
    assert_eq!(
        s.plane
            .retain_erasure_permit("subjA", ledger.erasure_permit("subjB")),
        Err(PermitRefusal::Binding)
    );
    let other = Scope {
        installation: "installation".into(),
        node: "elsewhere".into(),
    };
    let mut foreign = Ledger::new(other);
    assert_eq!(
        s.plane
            .retain_erasure_permit("subjA", foreign.erasure_permit("subjA")),
        Err(PermitRefusal::Binding)
    );

    let bare = stack().await;
    assert_eq!(
        bare.plane
            .retain_erasure_permit("subjA", ledger.erasure_permit("subjA")),
        Err(PermitRefusal::Unattached)
    );
    for (method, sub) in [("GET", "erasures"), ("POST", "erasures/x/cancel")] {
        let w = bare.admin_do(bare.token("dpo"), method, sub, "").await;
        assert_eq!((w.status, w.body.as_str()), (404, "404 page not found\n"));
    }
    let commit = request_and_commit(&bare, "subjA", "art17").await;
    assert_eq!(commit.status, 409, "{}", commit.body);
    assert_eq!(bare.plane.run_due_shreds(WINDOW * 10, ("", 0)).unwrap(), 0);
}

/// An audit failure on the schedule record schedules nothing and keeps the
/// permit staged; one on the cancel record keeps the erasure pending.
#[tokio::test]
async fn audit_failure_schedules_and_cancels_nothing() {
    let vault = scoped_vault();
    let s = erasure_stack(&vault).await;
    let mut ledger = Ledger::new(scope());
    s.plane
        .retain_erasure_permit("subjQ", ledger.erasure_permit("subjQ"))
        .unwrap();
    let req = s
        .admin_do(
            s.token("operator"),
            "POST",
            "erasure",
            r#"{"Subject":"subjQ","LegalBasis":"art17"}"#,
        )
        .await;
    let id = approval_id(&req);
    // Fail only the schedule record: the approval record goes first.
    let clock = Clock {
        monotonic: s.now(),
        unix: 1_800_000_000,
    };
    let dpo = s
        .plane
        .auth
        .authenticate(Some(&format!("Bearer {}", s.token("dpo"))))
        .unwrap();
    let (action, _) = s
        .plane
        .approvals
        .approve(&id, &dpo, clock.monotonic)
        .unwrap();
    s.journal.fail.store(true, Ordering::SeqCst);
    let refused = s.plane.commit_erasure(
        &action.params,
        &action.requester,
        &dpo,
        clock.unix,
        clock.monotonic,
    );
    assert_eq!(
        (refused.status, refused.body.as_slice()),
        (503, &b"audit unavailable\n"[..])
    );
    s.journal.fail.store(false, Ordering::SeqCst);
    let listed = s.plane.list_erasures(clock.monotonic);
    assert_eq!(listed.body, b"{\"count\":0,\"scheduled\":[]}\n");

    // The permit stayed staged: the same approved action now schedules.
    let ok = s.plane.commit_erasure(
        &action.params,
        &action.requester,
        &dpo,
        clock.unix,
        clock.monotonic,
    );
    assert_eq!(ok.status, 200, "{:?}", String::from_utf8_lossy(&ok.body));
    let shred: Value = serde_json::from_slice(&ok.body).unwrap();
    let shred = shred["shredId"].as_str().unwrap().to_owned();

    s.journal.fail.store(true, Ordering::SeqCst);
    let w = s
        .admin_do(
            s.token("dpo"),
            "POST",
            &format!("erasures/{shred}/cancel"),
            "",
        )
        .await;
    assert_eq!(w.status, 503, "{}", w.body);
    s.journal.fail.store(false, Ordering::SeqCst);
    let w = s.admin_do(s.token("auditor"), "GET", "erasures", "").await;
    assert_eq!(w.json()["count"], 1);
}

/// A shred the vault refuses (here a vault provisioned for another scope)
/// keeps the erasure pending and stops the run with an error.
#[tokio::test]
async fn refused_shred_stays_pending() {
    let vault = Arc::new(
        Vault::new_scoped(
            Mode::Hmac,
            64,
            Scope {
                installation: "installation".into(),
                node: "elsewhere".into(),
            },
        )
        .unwrap(),
    );
    let s = erasure_stack(&vault).await;
    let mut ledger = Ledger::new(scope());
    let p0 = vault.pseudonymize("subjR", "1.1.1.1").unwrap();
    s.plane
        .retain_erasure_permit("subjR", ledger.erasure_permit("subjR"))
        .unwrap();
    let commit = request_and_commit(&s, "subjR", "art17").await;
    assert_eq!(commit.status, 200, "{}", commit.body);
    assert!(s.plane.run_due_shreds(s.now() + WINDOW, ("", 0)).is_err());
    assert_eq!(vault.pseudonymize("subjR", "1.1.1.1").unwrap(), p0);
    let w = s.admin_do(s.token("auditor"), "GET", "erasures", "").await;
    assert_eq!(w.json()["count"], 1);
    assert!(!s.journal.has("erasure.completed"));
}

/// `executesInSec` is Go `int(executeAt.Sub(now).Seconds())`: whole seconds,
/// truncated toward zero, negative once overdue.
#[tokio::test]
async fn executes_in_sec_truncates_like_go() {
    let vault = scoped_vault();
    let s = erasure_stack(&vault).await;
    let mut ledger = Ledger::new(scope());
    s.plane
        .retain_erasure_permit("subjT", ledger.erasure_permit("subjT"))
        .unwrap();
    let dpo = s
        .plane
        .auth
        .authenticate(Some(&format!("Bearer {}", s.token("dpo"))))
        .unwrap();
    let params = BTreeMap::from([
        ("subject".to_owned(), "subjT".to_owned()),
        ("legalBasis".to_owned(), "art17".to_owned()),
    ]);
    let at = Duration::from_secs(100);
    let ok = s
        .plane
        .commit_erasure(&params, "operator-1", &dpo, 1_800_000_000, at);
    let body: Value = serde_json::from_slice(&ok.body).unwrap();
    assert_eq!(body["executeAtUnix"], 1_800_000_300_u64);
    for (now, expected) in [
        (at, 300),
        (at + Duration::from_millis(500), 299),
        (at + Duration::from_millis(300_500), 0),
        (at + Duration::from_secs(301), -1),
        (at + Duration::from_millis(301_900), -1),
    ] {
        let listed: Value = serde_json::from_slice(&s.plane.list_erasures(now).body).unwrap();
        assert_eq!(listed["scheduled"][0]["executesInSec"], expected, "{now:?}");
    }
}
