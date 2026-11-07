use sha2::{Digest, Sha256};
use std::time::Duration;

/// Semantic atomic operation only. No backend, Lua, network I/O or fleet proof.
pub trait AtomicCoordinator {
    fn observe(&mut self, request: &CoordinatorRequest)
    -> Result<Vec<i64>, CoordinatorUnavailable>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoordinatorUnavailable;
/// Caller must map ordered ticks into its accepted backend clock contract.
pub struct CoordinatorRequest {
    pub digest: String,
    pub subnet: String,
    pub now: Duration,
    pub window: Duration,
    pub ttl: Duration,
}
/// Reported counts are unqualified data, never a trusted fleet capability.
pub enum FleetInput {
    LocalOnly,
    Unavailable,
    Reported(Vec<i64>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    MissingKey,
    LocalOnly,
    LocalDegraded,
    CoordinatorReportedUnqualified,
}
impl FleetInput {
    pub(super) fn counts(self, new: bool, velocity: usize) -> (bool, usize, Scope) {
        match self {
            Self::LocalOnly => (new, velocity, Scope::LocalOnly),
            Self::Reported(reply) if reply.len() >= 2 => (
                reply[0] == 1,
                reply[1].clamp(0, i64::from(i32::MAX)) as usize,
                Scope::CoordinatorReportedUnqualified,
            ),
            _ => (new, velocity, Scope::LocalDegraded),
        }
    }
}

/// Standard SHA256/HMAC-SHA256 digest compatibility; hashes are pseudonyms.
/// No raw key is stored by a coordinator implementation in this context.
pub fn key_digest(sign: &[u8], key: &str) -> String {
    if sign.is_empty() {
        return hex::encode(Sha256::digest(key.as_bytes()));
    }
    let mut block = [0u8; 64];
    if sign.len() > 64 {
        block[..32].copy_from_slice(&Sha256::digest(sign));
    } else {
        block[..sign.len()].copy_from_slice(sign);
    }
    let inner_pad = block.map(|byte| byte ^ 0x36);
    let outer_pad = block.map(|byte| byte ^ 0x5c);
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(key.as_bytes());
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner.finalize());
    hex::encode(outer.finalize())
}
