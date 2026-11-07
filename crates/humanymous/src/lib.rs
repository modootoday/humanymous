//! Public Rust library facade. Optional adapters retain their owning contracts.
//! Namespace availability is not host, native-provider or product qualification.

pub mod build_profile;

/// Explicitly enabled adapter contexts; no automatic host initialization.
#[cfg(feature = "adapter-go")]
pub mod adapters {
    /// Rust Gate deployment contracts for external Go HTTP origins.
    pub use humanymous_go_adapter as go;
}
