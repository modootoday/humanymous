//! Progressive rate-driven bans and manual ban management (shipping row B18).
//!
//! Port of legacy internal/gate/ban.go and validBanKey (baseline 981d25c) as a
//! pure single-owner store: Runtime owns the clock, audit and enforcement order.
//! Deliberate fail-closed differences from the Go store, each pinned by tests:
//! - Keys are validated and canonicalized (`ip:` + parsed address), so
//!   `ip: 1.2.3.4` cannot coexist with `ip:1.2.3.4`.
//! - Permanent and broad (`cidr:`) bans need a consumed dual-control
//!   `RetainedActionPermit` for `Action::PermanentBan`; the AGENTS.md dual-control
//!   rule forbids a single operator or an automatic escalation creating them.
//!   A fourth auto strike therefore keeps the 24h term and reports
//!   `dual_control_required` instead of Go's automatic permanent ban.
//! - Lifting a permanent ban is refused here; its control action is not defined.
//!   A temporary ban cannot replace a permanent one either (Go `Add` overwrote it).
//! - Stores are bounded; a full store refuses instead of evicting a ban.
use super::seams::RateLimiter;
use crate::audit::control::{Action, RetainedActionPermit, Scope, target_commitment};
use crate::network::history::rate::{self, Level, Limiter, Observation};
use std::collections::BTreeMap;
use std::io;
use std::net::IpAddr;
use std::time::Duration;

const HOUR: Duration = Duration::from_secs(3600);
/// A strike is remembered this long after its last breach (SoT-27 section 2).
pub const STRIKE_DECAY: Duration = Duration::from_secs(7 * 24 * 3600);
/// Exclusive upper bound on a bulk temporary term (legacy `maxBanDurationSec`).
pub const MAX_BULK_TERM: Duration = Duration::from_secs(400 * 24 * 3600);
/// Domain separation for the dual-control target commitment.
const TARGET_DOMAIN: &[u8] = b"humanymous.ban.permanent/1\0";

/// Static refusals; no key or reason bytes are interpolated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Key is not a valid `ip:`, `fp:` or `cidr:` key.
    Key,
    /// Reason is empty, too long or has control characters.
    Reason,
    /// Temporary duration is zero.
    Duration,
    /// Permanent or broad keys need the dual-control path.
    DualControlRequired,
    /// Store or strike capacity is exhausted; nothing was evicted.
    Capacity,
    /// The rate history refused the observation (clock regression, bounds).
    Rate(rate::Unavailable),
}

/// Canonical validated ban key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key(String);

impl Key {
    /// Validate the `ip:`/`fp:`/`cidr:` format with real address material.
    pub fn parse(text: &str) -> Result<Self, Refusal> {
        if text.is_empty() || text.chars().any(|c| (c as u32) < 32 || c as u32 == 127) {
            return Err(Refusal::Key);
        }
        if let Some(ip) = text.strip_prefix("ip:") {
            let ip: IpAddr = ip.trim().parse().map_err(|_| Refusal::Key)?;
            return Ok(Self(format!("ip:{ip}")));
        }
        if let Some(fp) = text.strip_prefix("fp:") {
            if !(4..=128).contains(&fp.len()) || !fp.bytes().all(|b| (33..=126).contains(&b)) {
                return Err(Refusal::Key);
            }
            return Ok(Self(text.into()));
        }
        if let Some(cidr) = text.strip_prefix("cidr:") {
            let (ip, prefix) = cidr.trim().split_once('/').ok_or(Refusal::Key)?;
            let ip: IpAddr = ip.parse().map_err(|_| Refusal::Key)?;
            let width = if ip.is_ipv4() { 32 } else { 128 };
            if prefix.is_empty()
                || prefix.len() > 3
                || !prefix.bytes().all(|b| b.is_ascii_digit())
                || prefix.parse::<u8>().map_err(|_| Refusal::Key)? > width
            {
                return Err(Refusal::Key);
            }
            return Ok(Self(format!(
                "cidr:{ip}/{}",
                prefix.parse::<u8>().unwrap_or(0)
            )));
        }
        Err(Refusal::Key)
    }

    /// Wide-blast-radius range key; always dual-controlled.
    pub fn is_broad(&self) -> bool {
        self.0.starts_with("cidr:")
    }

    /// Canonical text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Dual-control target commitment for a permanent or broad ban of this key.
    pub fn commitment(&self) -> String {
        let mut bytes = TARGET_DOMAIN.to_vec();
        bytes.extend_from_slice(self.0.as_bytes());
        target_commitment(&bytes)
    }
}

/// Who created a ban.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Hard rate breach.
    Auto,
    /// Single operator, temporary and narrow only.
    Manual,
    /// Consumed dual-control grant.
    DualControl,
}

/// One active ban.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Canonical key.
    pub key: Key,
    /// Bounded operator or automatic reason.
    pub reason: String,
    /// Creation path.
    pub source: Source,
    /// Optional incident reference.
    pub incident: Option<String>,
    /// Creation time.
    pub created: Duration,
    /// Exclusive expiry; `None` is permanent.
    pub until: Option<Duration>,
    /// Auto strike count, zero for manual bans.
    pub strike: u32,
}

