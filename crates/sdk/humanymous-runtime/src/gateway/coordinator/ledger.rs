//! Fleet-shared ban, sticky-verdict and rate ledgers over the RESP client,
//! ported from Go `internal/gate/redisledger.go` and `redisratelimit.go`
//! (oracle 981d25c).
//!
//! Each ledger keeps an in-process mirror used both as a write-through cache and
//! as the outage fallback: when the coordinator is unreachable, slow, or serves a
//! value that fails its integrity checks, the node answers from its own view
//! instead of failing the edge. Every such answer is counted in
//! [`Degradation`], because a local answer can never claim fleet completeness.
//!
//! Time passed to these ledgers is wall-clock time since the Unix epoch: unlike
//! the single-node stores, values are compared across nodes.
//!
//! Integrity (Go parity): with a fleet signing key each stored value carries an
//! HMAC-SHA256 tag bound to its Redis key, so a coordinator can neither forge a
//! value nor relocate one to another key; freshness, a per-session high-water
//! mark and a per-key lift epoch refuse replayed or resurrected values. One
//! deliberate tightening: without a signing key a permanent or broad (`cidr:`)
//! ban read from the coordinator is ignored, because those need dual control
//! and an unsigned coordinator value cannot carry it.
use super::redis::{Client, RedisError, Reply};
use crate::gateway::ban::{Entry, Key, Outcome, Refusal, Source, Store};
use crate::gateway::seams::{BanLedger, RateLimiter, VerdictLedger};
use crate::gateway::verdict_store::{MAX_VERDICTS, Sticky, Verdict, VerdictStore};
use crate::network::history::Unavailable;
use crate::network::history::rate::{
    Config, ConfigurationEffect, Coverage, Level, Limiter, Observation,
};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// Ban key prefix (Go `redisBanPrefix`).
pub const BAN_PREFIX: &str = "hmn:ban:";
/// Sticky-verdict key prefix (Go `redisVerdictPrefix`).
pub const VERDICT_PREFIX: &str = "hmn:verdict:";
/// Rate counter key prefix.
pub const RATE_PREFIX: &str = "hmn:rl:";
/// How long a lift is remembered against resurrection (Go `liftEpochTTL`).
pub const LIFT_EPOCH_TTL: Duration = Duration::from_secs(30 * 24 * 3600);
/// Coordinator retention for a permanent ban (Go `banRetentionHorizon`).
pub const BAN_RETENTION_HORIZON: Duration = Duration::from_secs(400 * 24 * 3600);
/// Most SCAN round trips per ban enumeration (Go `scanMaxRounds`).
pub const SCAN_MAX_ROUNDS: usize = 4096;
/// Most ban keys per enumeration (Go `scanMaxKeys`).
pub const SCAN_MAX_KEYS: usize = 1 << 16;

/// Atomic sliding window (Go `slidingWindowLua`): evict hits older than the
/// window, record this hit, refresh the key TTL and return the rolling count.
pub const SLIDING_WINDOW_SCRIPT: &str = "local now=tonumber(ARGV[1])
local win=tonumber(ARGV[2])
redis.call('ZREMRANGEBYSCORE', KEYS[1], 0, now-win)
redis.call('ZADD', KEYS[1], now, ARGV[4])
redis.call('PEXPIRE', KEYS[1], tonumber(ARGV[3]))
return redis.call('ZCARD', KEYS[1])";

type HmacSha256 = Hmac<Sha256>;

/// Fleet-shared HMAC key; absent means unsigned values (Go parity).
#[derive(Clone)]
pub struct SigningKey(Arc<Zeroizing<Vec<u8>>>);

impl SigningKey {
    /// A non-empty fleet key.
    pub fn new(bytes: Vec<u8>) -> Option<Self> {
        (!bytes.is_empty()).then(|| Self(Arc::new(Zeroizing::new(bytes))))
    }
}

impl std::fmt::Debug for SigningKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SigningKey(..)")
    }
}

