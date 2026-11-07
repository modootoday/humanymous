//! Private test-only symbolic ordering model, never a journal or native authority.
//! No persistence, real clock, identity admission or cleanup proof is represented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ModelAttempt {
    id: u64,
    generation: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ModelAnchor {
    volume: u64,
    file: u128,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ModelOriginalPair {
    start_tick: u64,
    run_until: u64,
    cleanup_until: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ModelRequest {
    attempt: ModelAttempt,
    owner: u64,
    anchor: ModelAnchor,
    selector: u64,
    policy: u64,
    namespace: u64,
    nonce: u128,
    epoch: u64,
    original: ModelOriginalPair,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ModelCaps {
    attempts: usize,
    namespaces: usize,
    tombstones: usize,
    bytes: u64,
    pending_operations: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ModelCost {
    bytes: u64,
    pending_operations: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelRelation {
    Same,
    Disjoint,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ModelNamespaceRelation {
    existing: ModelAttempt,
    relation: ModelRelation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelPhase {
    Reserved,
    CreatePending,
    Unknown,
    Terminal,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct ModelRecord {
    request: ModelRequest,
    phase: ModelPhase,
    cost: ModelCost,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ModelNamespace {
    attempt: ModelAttempt,
    token: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelHistoryState {
    KnownFixtureHistory,
    UnknownFixtureHistory,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ModelTerminalObservation {
    attempt: ModelAttempt,
    owner: u64,
    anchor: ModelAnchor,
    namespace: u64,
    epoch: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelError {
    Invalid,
    Conflict,
    Capacity,
    Overflow,
    Expired,
    UnknownHistory,
    StaleEpoch,
    UnknownAttempt,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelReservationOutcome {
    ModelReserved,
    ModelExistingSameRecord,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct ModelLedger {
    caps: ModelCaps,
    epoch: u64,
    last_tick: u64,
    history_known: bool,
    records: Vec<ModelRecord>,
    namespaces: Vec<ModelNamespace>,
    tombstones: Vec<ModelRequest>,
    used_bytes: u64,
    pending_operations: usize,
}
type ModelSnapshot = ModelLedger;

impl ModelLedger {
    // NEW fixture only: never models successful recovery of missing history.
    fn new_fixture(caps: ModelCaps, epoch: u64, now: u64) -> Result<Self, ModelError> {
        if epoch == 0
            || [caps.attempts, caps.namespaces, caps.tombstones]
                .iter()
                .any(|n| *n == 0 || *n > 64)
            || caps.bytes == 0
            || caps.pending_operations == 0
        {
            return Err(ModelError::Invalid);
        }
        Ok(Self {
            caps,
            epoch,
            last_tick: now,
            history_known: true,
            records: Vec::new(),
            namespaces: Vec::new(),
            tombstones: Vec::new(),
            used_bytes: 0,
            pending_operations: 0,
        })
    }
    fn snapshot(&self) -> ModelSnapshot {
        self.clone()
    }
    fn check_tick(&self, now: u64) -> Result<(), ModelError> {
        if now < self.last_tick {
            Err(ModelError::Invalid)
        } else {
            Ok(())
        }
    }
    fn check_indexes(&self) -> Result<(), ModelError> {
        let active = self
            .records
            .iter()
            .filter(|record| record.phase != ModelPhase::Terminal);
        if active.clone().count() != self.namespaces.len()
            || active.clone().any(|record| {
                self.namespaces
                    .iter()
                    .filter(|slot| {
                        slot.attempt == record.request.attempt
                            && slot.token == record.request.namespace
                    })
                    .count()
                    != 1
            })
            || self.namespaces.iter().any(|slot| {
                active
                    .clone()
                    .filter(|record| {
                        record.request.attempt == slot.attempt
                            && record.request.namespace == slot.token
                    })
                    .count()
                    != 1
            })
            || self
                .records
                .iter()
                .filter(|record| record.phase == ModelPhase::Terminal)
                .count()
                != self.tombstones.len()
            || self.tombstones.iter().any(|tombstone| {
                self.records
                    .iter()
                    .filter(|record| {
                        record.phase == ModelPhase::Terminal && record.request == *tombstone
                    })
                    .count()
                    != 1
            })
        {
            return Err(ModelError::UnknownHistory);
        }
        Ok(())
    }
    fn admitted_history(&self, now: u64) -> Result<(), ModelError> {
        self.check_tick(now)?;
        if !self.history_known {
            return Err(ModelError::UnknownHistory);
        }
        self.check_indexes()
    }
    fn record_index(&self, attempt: ModelAttempt) -> Result<usize, ModelError> {
        self.records
            .iter()
            .position(|record| record.request.attempt == attempt)
            .ok_or(ModelError::UnknownAttempt)
    }
}

mod tests;
mod transitions;
