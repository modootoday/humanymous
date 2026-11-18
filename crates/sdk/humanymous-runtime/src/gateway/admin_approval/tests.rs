use super::*;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const MINUTE: Duration = Duration::from_secs(60);

fn params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn op(id: &str, role: Role) -> Operator {
    Operator {
        id: id.to_owned(),
        role,
    }
}

fn ban(store: &ApprovalStore, key: &str, requester: &str, now: Duration) -> PendingAction {
    store
        .create(
            "ban",
            &params(&[("key", key)]),
            requester,
            Role::Approver,
            now,
        )
        .expect("create")
}

#[test]
fn role_wire_names_round_trip_exactly() {
    for role in [Role::Auditor, Role::Operator, Role::Approver, Role::Dpo] {
        assert_eq!(Role::parse(role.as_str()), Some(role));
    }
    assert_eq!(Role::parse("DPO"), None);
    assert_eq!(Role::parse(""), None);
}

#[test]
fn capabilities_follow_the_go_role_model() {
    let table = [
        (Role::Auditor, [true, false, false, false]),
        (Role::Operator, [true, true, false, false]),
        (Role::Approver, [true, false, true, false]),
        (Role::Dpo, [true, true, true, true]),
    ];
    for (role, expect) in table {
        let who = op("x", role);
        assert_eq!(
            [
                who.can_read(),
                who.can_operate(),
                who.can_approve(),
                who.can_approve_erasure()
            ],
            expect,
            "{role:?}"
        );
    }
}

#[test]
fn bearer_authentication_denies_by_default() {
    let auth = AdminAuth::new();
    assert_eq!(auth.authenticate(Some("Bearer anything")), None);
    auth.add("tok-auditor", "aud-1", Role::Auditor)
        .expect("add");
    auth.add("tok-dpo", "dpo-1", Role::Dpo).expect("add");
    assert_eq!(
        auth.authenticate(Some("Bearer tok-dpo")),
        Some(op("dpo-1", Role::Dpo))
    );
    assert_eq!(
        auth.authenticate(Some("Bearer tok-auditor")),
        Some(op("aud-1", Role::Auditor))
    );
    for header in [
        None,
        Some(""),
        Some("Bearer "),
        Some("bearer tok-dpo"),
        Some("Basic tok-dpo"),
        Some("tok-dpo"),
        Some("Bearer tok-dp"),
        Some("Bearer tok-dpo "),
        Some("Bearer  tok-dpo"),
    ] {
        assert_eq!(auth.authenticate(header), None, "{header:?}");
    }
    assert_eq!(DENY_STATUS, 404);
}

#[test]
fn registration_refuses_empty_and_duplicate_tokens() {
    let auth = AdminAuth::new();
    assert_eq!(auth.add("", "a", Role::Dpo), Err(AuthRefusal::EmptyToken));
    assert_eq!(auth.add("t", "", Role::Dpo), Err(AuthRefusal::EmptyId));
    auth.add("t", "a", Role::Auditor).expect("add");
    assert_eq!(auth.add("t", "b", Role::Dpo), Err(AuthRefusal::Duplicate));
    assert_eq!(
        auth.authenticate(Some("Bearer t")),
        Some(op("a", Role::Auditor))
    );
}

/// Go `TestApprovalStoreRules` (ws12_test.go).
#[test]
fn approval_store_rules() {
    let store = ApprovalStore::new(b"k", MINUTE);
    let now = Duration::from_secs(1000);
    let action = ban(&store, "ip:1.1.1.1", "op-1", now);
    assert_eq!(
        store.approve(&action.id, &op("op-1", Role::Approver), now),
        Err(ApprovalError::SelfApproval)
    );
    assert_eq!(
        store.approve(&action.id, &op("op-2", Role::Auditor), now),
        Err(ApprovalError::WrongRole)
    );
    let (committed, ticket) = store
        .approve(&action.id, &op("op-2", Role::Approver), now)
        .expect("distinct approver");
    assert_eq!(committed, action);
    assert_eq!(ticket.len(), 64);
    assert_eq!(
        store.approve(&action.id, &op("op-2", Role::Approver), now),
        Err(ApprovalError::NoPending)
    );
}

/// Go `TestWargameR071_ApprovalExpiredCannotCommit`.
#[test]
fn r071_approval_expired_cannot_commit() {
    let store = ApprovalStore::new(b"k", Duration::from_secs(10));
    let now = Duration::from_secs(1000);
    let action = ban(&store, "ip:203.0.113.71", "op-1", now);
    let later = now + Duration::from_secs(11);
    assert_eq!(
        store.approve(&action.id, &op("op-2", Role::Approver), later),
        Err(ApprovalError::Expired)
    );
    assert_eq!(
        store.approve(&action.id, &op("op-2", Role::Approver), later),
        Err(ApprovalError::NoPending),
        "an expired action is removed"
    );
}