fn tag(key: &SigningKey, context: &str, payload: &[u8]) -> Vec<u8> {
    let mut mac = <HmacSha256 as KeyInit>::new_from_slice(&key.0).expect("HMAC accepts any key");
    mac.update(context.as_bytes());
    mac.update(&[0]);
    mac.update(payload);
    mac.finalize().into_bytes().to_vec()
}

/// Append a key-bound tag: `payload || 0x00 || hex(HMAC(context || 0x00 || payload))`.
pub fn seal(key: Option<&SigningKey>, context: &str, payload: &[u8]) -> Vec<u8> {
    let Some(key) = key else {
        return payload.to_vec();
    };
    let mut sealed = payload.to_vec();
    sealed.push(0);
    sealed.extend_from_slice(hex::encode(tag(key, context, payload)).as_bytes());
    sealed
}

/// Verify and strip a key-bound tag; `None` for an unsigned, relocated or
/// altered value in signed mode.
pub fn open<'a>(key: Option<&SigningKey>, context: &str, value: &'a [u8]) -> Option<&'a [u8]> {
    let Some(key) = key else {
        return Some(value);
    };
    let split = value.iter().rposition(|byte| *byte == 0)?;
    let (payload, sealed_tag) = (&value[..split], &value[split + 1..]);
    let expected = hex::encode(tag(key, context, payload));
    bool::from(expected.as_bytes().ct_eq(sealed_tag)).then_some(payload)
}

/// Counts of answers served from the local view instead of the coordinator.
#[derive(Debug, Default)]
pub struct Degradation {
    unavailable: AtomicU64,
    untrusted: AtomicU64,
}

impl Degradation {
    /// Coordinator errors, open breaker or partial enumeration.
    pub fn unavailable(&self) -> u64 {
        self.unavailable.load(Ordering::Relaxed)
    }
    /// Values refused by integrity, freshness or anti-rollback checks.
    pub fn untrusted(&self) -> u64 {
        self.untrusted.load(Ordering::Relaxed)
    }
    fn outage(&self) {
        self.unavailable.fetch_add(1, Ordering::Relaxed);
    }
    fn refused(&self) {
        self.untrusted.fetch_add(1, Ordering::Relaxed);
    }
}

fn millis(duration: Duration) -> String {
    duration.as_millis().max(1).to_string()
}

fn bulk(reply: Reply) -> Option<Vec<u8>> {
    match reply {
        Reply::Bulk(bytes) => Some(bytes),
        Reply::Simple(text) => Some(text.into_bytes()),
        _ => None,
    }
}

/// Wall-clock now since the Unix epoch.
pub fn unix_now() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

// --- RateLimiter ------------------------------------------------------------

/// Fleet-wide sliding-window counter: every node observing a key adds to one
/// rolling window, so a flood split across nodes is classified on aggregate.
/// Thresholds and the outage fallback come from the local limiter.
pub struct SharedRateLimiter {
    client: Arc<Client>,
    local: Limiter,
    window: Duration,
    incomplete_until: Option<Duration>,
    sequence: u64,
    degradation: Arc<Degradation>,
}

impl SharedRateLimiter {
    /// Shared counter with the same window and cutpoints as `config`.
    pub fn new(client: Arc<Client>, config: Config) -> Self {
        Self {
            client,
            window: config.window(),
            local: Limiter::new(config),
            incomplete_until: None,
            sequence: 0,
            degradation: Arc::new(Degradation::default()),
        }
    }

    /// Local-fallback accounting.
    pub fn degradation(&self) -> Arc<Degradation> {
        self.degradation.clone()
    }

