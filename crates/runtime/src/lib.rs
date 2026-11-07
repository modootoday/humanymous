//! Experimental loopback application/session owner. No browser or TLS claims.
pub mod application;
pub mod browser;
pub mod configuration;
pub mod http;
pub mod service;
pub mod session;
pub mod startup;

pub use humanymous_authority::Rejection;

#[cfg(test)]
mod tests;
