//! Required observer admission retains configuration before any provider call.
use super::{AuthorityGuard, AuthorityPhase, ObservedStartupOwner, Retained, RetainedFailure};
use crate::{
    authority::startup::observer::{
        self, AuthenticatedObserverReceipt, LossCause, NativePreparationUnavailable,
    },
    gateway::{configuration::Configuration, service::ServiceHost},
};

/// Startup bookkeeping only; none of these states is an origin/Ready capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequiredObserverStage {
    /// Configuration retained; native verification has not been attempted.
    Unprepared,
    /// The original Authority producer returned an opaque preparation.
    NativePrepared,
    /// Original Authority accepted its opaque receipt for the same owner.
    /// This still does not assert Runtime resource readiness or activation.
    ReceiptBound,
    /// Irreversible refusal or cancellation; no retry/reset is available.
    Refused,
}
/// Fixed, nonsecret refusal facts. No native identity is inferred from a code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequiredObserverRefusal {
    /// The original native preparation provider is unavailable.
    NativeUnavailable,
    /// Resource/route validation failed before unseal or bind.
    Configuration,
    /// Offer was requested before actual native preparation.
    PreparationRequired,
    /// No original consuming Offer/ACK producer is implemented yet.
    OfferAckUnavailable,
    /// Original Authority refused the receipt, epoch, phase or revocation state.
    ReceiptRejected,
    /// A spent attempt cannot reserve a second epoch or renew its lifetime.
    AlreadyConsumed,
    /// Explicit caller cancellation, not a successful service stop.
    Cancelled,
}

/// Owns the original resources and Authority guard. No secret or handle extraction.
pub struct RequiredObserverStartup {
    owner: ObservedStartupOwner<Configuration>,
    stage: RequiredObserverStage,
    first_refusal: Option<RequiredObserverRefusal>,
    native_unavailable: Option<NativePreparationUnavailable>,
}
/// Retains the same resource allocation and original cancellation guard on error.
/// Drop is not a worker join, credential receipt or durable audit flush.
pub struct RequiredObserverFailure {
    retained: RetainedFailure<Configuration, AuthorityGuard>,
    reason: RequiredObserverRefusal,
    native_unavailable: Option<NativePreparationUnavailable>,
}
impl ServiceHost {
    /// Retain private input without unseal, listener bind, credential generation
    /// or activation. The original native provider must run before these effects.
    pub fn begin_required_observer(configuration: Configuration) -> RequiredObserverStartup {
        RequiredObserverStartup {
            owner: ObservedStartupOwner {
                retained: Retained::new(configuration, AuthorityGuard(None)),
            },
            stage: RequiredObserverStage::Unprepared,
            first_refusal: None,
            native_unavailable: None,
        }
    }
}
impl RequiredObserverStartup {
    /// Bookkeeping view only; it cannot activate a service.
    pub fn stage(&self) -> RequiredObserverStage {
        self.stage
    }
    /// First refusal is conserved across cancellation and replay attempts.
    pub fn first_refusal(&self) -> Option<RequiredObserverRefusal> {
        self.first_refusal
    }

    /// Attempt the original Authority producer once. No DTO, bool, PID or Job
    /// observation substitutes for its opaque VerifiedObserverPreparation.
    pub fn prepare_native_once(&mut self) -> Result<(), RequiredObserverRefusal> {
        if self.stage != RequiredObserverStage::Unprepared {
            self.refuse(RequiredObserverRefusal::AlreadyConsumed);
            return Err(RequiredObserverRefusal::AlreadyConsumed);
        }
        // Spend before calling the provider; neither failure nor cancellation
        // can reopen the attempt. The resources already have their final Box.
        self.stage = RequiredObserverStage::Refused;
        let preparation = match observer::native_preparation() {
            Ok(preparation) => preparation,
            Err(original) => {
                self.native_unavailable = Some(original);
                self.refuse(RequiredObserverRefusal::NativeUnavailable);
                return Err(RequiredObserverRefusal::NativeUnavailable);
            }
        };
        self.owner.retained.guard_mut().0 = Some(AuthorityPhase::Preparation(preparation));
        if self.owner.retained.resources().validate().is_err() {
            self.refuse(RequiredObserverRefusal::Configuration);
            return Err(RequiredObserverRefusal::Configuration);
        }
        self.stage = RequiredObserverStage::NativePrepared;
        Ok(())
    }

