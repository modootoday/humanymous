//! Store-level ports of legacy cmd/server/pass_handler_test.go and
//! pass_wargame_axis_test.go (baseline 981d25c) plus the bounds of
//! pass_store.go. Digest construction (traceDigest) belongs to the proof layer
//! and is not ported here; fixed opaque digests stand in for it.
use super::*;

const T0: Duration = Duration::from_secs(1_000);

fn digest(tag: u8) -> TraceDigest {
    TraceDigest([tag; 32])
}

fn indexed(i: u32) -> TraceDigest {
    let mut bytes = [0_u8; 32];
    bytes[..4].copy_from_slice(&i.to_be_bytes());
    TraceDigest(bytes)
}

/// TestReserveTraceBlocksReplay.
#[test]
fn reserve_trace_blocks_replay() {
    let store = PassStore::new();
    assert!(store.reserve_trace(digest(1), T0));
    assert!(!store.reserve_trace(digest(1), T0));
}

/// TestWargameR333_PassTraceReplayBlocked.
#[test]
fn wargame_r333_trace_replay_blocked() {
    let store = PassStore::new();
    assert!(store.reserve_trace(digest(33), T0), "first admit");
    assert!(!store.reserve_trace(digest(33), T0), "replay must block");
}

/// TestWargameR338_PassAxisHandlerClose (wall-clock `time.Now()` twice in Go;
/// a later caller time here).
#[test]
fn wargame_r338_replay_lock() {
    let store = PassStore::new();
    assert!(store.reserve_trace(digest(38), T0));
    assert!(!store.reserve_trace(digest(38), T0 + Duration::from_millis(1)));
}

#[test]
fn distinct_digests_are_independent() {
    let store = PassStore::new();
    assert!(store.reserve_trace(digest(1), T0));
    assert!(store.reserve_trace(digest(2), T0));
}

/// As in Go, an expired digest remains a replay until the amortized sweep
/// (at most every `TRACE_TTL / 4`) removes it: the window errs long.
#[test]
fn replay_window_reopens_only_after_ttl_and_sweep() {
    let store = PassStore::new();
    let one = Duration::from_secs(1);
    assert!(store.reserve_trace(digest(7), T0));
    assert!(!store.reserve_trace(digest(7), T0 + TRACE_TTL));
    assert!(!store.reserve_trace(digest(7), T0 + TRACE_TTL + one));
    let swept = T0 + TRACE_TTL + TRACE_TTL / 4 + one;
    assert!(store.reserve_trace(digest(7), swept));
}

#[test]
fn clock_rollback_never_expires_a_trace() {
    let store = PassStore::new();
    assert!(store.reserve_trace(digest(9), T0 + TRACE_TTL * 2));
    assert!(!store.reserve_trace(digest(9), T0));
}

/// Saturation fails closed: a fresh digest is treated as a replay until the
/// retained entries expire.
#[test]
fn trace_registry_saturation_fails_closed_then_drains() {
    let store = PassStore::new();
    for i in 0..u32::try_from(MAX_TRACES).unwrap() {
        assert!(store.reserve_trace(indexed(i), T0));
    }
    assert!(!store.reserve_trace(digest(0xee), T0 + Duration::from_secs(1)));
    let later = T0 + TRACE_TTL + Duration::from_secs(1);
    assert!(store.reserve_trace(digest(0xee), later));
}

#[test]
fn session_is_created_once_and_mutated_under_lock() {
    let store = PassStore::new();
    let nonce = PassNonce::from_entropy([0xab; NONCE_BYTES]);
    store
        .with_session("sid-1", T0, |s| {
            assert_eq!(s.issued_at, T0);
            s.instance += 1;
            s.tries = PASS_MAX_TRIES;
            s.nonce = Some(nonce.clone());
        })
        .unwrap();
    let (instance, exhausted, stored) = store
        .with_session("sid-1", T0 + Duration::from_secs(5), |s| {
            (s.instance, s.exhausted(), s.nonce.clone())
        })
        .unwrap();
    assert_eq!((instance, exhausted), (1, true));
    assert_eq!(stored, Some(nonce));
    assert_eq!(store.session_count(), 1);
}

