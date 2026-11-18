//! Loopback application/session owner; component support is not product qualification.
pub mod admin_approval;
pub mod application;
pub mod attest_floor;
pub mod attestation;
pub mod ban;
pub mod browser;
pub mod configuration;
pub mod connection_cap;
pub mod control;
pub mod coordinator;
pub mod credential;
pub mod external_input;
pub mod guard;
pub mod http;
#[cfg(feature = "hosted-pilot")]
pub mod lab_h2_measure;
pub mod logging;
pub mod management;
pub mod model_admission;
pub mod observatory;
pub mod operations;
pub mod pass_proof;
pub mod pass_store;
pub mod request;
pub mod seams;
pub mod service;
pub mod session;
pub mod settings;
pub mod shipping;
pub mod smuggle;
pub mod startup;
pub mod tls;
pub mod token;
pub mod verdict_store;
pub mod virtual_usb;

pub use crate::authority::Rejection;

#[cfg(test)]
mod tests;
