//! Trusted native atomicity only; callers authenticate typed state and WAL commits.
use super::PrivateState;
use crate::audit::keys::custody::{Durability, Owner};
use std::{io, path::Path};

/// Exact private bytes for explicit caller-authenticated interruption recovery.
/// This is not an authenticated payload and has no Debug/serialization output.
pub struct SnapshotInspection {
    /// Last atomically promoted payload, if any.
    pub committed: Option<Vec<u8>>,
    /// Interrupted pending payload, if any; never implicitly accepted.
    pub pending: Option<Vec<u8>>,
}
/// Exclusive bounded native snapshot. Trusted runtime code owns semantic CAS,
/// signed audit correlation, scope, candidate lifecycle and apply decisions.
pub struct TrustedSnapshot {
    store: PrivateState,
}
impl TrustedSnapshot {
    /// Acquire an existing provisioned private directory and stable lifetime
    /// lock. Namespace is trusted lowercase ASCII/hyphens; max_bytes is1..4MiB.
    /// Opening creates no payload, repairs no permissions and resets no state.
    pub fn open(
        directory: &Path,
        owner: Owner,
        namespace: &str,
        max_bytes: u64,
    ) -> io::Result<Self> {
        Ok(Self {
            store: PrivateState::open_bounded(directory, owner, namespace, max_bytes)?,
        })
    }
    /// Read committed bytes only when no native pending replacement exists.
    /// Missing payload is distinct from authenticated initial operational state.
    pub fn read(&self) -> io::Result<Option<Vec<u8>>> {
        self.store.read()
    }
    /// Inspect both native states without selecting or authenticating either.
    /// A previously failed write requires dropping/reopening before inspection.
    pub fn inspect(&self) -> io::Result<SnapshotInspection> {
        Ok(SnapshotInspection {
            committed: self.store.committed()?,
            pending: self.store.pending()?,
        })
    }
    /// Write exact nonempty bounded bytes, sync and atomically promote. Success
    /// reports native durability, not authentication, WAL commit or effect apply.
    /// Unknown failure poisons this owner until drop/reopen; no retry is inferred.
    pub fn replace(&mut self, bytes: &[u8]) -> io::Result<Durability> {
        self.store.commit(bytes)
    }
    /// Explicitly promote only the exact committed and pending bytes already
    /// authenticated by the trusted caller against its typed payload/WAL policy.
    /// This method performs equality checks, not cryptographic authentication.
    pub fn promote_pending(
        &mut self,
        expected_committed: Option<&[u8]>,
        expected_pending: &[u8],
    ) -> io::Result<Durability> {
        let actual = self.inspect()?;
        if expected_pending.is_empty()
            || actual.committed.as_deref() != expected_committed
            || actual.pending.as_deref() != Some(expected_pending)
        {
            return Err(io::Error::other("trusted snapshot recovery bytes mismatch"));
        }
        self.store.promote_pending()
    }
}
