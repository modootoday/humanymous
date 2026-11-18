//! Offline training inputs; no serving or model activation authority.
pub mod evaluate;
pub mod gate;
pub mod ml_train;
pub mod mlcorrect_train;
pub mod record;
pub mod rng;
pub mod sink;
// Private offline SGD; reached through the promotion gate composition root.
pub(crate) mod sgd;
pub mod standardize;