#[test]
fn session_ids_are_bounded() {
    let store = PassStore::new();
    assert_eq!(store.with_session("", T0, |_| ()), Err(Refusal::SessionId));
    let long = "s".repeat(MAX_SESSION_ID_BYTES + 1);
    assert_eq!(
        store.with_session(&long, T0, |_| ()),
        Err(Refusal::SessionId)
    );
    let max = "s".repeat(MAX_SESSION_ID_BYTES);
    assert_eq!(store.with_session(&max, T0, |_| ()), Ok(()));
}

/// Stricter than Go: a store full of fresh sessions refuses instead of
/// serving an unstored ephemeral session; stale sessions are swept first.
#[test]
fn session_cap_sweeps_stale_then_refuses_fresh_overflow() {
    let store = PassStore::new();
    for i in 0..MAX_PASS_SESSIONS {
        store.with_session(&format!("s{i}"), T0, |_| ()).unwrap();
    }
    assert_eq!(
        store.with_session("overflow", T0 + SESSION_STALE, |_| ()),
        Err(Refusal::Saturated)
    );
    // Existing sessions stay reachable at the cap.
    assert_eq!(
        store.with_session("s0", T0 + SESSION_STALE, |s| s.tries),
        Ok(0)
    );
    let later = T0 + SESSION_STALE + Duration::from_secs(1);
    assert_eq!(store.with_session("overflow", later, |_| ()), Ok(()));
    assert_eq!(store.session_count(), 1);
}

#[test]
fn nonce_is_36_hex_and_fresh() {
    let a = PassNonce::generate().unwrap();
    let b = PassNonce::generate().unwrap();
    assert_eq!(a.to_hex().len(), 36);
    assert!(
        a.to_hex()
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
    assert_ne!(a, b);
    assert_eq!(
        PassNonce::from_entropy([0x0f; NONCE_BYTES]).to_hex(),
        "0f".repeat(NONCE_BYTES)
    );
    assert_eq!(format!("{a:?}"), "PassNonce(..)");
}

#[test]
fn record_counts_label_outcome_difficulty() {
    let store = PassStore::new();
    store.record("human", 3, true).unwrap();
    store.record("human", 3, true).unwrap();
    store.record("bot", 5, false).unwrap();
    let metrics = store.metrics();
    let rendered: Vec<(String, u64)> = metrics.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    assert_eq!(
        rendered,
        vec![
            ("bot|fail|d5".to_owned(), 1),
            ("human|pass|d3".to_owned(), 2)
        ]
    );
}

/// Stricter than Go: header-derived labels cannot grow the KPI map without
/// bound or forge the `|` key separator.
#[test]
fn record_bounds_labels_and_key_space() {
    let store = PassStore::new();
    for bad in [
        "",
        "a|pass|d1",
        "with space",
        &"x".repeat(MAX_LABEL_BYTES + 1),
    ] {
        assert_eq!(store.record(bad, 1, true), Err(Refusal::Label), "{bad}");
    }
    assert_eq!(store.record("redteam/strategy:v2", 1, true), Ok(()));
    let keys = u32::try_from(MAX_METRIC_KEYS).unwrap();
    for difficulty in 0..keys - 1 {
        store.record("bot", difficulty, false).unwrap();
    }
    assert_eq!(store.metrics().len(), MAX_METRIC_KEYS);
    let difficulty = keys;
    assert_eq!(
        store.record("bot", difficulty, false),
        Err(Refusal::MetricsFull)
    );
    assert_eq!(store.record("bot", 0, false), Ok(()));
    assert_eq!(Refusal::MetricsFull.code(), "pass_kpi_full");
}
