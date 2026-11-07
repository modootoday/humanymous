//! Sealed startup ownership. No native producer or transport is qualified yet.
mod state;
use state::Owner;
pub use state::{
    EpochError, EpochSnapshot, LossCause, ObserverEffectGate, Revocation, StartupStage,
};

/// The native backend cannot currently mint a production preparation.
#[derive(Debug, PartialEq, Eq)]
pub enum NativePreparationUnavailable {
    /// Native instance, witness and publication constructors are not accepted.
    Unsupported,
}

// Deliberately uninhabited, including in unit tests. A test phase model must not
// manufacture a production native capability. Replace only after joint review.
enum NativeProof {}

/// Pre-unseal native/W preparation; not an observer credential receipt or Ready.
pub struct VerifiedObserverPreparation {
    owner: Owner,
    _proof: NativeProof,
}

/// Later, actual post-Offer/ACK ownership; no public receipt constructor exists.
pub struct PreparedObserverSession {
    owner: Owner,
    _proof: NativeProof,
}

/// Observed delivery facts are not permission to activate or resend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadyDelivery {
    /// No Ready bytes submitted.
    NotSubmitted,
    /// Only part of Ready was submitted.
    Partial,
    /// Complete Ready was submitted; peer receipt is a separate fact.
    Submitted,
    /// Submission outcome is not known.
    Unknown,
}

/// Retains the Authority guard. Runtime must retain its own resources alongside
/// this value until explicit bounded cleanup, never detach them on an error.
pub struct ObserverStartupFailure {
    owner: Owner,
    stage: StartupStage,
    delivery: ReadyDelivery,
    _proof: NativeProof,
}

/// Explicit fail-closed producer seam; no DTO, parent flag or bool is accepted.
pub fn native_preparation() -> Result<VerifiedObserverPreparation, NativePreparationUnavailable> {
    Err(NativePreparationUnavailable::Unsupported)
}

impl VerifiedObserverPreparation {
    /// Restricted view of the same epoch. It cannot activate or construct a proof.
    pub fn effect_gate(&self) -> ObserverEffectGate {
        self.owner.gate()
    }
    /// Synchronous revocation retains ownership for Runtime's later cleanup.
    pub fn cancel(self, cause: LossCause) -> ObserverStartupFailure {
        let stage = self.owner.stage();
        self.owner.revoke(cause);
        ObserverStartupFailure {
            owner: self.owner,
            stage,
            delivery: ReadyDelivery::NotSubmitted,
            _proof: self._proof,
        }
    }
}

impl PreparedObserverSession {
    /// Restricted view of the same post-receipt epoch, not a Ready proof.
    pub fn effect_gate(&self) -> ObserverEffectGate {
        self.owner.gate()
    }
    /// No Ready emission API exists yet, so this phase has submitted no Ready.
    pub fn cancel(self, cause: LossCause) -> ObserverStartupFailure {
        let stage = self.owner.stage();
        self.owner.revoke(cause);
        ObserverStartupFailure {
            owner: self.owner,
            stage,
            delivery: ReadyDelivery::NotSubmitted,
            _proof: self._proof,
        }
    }
}

impl ObserverStartupFailure {
    /// Failed phase before irreversible revocation.
    pub fn stage(&self) -> StartupStage {
        self.stage
    }
    /// No inferred ACK, flush, drain or join completion.
    pub fn ready_delivery(&self) -> ReadyDelivery {
        self.delivery
    }
    /// The guard remains owned even after callers inspect failure facts.
    pub fn effect_gate(&self) -> ObserverEffectGate {
        self.owner.gate()
    }
}

#[cfg(test)]
mod tests;
