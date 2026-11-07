//! Experimental loopback application/session owner. No browser or TLS claims.
pub mod application;
pub mod configuration;
pub mod http;
pub mod session;
pub mod startup;

pub use humanymous_authority::Rejection;

#[cfg(test)]
mod tests;
