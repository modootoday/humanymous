//! Retention classification and observer-side liveness. Neither module deletes
//! storage or authenticates network messages on behalf of its caller.
mod registry;
mod retention;
pub use registry::{NodeRegistry, NodeStatus, RegistryError};
pub use retention::{RetentionPolicy, Tier};
