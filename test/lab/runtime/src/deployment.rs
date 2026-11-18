//! Roadmap 4 items 2h/2i: the shipped compose manifest brings up the runtime.
//!
//! `deployments/compose.release.yaml` (the Go-parity adopter drop-in) is run by
//! the runner with `deployments/lab/compose.deployment.yaml` on top. These
//! actors fill only what the manifest leaves to the adopter:
//!
//! - `edge`: the manifest's placeholder TLS front proxy. A plain L4 relay from
//!   `0.0.0.0:8443` to the runtime's loopback listener and from the runtime's
//!   loopback origin address to the owned origin on `origin-net`, exactly the
//!   contract the manifest comments state (TLS termination is the adopter's).
//! - `custody`: the documented step 3 ("provision sealed custody on the
//!   hmn-data volume"), which has no shipped Rust command; also the attested
//!   installation directories and the offline audit read-back.
//! - `client`: an ingress-only HTTP/1.1 client for the documented routes.
//! - `challenger`: a network-none verifier that pins the enrollment key the
//!   deployed installation generated and issues fresh nonces.
pub mod challenger;
pub mod client;
pub mod custody;
pub mod edge;

/// Volume mount of the manifest's `hmn-data` volume.
pub const STATE: &str = "/var/lib/humanymous";
/// Attested installation directory inside the deployed volume.
pub const INSTALLATION: &str = "/var/lib/humanymous/installation";
/// A second, separately keyed installation used as an unknown target.
pub const FOREIGN: &str = "/var/lib/humanymous/foreign-installation";
/// The edge's ingress port (the manifest publishes it as 443).
pub const EDGE_PORT: u16 = 8443;
/// Runtime listener from the documented example configuration.
pub const RUNTIME_LISTEN: &str = "127.0.0.1:8444";
/// Runtime origin address from the documented example configuration.
pub const ORIGIN_LOOPBACK: &str = "127.0.0.1:8080";
/// Owned origin on the manifest's `origin-net`.
pub const ORIGIN_UPSTREAM: &str = "dep-origin:18080";
/// Owned origin port (the same recording origin as the origin/audit stage).
pub const ORIGIN_PORT: u16 = 18080;
/// Client report schema.
pub const CLIENT_SCHEMA: &str = "humanymous.deployment-client/1";
/// Challenger report schema.
pub const CHALLENGER_SCHEMA: &str = "humanymous.deployment-attestation/1";

pub(crate) use crate::origin_audit::{emit, read_line};

#[cfg(test)]
mod tests {
    use super::*;

    // The example file is outside the Docker build context; the runner's tests
    // bind these constants to the shipped example bytes.
    #[test]
    fn edge_and_installation_paths_stay_inside_the_manifest_layout() {
        for path in [INSTALLATION, FOREIGN] {
            assert!(path.starts_with(&format!("{STATE}/")));
        }
        assert!(ORIGIN_UPSTREAM.ends_with(&format!(":{ORIGIN_PORT}")));
        for address in [RUNTIME_LISTEN, ORIGIN_LOOPBACK] {
            let parsed: std::net::SocketAddr = address.parse().unwrap();
            assert!(parsed.ip().is_loopback());
        }
    }
}
