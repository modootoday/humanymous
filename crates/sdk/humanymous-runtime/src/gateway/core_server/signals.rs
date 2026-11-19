//! Server-derived L5 signals the legacy Core computed inline (port of
//! cmd/server/server.go `fingerprintChurnSignal` and `vpnWebRTCLeakSignal` at
//! baseline 981d25c). Signal identities, values, verdicts and notes are the
//! frozen ones.
//!
//! Deliberate difference from Go: the per-session fingerprint memory is
//! bounded (`capacity` sessions, idle entries expire with the collector-store
//! TTL); Go's map grew without bound. A full table refuses the observation
//! instead of evicting a live session.
use crate::network::history::Unavailable;
use crate::network::http::{local_address as local, parse_ip};
use humanymous_core::signals::observation::{Signal, Source, Verdict};
use serde_json::json;
use std::collections::BTreeMap;
use std::time::Duration;

/// Bounded per-session last-fingerprint memory (Go `app.lastFP`).
#[derive(Debug)]
pub struct FingerprintTracker {
    last: BTreeMap<String, (String, Duration)>,
    capacity: usize,
}

impl FingerprintTracker {
    /// Empty tracker holding at most `capacity` sessions.
    pub fn new(capacity: usize) -> Self {
        Self {
            last: BTreeMap::new(),
            capacity,
        }
    }

    /// Go `fingerprintChurnSignal`: record `fp` for `sid` and return the churn
    /// signal when a different non-empty fingerprint was recorded before.
    pub fn observe(
        &mut self,
        sid: &str,
        fp: &str,
        now: Duration,
    ) -> Result<Option<Signal>, Unavailable> {
        if sid.is_empty() || fp.is_empty() {
            return Ok(None);
        }
        if !self.last.contains_key(sid) && self.last.len() >= self.capacity {
            return Err(Unavailable::Budget);
        }
        let previous = self.last.insert(sid.to_owned(), (fp.to_owned(), now));
        Ok(previous
            .filter(|(prev, _)| !prev.is_empty() && prev != fp)
            .map(|(prev, _)| {
                Signal::new(
                    "l5.correlation.fp_churn_proxy",
                    json!({"prev": prev, "next": fp}),
                    Verdict::Bot,
                    1.0,
                    Source::Server,
                    "fingerprintId rotated mid-session (proxy fp-churn evasion)",
                )
            }))
    }

    /// Forget sessions idle for longer than `ttl`.
    pub fn gc(&mut self, now: Duration, ttl: Duration) {
        self.last
            .retain(|_, (_, seen)| now.saturating_sub(*seen) <= ttl);
    }

    /// Sessions retained.
    pub fn len(&self) -> usize {
        self.last.len()
    }

    /// Whether no session is retained.
    pub fn is_empty(&self) -> bool {
        self.last.is_empty()
    }
}

/// Go `vpnWebRTCLeakSignal`: the first public WebRTC candidate that differs
/// from a public TCP client address. Private, loopback and link-local TCP
/// peers are never compared.
pub fn webrtc_leak(public_addrs: &[String], tcp_client_ip: &str) -> Option<Signal> {
    let tcp = parse_ip(tcp_client_ip).filter(|ip| !local(*ip))?;
    let tcp_text = tcp.to_string();
    public_addrs
        .iter()
        .filter_map(|raw| parse_ip(raw.trim()))
        .filter(|ip| !local(*ip) && !ip.is_unspecified())
        .map(|ip| ip.to_string())
        .find(|candidate| *candidate != tcp_text)
        .map(|candidate| {
            Signal::new(
                "l5.proxy.vpn_webrtc_leak",
                json!({"tcp": tcp_text, "webrtc": candidate}),
                Verdict::Bot,
                1.0,
                Source::Server,
                "WebRTC public candidate ≠ TCP client IP (VPN/tunnel leak)",
            )
        })
}
