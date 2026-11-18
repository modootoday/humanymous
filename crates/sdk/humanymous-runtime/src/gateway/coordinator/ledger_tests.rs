//! Ports of legacy internal/gate/redisledger_test.go, redisratelimit behaviour
//! and wargame_axis_redis_r241_test.go (oracle 981d25c) against an in-process
//! RESP coordinator shared by several simulated gate nodes.
use super::*;
use crate::gateway::coordinator::redis::{ClientSettings, read_reply};
use crate::network::history::rate::Limits;
use std::{
    io::{BufReader, Write},
    net::TcpListener,
    thread,
};

const T0: Duration = Duration::from_secs(1_800_000_000);
const TTL: Duration = Duration::from_secs(600);

#[derive(Default)]
struct Coordinator {
    strings: BTreeMap<Vec<u8>, Vec<u8>>,
    windows: BTreeMap<Vec<u8>, Vec<(i128, Vec<u8>)>>,
    commands: Vec<Vec<Vec<u8>>>,
    /// Serve a SCAN cursor that never terminates.
    cycling_scan: bool,
}

type Shared = Arc<Mutex<Coordinator>>;

fn bulk_reply(bytes: &[u8]) -> Vec<u8> {
    [format!("${}\r\n", bytes.len()).as_bytes(), bytes, b"\r\n"].concat()
}

fn answer(state: &mut Coordinator, args: &[Vec<u8>]) -> Vec<u8> {
    state.commands.push(args.to_vec());
    match args[0].as_slice() {
        b"GET" => state
            .strings
            .get(&args[1])
            .map_or_else(|| b"$-1\r\n".to_vec(), |value| bulk_reply(value)),
        b"SET" => {
            state.strings.insert(args[1].clone(), args[2].clone());
            b"+OK\r\n".to_vec()
        }
        b"DEL" => {
            let removed = state.strings.remove(&args[1]).is_some();
            format!(":{}\r\n", u8::from(removed)).into_bytes()
        }
        b"SCAN" => {
            let prefix = &args[3][..args[3].len() - 1];
            let keys: Vec<Vec<u8>> = state
                .strings
                .keys()
                .filter(|key| key.starts_with(prefix))
                .cloned()
                .collect();
            let cursor: &[u8] = if state.cycling_scan { b"7" } else { b"0" };
            let mut reply = b"*2\r\n".to_vec();
            reply.extend(bulk_reply(cursor));
            reply.extend(format!("*{}\r\n", keys.len()).as_bytes());
            for key in keys {
                reply.extend(bulk_reply(&key));
            }
            reply
        }
        b"EVAL" => {
            let number = |index: usize| -> i128 {
                std::str::from_utf8(&args[index]).unwrap().parse().unwrap()
            };
            let (now, window) = (number(4), number(5));
            let hits = state.windows.entry(args[3].clone()).or_default();
            hits.retain(|(score, _)| *score > now - window);
            hits.push((now, args[7].clone()));
            format!(":{}\r\n", hits.len()).into_bytes()
        }
        _ => b"-ERR unknown command\r\n".to_vec(),
    }
}

/// Coordinator serving every connection on its own thread.
fn coordinator() -> (String, Shared) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let shared: Shared = Arc::default();
    let state = shared.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { return };
            let state = state.clone();
            thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut writer = stream;
                while let Ok(Reply::Array(items)) = read_reply(&mut reader, 0) {
                    let args: Vec<Vec<u8>> = items
                        .into_iter()
                        .map(|item| match item {
                            Reply::Bulk(bytes) => bytes,
                            _ => Vec::new(),
                        })
                        .collect();
                    let reply = answer(&mut state.lock().unwrap(), &args);
                    if writer.write_all(&reply).is_err() {
                        return;
                    }
                }
            });
        }
    });
    (address, shared)
}

fn node(address: &str) -> Arc<Client> {
    Arc::new(Client::new(address, ClientSettings::default()))
}

/// Client whose coordinator is down for the whole test.
fn unreachable() -> Arc<Client> {
    let address = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    Arc::new(Client::new(
        address.to_string(),
        ClientSettings {
            cooldown: Duration::from_secs(60),
            ..ClientSettings::default()
        },
    ))
}

