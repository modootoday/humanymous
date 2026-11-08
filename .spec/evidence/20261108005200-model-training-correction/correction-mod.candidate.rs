//! REVIEW ONLY: Core model/correction/mod.rs. Offline sensing and ledger DATA.
//! Does not implement ThresholdProvider or call Runtime/model activation setters.
pub(crate) mod numeric;
pub(crate) mod features;
pub(crate) mod lifecycle;
pub(crate) mod gates;
pub(crate) mod controller;

pub(crate) const MAX_OBSERVATIONS: u64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    Config, Nonfinite, Capacity, Shape, Missing, Schema, Digest, Signature,
    Unsigned, NoStaged, NoPrevious, Generation, Expired, Authority, SamePrincipal,
    ClockRegression, Allocation, Time, Support,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "offline correction: {self:?}") }
}
impl std::error::Error for Error {}

pub(crate) fn probability(p: f32) -> Result<(), Error> {
    if p.is_finite() && (0.0..=1.0).contains(&p) { Ok(()) } else { Err(Error::Nonfinite) }
}
pub(crate) fn increment(n: u64) -> Result<u64, Error> {
    let next = n.checked_add(1).ok_or(Error::Capacity)?;
    if next > MAX_OBSERVATIONS { Err(Error::Capacity) } else { Ok(next) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome { Unknown, PassSolved, ChallengeFailed, ChallengeAbandoned, CatalogBot }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Label { Ambiguous, Human, Bot }

/// Source compatibility diagnostic ONLY; not label authentication/admission.
pub(crate) fn legacy_label(outcome: Outcome) -> Label {
    match outcome {
        Outcome::PassSolved => Label::Human,
        Outcome::CatalogBot | Outcome::ChallengeFailed | Outcome::ChallengeAbandoned => Label::Bot,
        Outcome::Unknown => Label::Ambiguous,
    }
}
/// Deliberate weak-label quarantine: challenge failure/abandonment never auto-train.
pub(crate) fn reviewed_label(outcome: Outcome) -> Label {
    match outcome {
        Outcome::PassSolved => Label::Human,
        Outcome::CatalogBot => Label::Bot,
        _ => Label::Ambiguous,
    }
}

#[cfg(test)]
mod tests;
