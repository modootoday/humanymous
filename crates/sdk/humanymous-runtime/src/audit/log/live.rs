//! Live-log reads ported from Go `internal/audit/chain.go` and `verifier.go`
//! (oracle 981d25c): the bounded in-memory tail window behind `Recent(n)`,
//! the total `Len()`, and `SelfVerify` over the log's own durable history.
//! The WAL stays the only authority; the window is a convenience copy of
//! records that are already durable and never feeds recovery or proofs.
use super::Log;
use crate::audit::{
    checkpoint::Witness,
    record::Record,
    verification::{Report, verify, verify_with_witness},
};

/// Go `defaultRingCap`: records kept in the in-memory tail window unless a
/// capacity is set with [`Log::set_ring_capacity`].
pub const DEFAULT_RING_CAPACITY: usize = 4096;

impl Log {
    /// Bound the in-memory tail window (Go `Config.RingCap`). Zero selects
    /// [`DEFAULT_RING_CAPACITY`]. A longer current window keeps its newest
    /// `capacity` records, as Go replay did; afterwards the window may grow to
    /// twice the capacity before it is cut back to the newest `capacity`.
    pub fn set_ring_capacity(&mut self, capacity: usize) {
        self.ring_capacity = if capacity == 0 {
            DEFAULT_RING_CAPACITY
        } else {
            capacity
        };
        let excess = self.ring.len().saturating_sub(self.ring_capacity);
        self.ring.drain(..excess);
    }

    /// Up to the last `n` sealed records from the in-memory window, oldest
    /// first (Go `Recent`). Zero or a count beyond the window returns the
    /// whole window. Use [`Log::records`] for the authenticated full chain.
    pub fn recent(&self, n: usize) -> Vec<Record> {
        let n = if n == 0 || n > self.ring.len() {
            self.ring.len()
        } else {
            n
        };
        self.ring.range(self.ring.len() - n..).cloned().collect()
    }

    /// Total sealed records, equal to the sequence (Go `Len`); the in-memory
    /// window may hold fewer.
    pub fn len(&self) -> u64 {
        self.sequence
    }

    /// No record has been sealed yet.
    pub fn is_empty(&self) -> bool {
        self.sequence == 0
    }

    /// Writer Ed25519 verification key for exported signed tree heads (Go
    /// `PublicKey`). Public material only; the signing seed never leaves.
    pub fn writer_public_key(&self) -> [u8; 32] {
        self.signer.verifying_key().to_bytes()
    }

    /// Co-signer verification key when a witness is configured (Go
    /// `WitnessPublicKey`), whether in-process or an external client.
    pub fn witness_public_key(&self) -> Option<[u8; 32]> {
        self.external_witness
            .as_ref()
            .map(|witness| witness.public_key())
            .or_else(|| self.config.witness.as_ref().map(Witness::public_key))
    }

    /// Replay this log's own durable chain with its HMAC and writer keys (Go
    /// `SelfVerify`, for the admin integrity endpoint). When a witness is
    /// configured every checkpoint must also carry a valid co-signature.
    /// Unreadable storage verifies as an empty chain, which never passes.
    /// Production audits still run an independent verifier holding only the
    /// public keys.
    pub fn self_verify(&self) -> Report {
        let records = self.wal.records().unwrap_or_default();
        let writer = self.writer_public_key();
        match self.witness_public_key() {
            Some(witness) => verify_with_witness(
                &records,
                &self.checkpoints,
                &self.config.hmac_key,
                Some(&writer),
                &witness,
            ),
            None => verify(
                &records,
                &self.checkpoints,
                &self.config.hmac_key,
                Some(&writer),
            ),
        }
    }

    /// Append a sealed record to the window, cutting it back to the newest
    /// `ring_capacity` records once it exceeds twice that (Go `Append`).
    pub(super) fn remember(&mut self, record: &Record) {
        self.ring.push_back(record.clone());
        if self.ring.len() > 2 * self.ring_capacity {
            let excess = self.ring.len() - self.ring_capacity;
            self.ring.drain(..excess);
        }
    }
}

#[cfg(test)]
mod tests;