    fn shared(&mut self, key: &str, now: Duration) -> Result<usize, RedisError> {
        self.sequence = self.sequence.wrapping_add(1);
        let now_nanos = now.as_nanos().to_string();
        let member = format!("{now_nanos}-{}", self.sequence);
        let reply = self.client.command(&[
            b"EVAL",
            SLIDING_WINDOW_SCRIPT.as_bytes(),
            b"1",
            format!("{RATE_PREFIX}{key}").as_bytes(),
            now_nanos.as_bytes(),
            self.window.as_nanos().to_string().as_bytes(),
            millis(self.window.saturating_mul(6)).as_bytes(),
            member.as_bytes(),
        ])?;
        match reply {
            Reply::Integer(count) => Ok(usize::try_from(count.max(0)).unwrap_or(usize::MAX)),
            _ => Err(RedisError::Protocol("rate reply not an integer")),
        }
    }
}

impl RateLimiter for SharedRateLimiter {
    fn observe(&mut self, key: &str, now: Duration) -> Result<Observation, Unavailable> {
        if key.is_empty() {
            return Ok(Observation::IgnoredEmptyKey);
        }
        match self.shared(key, now) {
            Ok(count) => {
                // After a window expansion the shared set cannot recover hits it
                // already trimmed, exactly like the local history.
                let coverage = match self.incomplete_until {
                    Some(until) if now < until => Coverage::WindowExpanded {
                        complete_after: until,
                    },
                    _ => Coverage::Retained,
                };
                Ok(Observation::Counted {
                    count,
                    level: self.local.level(count),
                    coverage,
                })
            }
            Err(_) => {
                self.degradation.outage();
                self.local.observe(key, now)
            }
        }
    }
    fn level(&self, count: usize) -> Level {
        self.local.level(count)
    }
    fn configure(
        &mut self,
        window: Duration,
        soft: usize,
        hard: usize,
        now: Duration,
    ) -> Result<ConfigurationEffect, Unavailable> {
        let effect = self.local.configure(window, soft, hard, now)?;
        if window > self.window {
            let until = now.saturating_add(window);
            self.incomplete_until = Some(self.incomplete_until.map_or(until, |old| old.max(until)));
        }
        self.window = window;
        Ok(effect)
    }
    fn gc(&mut self, now: Duration) -> Result<(), Unavailable> {
        self.local.gc(now).map(|_| ())
    }
}

// --- VerdictLedger ----------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct VerdictWire {
    v: String,
    r: f64,
    h: String,
    t: Vec<String>,
    u: u64,
}

fn verdict_from_wire(name: &str) -> Option<Verdict> {
    Some(match name {
        "allow" => Verdict::Allow,
        "challenge" => Verdict::Challenge,
        "deny" => Verdict::Deny,
        "none" => Verdict::Unknown,
        _ => return None,
    })
}

/// Fleet-shared sticky verdicts: a verdict assigned on one node is visible on
/// every other, so reconnecting to a fresh node cannot shed a DENY.
pub struct SharedVerdictLedger {
    client: Arc<Client>,
    local: VerdictStore,
    ttl: Duration,
    signing: Option<SigningKey>,
    high_water: Mutex<HashMap<String, Duration>>,
    degradation: Arc<Degradation>,
}

impl SharedVerdictLedger {
    /// Shared ledger whose values expire after `ttl`.
    pub fn new(client: Arc<Client>, ttl: Duration, signing: Option<SigningKey>) -> Self {
        Self {
            client,
            local: VerdictStore::new(ttl),
            ttl,
            signing,
            high_water: Mutex::new(HashMap::new()),
            degradation: Arc::new(Degradation::default()),
        }
    }

    /// Local-fallback accounting.
    pub fn degradation(&self) -> Arc<Degradation> {
        self.degradation.clone()
    }

    fn untrusted(&self, sid: &str, now: Duration) -> Sticky {
        self.degradation.refused();
        self.local.get(sid, now)
    }
}

