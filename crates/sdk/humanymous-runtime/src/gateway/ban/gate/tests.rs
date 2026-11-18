//! Kernel ports of legacy internal/gate/ban_integration_test.go and the
//! `gate.ban` monitor case of settings_runtime_test.go at baseline 981d25c.
//! Expected sequences were measured by running the Go oracle (Go 1.25.3):
//! with soft 8 and hard 10 the 8th and 9th requests record a soft breach and
//! the 10th is blocked with `ban.applied` then `ban.enforced`.
use super::*;
use crate::audit::log::{Config as LogConfig, Log};
use crate::audit::storage::{Wal, WalOptions};
use crate::audit::verification::verify;
use crate::gateway::ban::{Store, escalation};
use crate::gateway::token::bind_key;
use crate::network::history::rate::{Config, Limits};
use std::sync::atomic::{AtomicU64, Ordering};

const T0: Duration = Duration::from_secs(1_000_000);
const HOUR: Duration = Duration::from_secs(3600);
const SUBJECT: Subject = Subject {
    tenant: "test",
    pseudonym: "",
    route_class: "static",
};

fn store(soft: usize, hard: usize) -> Store {
    let limits = Limits::new(64, 64, 128, Duration::from_secs(60), 1 << 20, 64).unwrap();
    let rate = Config::new(Duration::from_secs(60), soft, hard, limits).unwrap();
    Store::new(rate, 64).unwrap()
}
fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}
fn at(i: u64) -> Duration {
    T0 + Duration::from_millis(i)
}
fn events(gate: &Gate) -> Vec<(&str, &str, &str)> {
    gate.records
        .iter()
        .map(|r| {
            (
                r.event_type.as_str(),
                r.actor.kind.as_str(),
                r.enforcement_action.as_str(),
            )
        })
        .collect()
}
fn browser() -> Option<String> {
    bind_key(
        "Mozilla/5.0 Chrome/126.0 Safari/537.36",
        "en-US,en;q=0.9",
        "\"Chromium\";v=\"126\"",
    )
}

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Chain {
    dir: std::path::PathBuf,
    log: Option<Log>,
}
impl Chain {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "humanymous-ban-gate-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let config = LogConfig {
            node_id: "test".into(),
            hmac_key: b"k".to_vec(),
            signing_seed: [7; 32],
            checkpoint_every: 8,
            witness: None,
        };
        let wal = Wal::open(&dir, WalOptions::default()).unwrap();
        let log = Log::open(wal, config, None, true).unwrap();
        Self {
            dir,
            log: Some(log),
        }
    }
    fn log(&mut self) -> &mut Log {
        self.log.as_mut().unwrap()
    }
}
impl Drop for Chain {
    fn drop(&mut self) {
        drop(self.log.take());
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Go TestAutoBanEndToEnd: a flood past the hard cutpoint is banned and then
/// dropped before the origin; the ban.applied record is in a chain that
/// verifies; an unrelated address is not collaterally banned.
#[test]
fn flood_is_auto_banned_audited_and_spares_other_address() {
    let mut ledger = store(8, 10);
    let mut chain = Chain::new();
    let keys = keys(ip("203.0.113.9"), None).unwrap();
    let mut origin_hits = 0;
    let mut blocked_at = None;
    for i in 0..20 {
        let gate = evaluate(&mut ledger, &keys, Mode::Enforce, &SUBJECT, at(i)).unwrap();
        // The caller appends every record before acting on the decision.
        for record in &gate.records {
            chain.log().append(record.clone()).unwrap();
        }
        let expected: &[(&str, &str, &str)] = match i {
            0..=6 => &[],
            7 | 8 => &[("ratelimit.soft_exceeded", "subject", "rate_limit")],
            9 => &[
                ("ban.applied", "system", "block"),
                ("ban.enforced", "subject", "block"),
            ],
            _ => &[("ban.enforced", "subject", "block")],
        };
        assert_eq!(events(&gate), expected, "request {i}");
        match gate.block {
            Some(block) => {
                blocked_at.get_or_insert(i);
                assert_eq!(block.entry.key.as_str(), "ip:203.0.113.9");
                // Whole seconds, truncated like Go's int(remainingSecs).
                assert_eq!(block.retry_after, Some(if i == 9 { 3600 } else { 3599 }));
                assert!(!block.dual_control_required);
            }
            None => origin_hits += 1,
        }
    }
    assert_eq!(blocked_at, Some(9));
    assert_eq!(origin_hits, 9, "the origin is never contacted once banned");

    let records = chain.log().records().unwrap();
    let applied = records
        .iter()
        .find(|r| r.event_type == "ban.applied")
        .expect("no ban.applied record");
    assert_eq!(applied.verdict, "DENY");
    assert_eq!(applied.triggered_rules, Some(vec!["HR-21".to_owned()]));
    assert_eq!(applied.enforcement_mode, "enforce");
    assert_eq!(
        applied.fail_reason,
        "auto ban (strike 1): rate limit exceeded (auto)"
    );
    let checkpoints = chain.log().checkpoints().unwrap();
    let public = ed25519_dalek::SigningKey::from_bytes(&[7; 32])
        .verifying_key()
        .to_bytes();
    let report = verify(&records, &checkpoints, b"k", Some(&public));
    assert!(report.ok, "audit chain broken: {report:?}");

    let other = keys_for("198.51.100.2");
    let gate = evaluate(&mut ledger, &other, Mode::Enforce, &SUBJECT, at(20)).unwrap();
    assert_eq!(gate.block, None, "an unrelated IP was collaterally banned");
}

fn keys_for(address: &str) -> Vec<Key> {
    keys(ip(address), None).unwrap()
}

/// Go TestFingerprintAutoBan: a flood that rotates its address but keeps one
/// header fingerprint is banned by the `fp:` key, and a fresh address with the
/// same fingerprint is then blocked. The fingerprint is Go's measured bindKey.
#[test]
fn ip_rotating_flood_is_banned_by_fingerprint() {
    let fingerprint = browser();
    assert_eq!(fingerprint.as_deref(), Some("d_Tues8bRjsNLZHw"));
    let mut ledger = store(8, 10);
    let mut first_block = None;
    for i in 0..20u64 {
        let address = format!("10.0.{}.{}", i / 250, i % 250);
        let keys = keys(ip(&address), fingerprint.as_deref()).unwrap();
        let gate = evaluate(&mut ledger, &keys, Mode::Enforce, &SUBJECT, at(i)).unwrap();
        if gate.block.is_some() {
            first_block.get_or_insert(i);
        }
    }
    assert_eq!(first_block, Some(9));
    let keys = keys(ip("172.16.9.9"), fingerprint.as_deref()).unwrap();
    let gate = evaluate(&mut ledger, &keys, Mode::Enforce, &SUBJECT, at(20)).unwrap();
    let block = gate
        .block
        .expect("rotated-IP flood not caught by fingerprint ban");
    assert_eq!(block.entry.key.as_str(), "fp:d_Tues8bRjsNLZHw");
    let bans = BanLedger::list(&mut ledger, at(20));
    assert!(bans.iter().any(|b| b.key.as_str().starts_with("fp:")));
    assert!(bans.iter().all(|b| !b.key.as_str().starts_with("ip:")));
    // Without a User-Agent there is no fingerprint key at all.
    assert_eq!(keys_for("172.16.9.9").len(), 1);
    assert_eq!(bind_key("", "en", "x"), None);
}

/// Kernel half of Go TestConsoleBanManagement: a manual temporary ban blocks
/// with Retry-After and the incident pointer, and lifting it clears the gate.
/// The authenticated admin HTTP routes are not part of this kernel.
#[test]
fn manual_temporary_ban_blocks_with_retry_after_until_lifted() {
    let mut ledger = store(1000, 1000);
    let key = Key::parse("ip:192.0.2.44").unwrap();
    BanLedger::add_temporary(
        &mut ledger,
        &key,
        "manual investigation",
        Some("INC-9"),
        HOUR,
        T0,
    )
    .unwrap();
    let keys = keys_for("192.0.2.44");
    let later = T0 + Duration::from_millis(1500);
    let gate = evaluate(&mut ledger, &keys, Mode::Enforce, &SUBJECT, later).unwrap();
    let block = gate.block.clone().expect("manually-banned IP not blocked");
    assert_eq!(block.retry_after, Some(3598));
    assert_eq!(
        block.headers(),
        vec![
            ("X-Hmn-Incident", "see-audit-log".to_owned()),
            ("Retry-After", "3598".to_owned()),
        ]
    );
    assert_eq!(BLOCK_STATUS, 403);
    assert_eq!(
        BLOCK_BODY,
        "Source temporarily blocked (rate limit / abuse protection)."
    );
    assert_eq!(
        events(&gate),
        [("ban.enforced", "subject", "block")],
        "a blocked request records only the enforcement"
    );
    assert_eq!(
        gate.records[0].fail_reason,
        "manual ban (strike 0): manual investigation"
    );
    let listed = BanLedger::list(&mut ledger, later);
    assert!(listed.iter().any(|b| b.key == key));
    assert_eq!(BanLedger::lift(&mut ledger, &key), Ok(true));
    let gate = evaluate(&mut ledger, &keys, Mode::Enforce, &SUBJECT, later).unwrap();
    assert_eq!(gate.block, None, "lifted ban still blocking");
    assert!(gate.records.is_empty());
}

/// Go TestSettingsGateBanMonitorIsAuditOnly plus the measured monitor sequence:
/// soft then hard rate records, a would_block record for an existing ban, and
/// no ban state is ever created or enforced. Shadow only renames the mode.
#[test]
fn monitor_and_shadow_modes_are_audit_only() {
    for (mode, name) in [(Mode::Monitor, "monitor"), (Mode::Shadow, "shadow")] {
        let mut ledger = store(1, 2);
        let banned = Key::parse("ip:198.51.100.45").unwrap();
        BanLedger::add_temporary(&mut ledger, &banned, "manual investigation", None, HOUR, T0)
            .unwrap();
        let mut seen = Vec::new();
        for (i, address) in [
            "198.51.100.44",
            "198.51.100.44",
            "198.51.100.44",
            "198.51.100.45",
        ]
        .into_iter()
        .enumerate()
        {
            let keys = keys_for(address);
            let gate = evaluate(&mut ledger, &keys, mode, &SUBJECT, at(i as u64)).unwrap();
            assert_eq!(gate.block, None, "{name} request {i} blocked");
            assert!(gate.records.iter().all(|r| r.enforcement_mode == name));
            seen.push(
                events(&gate)
                    .into_iter()
                    .map(|(e, k, a)| format!("{e}/{k}/{a}"))
                    .collect::<Vec<_>>(),
            );
            if i == 3 {
                assert_eq!(gate.records[0].verdict, "DENY");
                assert_eq!(
                    gate.records[0].fail_reason,
                    "manual ban (strike 0): manual investigation (audit only)"
                );
            }
        }
        assert_eq!(
            seen,
            [
                vec!["ratelimit.soft_exceeded/subject/would_rate_limit"],
                vec!["ratelimit.hard_exceeded/subject/would_auto_ban"],
                vec!["ratelimit.hard_exceeded/subject/would_auto_ban"],
                vec![
                    "ban.enforced/subject/would_block",
                    "ratelimit.soft_exceeded/subject/would_rate_limit",
                ],
            ],
            "{name}"
        );
        let bans = BanLedger::list(&mut ledger, at(4));
        assert_eq!(bans.len(), 1, "{name} mode created ban state");
    }
}

#[test]
fn off_mode_checks_and_counts_nothing() {
    let mut ledger = store(2, 3);
    let keys = keys_for("198.51.100.44");
    for i in 0..5 {
        let gate = evaluate(&mut ledger, &keys, Mode::Off, &SUBJECT, at(i)).unwrap();
        assert_eq!(gate, Gate::default());
    }
    let gate = evaluate(&mut ledger, &keys, Mode::Enforce, &SUBJECT, at(5)).unwrap();
    assert!(gate.records.is_empty(), "off mode must not count requests");
    assert_eq!(gate.block, None);
}

/// Go measured `banReason` strings for auto, manual temporary and permanent bans.
#[test]
fn reason_matches_legacy_strings() {
    let entry = |source, strike, until| Entry {
        key: Key::parse("ip:192.0.2.1").unwrap(),
        reason: "rate limit exceeded (auto)".into(),
        source,
        incident: None,
        created: T0,
        until,
        strike,
    };
    assert_eq!(
        reason(&entry(Source::Auto, 2, Some(T0 + HOUR))),
        "auto ban (strike 2): rate limit exceeded (auto)"
    );
    let mut manual = entry(Source::Manual, 0, Some(T0 + HOUR));
    manual.reason = "manual investigation".into();
    assert_eq!(
        reason(&manual),
        "manual ban (strike 0): manual investigation"
    );
    let mut permanent = entry(Source::DualControl, 0, None);
    permanent.reason = "abuse".into();
    assert_eq!(reason(&permanent), "manual ban (permanent): abuse");
}

/// A permanent ban carries no Retry-After; an expiring one never reports zero.
#[test]
fn retry_after_is_whole_remaining_seconds_of_a_temporary_ban() {
    let block = |until: Option<Duration>, now: Duration| {
        let entry = Entry {
            key: Key::parse("ip:192.0.2.1").unwrap(),
            reason: "r".into(),
            source: Source::Manual,
            incident: None,
            created: T0,
            until,
            strike: 0,
        };
        let mut gate = Gate::default();
        enforce(&mut gate, &SUBJECT, entry, false, now);
        gate.block.unwrap().retry_after
    };
    assert_eq!(block(Some(T0 + HOUR), T0), Some(3600));
    assert_eq!(
        block(Some(T0 + HOUR), T0 + HOUR - Duration::from_millis(1)),
        None
    );
    assert_eq!(block(None, T0), None);
}

/// Fail closed: a rate history that refuses (clock regression) is an error the
/// caller must turn into a refusal, never a silent pass.
#[test]
fn ledger_refusal_is_returned_not_passed() {
    let mut ledger = store(8, 10);
    let metered = keys_for("203.0.113.9");
    evaluate(&mut ledger, &metered, Mode::Enforce, &SUBJECT, at(10)).unwrap();
    let refused = evaluate(&mut ledger, &metered, Mode::Enforce, &SUBJECT, at(0));
    assert!(matches!(refused, Err(Refusal::Rate(_))), "{refused:?}");
    assert_eq!(
        keys(ip("::ffff:203.0.113.9"), Some("x")),
        Err(Refusal::Key),
        "a fingerprint shorter than a key allows is refused"
    );
}

/// Review finding: an audit-only mode must never deny. A rate-history refusal
/// (clock regression here) is recorded as `ratelimit.unavailable` and the
/// request continues, while enforce mode still returns the refusal.
#[test]
fn monitor_and_shadow_record_rate_refusal_and_never_block() {
    for (mode, name) in [(Mode::Monitor, "monitor"), (Mode::Shadow, "shadow")] {
        let mut ledger = store(8, 10);
        let metered = keys_for("203.0.113.9");
        evaluate(&mut ledger, &metered, mode, &SUBJECT, at(10)).unwrap();
        let gate = evaluate(&mut ledger, &metered, mode, &SUBJECT, at(0))
            .unwrap_or_else(|refusal| panic!("{name} must not refuse: {refusal:?}"));
        assert_eq!(gate.block, None, "{name} must never block");
        assert_eq!(
            events(&gate),
            [("ratelimit.unavailable", "subject", "would_refuse")],
            "{name}"
        );
        assert_eq!(gate.records[0].enforcement_mode, name);
        assert_eq!(
            gate.records[0].fail_reason,
            "rate history unavailable (audit only)"
        );
        assert!(BanLedger::list(&mut ledger, at(11)).is_empty());
    }
    let mut ledger = store(8, 10);
    let metered = keys_for("203.0.113.9");
    evaluate(&mut ledger, &metered, Mode::Enforce, &SUBJECT, at(10)).unwrap();
    assert!(evaluate(&mut ledger, &metered, Mode::Enforce, &SUBJECT, at(0)).is_err());
}

/// The fourth auto strike keeps the 24h term (no automatic permanent ban) and
/// the gate's Block carries `dual_control_required` so the composition can
/// propose the dual-controlled permanent ban.
#[test]
fn fourth_strike_block_flags_dual_control() {
    let mut ledger = store(8, 10);
    let metered = keys_for("203.0.113.9");
    let mut i = 0;
    for strike in 1..=4u32 {
        let block = loop {
            let gate = evaluate(&mut ledger, &metered, Mode::Enforce, &SUBJECT, at(i)).unwrap();
            i += 1;
            if let Some(block) = gate.block.clone() {
                assert_eq!(
                    events(&gate),
                    [
                        ("ban.applied", "system", "block"),
                        ("ban.enforced", "subject", "block")
                    ]
                );
                break block;
            }
        };
        assert_eq!(block.entry.strike, strike);
        assert_eq!(block.dual_control_required, strike == 4, "strike {strike}");
        assert!(!block.entry.permanent());
        let term = block.entry.until.unwrap() - block.entry.created;
        assert_eq!(term, escalation(strike));
        assert!(block.retry_after.is_some());
        assert_eq!(BanLedger::lift(&mut ledger, &block.entry.key), Ok(true));
    }
}
