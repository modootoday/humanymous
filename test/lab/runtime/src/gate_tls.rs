//! Roadmap 4 item 2a: full Gate TLS/H2/capture across the Docker topology.
//!
//! The same three-container shape as the attack catalog (`tg-origin` on
//! `backend`, `tg-gate` on `ingress`+`backend`, `tg-client` on `ingress`), but
//! the shipped `humanymous-runtime` binary terminates TLS itself:
//!
//! - `tg-gate` runs the attack-catalog supervisor in TLS mode
//!   (`--gate-tls`). It generates an ephemeral lab CA and a server certificate
//!   for the installation authority in its private tmpfs, and starts each
//!   runtime session with the closed launch envelope
//!   [`TLS_INGRESS_SCHEMA`] and that provisioned pair (B02 provisioned
//!   source; no self-signed fallback). The ingress relay is plain L4 TCP
//!   passthrough, so the runtime observes the original client's ClientHello,
//!   ALPN, H1 header order and H2 frames.
//! - `tg-client` trusts only the lab CA and checks that every handshake
//!   presented the certificate the Gate generated.
//!
//! Go parity (981d25c): `cmd/gate/tls.go` operator certificate edge
//! (`-tls-cert/-tls-key`), `cmd/server/tlsconn.go` ClientHello capture before
//! the handshake consumes it, `h1capture.go` raw H1 header order and
//! `h2capture.go` preface/SETTINGS/HEADERS capture; `test/gate/e2e.mjs` edge
//! enforcement (bot DENY at the edge with the origin untouched, smuggling,
//! forged forwarding, unsigned/replayed requests, origin direct hit).
pub mod client;
#[cfg(test)]
mod loopback;
pub mod pki;

use serde_json::{Value, json};
use std::path::Path;

pub use humanymous_runtime::gateway::service::TLS_INGRESS_SCHEMA;

/// Origin service name and port on the backend network.
pub const ORIGIN_UPSTREAM: &str = "tg-origin:18080";
/// Origin port, probed directly (and refused by topology) from ingress.
pub const ORIGIN_PORT: u16 = 18080;
/// Gate TLS ingress relay port on the ingress network.
pub const FRONTEND_PORT: u16 = 8443;
/// The installation authority, the certificate name and the H2 `:authority`.
pub const SERVER_NAME: &str = "owned.test";
/// Client report schema.
pub const REPORT_SCHEMA: &str = "humanymous.lab-gate-tls/1";

/// Attack-catalog rows executed through the TLS edge with their unchanged
/// catalog expectations (`gate` case).
pub const GATE_CATALOG: &[&str] = &[
    "synthetic-selenium-deny",
    "rit-body-mutation",
    "rit-query-target-mutation",
    "rit-header-order-mutation",
    "rit-expired-deadline",
    "rit-missing-token",
    "missing-session",
    "forwarded-local-admin-forgery",
    "required-collector-refusal",
    "restart-stale-session",
    "method-route-mutation",
];
/// TLS edge rows with explicit expectations (`gate` case).
///
/// The catalog's `duplicate-content-length` expects a plain-HTTP 400; over the
/// runtime's TLS ingress the same ambiguous framing closes the connection
/// without any HTTP response, so it runs here with that measured expectation.
pub const GATE_EDGE: &[&str] = &[
    "tls-edge-rustls-native-challenged",
    "tls-edge-replay-refused",
    "tls-edge-ambiguous-framing-refused",
];
/// Original-hop capture rows (`raw-tls-h2` case).
pub const CAPTURE: &[&str] = &[
    "tls-h1-required-raw-tls",
    "tls-h2-required-original-h2",
    "tls-h1-required-original-h2-refused",
    "tls-chrome-claim-rustls-hello",
    "plaintext-on-tls-ingress-refused",
];

/// The closed launch envelope around an unchanged v1 document.
pub fn envelope(document: Value, chain: &Path, key: &Path) -> Value {
    json!({"schema":TLS_INGRESS_SCHEMA,"configuration":document,
        "certificate":{"chain":chain,"key":key}})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_unique_and_catalog_rows_exist_in_the_catalog() {
        let mut all: Vec<&str> = GATE_CATALOG
            .iter()
            .chain(GATE_EDGE)
            .chain(CAPTURE)
            .copied()
            .collect();
        let count = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), count);
        let ids: Vec<&str> = humanymous_attacker::network::entries(1)
            .iter()
            .map(|entry| entry.id)
            .collect();
        for id in GATE_CATALOG {
            assert!(ids.contains(id), "{id}");
        }
        for id in GATE_EDGE.iter().chain(CAPTURE) {
            assert!(!ids.contains(id), "{id}");
        }
    }

    #[test]
    fn envelope_wraps_the_attack_catalog_document_without_changing_it() {
        let document = crate::attack_catalog::document("/a", "/k", "p", true, Value::Null, 1);
        let wrapped = envelope(
            document.clone(),
            Path::new("/t/chain.pem"),
            Path::new("/t/key.pem"),
        );
        assert_eq!(wrapped["schema"], TLS_INGRESS_SCHEMA);
        assert_eq!(wrapped["configuration"], document);
        assert_eq!(wrapped["certificate"]["key"], "/t/key.pem");
        assert_eq!(wrapped.as_object().unwrap().len(), 3);
    }
}
