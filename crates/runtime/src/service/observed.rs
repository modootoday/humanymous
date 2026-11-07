//! Sealed pre-unseal retention. Native preparation remains Unsupported.
mod retained;
use super::ServiceHost;
use crate::configuration::Configuration;
use humanymous_authority::startup::observer::{
    LossCause, ObserverStartupFailure, PreparedObserverSession, VerifiedObserverPreparation,
};
use retained::{Retained, RetainedFailure};

enum AuthorityPhase {
    Preparation(VerifiedObserverPreparation),
    #[expect(
        dead_code,
        reason = "No receipt producer is accepted; remove when the bounded native ACK transition is wired"
    )]
    ReceiptBound(PreparedObserverSession),
    Failed(ObserverStartupFailure),
}
struct AuthorityGuard(Option<AuthorityPhase>);
impl AuthorityGuard {
    fn cancelled(mut self, cause: LossCause) -> Self {
        self.0 = self.0.take().map(|phase| cancel_phase(phase, cause));
        self
    }
}
fn cancel_phase(phase: AuthorityPhase, cause: LossCause) -> AuthorityPhase {
    AuthorityPhase::Failed(match phase {
        AuthorityPhase::Preparation(preparation) => preparation.cancel(cause),
        AuthorityPhase::ReceiptBound(session) => session.cancel(cause),
        AuthorityPhase::Failed(failure) => failure,
    })
}
impl Drop for AuthorityGuard {
    fn drop(&mut self) {
        if let Some(phase) = self.0.take() {
            // Authority synchronously revokes before Runtime resources drop.
            // This cannot certify monitor join, request drain or audit flush.
            drop(cancel_phase(phase, LossCause::OwnerDropped));
        }
    }
}

struct ObservedStartupOwner<R> {
    retained: Retained<R, AuthorityGuard>,
}
type RuntimeActivationFailure<R> = RetainedFailure<R, AuthorityGuard>;
impl<R> ObservedStartupOwner<R> {
    fn cancel(self, cause: LossCause) -> RuntimeActivationFailure<R> {
        self.retained.fail(|guard| guard.cancelled(cause))
    }
}
impl ServiceHost {
    #[expect(
        dead_code,
        reason = "Native preparation is Unsupported and transport wiring unaccepted; remove when the verified pre-unseal caller exists"
    )]
    fn prepare_observed(
        configuration: Configuration,
        preparation: VerifiedObserverPreparation,
    ) -> Result<ObservedStartupOwner<Configuration>, RuntimeActivationFailure<Configuration>> {
        // Attach the exact boxed resource owner before validation or future
        // unseal/bind work. There is no public parent flag or substitute proof.
        let owner = ObservedStartupOwner {
            retained: Retained::new(
                configuration,
                AuthorityGuard(Some(AuthorityPhase::Preparation(preparation))),
            ),
        };
        if owner.retained.resources().validate().is_err() {
            return Err(owner.cancel(LossCause::StartupCancelled));
        }
        Ok(owner)
    }
}
