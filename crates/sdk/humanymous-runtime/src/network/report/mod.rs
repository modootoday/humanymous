//! Assemble captured protocol observations into the shared L5 report contract.
mod assembly;
mod claims;
use crate::network::{http::HeaderInfo, http2::Fingerprint, tcp, tls::ClientHello};
pub use assembly::build;

/// Actual capture and operator intelligence for one request/connection.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct Observation {
    /// Original ClientHello, absent when the TLS plane is not captured.
    pub hello: Option<ClientHello>,
    /// Captured HTTP/2 profile, absent when not observed.
    pub h2: Option<Fingerprint>,
    /// Header evidence with explicit provenance flags.
    pub header: HeaderInfo,
    /// Operator-supplied hosting/datacenter classification.
    pub is_datacenter_ip: bool,
    /// Operator-supplied proxy/VPN classification.
    pub is_proxy: bool,
    /// Operator-supplied Tor exit classification.
    pub is_tor_exit: bool,
    /// Client-asserted address, never trusted transport peer identity.
    pub client_forwarded_ip: String,
    /// Optional actual TCP capture.
    pub tcp: tcp::Observation,
}