#[test]
fn ttl_boundary_and_clock_regression_do_not_expire() {
    let store = ApprovalStore::new(b"k", Duration::from_secs(10));
    let now = Duration::from_secs(1000);
    let first = ban(&store, "ip:203.0.113.1", "op-1", now);
    let second = ban(&store, "ip:203.0.113.2", "op-1", now);
    assert!(
        store
            .approve(
                &first.id,
                &op("op-2", Role::Approver),
                now + Duration::from_secs(10)
            )
            .is_ok(),
        "exactly the TTL is still valid, as in Go"
    );
    assert!(
        store
            .approve(&second.id, &op("op-2", Role::Approver), Duration::ZERO)
            .is_ok(),
        "a time before creation never expires an entry"
    );
}

/// Go `TestWargameR072_ApprovalUnknownIDRejected`.
#[test]
fn r072_approval_unknown_id_rejected() {
    let store = ApprovalStore::new(b"k", MINUTE);
    assert_eq!(
        store.approve("no-such-id", &op("op-2", Role::Approver), Duration::ZERO),
        Err(ApprovalError::NoPending)
    );
}

/// Go `TestWargameR073_ConcurrentApproveExactlyOnce`.
#[test]
fn r073_concurrent_approve_exactly_once() {
    let store = Arc::new(ApprovalStore::new(b"k", MINUTE));
    let action = ban(&store, "ip:203.0.113.73", "op-1", Duration::ZERO);
    let wins = Arc::new(AtomicUsize::new(0));
    let threads: Vec<_> = (0..16)
        .map(|index| {
            let (store, wins, id) = (store.clone(), wins.clone(), action.id.clone());
            std::thread::spawn(move || {
                let approver = op(&format!("appr-{index}"), Role::Approver);
                if store.approve(&id, &approver, Duration::ZERO).is_ok() {
                    wins.fetch_add(1, Ordering::SeqCst);
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().expect("join");
    }
    assert_eq!(wins.load(Ordering::SeqCst), 1);
}

/// Go `TestWargameR074_DoubleCommitAfterSuccessFails`.
#[test]
fn r074_double_commit_after_success_fails() {
    let store = ApprovalStore::new(b"k", MINUTE);
    let action = ban(&store, "ip:203.0.113.74", "op-1", Duration::ZERO);
    store
        .approve(&action.id, &op("op-2", Role::Approver), Duration::ZERO)
        .expect("first");
    assert_eq!(
        store.approve(&action.id, &op("op-3", Role::Approver), Duration::ZERO),
        Err(ApprovalError::NoPending)
    );
}

/// Go `TestWargameR075_DPOCanApproveBanRole`.
#[test]
fn r075_dpo_can_approve_ban_role() {
    let store = ApprovalStore::new(b"k", MINUTE);
    let action = ban(&store, "ip:203.0.113.75", "op-1", Duration::ZERO);
    assert!(
        store
            .approve(&action.id, &op("dpo-1", Role::Dpo), Duration::ZERO)
            .is_ok()
    );
}

/// Go `TestWargameR076_ApproverCannotSelfRequestBanThenApprove`.
#[test]
fn r076_approver_cannot_self_request_ban_then_approve() {
    let store = ApprovalStore::new(b"k", MINUTE);
    let action = ban(&store, "ip:203.0.113.76", "dual-hat", Duration::ZERO);
    assert_eq!(
        store.approve(&action.id, &op("dual-hat", Role::Approver), Duration::ZERO),
        Err(ApprovalError::SelfApproval)
    );
}

#[test]
fn erasure_needs_dpo_not_approver() {
    let store = ApprovalStore::new(b"k", MINUTE);
    let action = store
        .create(
            "erasure",
            &params(&[("subject", "subj-r79"), ("legalBasis", "test")]),
            "op-1",
            Role::Dpo,
            Duration::ZERO,
        )
        .expect("create");
    assert_eq!(
        store.approve(&action.id, &op("appr-1", Role::Approver), Duration::ZERO),
        Err(ApprovalError::WrongRole)
    );
    assert!(
        store
            .approve(&action.id, &op("dpo-1", Role::Dpo), Duration::ZERO)
            .is_ok()
    );
}

/// Go `TestWargameR077_PendingListHidesExpired`.
#[test]
fn r077_pending_list_hides_expired() {
    let store = ApprovalStore::new(b"k", Duration::from_secs(5));
    let now = Duration::from_secs(2000);
    ban(&store, "ip:203.0.113.77", "op-1", now);
    assert_eq!(store.pending(now).len(), 1);
    assert!(store.pending(now + Duration::from_secs(6)).is_empty());
    assert!(store.pending(now).is_empty(), "expired entries are removed");
}

/// Go `TestWargameR078_TicketBindsParams`, plus the Go ticket byte layout.
#[test]
fn r078_ticket_binds_params() {
    let store = ApprovalStore::new(b"secret-ticket-key", MINUTE);
    let first = PendingAction {
        id: "abc123".into(),
        kind: "ban".into(),
        params: params(&[("key", "ip:203.0.113.78"), ("durationSec", "0")]),
        requester: "op-1".into(),
        needs_role: Role::Approver,
        created: Duration::ZERO,
    };
    let mut second = first.clone();
    second.params.insert("key".into(), "ip:198.51.100.1".into());
    assert_ne!(store.ticket(&first, "appr"), store.ticket(&second, "appr"));
    // HMAC-SHA256("secret-ticket-key", "abc123|ban|op-1|appr|key=ip:203.0.113.78
    // |durationSec=0|subject=|legalBasis=|overlayId=|parentConfigVersion=
    // |mutationClass=|body=|action="), as Go writes it.
    assert_eq!(
        store.ticket(&first, "appr"),
        "7f50509ca15d98dbcb1b19ce484397292ed83d9e59229c8e77bf6c45ca1f1a15"
    );
    let mut unbound = first.clone();
    unbound.params.insert("note".into(), "ignored".into());
    assert_eq!(
        store.ticket(&first, "appr"),
        store.ticket(&unbound, "appr"),
        "only the Go binding keys enter the ticket"
    );
}

/// Go `TestWargameR178_ApprovalTicketDiffersByApprover`.
#[test]
fn r178_approval_ticket_differs_by_approver() {
    let store = ApprovalStore::new(b"k", MINUTE);
    let action = ban(&store, "ip:203.0.113.178", "op", Duration::ZERO);
    assert_ne!(
        store.ticket(&action, "appr-1"),
        store.ticket(&action, "appr-2")
    );
}

/// Go `TestWargameR179_ApprovalTicketDiffersByKind`.
#[test]
fn r179_approval_ticket_differs_by_kind() {
    let store = ApprovalStore::new(b"k", MINUTE);
    let action = ban(&store, "ip:203.0.113.179", "op", Duration::ZERO);
    let mut erasure = action.clone();
    erasure.kind = "erasure".into();
    assert_ne!(store.ticket(&action, "a"), store.ticket(&erasure, "a"));
}

/// Go `TestWargameR180_HasRoleDPOSuperset`.
#[test]
fn r180_has_role_dpo_superset() {
    assert!(has_role(Role::Dpo, Role::Approver));
    assert!(!has_role(Role::Approver, Role::Dpo));
    assert!(!has_role(Role::Auditor, Role::Approver));
    assert!(!has_role(Role::Operator, Role::Approver));
    assert!(has_role(Role::Dpo, Role::Dpo));
}

/// Go `TestWargameR188_ConcurrentCreateDistinctIDs`.
#[test]
fn r188_concurrent_create_distinct_ids() {
    let store = ApprovalStore::new(b"k", MINUTE);
    let mut ids = BTreeSet::new();
    for _ in 0..20 {
        let action = ban(&store, "ip:203.0.113.188", "op", Duration::ZERO);
        assert_eq!(action.id.len(), ID_BYTES * 2);
        assert!(ids.insert(action.id), "duplicate approval id");
    }
}

/// Go `TestWargameR189_PendingParamsImmutableSnapshot`.
#[test]
fn r189_pending_params_immutable_snapshot() {
    let store = ApprovalStore::new(b"k", MINUTE);
    let mut caller = params(&[("key", "ip:203.0.113.189")]);
    let action = store
        .create("ban", &caller, "op", Role::Approver, Duration::ZERO)
        .expect("create");
    caller.insert("key".into(), "ip:198.51.100.1".into());
    let held = store.pending(Duration::ZERO);
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].id, action.id);
    assert_eq!(held[0].params["key"], "ip:203.0.113.189");
}

#[test]
fn full_store_refuses_after_collecting_expired() {
    let store = ApprovalStore::new(b"k", Duration::from_secs(5));
    for _ in 0..MAX_PENDING {
        ban(&store, "ip:203.0.113.1", "op", Duration::ZERO);
    }
    assert_eq!(
        store.create(
            "ban",
            &params(&[]),
            "op",
            Role::Approver,
            Duration::from_secs(5)
        ),
        Err(ApprovalError::Saturated)
    );
    assert!(
        store
            .create(
                "ban",
                &params(&[]),
                "op",
                Role::Approver,
                Duration::from_secs(6)
            )
            .is_ok()
    );
    assert_eq!(store.pending(Duration::from_secs(6)).len(), 1);
}
