//! Roadmap unit 35: separate Gate, origin and attacker containers driven
//! through the real `humanymous-runtime` gateway, plus (increment 2) a
//! witness-required Gate whose audit checkpoints are counter-signed by an
//! independent witness container, shell-free health probes and
//! dependency-loss-after-readiness drivers.
//!
//! The gateway validates loopback-only listen and origin addresses and refuses
//! non-loopback peers, so origin and attacker share the Gate container's
//! network namespace on an internal Docker network. Authority is absent.
//!
//! Increment 4: an owned DNS canary for the browser topology, and drivers
//! whose collect reports are the bytes the real Rust WASM collector posted
//! from Firefox in the browser topology (supplied by the runner on stdin),
//! never a hand-written report.
//!
//! TLS profile: the shipped gateway binary with a `tls` section whose
//! certificate is issued by a per-run lab CA minted inside its container
//! (`lab_ca`), driven over real TLS (HTTP/1.1 and H2 via ALPN), raw owned
//! ClientHello parrots and malformed H2 frames (`tls_driver`). The client still
//! shares the Gate's loopback namespace, so this is not original-client
//! cross-namespace evidence.
mod dns_canary;
mod driver;
mod gate;
mod health;
mod lab_ca;
mod origin;
mod report;
mod tls_driver;
mod witness;

pub use dns_canary::probe as dns_probe;
pub use dns_canary::run as dns_canary;
pub use driver::origin_lost as driver_origin_lost;
pub use driver::run as driver;
pub use driver::witnessed as driver_witnessed;
pub use gate::run as gate;
pub use gate::tls as gate_tls;
pub use gate::witnessed as gate_witnessed;
pub use health::run as health;
pub use origin::run as origin;
pub use origin::run_public_observe as origin_public_observe;
pub use tls_driver::run as driver_tls;
pub use witness::run as witness;

use serde_json::{Value, json};
use std::net::SocketAddr;

/// Gate ingress inside the shared namespace.
pub const GATE: &str = "127.0.0.1:18443";
/// Owned application origin; the only address the Gate forwards to.
pub const ORIGIN: &str = "127.0.0.1:18080";
/// Lab-only origin observation endpoint; never configured as a Gate origin.
pub const OBSERVE: &str = "127.0.0.1:18081";
/// Witnessed Gate readiness marker on its private tmpfs. Present only while
/// the runtime's serving-epoch latch reports serving.
pub const SERVING_MARKER: &str = "/tmp/product-e2e-gate.serving";

/// Public TLS edge of profile `tls` (owner decision B01 #1): the shipped
/// gateway binds every interface of the Gate container with `public_listen`.
pub const PUBLIC_GATE_LISTEN: &str = "0.0.0.0:18443";
/// The TLS Gate as the driver reaches it from its own network namespace.
pub const TLS_GATE: &str = "tgate:18443";
/// Origin observation endpoint of profile `tls`, bound beside the Gate so the
/// driver can read the origin's records across namespaces. The application
/// origin itself stays loopback-only ([`ORIGIN`]).
pub const PUBLIC_OBSERVE_LISTEN: &str = "0.0.0.0:18081";
/// That observation endpoint as the driver reaches it.
pub const TLS_OBSERVE: &str = "tgate:18081";

/// One socket address: a literal as given, a compose service name through
/// the container resolver (first address).
pub fn address(text: &str) -> SocketAddr {
    text.parse().unwrap_or_else(|_| {
        std::net::ToSocketAddrs::to_socket_addrs(text)
            .ok()
            .and_then(|mut all| all.next())
            .unwrap_or_else(|| panic!("{text} does not resolve"))
    })
}

/// The one private configuration document. The driver uses the same scope and
/// routes with placeholder custody fields, because request signing binds only
/// the trusted scope, never the audit source.
pub fn configuration(audit: &str, keys: &str, passphrase: &str) -> Value {
    let required = |raw_tls: bool, collector: bool| json!({"raw_tls":raw_tls,"original_h2":false,"raw_header_order":true,"collector":collector,"rit":true});
    json!({"listen":GATE,"origin":ORIGIN,
      "scope":{"installation":"product-e2e","tenant":"lab","origin":"origin","revision":"r1","authority":"owned.test"},
      "routes":[{"method":"POST","path":"/effect","required":required(false,false)},
                {"method":"POST","path":"/tls","required":required(true,false)},
                {"method":"POST","path":"/collector","required":required(false,true)}],
      "audit":{"directory":audit,"node_id":"product-e2e-gate","keys":{"directory":keys,"passphrase":passphrase},"allow_new":true,"trusted_head":null},
      "owner":{"kind":"unix","uid":65532},"session_capacity":64,"session_ttl_ms":60000,"timeout_ms":5000,"rate_per_second":1000})
}

/// Route that requires the R14 same-owner original H2 observation; only the
/// TLS Gate configures it.
pub const H2_ROUTE: &str = "/h2";

/// The TLS Gate's private configuration: the shared document plus a `tls`
/// section naming the provisioned lab chain and key, and one route requiring
/// the original H2 observation. The driver derives its signing scope from the
/// same function with placeholder paths.
pub fn tls_configuration(
    audit: &str,
    keys: &str,
    passphrase: &str,
    chain: &str,
    key: &str,
) -> Value {
    let mut config = configuration(audit, keys, passphrase);
    config["listen"] = json!(PUBLIC_GATE_LISTEN);
    config["public_listen"] = json!(true);
    config["tls"] = json!({"certificate":chain,"key":key});
    if let Some(routes) = config["routes"].as_array_mut() {
        routes.push(json!({"method":"POST","path":H2_ROUTE,"required":
            {"raw_tls":false,"original_h2":true,"raw_header_order":true,"collector":false,"rit":true}}));
    }
    config
}
