//! Same allocation survives failure. Guard destruction precedes resource release.
pub(super) struct Retained<R, G> {
    // Rust drops fields in declaration order. A production guard must revoke
    // synchronously here before custody/configuration/host resources release.
    guard: G,
    resources: Box<R>,
}
pub(super) struct RetainedFailure<R, G> {
    retained: Retained<R, G>,
}
impl<R, G> Retained<R, G> {
    pub(super) fn new(resources: R, guard: G) -> Self {
        Self {
            guard,
            resources: Box::new(resources),
        }
    }
    pub(super) fn resources(&self) -> &R {
        &self.resources
    }
    // Scoped to the retained owner's implementation. No SDK caller can replace
    // a guard, extract a resource or construct native authority through this.
    pub(super) fn guard_mut(&mut self) -> &mut G {
        &mut self.guard
    }
    // A private consuming guard transition, not a proof, effect or Ready factory.
    pub(super) fn fail(self, cancel: impl FnOnce(G) -> G) -> RetainedFailure<R, G> {
        RetainedFailure {
            retained: Self {
                guard: cancel(self.guard),
                resources: self.resources,
            },
        }
    }
}
impl<R, G> RetainedFailure<R, G> {
    pub(super) fn guard(&self) -> &G {
        &self.retained.guard
    }
    pub(super) fn guard_mut(&mut self) -> &mut G {
        &mut self.retained.guard
    }
    // Error inspection does not extract either resource or guard and cannot
    // report cleanup completion. The concrete bounded cleanup API is pending.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Native preparation and bounded cleanup wiring are unavailable; remove when retained failure inspection is wired"
        )
    )]
    pub(super) fn resources(&self) -> &R {
        self.retained.resources()
    }
}

#[cfg(test)]
mod tests;