fn key_material() -> Option<SigningKey> {
    SigningKey::new(b"fleet-key-for-tests".to_vec())
}

fn rate_config() -> Config {
    let limits = Limits::new(64, 64, 128, Duration::from_secs(60), 1 << 20, 64).unwrap();
    Config::new(Duration::from_secs(10), 3, 5, limits).unwrap()
}

fn sticky(verdict: Verdict, updated: Duration) -> Sticky {
    Sticky {
        verdict,
        risk: 0.9,
        rule: "rule".into(),
        top: vec!["signal".into()],
        updated,
    }
}

fn ban_key(text: &str) -> Key {
    Key::parse(text).unwrap()
}

fn ban_node(address: &str, signing: Option<SigningKey>) -> SharedBanLedger {
    SharedBanLedger::new(node(address), rate_config(), 16, signing)
        .unwrap()
        .with_clock(|| T0)
}

// --- seal -------------------------------------------------------------------

#[test]
fn seal_binds_the_value_to_its_key_and_refuses_tampering() {
    let signing = key_material();
    let sealed = seal(signing.as_ref(), "hmn:verdict:a", b"payload");
    assert_eq!(
        open(signing.as_ref(), "hmn:verdict:a", &sealed),
        Some(b"payload".as_slice())
    );
    assert_eq!(open(signing.as_ref(), "hmn:verdict:b", &sealed), None);
    let mut altered = sealed.clone();
    altered[0] ^= 1;
    assert_eq!(open(signing.as_ref(), "hmn:verdict:a", &altered), None);
    assert_eq!(open(signing.as_ref(), "hmn:verdict:a", b"payload"), None);
    let other = SigningKey::new(b"other".to_vec());
    assert_eq!(open(other.as_ref(), "hmn:verdict:a", &sealed), None);
    assert_eq!(seal(None, "k", b"plain"), b"plain");
    assert_eq!(open(None, "k", b"plain"), Some(b"plain".as_slice()));
    assert!(SigningKey::new(Vec::new()).is_none());
    assert_eq!(format!("{:?}", signing.unwrap()), "SigningKey(..)");
}

// --- verdicts ---------------------------------------------------------------

#[test]
fn a_verdict_set_on_one_node_is_seen_on_another() {
    let (address, shared) = coordinator();
    let first = SharedVerdictLedger::new(node(&address), TTL, key_material());
    let second = SharedVerdictLedger::new(node(&address), TTL, key_material());
    assert!(first.set("sid", sticky(Verdict::Deny, T0)));
    assert_eq!(second.get("sid", T0), sticky(Verdict::Deny, T0));
    let state = shared.lock().unwrap();
    let set = state
        .commands
        .iter()
        .find(|command| command[0] == b"SET")
        .unwrap();
    assert_eq!(set[1], b"hmn:verdict:sid");
    assert_eq!(set[3..], [b"PX".to_vec(), b"600000".to_vec()]);
}

#[test]
fn an_absent_verdict_is_unknown() {
    let (address, _) = coordinator();
    let ledger = SharedVerdictLedger::new(node(&address), TTL, key_material());
    assert_eq!(ledger.get("nobody", T0).verdict, Verdict::Unknown);
    assert_eq!(ledger.degradation().untrusted(), 0);
}