impl Entry {
    /// Whether the ban never expires.
    pub fn permanent(&self) -> bool {
        self.until.is_none()
    }
    fn active(&self, now: Duration) -> bool {
        self.until.is_none_or(|until| now < until)
    }
    /// Seconds until a temporary ban expires; zero for permanent or past.
    pub fn remaining(&self, now: Duration) -> Duration {
        self.until
            .map_or(Duration::ZERO, |until| until.saturating_sub(now))
    }
}

/// Auto-ban term for a strike count (SoT-27 section 2): 1h, 6h, 24h. Go's
/// fourth-strike permanent ban needs dual control here, so the term stays 24h.
pub fn escalation(strike: u32) -> Duration {
    match strike {
        0 | 1 => HOUR,
        2 => 6 * HOUR,
        _ => 24 * HOUR,
    }
}

/// Result of observing one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Below the hard cutpoint; no ban state changed.
    Clear(Level),
    /// Already banned before this request.
    Banned(Entry),
    /// Hard breach applied a new auto-ban.
    AutoBanned {
        /// The applied ban.
        entry: Entry,
        /// Strike reached the legacy permanent tier; propose dual control.
        dual_control_required: bool,
    },
}

#[derive(Debug, Clone, Copy)]
struct Strike {
    count: u32,
    last: Duration,
}

/// Bounded ban store over a rate counter: the local history by default, or a
/// fleet-shared counter (B23) behind the same [`RateLimiter`] seam.
pub struct Store<L = Limiter> {
    bans: BTreeMap<Key, Entry>,
    strikes: BTreeMap<Key, Strike>,
    limiter: L,
    capacity: usize,
}

fn reason(text: &str) -> Result<String, Refusal> {
    if text.is_empty() || text.len() > 256 || text.chars().any(char::is_control) {
        return Err(Refusal::Reason);
    }
    Ok(text.into())
}

impl Store {
    /// Empty store; `capacity` bounds both active bans and strike records.
    pub fn new(rate: rate::Config, capacity: usize) -> Result<Self, Refusal> {
        Self::with_limiter(Limiter::new(rate), capacity)
    }
}

impl<L: RateLimiter> Store<L> {
    /// Empty store over an explicit rate counter.
    pub fn with_limiter(limiter: L, capacity: usize) -> Result<Self, Refusal> {
        if capacity == 0 {
            return Err(Refusal::Capacity);
        }
        Ok(Self {
            bans: BTreeMap::new(),
            strikes: BTreeMap::new(),
            limiter,
            capacity,
        })
    }

    /// The rate counter, for configuration and degradation reporting.
    pub fn limiter(&mut self) -> &mut L {
        &mut self.limiter
    }

    /// Active ban for a key, expiring it lazily.
    pub fn check(&mut self, key: &Key, now: Duration) -> Option<&Entry> {
        if self.bans.get(key).is_some_and(|b| !b.active(now)) {
            self.bans.remove(key);
        }
        self.bans.get(key)
    }

    /// Count a request and apply an escalating auto-ban on a hard breach.
    pub fn observe(&mut self, key: &Key, now: Duration) -> Result<Outcome, Refusal> {
        let level = match RateLimiter::observe(&mut self.limiter, key.as_str(), now)
            .map_err(Refusal::Rate)?
        {
            Observation::Counted { level, .. } => level,
            Observation::IgnoredEmptyKey => return Err(Refusal::Key),
        };
        if level != Level::Hard {
            return Ok(Outcome::Clear(level));
        }
        if let Some(entry) = self.check(key, now) {
            return Ok(Outcome::Banned(entry.clone()));
        }
        let previous = self
            .strikes
            .get(key)
            .filter(|s| now.saturating_sub(s.last) <= STRIKE_DECAY)
            .map_or(0, |s| s.count);
        if !self.strikes.contains_key(key) && self.strikes.len() >= self.capacity {
            return Err(Refusal::Capacity);
        }
        if !self.bans.contains_key(key) && self.bans.len() >= self.capacity {
            return Err(Refusal::Capacity);
        }
        let count = previous.saturating_add(1);
        self.strikes
            .insert(key.clone(), Strike { count, last: now });
        let entry = Entry {
            key: key.clone(),
            reason: "rate limit exceeded (auto)".into(),
            source: Source::Auto,
            incident: None,
            created: now,
            until: Some(now.saturating_add(escalation(count))),
            strike: count,
        };
        self.bans.insert(key.clone(), entry.clone());
        Ok(Outcome::AutoBanned {
            entry,
            dual_control_required: count >= 4,
        })
    }

    /// Classify without mutating ban state (monitor/shadow mode).
    pub fn observe_rate(&mut self, key: &Key, now: Duration) -> Result<Level, Refusal> {
        match RateLimiter::observe(&mut self.limiter, key.as_str(), now).map_err(Refusal::Rate)? {
            Observation::Counted { level, .. } => Ok(level),
            Observation::IgnoredEmptyKey => Err(Refusal::Key),
        }
    }

