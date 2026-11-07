use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Startup stages, not native proof or service-management authorization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupStage {
    /// Verified native/W preparation must precede unseal.
    NativePrepared,
    /// Actual writer preparation precedes Offer.
    WriterPrepared,
    /// Offer is pending; no claimed receipt is sufficient.
    OfferPending,
    /// Actual observer receipt binds the same epoch.
    ReceiptBound,
    /// Ready is incomplete; origin remains closed.
    ReadyEnqueuePending,
    /// Model of complete Ready enqueue and activation at one linearization point.
    Active,
    /// Irreversible first loss, cancellation or owner drop.
    TerminalRevoked,
}

/// First loss is independent from an authorized native-manager stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LossCause {
    /// Observer liveness was lost.
    ObserverLost,
    /// Independent witness liveness was lost.
    WitnessLost,
    /// The absolute epoch budget expired.
    EpochExpired,
    /// Startup was cancelled.
    StartupCancelled,
    /// Unique ownership was dropped without a continuing owner.
    OwnerDropped,
}

/// Immutable first-loss observation; not an effect or cleanup receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Revocation {
    /// Local monotonic first-loss instant.
    pub at: Instant,
    /// First cause; later cancellation cannot replace it.
    pub cause: LossCause,
}
impl Revocation {
    /// Never extend the original absolute close budget, including on repeated loss.
    pub fn close_deadline(self, original: Instant, drain: Duration) -> Option<Instant> {
        self.at.checked_add(drain).map(|limit| original.min(limit))
    }
}

/// Diagnostic state only. It cannot construct a production capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EpochSnapshot {
    /// Current local phase.
    pub stage: StartupStage,
    /// First revocation, if observed.
    pub revocation: Option<Revocation>,
}

/// A poisoned guard fails closed rather than recovering a live epoch.
#[derive(Debug, PartialEq, Eq)]
pub enum EpochError {
    /// State cannot be trusted.
    Poisoned,
}

struct State {
    stage: StartupStage,
    loss: Option<Revocation>,
}
pub(super) struct Owner(Arc<Mutex<State>>);

/// Restricted clones share one state; they have no activation or revoke method.
#[derive(Clone)]
pub struct ObserverEffectGate(Arc<Mutex<State>>);
impl ObserverEffectGate {
    /// Inspection is not permission followed by asynchronous origin I/O.
    pub fn snapshot(&self) -> Result<EpochSnapshot, EpochError> {
        let state = self.0.lock().map_err(|_| EpochError::Poisoned)?;
        Ok(EpochSnapshot {
            stage: state.stage,
            revocation: state.loss,
        })
    }
}
impl Owner {
    pub(super) fn gate(&self) -> ObserverEffectGate {
        ObserverEffectGate(self.0.clone())
    }
    pub(super) fn stage(&self) -> StartupStage {
        self.gate()
            .snapshot()
            .map_or(StartupStage::TerminalRevoked, |state| state.stage)
    }
    pub(super) fn revoke(&self, cause: LossCause) {
        self.revoke_at(Instant::now(), cause);
    }
    fn revoke_at(&self, at: Instant, cause: LossCause) {
        if let Ok(mut state) = self.0.lock() {
            state.loss.get_or_insert(Revocation { at, cause });
            state.stage = StartupStage::TerminalRevoked;
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.revoke(LossCause::OwnerDropped);
    }
}

#[cfg(test)]
impl Owner {
    // Pure INTERNAL model only. This cannot construct NativeProof or either
    // public production preparation/session, including in unit tests.
    pub(super) fn model() -> Self {
        Self(Arc::new(Mutex::new(State {
            stage: StartupStage::NativePrepared,
            loss: None,
        })))
    }
    pub(super) fn advance(&self, expected: StartupStage, next: StartupStage) -> bool {
        let Ok(mut state) = self.0.lock() else {
            return false;
        };
        let allowed = matches!(
            (expected, next),
            (StartupStage::NativePrepared, StartupStage::WriterPrepared)
                | (StartupStage::WriterPrepared, StartupStage::OfferPending)
                | (StartupStage::OfferPending, StartupStage::ReceiptBound)
                | (
                    StartupStage::ReceiptBound,
                    StartupStage::ReadyEnqueuePending
                )
                | (StartupStage::ReadyEnqueuePending, StartupStage::Active)
        );
        if !allowed || state.loss.is_some() || state.stage != expected {
            return false;
        }
        state.stage = next;
        true
    }
    pub(super) fn lose_at(&self, at: Instant, cause: LossCause) {
        self.revoke_at(at, cause);
    }
}