#[test]
fn relocated_forged_and_stale_verdicts_fall_back_to_the_local_view() {
    let (address, shared) = coordinator();
    let writer = SharedVerdictLedger::new(node(&address), TTL, key_material());
    let reader = SharedVerdictLedger::new(node(&address), TTL, key_material());
    writer.set("deny", sticky(Verdict::Deny, T0));
    // Relocate the DENY value onto another session's key.
    {
        let mut state = shared.lock().unwrap();
        let value = state.strings[b"hmn:verdict:deny".as_slice()].clone();
        state.strings.insert(b"hmn:verdict:other".to_vec(), value);
        state
            .strings
            .insert(b"hmn:verdict:forged".to_vec(), br#"{"v":"allow"}"#.to_vec());
    }
    assert_eq!(reader.get("other", T0).verdict, Verdict::Unknown);
    assert_eq!(reader.get("forged", T0).verdict, Verdict::Unknown);
    // Freshness: a value older than the TTL is not trusted.
    assert_eq!(
        reader
            .get("deny", T0 + TTL + Duration::from_secs(1))
            .verdict,
        Verdict::Unknown
    );
    assert_eq!(reader.degradation().untrusted(), 3);
    assert_eq!(reader.get("deny", T0).verdict, Verdict::Deny);
}

#[test]
fn a_rolled_back_verdict_is_refused() {
    let (address, shared) = coordinator();
    let writer = SharedVerdictLedger::new(node(&address), TTL, key_material());
    let reader = SharedVerdictLedger::new(node(&address), TTL, key_material());
    writer.set("sid", sticky(Verdict::Allow, T0));
    let old = shared.lock().unwrap().strings[b"hmn:verdict:sid".as_slice()].clone();
    let later = T0 + Duration::from_secs(5);
    writer.set("sid", sticky(Verdict::Deny, later));
    assert_eq!(reader.get("sid", later).verdict, Verdict::Deny);
    // Replay the earlier, still-signed ALLOW.
    shared
        .lock()
        .unwrap()
        .strings
        .insert(b"hmn:verdict:sid".to_vec(), old);
    assert_eq!(reader.get("sid", later).verdict, Verdict::Unknown);
    assert_eq!(reader.degradation().untrusted(), 1);
    // The high-water mark ages out with the TTL.
    reader.gc(later + TTL + Duration::from_secs(1));
    assert_eq!(reader.get("sid", T0).verdict, Verdict::Allow);
}

#[test]
fn a_coordinator_outage_serves_the_local_verdict() {
    let ledger = SharedVerdictLedger::new(unreachable(), TTL, key_material());
    assert!(ledger.set("sid", sticky(Verdict::Deny, T0)));
    assert_eq!(ledger.get("sid", T0).verdict, Verdict::Deny);
    assert_eq!(ledger.degradation().unavailable(), 2);
}

// --- rate -------------------------------------------------------------------

#[test]
fn a_flood_split_across_nodes_is_counted_on_aggregate() {
    let (address, shared) = coordinator();
    let mut first = SharedRateLimiter::new(node(&address), rate_config());
    let mut second = SharedRateLimiter::new(node(&address), rate_config());
    let mut last = None;
    for step in 0..5u64 {
        let limiter = if step % 2 == 0 {
            &mut first
        } else {
            &mut second
        };
        last = Some(
            limiter
                .observe("ip:1.2.3.4", T0 + Duration::from_millis(step))
                .unwrap(),
        );
    }
    assert_eq!(
        last,
        Some(Observation::Counted {
            count: 5,
            level: Level::Hard,
            coverage: Coverage::Retained
        })
    );
    // Hits outside the window are evicted by the script.
    assert!(matches!(
        first.observe("ip:1.2.3.4", T0 + Duration::from_secs(20)),
        Ok(Observation::Counted { count: 1, .. })
    ));
    let state = shared.lock().unwrap();
    let eval = &state.commands[0];
    assert_eq!(eval[1], SLIDING_WINDOW_SCRIPT.as_bytes());
    assert_eq!(eval[3], b"hmn:rl:ip:1.2.3.4");
    assert_eq!(eval[5], b"10000000000");
    assert_eq!(eval[6], b"60000");
    assert_eq!(
        first.observe("", T0 + Duration::from_secs(21)),
        Ok(Observation::IgnoredEmptyKey)
    );
}

#[test]
fn a_rate_outage_falls_back_to_the_local_counter() {
    let mut limiter = SharedRateLimiter::new(unreachable(), rate_config());
    for step in 0..3u64 {
        limiter
            .observe("ip:1.2.3.4", T0 + Duration::from_millis(step))
            .unwrap();
    }
    assert!(matches!(
        limiter.observe("ip:1.2.3.4", T0 + Duration::from_millis(9)),
        Ok(Observation::Counted {
            count: 4,
            level: Level::Soft,
            ..
        })
    ));
    assert_eq!(limiter.degradation().unavailable(), 4);
}

#[test]
fn widening_the_shared_window_reports_the_coverage_gap() {
    let (address, _) = coordinator();
    let mut limiter = SharedRateLimiter::new(node(&address), rate_config());
    let effect = limiter
        .configure(Duration::from_secs(30), 3, 5, T0)
        .unwrap();
    assert!(effect.window_expanded);
    let until = T0 + Duration::from_secs(30);
    assert!(matches!(
        limiter.observe("ip:1.2.3.4", T0),
        Ok(Observation::Counted {
            coverage: Coverage::WindowExpanded { complete_after },
            ..
        }) if complete_after == until
    ));
    assert!(matches!(
        limiter.observe("ip:1.2.3.4", until),
        Ok(Observation::Counted {
            coverage: Coverage::Retained,
            ..
        })
    ));
}

// --- bans -------------------------------------------------------------------

fn flood(ledger: &mut SharedBanLedger, key: &Key, now: Duration) -> Entry {
    for step in 0..8 {
        if let Outcome::AutoBanned { entry, .. } = ledger
            .observe(key, now + Duration::from_millis(step))
            .unwrap()
        {
            return entry;
        }
    }
    panic!("hard flood must auto-ban");
}

#[test]
fn an_auto_ban_on_one_node_is_enforced_on_another() {
    let (address, shared) = coordinator();
    let mut first = ban_node(&address, key_material());
    let mut second = ban_node(&address, key_material());
    let key = ban_key("ip:9.9.9.9");
    let entry = flood(&mut first, &key, T0);
    assert_eq!(second.check(&key, T0), Some(entry.clone()));
    let state = shared.lock().unwrap();
    let set = state
        .commands
        .iter()
        .find(|command| command[0] == b"SET")
        .unwrap();
    assert_eq!(set[1], b"hmn:ban:ip:9.9.9.9");
    let expiry: u64 = std::str::from_utf8(&set[4]).unwrap().parse().unwrap();
    assert!((3_600_000..3_600_100).contains(&expiry), "{expiry}");
}

#[test]
fn a_flood_split_across_nodes_bans_on_aggregate() {
    let (address, _) = coordinator();
    let mut first = ban_node(&address, key_material());
    let mut second = ban_node(&address, key_material());
    let key = ban_key("ip:9.9.9.9");
    let mut banned = None;
    for step in 0..6u64 {
        let ledger = if step % 2 == 0 {
            &mut first
        } else {
            &mut second
        };
        if let Outcome::AutoBanned { entry, .. } = ledger
            .observe(&key, T0 + Duration::from_millis(step))
            .unwrap()
        {
            banned = Some(entry);
            break;
        }
    }
    assert!(banned.is_some(), "aggregate hard breach must ban");
}

#[test]
fn a_lifted_ban_cannot_be_resurrected_by_replay() {
    let (address, shared) = coordinator();
    let mut first = ban_node(&address, key_material());
    let key = ban_key("fp:abcdef");
    first
        .add_temporary(&key, "abuse", None, Duration::from_secs(3600), T0)
        .unwrap();
    let old = shared.lock().unwrap().strings[b"hmn:ban:fp:abcdef".as_slice()].clone();
    let mut second = ban_node(&address, key_material()).with_clock(|| T0 + Duration::from_secs(1));
    assert!(second.check(&key, T0).is_some());
    assert!(!second.lift(&key).unwrap());
    assert!(
        !shared
            .lock()
            .unwrap()
            .strings
            .contains_key(b"hmn:ban:fp:abcdef".as_slice())
    );
    shared
        .lock()
        .unwrap()
        .strings
        .insert(b"hmn:ban:fp:abcdef".to_vec(), old);
    assert_eq!(second.check(&key, T0 + Duration::from_secs(2)), None);
    assert_eq!(second.degradation().untrusted(), 1);
    // The lift epoch is bounded in time.
    second
        .gc(T0 + LIFT_EPOCH_TTL + Duration::from_secs(2))
        .unwrap();
    assert!(second.lift_epoch.is_empty());
}

#[test]
fn relocated_and_expired_bans_are_refused() {
    let (address, shared) = coordinator();
    let mut ledger = ban_node(&address, key_material());
    let key = ban_key("fp:abcdef");
    ledger
        .add_temporary(&key, "abuse", None, Duration::from_secs(60), T0)
        .unwrap();
    let value = shared.lock().unwrap().strings[b"hmn:ban:fp:abcdef".as_slice()].clone();
    shared
        .lock()
        .unwrap()
        .strings
        .insert(b"hmn:ban:fp:zzzzzz".to_vec(), value);
    let mut reader = ban_node(&address, key_material());
    assert_eq!(reader.check(&ban_key("fp:zzzzzz"), T0), None);
    assert_eq!(reader.check(&key, T0 + Duration::from_secs(60)), None);
    assert_eq!(reader.degradation().untrusted(), 2);
}

#[test]
fn an_unsigned_coordinator_cannot_impose_permanent_or_broad_bans() {
    let (address, shared) = coordinator();
    let entry = |key: &str, until: Option<u64>| {
        serde_json::to_vec(&BanWire {
            key: key.into(),
            reason: "injected".into(),
            source: "dual_control".into(),
            incident: None,
            created: u64::try_from(T0.as_nanos()).unwrap(),
            until,
            strike: 0,
        })
        .unwrap()
    };
    let later = u64::try_from((T0 + Duration::from_secs(60)).as_nanos()).unwrap();
    {
        let mut state = shared.lock().unwrap();
        state
            .strings
            .insert(b"hmn:ban:ip:1.1.1.1".to_vec(), entry("ip:1.1.1.1", None));
        state.strings.insert(
            b"hmn:ban:cidr:10.0.0.0/8".to_vec(),
            entry("cidr:10.0.0.0/8", Some(later)),
        );
        state.strings.insert(
            b"hmn:ban:ip:2.2.2.2".to_vec(),
            entry("ip:2.2.2.2", Some(later)),
        );
    }
    let mut ledger = ban_node(&address, None);
    assert_eq!(ledger.check(&ban_key("ip:1.1.1.1"), T0), None);
    assert_eq!(ledger.check(&ban_key("cidr:10.0.0.0/8"), T0), None);
    assert!(ledger.check(&ban_key("ip:2.2.2.2"), T0).is_some());
    // The same values are accepted from a signed coordinator only when sealed.
    let mut signed = ban_node(&address, key_material());
    assert_eq!(signed.check(&ban_key("ip:2.2.2.2"), T0), None);
}

#[test]
fn list_unions_the_coordinator_and_local_view() {
    let (address, shared) = coordinator();
    let mut first = ban_node(&address, key_material());
    let mut second = ban_node(&address, key_material());
    first
        .add_temporary(
            &ban_key("fp:aaaaaa"),
            "a",
            None,
            Duration::from_secs(60),
            T0,
        )
        .unwrap();
    second
        .add_temporary(
            &ban_key("fp:bbbbbb"),
            "b",
            None,
            Duration::from_secs(60),
            T0,
        )
        .unwrap();
    let mut keys: Vec<String> = second
        .list(T0)
        .into_iter()
        .map(|entry| entry.key.as_str().to_owned())
        .collect();
    keys.sort();
    assert_eq!(keys, ["fp:aaaaaa", "fp:bbbbbb"]);
    // A cursor that never terminates is cut off and reported as degraded.
    shared.lock().unwrap().cycling_scan = true;
    assert_eq!(second.list(T0).len(), 2);
    assert_eq!(second.degradation().unavailable(), 1);
}

#[test]
fn a_ban_outage_serves_the_local_view() {
    let mut ledger = SharedBanLedger::new(unreachable(), rate_config(), 16, key_material())
        .unwrap()
        .with_clock(|| T0);
    let key = ban_key("fp:abcdef");
    ledger
        .add_temporary(&key, "abuse", None, Duration::from_secs(60), T0)
        .unwrap();
    assert!(ledger.check(&key, T0).is_some());
    assert_eq!(ledger.list(T0).len(), 1);
    assert!(ledger.degradation().unavailable() >= 3);
}

#[test]
fn wargame_r241_r250_seal_axis() {
    let signing = key_material();
    let key = signing.as_ref();
    // R241 round trip, R242/R250 relocation, R246 cross-context ban vs verdict.
    let sealed = seal(key, "hmn:verdict:sid-A", br#"{"v":"ALLOW"}"#);
    assert_eq!(
        open(key, "hmn:verdict:sid-A", &sealed),
        Some(br#"{"v":"ALLOW"}"#.as_slice())
    );
    assert_eq!(open(key, "hmn:verdict:sid-B", &sealed), None);
    assert_eq!(open(key, "hmn:ban:ip:1.2.3.4", &sealed), None);
    // R243/R250 unsigned injection.
    assert_eq!(open(key, "hmn:verdict:sid-A", br#"{"v":"ALLOW"}"#), None);
    // R244 a different payload spliced onto a stolen tag.
    let split = sealed.iter().rposition(|byte| *byte == 0).unwrap();
    let forged = [br#"{"v":"DENY"}"#.as_slice(), &sealed[split..]].concat();
    assert_eq!(open(key, "hmn:verdict:sid-A", &forged), None);
    // R245 no key: legacy passthrough.
    assert_eq!(
        open(None, "hmn:verdict:sid-A", b"raw-legacy"),
        Some(b"raw-legacy".as_slice())
    );
    // R247 empty payload.
    let empty = seal(key, "ctx", b"");
    assert_eq!(open(key, "ctx", &empty), Some(b"".as_slice()));
    // R248 wrong fleet secret.
    let other = SigningKey::new(b"k2".to_vec());
    assert_eq!(open(other.as_ref(), "hmn:verdict:sid-A", &sealed), None);
    // R249 truncated tag.
    assert_eq!(
        open(key, "hmn:verdict:sid-A", &sealed[..sealed.len() / 2]),
        None
    );
    assert_eq!(
        open(key, "hmn:verdict:sid-A", &sealed[..sealed.len() - 1]),
        None
    );
}

// --- wargame axis D: shared limiter rows (R276, R277) ----------------------

fn millisecond_config(max_keys: usize) -> Config {
    let limits = Limits::new(max_keys, 64, 128, Duration::from_secs(60), 1 << 20, 64).unwrap();
    Config::new(Duration::from_millis(1), 5, 10, limits).unwrap()
}

#[test]
fn wargame_r276_shared_limiter_level_delegates() {
    // Go builds the limiter around a local 1 s 5/10 limiter and never dials:
    // classification is the local cutpoints, with no coordinator round trip.
    let limits = Limits::new(64, 64, 128, Duration::from_secs(60), 1 << 20, 64).unwrap();
    let config = Config::new(Duration::from_secs(1), 5, 10, limits).unwrap();
    let (address, state) = coordinator();
    let limiter = SharedRateLimiter::new(node(&address), config);
    assert_eq!(limiter.level(4), Level::Normal);
    assert_eq!(limiter.level(5), Level::Soft);
    assert_eq!(limiter.level(9), Level::Soft);
    assert_eq!(limiter.level(10), Level::Hard);
    assert!(state.lock().unwrap().commands.is_empty());
}

#[test]
fn wargame_r277_shared_limiter_gc_sweeps_the_local_fallback() {
    // During an outage the local fallback holds the keys; GC must sweep it or
    // a flood of distinct keys during the outage grows it without bound.
    let mut limiter = SharedRateLimiter::new(unreachable(), millisecond_config(1));
    assert!(matches!(
        limiter.observe("k", T0),
        Ok(Observation::Counted { count: 1, .. })
    ));
    assert_eq!(limiter.observe("other", T0), Err(Unavailable::Budget));
    let later = T0 + Duration::from_secs(3600);
    assert_eq!(limiter.gc(later), Ok(()));
    assert!(matches!(
        limiter.observe("other", later),
        Ok(Observation::Counted { count: 1, .. })
    ));
    // GC runs on the local limiter's ordered clock.
    assert_eq!(limiter.gc(T0), Err(Unavailable::TimeRegression));
    assert_eq!(limiter.degradation().unavailable(), 3);
}
