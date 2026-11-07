//! Loopback application/session owner; component support is not product qualification.
pub mod application;
pub mod browser;
pub mod configuration;
pub mod control;
pub mod http;
pub mod management;
pub mod service;
pub mod session;
pub mod settings;
pub mod shipping;
pub mod startup;
pub mod tls;

pub use crate::authority::Rejection;

#[cfg(test)]
mod tests;
