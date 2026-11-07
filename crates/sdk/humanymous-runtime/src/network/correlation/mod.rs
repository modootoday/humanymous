//! Local correlation observations and unqualified coordinator semantics only.
mod config;
mod registry;
mod shared;
mod velocity;
use crate::network::history::Unavailable;
pub use config::{Config, Limits};
pub use registry::{Outcome, Registry, Stats};
pub use shared::{
    AtomicCoordinator, CoordinatorRequest, CoordinatorUnavailable, FleetInput, Scope, key_digest,
};

#[cfg(test)]
mod tests;
