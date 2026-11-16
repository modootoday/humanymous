//! Loopback application/session owner; component support is not product qualification.
pub mod application;
pub mod ban;
pub mod browser;
pub mod configuration;
pub mod connection_cap;
pub mod control;
pub mod credential;
pub mod guard;
pub mod http;
#[cfg(feature = "hosted-pilot")]
pub mod lab_h2_measure;
pub mod logging;
pub mod management;
pub mod pass_store;
pub mod service;
pub mod session;
pub mod settings;
pub mod shipping;
pub mod startup;
pub mod tls;
pub mod token;

pub use crate::authority::Rejection;

#[cfg(test)]
mod tests;