    /// Current explicit refusal seam. No Offer is sent and no credential is
    /// generated. Replace ONLY with the original Authority's consuming same-owner
    /// exchange API; accepting a separately supplied receipt would permit rebinding.
    pub fn offer_ack_once(&mut self) -> Result<(), RequiredObserverRefusal> {
        let reason = match self.stage {
            RequiredObserverStage::Unprepared => RequiredObserverRefusal::PreparationRequired,
            RequiredObserverStage::NativePrepared => RequiredObserverRefusal::OfferAckUnavailable,
            RequiredObserverStage::ReceiptBound => RequiredObserverRefusal::AlreadyConsumed,
            RequiredObserverStage::Refused => RequiredObserverRefusal::AlreadyConsumed,
        };
        self.refuse(reason);
        Err(reason)
    }

    /// Consume ONLY the original Authority's authenticated receipt. Authority
    /// owns the same-epoch/OfferPending/no-loss checks; DATA/Job observations and
    /// caller booleans cannot construct this input. No Ready is produced here.
    ///
    /// This consumer requires Authority's separately proposed into_prepared API;
    /// its transport/receipt origin remains uninhabited until actually qualified.
    pub fn bind_authenticated_receipt(
        &mut self,
        receipt: AuthenticatedObserverReceipt,
    ) -> Result<(), RequiredObserverRefusal> {
        if self.stage != RequiredObserverStage::NativePrepared {
            self.refuse(RequiredObserverRefusal::AlreadyConsumed);
            return Err(RequiredObserverRefusal::AlreadyConsumed);
        }
        // Spend the transition before consuming the original opaque preparation.
        // If the producer panics, the guard drops/revokes and this attempt cannot
        // retry. The boxed Runtime resources remain in this owner throughout.
        self.stage = RequiredObserverStage::Refused;
        let preparation = match self.owner.retained.guard_mut().0.take() {
            Some(AuthorityPhase::Preparation(preparation)) => preparation,
            other => {
                self.owner.retained.guard_mut().0 = other;
                self.refuse(RequiredObserverRefusal::ReceiptRejected);
                return Err(RequiredObserverRefusal::ReceiptRejected);
            }
        };
        match preparation.into_prepared(receipt) {
            Ok(session) => {
                self.owner.retained.guard_mut().0 = Some(AuthorityPhase::ReceiptBound(session));
                self.stage = RequiredObserverStage::ReceiptBound;
                Ok(())
            }
            Err(original_failure) => {
                self.owner.retained.guard_mut().0 = Some(AuthorityPhase::Failed(original_failure));
                self.refuse(RequiredObserverRefusal::ReceiptRejected);
                Err(RequiredObserverRefusal::ReceiptRejected)
            }
        }
    }

    /// Revoke synchronously and retain ownership; this does not certify cleanup.
    pub fn cancel(mut self) -> RequiredObserverFailure {
        self.refuse(RequiredObserverRefusal::Cancelled);
        RequiredObserverFailure {
            retained: self.owner.cancel(LossCause::StartupCancelled),
            reason: self
                .first_refusal
                .unwrap_or(RequiredObserverRefusal::Cancelled),
            native_unavailable: self.native_unavailable,
        }
    }
    fn refuse(&mut self, reason: RequiredObserverRefusal) {
        self.first_refusal.get_or_insert(reason);
        self.stage = RequiredObserverStage::Refused;
        self.owner
            .retained
            .guard_mut()
            .cancel_current(LossCause::StartupCancelled);
    }
}
impl RequiredObserverFailure {
    /// Reassert revocation before later bounded cleanup without extracting,
    /// replacing or releasing the retained guard/resources. This is idempotent
    /// cancellation, not a cleanup-completion or graceful-stop receipt.
    pub fn revoke(&mut self) {
        self.retained
            .guard_mut()
            .cancel_current(LossCause::StartupCancelled);
    }
    /// Static first-failure reason, not a successful stop/ACK/delivery assertion.
    pub fn reason(&self) -> RequiredObserverRefusal {
        self.reason
    }
    /// Original typed native refusal, retained without translating it to proof.
    pub fn native_unavailable(&self) -> Option<&NativePreparationUnavailable> {
        self.native_unavailable.as_ref()
    }
    /// Borrow the original retained Authority failure without extracting its
    /// guard or resources. This is not ACK, cleanup or service-stop success.
    pub fn authority_failure(&self) -> Option<&observer::ObserverStartupFailure> {
        match self.retained.guard().0.as_ref() {
            Some(AuthorityPhase::Failed(original)) => Some(original),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
