//! Fleet correlation step over the RESP client, ported from Go
//! `internal/correlation/correlation.go` (`fleetCorrLua`, `observeFleet`,
//! oracle 981d25c).
//!
//! One atomic EVAL per observation records the subnet in the key's distinct
//! subnet set and, when it is globally new, stamps it into the key's arrival
//! window; the reply is `[added, fleetVelocity]`. The reply is handed to
//! `network::correlation::Registry` as unqualified [`FleetInput`]: only the
//! weight-zero velocity residual consumes it, scoring is unchanged, and any
//! failure degrades to the registry's local counts (Go fail-open fallback).
//!
//! Two deliberate tightenings, each pinned by a test:
//! - The request digest must be the fixed-width 64 lowercase hex pseudonym
//!   from `key_digest`; anything else is refused before any I/O, so a raw
//!   fingerprint can never become a coordinator key at rest.
//! - A reply element that is not an integer is unavailable rather than Go's
//!   zero value, so a broken script cannot report a fabricated "not new".
use super::redis::{Client, Reply};
use crate::network::correlation::{
    AtomicCoordinator, CoordinatorRequest, CoordinatorUnavailable, FleetInput,
};
use std::sync::Arc;
use std::time::Duration;

/// Distinct-subnet set key prefix (Go `"hmn:corr:sub:"`).
pub const SUBNET_PREFIX: &str = "hmn:corr:sub:";
/// Arrival window key prefix (Go `"hmn:corr:vel:"`).
pub const VELOCITY_PREFIX: &str = "hmn:corr:vel:";

/// Atomic fleet correlation step (Go `fleetCorrLua`). KEYS[1] is the distinct
/// subnet set, KEYS[2] the arrival window; ARGV is now (ns), window (ns), TTL
/// (ms) and the subnet. The set cardinality is deliberately not returned.
pub const FLEET_CORRELATION_SCRIPT: &str = "local added = redis.call('SADD', KEYS[1], ARGV[4])
redis.call('PEXPIRE', KEYS[1], tonumber(ARGV[3]))
if added == 1 then redis.call('ZADD', KEYS[2], ARGV[1], ARGV[4]) end
redis.call('ZREMRANGEBYSCORE', KEYS[2], 0, tonumber(ARGV[1]) - tonumber(ARGV[2]))
redis.call('PEXPIRE', KEYS[2], tonumber(ARGV[3]))
return { added, redis.call('ZCARD', KEYS[2]) }";

const DIGEST_LEN: usize = 64;

/// Redis-backed [`AtomicCoordinator`] for the correlation registry. Request
/// times are wall-clock durations since the Unix epoch, compared across nodes.
#[derive(Debug)]
pub struct SharedCorrelation {
    client: Arc<Client>,
}

impl SharedCorrelation {
    /// Coordinator over `client`; the connection is dialled on first use.
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }

    /// Run one step and project it for `Registry::observe`: a reply becomes
    /// [`FleetInput::Reported`] (a short reply still degrades there), and any
    /// failure becomes [`FleetInput::Unavailable`].
    pub fn fleet_input(&mut self, request: &CoordinatorRequest) -> FleetInput {
        match self.observe(request) {
            Ok(reply) => FleetInput::Reported(reply),
            Err(CoordinatorUnavailable) => FleetInput::Unavailable,
        }
    }
}

impl AtomicCoordinator for SharedCorrelation {
    fn observe(
        &mut self,
        request: &CoordinatorRequest,
    ) -> Result<Vec<i64>, CoordinatorUnavailable> {
        if !pseudonymous(&request.digest) {
            return Err(CoordinatorUnavailable);
        }
        let reply = self
            .client
            .command(&[
                b"EVAL",
                FLEET_CORRELATION_SCRIPT.as_bytes(),
                b"2",
                format!("{SUBNET_PREFIX}{}", request.digest).as_bytes(),
                format!("{VELOCITY_PREFIX}{}", request.digest).as_bytes(),
                request.now.as_nanos().to_string().as_bytes(),
                request.window.as_nanos().to_string().as_bytes(),
                millis(request.ttl).as_bytes(),
                request.subnet.as_bytes(),
            ])
            .map_err(|_| CoordinatorUnavailable)?;
        let Reply::Array(items) = reply else {
            return Err(CoordinatorUnavailable);
        };
        items
            .into_iter()
            .map(|item| match item {
                Reply::Integer(value) => Ok(value),
                _ => Err(CoordinatorUnavailable),
            })
            .collect()
    }
}

fn pseudonymous(digest: &str) -> bool {
    digest.len() == DIGEST_LEN
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn millis(duration: Duration) -> String {
    duration.as_millis().to_string()
}

#[cfg(test)]
#[path = "correlation_tests.rs"]
mod tests;
