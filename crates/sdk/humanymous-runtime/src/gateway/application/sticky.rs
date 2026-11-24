//! Server-side sticky verdicts for the verdict fast path (owner decision #4,
//! option 1 chosen 2026-11-24 on issue #75; `plan:root/20261123121500`).
//!
//! Go issued a fingerprint-bound `hmn_vt` cookie on a collect ALLOW and its
//! `verdictTokenGate` trusted it for 30 minutes. The runtime browser profile
//! is cookie-free and every protected request is authenticated by request
//! integrity under a server-held session key, so the verdict stays on the
//! server: a scored ALLOW on an enforcing route records, for that session,
//! the client binding (Go `tokenBind`: header fingerprint and client subnet)
//! and an expiry. The verdict-token gate kernel reads it in place of the
//! cookie:
//! - no entry: `Token::Absent` (full scoring);
//! - an entry whose binding differs: `Token::Invalid(BindingMismatch)`;
//! - an expired entry: `Token::Invalid(Expired)`;
//! - otherwise `Token::Valid`.
//!
//! An invalid entry is dropped, as Go cleared the cookie, so the request is
//! re-scored and a fresh ALLOW records a new entry. Entries are bounded: a new
//! session beyond `max_sessions` (after expired entries are swept) records
//! nothing and is simply scored every time (fail closed toward scoring).
use crate::gateway::gates::Token;
use crate::gateway::token::Reason;
use std::collections::BTreeMap;
use std::time::Duration;

struct Entry {
    until: Duration,
    binding: String,
}

/// Bounded per-session ALLOW verdicts on a monotonic clock.
pub(super) struct StickyVerdicts {
    ttl: Duration,
    capacity: usize,
    entries: BTreeMap<String, Entry>,
}

impl StickyVerdicts {
    pub(super) fn new(ttl: Duration, capacity: usize) -> Self {
        Self {
            ttl,
            capacity,
            entries: BTreeMap::new(),
        }
    }

    /// Record a scored ALLOW for `sid` bound to `binding` at `now`. Returns
    /// whether an entry was recorded (false when full).
    pub(super) fn record(&mut self, sid: &str, binding: &str, now: Duration) -> bool {
        if !self.entries.contains_key(sid) && self.entries.len() >= self.capacity {
            self.gc(now);
            if self.entries.len() >= self.capacity {
                return false;
            }
        }
        let until = now.saturating_add(self.ttl);
        self.entries.insert(
            sid.to_owned(),
            Entry {
                until,
                binding: binding.to_owned(),
            },
        );
        true
    }

    /// The gate input for `sid` presenting `binding` at `now`. An invalid
    /// entry is removed.
    pub(super) fn check(&mut self, sid: &str, binding: &str, now: Duration) -> Token {
        let Some(entry) = self.entries.get(sid) else {
            return Token::Absent;
        };
        let token = if entry.binding != binding {
            Token::Invalid(Reason::BindingMismatch)
        } else if now >= entry.until {
            Token::Invalid(Reason::Expired)
        } else {
            Token::Valid
        };
        if token != Token::Valid {
            self.entries.remove(sid);
        }
        token
    }

    /// Drop expired entries (legacy `Server.GC`).
    pub(super) fn gc(&mut self, now: Duration) {
        self.entries.retain(|_, entry| now < entry.until);
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sticky_verdict_binding_expiry_and_bound() {
        let mut sticky = StickyVerdicts::new(Duration::from_secs(60), 2);
        let t0 = Duration::from_secs(10);
        assert_eq!(sticky.check("a", "fp", t0), Token::Absent);
        assert!(sticky.record("a", "fp", t0));
        assert_eq!(
            sticky.check("a", "fp", t0 + Duration::from_secs(59)),
            Token::Valid
        );
        // Another fingerprint drops the entry; the next check is absent.
        assert_eq!(
            sticky.check("a", "other", t0),
            Token::Invalid(Reason::BindingMismatch)
        );
        assert_eq!(sticky.check("a", "fp", t0), Token::Absent);
        assert!(sticky.record("a", "fp", t0));
        assert_eq!(
            sticky.check("a", "fp", t0 + Duration::from_secs(60)),
            Token::Invalid(Reason::Expired)
        );
        assert_eq!(sticky.len(), 0);
        // Bounded: a third session is refused until an entry expires.
        assert!(sticky.record("a", "fp", t0));
        assert!(sticky.record("b", "fp", t0));
        assert!(!sticky.record("c", "fp", t0));
        assert!(sticky.record("a", "fp", t0), "refresh of a known session");
        assert!(sticky.record("c", "fp", t0 + Duration::from_secs(60)));
        assert_eq!(sticky.len(), 1);
        sticky.gc(t0 + Duration::from_secs(200));
        assert_eq!(sticky.len(), 0);
    }
}
