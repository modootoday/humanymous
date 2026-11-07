use super::{Limits, Record};
use crate::history::{OrderedTime, Unavailable};
use std::{
    collections::{BTreeMap, VecDeque},
    time::Duration,
};

#[derive(Debug, Clone, Copy)]
/// Caller-supplied retention and finite resource limits.
pub struct Config {
    ttl: Duration,
    limits: Limits,
}
impl Config {
    /// Reject zero TTL without supplying a production retention default.
    pub fn new(ttl: Duration, limits: Limits) -> Result<Self, Unavailable> {
        if ttl.is_zero() {
            return Err(Unavailable::InvalidConfiguration);
        }
        Ok(Self { ttl, limits })
    }
}
/// Address-only legacy backfill never authenticates a connection epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Association {
    /// No matching address entry existed.
    Absent,
    /// Legacy text-address match, not authenticated same-epoch TLS evidence.
    AddressOnly,
}
struct Session {
    records: VecDeque<Record>,
    seen: Duration,
}
/// Bounded caller history and legacy address-only TLS association.
pub struct Log {
    config: Config,
    sessions: BTreeMap<String, Session>,
    by_addr: BTreeMap<String, Record>,
    clock: OrderedTime,
    bytes: usize,
}
impl Log {
    /// Create empty history; runtime retains exclusive access and clock custody.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            sessions: BTreeMap::new(),
            by_addr: BTreeMap::new(),
            clock: OrderedTime::new(),
            bytes: 0,
        }
    }
    /// Replace the last TLS-shaped address record without authenticating it.
    pub fn record_tls(&mut self, record: Record) -> Result<(), Unavailable> {
        self.clock.check(record.time)?;
        self.config.limits.validate(&record)?;
        let previous = self.by_addr.get(&record.remote_addr);
        if previous.is_none() && self.by_addr.len() >= self.config.limits.max_addresses {
            return Err(Unavailable::Budget);
        }
        let old = previous.map_or(0, |r| r.bytes() + r.remote_addr.len());
        let bytes = (self.bytes - old)
            .checked_add(record.bytes())
            .and_then(|bytes| bytes.checked_add(record.remote_addr.len()))
            .filter(|bytes| *bytes <= self.config.limits.max_payload_bytes)
            .ok_or(Unavailable::Budget)?;
        self.clock.commit(record.time);
        self.by_addr.insert(record.remote_addr.clone(), record);
        self.bytes = bytes;
        Ok(())
    }
    /// Append one record with legacy backfill, preserving the newest64 per session.
    pub fn record_http(&mut self, mut record: Record) -> Result<Association, Unavailable> {
        self.clock.check(record.time)?;
        self.config.limits.validate(&record)?;
        let association = if let Some(tls) = self.by_addr.get(&record.remote_addr) {
            record.ja3.clone_from(&tls.ja3);
            record.ja4.clone_from(&tls.ja4);
            record.engine.clone_from(&tls.engine);
            record.alpn.clone_from(&tls.alpn);
            record.tls_version.clone_from(&tls.tls_version);
            record.ext_order.clone_from(&tls.ext_order);
            Association::AddressOnly
        } else {
            Association::Absent
        };
        let previous = self.sessions.get(&record.session_id);
        if previous.is_none() && self.sessions.len() >= self.config.limits.max_sessions {
            return Err(Unavailable::Budget);
        }
        let old = previous
            .filter(|s| s.records.len() == 64)
            .and_then(|s| s.records.front())
            .map_or(0, Record::bytes);
        let key_bytes = if previous.is_none() {
            record.session_id.len()
        } else {
            0
        };
        let bytes = (self.bytes - old)
            .checked_add(record.bytes())
            .and_then(|bytes| bytes.checked_add(key_bytes))
            .filter(|bytes| *bytes <= self.config.limits.max_payload_bytes)
            .ok_or(Unavailable::Budget)?;
        self.clock.commit(record.time);
        let session = self
            .sessions
            .entry(record.session_id.clone())
            .or_insert_with(|| Session {
                records: VecDeque::new(),
                seen: record.time,
            });
        if session.records.len() == 64 {
            session.records.pop_front();
        }
        session.seen = record.time;
        session.records.push_back(record);
        self.bytes = bytes;
        Ok(association)
    }
    /// Copy retained records oldest first; absent history yields an empty copy.
    pub fn session(&self, id: &str) -> Vec<Record> {
        self.sessions
            .get(id)
            .map_or_else(Vec::new, |s| s.records.iter().cloned().collect())
    }
    /// Remove histories strictly older than TTL and commit the accepted time.
    pub fn gc(&mut self, now: Duration) -> Result<(), Unavailable> {
        self.clock.check(now)?;
        self.sessions.retain(|id, session| {
            if now - session.seen > self.config.ttl {
                self.bytes -= id.len() + session.records.iter().map(Record::bytes).sum::<usize>();
                false
            } else {
                true
            }
        });
        self.by_addr.retain(|addr, record| {
            if now - record.time > self.config.ttl {
                self.bytes -= addr.len() + record.bytes();
                false
            } else {
                true
            }
        });
        self.clock.commit(now);
        Ok(())
    }
    /// Clear both maps while retaining ordered-time custody.
    pub fn reset(&mut self, now: Duration) -> Result<(), Unavailable> {
        self.clock.check(now)?;
        self.sessions.clear();
        self.by_addr.clear();
        self.bytes = 0;
        self.clock.commit(now);
        Ok(())
    }
}