impl VerdictLedger for SharedVerdictLedger {
    fn set(&self, sid: &str, sticky: Sticky) -> bool {
        let stored = self.local.set(sid, sticky.clone());
        let Ok(updated) = u64::try_from(sticky.updated.as_nanos()) else {
            return stored;
        };
        let Ok(payload) = serde_json::to_vec(&VerdictWire {
            v: sticky.verdict.as_str().into(),
            r: sticky.risk,
            h: sticky.rule,
            t: sticky.top,
            u: updated,
        }) else {
            return stored;
        };
        let key = format!("{VERDICT_PREFIX}{sid}");
        let value = seal(self.signing.as_ref(), &key, &payload);
        if self
            .client
            .command(&[
                b"SET",
                key.as_bytes(),
                &value,
                b"PX",
                millis(self.ttl).as_bytes(),
            ])
            .is_err()
        {
            self.degradation.outage();
        }
        stored
    }

    fn get(&self, sid: &str, now: Duration) -> Sticky {
        let key = format!("{VERDICT_PREFIX}{sid}");
        let reply = match self.client.command(&[b"GET", key.as_bytes()]) {
            Ok(reply) => reply,
            Err(_) => {
                self.degradation.outage();
                return self.local.get(sid, now);
            }
        };
        if reply == Reply::Nil {
            return Sticky::default();
        }
        let Some(value) = bulk(reply) else {
            return self.untrusted(sid, now);
        };
        let Some(payload) = open(self.signing.as_ref(), &key, &value) else {
            return self.untrusted(sid, now);
        };
        let Ok(wire) = serde_json::from_slice::<VerdictWire>(payload) else {
            return self.untrusted(sid, now);
        };
        let (Some(verdict), updated) = (verdict_from_wire(&wire.v), Duration::from_nanos(wire.u))
        else {
            return self.untrusted(sid, now);
        };
        if now.saturating_sub(updated) > self.ttl {
            return self.untrusted(sid, now);
        }
        // Anti-rollback: never accept a value older than the freshest one this
        // node has seen for the session (bounded by the verdict cap).
        let Ok(mut high_water) = self.high_water.lock() else {
            return self.untrusted(sid, now);
        };
        match high_water.get(sid) {
            Some(seen) if updated < *seen => {
                drop(high_water);
                return self.untrusted(sid, now);
            }
            Some(_) => {
                high_water.insert(sid.to_owned(), updated);
            }
            None if high_water.len() < MAX_VERDICTS => {
                high_water.insert(sid.to_owned(), updated);
            }
            None => {}
        }
        Sticky {
            verdict,
            risk: wire.r,
            rule: wire.h,
            top: wire.t,
            updated,
        }
    }

    fn gc(&self, now: Duration) {
        self.local.gc(now);
        if let Ok(mut high_water) = self.high_water.lock() {
            high_water.retain(|_, seen| now.saturating_sub(*seen) <= self.ttl);
        }
    }
}

// --- BanLedger --------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct BanWire {
    key: String,
    reason: String,
    source: String,
    incident: Option<String>,
    created: u64,
    until: Option<u64>,
    strike: u32,
}

fn source_name(source: Source) -> &'static str {
    match source {
        Source::Auto => "auto",
        Source::Manual => "manual",
        Source::DualControl => "dual_control",
    }
}

fn wire_entry(entry: &Entry) -> Option<BanWire> {
    Some(BanWire {
        key: entry.key.as_str().into(),
        reason: entry.reason.clone(),
        source: source_name(entry.source).into(),
        incident: entry.incident.clone(),
        created: u64::try_from(entry.created.as_nanos()).ok()?,
        until: match entry.until {
            Some(until) => Some(u64::try_from(until.as_nanos()).ok()?),
            None => None,
        },
        strike: entry.strike,
    })
}

fn entry_from_wire(wire: BanWire) -> Option<Entry> {
    Some(Entry {
        key: Key::parse(&wire.key).ok()?,
        reason: wire.reason,
        source: match wire.source.as_str() {
            "auto" => Source::Auto,
            "manual" => Source::Manual,
            "dual_control" => Source::DualControl,
            _ => return None,
        },
        incident: wire.incident,
        created: Duration::from_nanos(wire.created),
        until: wire.until.map(Duration::from_nanos),
        strike: wire.strike,
    })
}

