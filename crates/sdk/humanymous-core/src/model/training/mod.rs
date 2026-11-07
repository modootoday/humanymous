//! Offline training inputs; no serving or model activation authority.
pub mod evaluate;
pub mod record;
pub mod rng;
// The offline composition root is a separately allocated migration slice.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "Private offline SGD awaits its bootstrap caller")
)]
pub(crate) mod sgd;
pub mod standardize;
