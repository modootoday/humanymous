//! Roadmap 2: the Rust attacker network catalog against the shipped runtime
//! binary across internal Docker networks.
//!
//! Three actors share the runtime image and run in separate containers:
//!
//! - `ac-origin`: the owned recording origin (`--origin-audit-origin`) on the
//!   backend network only; every TCP contact is logged with its peer.
//! - `ac-gate`: a supervisor that provisions sealed custody once and then runs
//!   one session of the shipped `humanymous-runtime` binary per catalog case,
//!   on the runner's command. As in the origin/audit stage, two plain TCP
//!   relays bridge the loopback-only gateway to ingress and to the origin.
//!   After every session it authenticates the persisted history offline and
//!   reports the projected records.
//! - `ac-attacker`: ingress only. It runs the `humanymous-attacker` network
//!   catalog through the Gate ingress with the catalog's own step logic and
//!   expectations; the runner starts, restarts and stops Gate sessions on its
//!   request and returns the per-case audit and origin effects.
//!
//! Go parity (981d25c): `deployments/compose/bots.yaml` runs the catalog from a
//! container on the internal `lab-a` network only, against the Gate/Core on
//! that network; `scripts/assert-attack.mjs` fails on any bot ALLOW, a DENY of
//! the human baseline, any errored or skipped profile and an incomplete
//! catalog.
pub mod client;
pub mod gate;

use serde_json::{Value, json};

/// Runtime listener inside the Gate namespace (loopback only).
pub const GATE_LISTEN: &str = "127.0.0.1:18443";
/// Runtime origin address inside the Gate namespace: the backend relay.
pub const ORIGIN_LOOPBACK: &str = "127.0.0.1:18080";
/// Origin service name and port on the backend network.
pub const ORIGIN_UPSTREAM: &str = "ac-origin:18080";
/// Gate ingress relay port on the ingress network.
pub const FRONTEND_PORT: u16 = 8080;
/// Audit node identity.
pub const NODE: &str = "attack-catalog-gate";
/// Attacker report schema.
pub const REPORT_SCHEMA: &str = "humanymous.lab-attack-catalog/1";
/// Catalog ordering seed (the owned-child wargame uses the same seed).
pub const SEED: u64 = 42;

/// The Gate's private configuration document: the same routes as the
/// owned-child wargame installation, so every catalog expectation applies
/// unchanged. The attacker builds it with placeholder custody fields, because
/// request signing binds only the trusted scope and routes.
pub fn document(
    audit: &str,
    keys: &str,
    passphrase: &str,
    allow_new: bool,
    trusted_head: Value,
    uid: u32,
) -> Value {
    let required = |raw_tls, original_h2, collector| json!({"raw_tls":raw_tls,"original_h2":original_h2,"raw_header_order":true,"collector":collector,"rit":true});
    json!({"listen":GATE_LISTEN,"origin":ORIGIN_LOOPBACK,
      "scope":{"installation":"attack-catalog","tenant":"lab","origin":"origin","revision":"r1","authority":"owned.test"},
      "routes":[{"method":"POST","path":"/effect","required":required(false,false,false)},
                {"method":"POST","path":"/tls","required":required(true,false,false)},
                {"method":"POST","path":"/h2","required":required(false,true,false)},
                {"method":"POST","path":"/collector","required":required(false,false,true)}],
      "audit":{"directory":audit,"node_id":NODE,"keys":{"directory":keys,"passphrase":passphrase},"allow_new":allow_new,"trusted_head":trusted_head},
      "owner":{"kind":"unix","uid":uid},"session_capacity":16,"session_ttl_ms":60000,"timeout_ms":5000,"rate_per_second":1000})
}

#[cfg(test)]
mod tests {
    use super::*;
    use humanymous_runtime::gateway::configuration::Configuration;

    #[test]
    fn document_is_a_valid_loopback_gateway_configuration() {
        let parsed: Configuration =
            serde_json::from_value(document("/a", "/k", "p", true, Value::Null, 65532)).unwrap();
        assert!(parsed.validate().is_ok());
        assert_eq!(parsed.listen.to_string(), GATE_LISTEN);
        assert_eq!(parsed.origin.to_string(), ORIGIN_LOOPBACK);
        assert_eq!(parsed.routes.len(), 4);
        assert!(ORIGIN_UPSTREAM.ends_with(":18080"));
    }

    /// The routes are the owned-child wargame installation's routes.
    #[test]
    fn routes_match_the_owned_child_installation() {
        let source = include_str!("installation.rs");
        for (path, required) in [
            ("/effect", "required(false,false,false)"),
            ("/tls", "required(true,false,false)"),
            ("/h2", "required(false,true,false)"),
            ("/collector", "required(false,false,true)"),
        ] {
            assert!(
                source.contains(&format!(
                    "{{\"method\":\"POST\",\"path\":\"{path}\",\"required\":{required}}}"
                )),
                "{path}"
            );
        }
    }
}