/// Fleet-shared bans: a ban raised on any node is enforced on every node. Auto
/// bans are driven by the shared rate counter, so a flood split across nodes
/// escalates on aggregate; strikes stay in the local mirror.
pub struct SharedBanLedger {
    client: Arc<Client>,
    local: Store<SharedRateLimiter>,
    signing: Option<SigningKey>,
    lift_epoch: BTreeMap<Key, Duration>,
    clock: Box<dyn Fn() -> Duration + Send + Sync>,
    degradation: Arc<Degradation>,
}

impl SharedBanLedger {
    /// Shared ledger with a shared rate counter over `config`.
    pub fn new(
        client: Arc<Client>,
        config: Config,
        capacity: usize,
        signing: Option<SigningKey>,
    ) -> Result<Self, Refusal> {
        let limiter = SharedRateLimiter::new(client.clone(), config);
        let degradation = limiter.degradation();
        Ok(Self {
            client,
            local: Store::with_limiter(limiter, capacity)?,
            signing,
            lift_epoch: BTreeMap::new(),
            clock: Box::new(unix_now),
            degradation,
        })
    }

    /// Replace the wall clock used where the seam supplies none (lift, write).
    pub fn with_clock(mut self, clock: impl Fn() -> Duration + Send + Sync + 'static) -> Self {
        self.clock = Box::new(clock);
        self
    }

    /// Local-fallback accounting, shared with the rate counter.
    pub fn degradation(&self) -> Arc<Degradation> {
        self.degradation.clone()
    }

    /// The shared rate counter, for hot configuration.
    pub fn limiter(&mut self) -> &mut SharedRateLimiter {
        self.local.limiter()
    }

    /// Evict expired local state and stale lift epochs.
    pub fn gc(&mut self, now: Duration) -> Result<(), Refusal> {
        self.local.gc(now)?;
        self.lift_epoch
            .retain(|_, lifted| now.saturating_sub(*lifted) <= LIFT_EPOCH_TTL);
        Ok(())
    }

    fn write(&self, entry: &Entry) {
        let Some(payload) = wire_entry(entry).and_then(|wire| serde_json::to_vec(&wire).ok())
        else {
            return;
        };
        let key = format!("{BAN_PREFIX}{}", entry.key.as_str());
        let expiry = match entry.until {
            None => BAN_RETENTION_HORIZON,
            Some(until) => {
                let remaining = until.saturating_sub((self.clock)());
                if remaining.is_zero() {
                    return;
                }
                remaining
            }
        };
        let value = seal(self.signing.as_ref(), &key, &payload);
        if self
            .client
            .command(&[
                b"SET",
                key.as_bytes(),
                &value,
                b"PX",
                millis(expiry).as_bytes(),
            ])
            .is_err()
        {
            self.degradation.outage();
        }
    }

    /// Decode a coordinator value stored under `redis_key`, refusing anything
    /// forged, relocated, inactive, resurrected or needing dual control.
    fn trusted(&self, redis_key: &str, value: &[u8], now: Duration) -> Option<Entry> {
        let payload = open(self.signing.as_ref(), redis_key, value)?;
        let entry = entry_from_wire(serde_json::from_slice(payload).ok()?)?;
        if format!("{BAN_PREFIX}{}", entry.key.as_str()) != redis_key {
            return None;
        }
        if self.signing.is_none() && (entry.permanent() || entry.key.is_broad()) {
            return None;
        }
        if !entry.until.is_none_or(|until| now < until) {
            return None;
        }
        if self
            .lift_epoch
            .get(&entry.key)
            .is_some_and(|lifted| entry.created <= *lifted)
        {
            return None;
        }
        Some(entry)
    }