    fn insert(&mut self, entry: Entry) -> Result<Entry, Refusal> {
        if !self.bans.contains_key(&entry.key) && self.bans.len() >= self.capacity {
            return Err(Refusal::Capacity);
        }
        self.bans.insert(entry.key.clone(), entry.clone());
        Ok(entry)
    }

    /// Single-operator temporary ban of a narrow key. Refuses with
    /// `DualControlRequired` when the key already has a permanent ban.
    pub fn add_temporary(
        &mut self,
        key: &Key,
        why: &str,
        incident: Option<&str>,
        term: Duration,
        now: Duration,
    ) -> Result<Entry, Refusal> {
        if key.is_broad() {
            return Err(Refusal::DualControlRequired);
        }
        if term.is_zero() {
            return Err(Refusal::Duration);
        }
        // A temporary ban must not replace (and so downgrade) a permanent one.
        if self.bans.get(key).is_some_and(Entry::permanent) {
            return Err(Refusal::DualControlRequired);
        }
        let entry = Entry {
            key: key.clone(),
            reason: reason(why)?,
            source: Source::Manual,
            incident: incident.map(reason).transpose()?,
            created: now,
            until: Some(now.saturating_add(term)),
            strike: 0,
        };
        self.insert(entry)
    }

    /// Operator bulk temporary ban (port of legacy `adminBulkBan`). The term
    /// must be positive and below [`MAX_BULK_TERM`]; unparsable keys and broad
    /// keys (which need dual control) are skipped and counted, never applied,
    /// as are keys a permanent ban already holds.
    /// Returns `(applied, skipped)`; a capacity refusal stops the batch.
    pub fn bulk_temporary(
        &mut self,
        keys: &[&str],
        why: &str,
        term: Duration,
        now: Duration,
    ) -> Result<(usize, usize), Refusal> {
        if term.is_zero() || term >= MAX_BULK_TERM {
            return Err(Refusal::Duration);
        }
        reason(why)?;
        let (mut applied, mut skipped) = (0, 0);
        for raw in keys {
            match Key::parse(raw) {
                Ok(key) if !key.is_broad() => {
                    match self.add_temporary(&key, why, None, term, now) {
                        Ok(_) => applied += 1,
                        // A permanent ban already holds this key; never downgrade it.
                        Err(Refusal::DualControlRequired) => skipped += 1,
                        Err(refusal) => return Err(refusal),
                    }
                }
                _ => skipped += 1,
            }
        }
        Ok((applied, skipped))
    }

    /// Permanent or broad ban: consumes a retained dual-control grant bound to
    /// `Action::PermanentBan` and this key's commitment. `term` of `None` is
    /// permanent. The caller's audit happens inside the permit contract.
    #[allow(clippy::too_many_arguments)]
    pub fn add_controlled(
        &mut self,
        permit: RetainedActionPermit,
        scope: &Scope,
        key: &Key,
        why: &str,
        term: Option<Duration>,
        now: Duration,
        now_unix: u64,
    ) -> io::Result<Entry> {
        let reason = reason(why).map_err(|_| io::Error::other("invalid ban reason"))?;
        if term.is_some_and(|t| t.is_zero()) {
            return Err(io::Error::other("invalid ban term"));
        }
        if !self.bans.contains_key(key) && self.bans.len() >= self.capacity {
            return Err(io::Error::other("ban store capacity exhausted"));
        }
        permit.execute(
            scope,
            Action::PermanentBan,
            &key.commitment(),
            now_unix,
            |receipt| {
                let entry = Entry {
                    key: key.clone(),
                    reason,
                    source: Source::DualControl,
                    incident: Some(receipt.operation_id.clone()),
                    created: now,
                    until: term.map(|t| now.saturating_add(t)),
                    strike: 0,
                };
                self.bans.insert(key.clone(), entry.clone());
                Ok(entry)
            },
        )
    }

    /// Remove a temporary ban. Permanent bans refuse: no control action is
    /// defined for their lift yet.
    pub fn lift(&mut self, key: &Key) -> Result<bool, Refusal> {
        match self.bans.get(key) {
            Some(entry) if entry.permanent() => Err(Refusal::DualControlRequired),
            Some(_) => Ok(self.bans.remove(key).is_some()),
            None => Ok(false),
        }
    }

    /// Evict expired bans and stale strikes; rate history runs its own bounded GC.
    pub fn gc(&mut self, now: Duration) -> Result<(), Refusal> {
        RateLimiter::gc(&mut self.limiter, now).map_err(Refusal::Rate)?;
        self.bans.retain(|_, b| b.active(now));
        self.strikes
            .retain(|_, s| now.saturating_sub(s.last) <= STRIKE_DECAY);
        Ok(())
    }

    /// Active bans in key order; expired entries are swept.
    pub fn list(&mut self, now: Duration) -> Vec<Entry> {
        self.bans.retain(|_, b| b.active(now));
        self.bans.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests;