    fn scan(&self) -> Result<Vec<Vec<u8>>, Vec<Vec<u8>>> {
        let mut keys = Vec::new();
        let mut cursor = b"0".to_vec();
        let mut seen = BTreeSet::new();
        let pattern = format!("{BAN_PREFIX}*");
        for _ in 0..SCAN_MAX_ROUNDS {
            let Ok(Reply::Array(parts)) = self.client.command(&[
                b"SCAN",
                &cursor,
                b"MATCH",
                pattern.as_bytes(),
                b"COUNT",
                b"256",
            ]) else {
                return Err(keys);
            };
            let mut parts = parts.into_iter();
            let (Some(next), Some(Reply::Array(batch)), None) =
                (parts.next().and_then(bulk), parts.next(), parts.next())
            else {
                return Err(keys);
            };
            for key in batch.into_iter().filter_map(bulk) {
                keys.push(key);
                if keys.len() >= SCAN_MAX_KEYS {
                    return Err(keys);
                }
            }
            if next == b"0" {
                return Ok(keys);
            }
            if !seen.insert(next.clone()) {
                return Err(keys);
            }
            cursor = next;
        }
        Err(keys)
    }
}

impl BanLedger for SharedBanLedger {
    fn check(&mut self, key: &Key, now: Duration) -> Option<Entry> {
        let redis_key = format!("{BAN_PREFIX}{}", key.as_str());
        let reply = match self.client.command(&[b"GET", redis_key.as_bytes()]) {
            Ok(reply) => reply,
            Err(_) => {
                self.degradation.outage();
                return self.local.check(key, now).cloned();
            }
        };
        if reply == Reply::Nil {
            return None;
        }
        match bulk(reply).and_then(|value| self.trusted(&redis_key, &value, now)) {
            Some(entry) => Some(entry),
            None => {
                self.degradation.refused();
                self.local.check(key, now).cloned()
            }
        }
    }

    fn observe(&mut self, key: &Key, now: Duration) -> Result<Outcome, Refusal> {
        let outcome = self.local.observe(key, now)?;
        if let Outcome::AutoBanned { entry, .. } = &outcome {
            self.write(entry);
        }
        Ok(outcome)
    }

    fn observe_rate(&mut self, key: &Key, now: Duration) -> Result<Level, Refusal> {
        self.local.observe_rate(key, now)
    }

    fn add_temporary(
        &mut self,
        key: &Key,
        why: &str,
        incident: Option<&str>,
        term: Duration,
        now: Duration,
    ) -> Result<Entry, Refusal> {
        let entry = self.local.add_temporary(key, why, incident, term, now)?;
        self.write(&entry);
        Ok(entry)
    }

    fn lift(&mut self, key: &Key) -> Result<bool, Refusal> {
        let lifted = self.local.lift(key)?;
        self.lift_epoch.insert(key.clone(), (self.clock)());
        let redis_key = format!("{BAN_PREFIX}{}", key.as_str());
        if self
            .client
            .command(&[b"DEL", redis_key.as_bytes()])
            .is_err()
        {
            self.degradation.outage();
        }
        Ok(lifted)
    }

    fn list(&mut self, now: Duration) -> Vec<Entry> {
        let keys = match self.scan() {
            Ok(keys) => keys,
            Err(partial) => {
                self.degradation.outage();
                partial
            }
        };
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        for redis_key in keys {
            let Ok(text) = String::from_utf8(redis_key) else {
                continue;
            };
            let reply = match self.client.command(&[b"GET", text.as_bytes()]) {
                Ok(reply) => reply,
                Err(_) => {
                    self.degradation.outage();
                    return self.local.list(now);
                }
            };
            let Some(entry) = bulk(reply).and_then(|value| self.trusted(&text, &value, now)) else {
                continue;
            };
            if seen.insert(entry.key.clone()) {
                out.push(entry);
            }
        }
        out.extend(
            self.local
                .list(now)
                .into_iter()
                .filter(|entry| !seen.contains(&entry.key)),
        );
        out
    }
}

#[cfg(test)]
#[path = "ledger_tests.rs"]
mod tests;
