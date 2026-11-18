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
mod dns_canary;
mod driver;
mod gate;
mod health;
mod origin;
mod report;
mod witness;

pub use dns_canary::probe as dns_probe;
pub use dns_canary::run as dns_canary;
pub use driver::origin_lost as driver_origin_lost;
pub use driver::run as driver;
pub use driver::witnessed as driver_witnessed;
pub use gate::human_test as gate_human_test;
pub use gate::run as gate;
pub use gate::witnessed as gate_witnessed;
pub use health::run as health;
pub use origin::run as origin;
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

pub fn address(text: &str) -> SocketAddr {
    text.parse().expect("fixed loopback address")
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

/// In-image copy of the legacy web pages and the static assets they
/// reference (build/rust/product-e2e.Dockerfile, final stage).
pub const HUMAN_TEST_WEB_DIR: &str = "/usr/local/share/humanymous/web";

/// Human tester hosting profile (deployments/lab/compose.human-test.yaml):
/// the same private document plus the explicit legacy section that mounts
/// `/pass`, `/api/pass/*`, `/demo` and `/static/*` from `web_dir`. The
/// protected routes above never collide with those paths (the gateway refuses
/// collisions). Product e2e never uses this; its document stays unchanged.
pub fn human_test_configuration(mut base: Value, web_dir: &str) -> Value {
    base["legacy"] = json!({"web_dir":web_dir,"pass":true,"demo":true,"static_assets":true});
    base
}

#[cfg(test)]
mod tests {
    use humanymous_core::scoring::engine::Engine;
    use humanymous_runtime::gateway::{configuration::Configuration, legacy::Surfaces};
    use std::path::Path;

    fn base() -> serde_json::Value {
        super::configuration("/audit", "/keys", "passphrase")
    }

    /// Product e2e evidence depends on this exact document.
    #[test]
    fn product_gate_document_is_unchanged() {
        assert_eq!(
            serde_json::to_string(&base()).unwrap(),
            concat!(
                r#"{"audit":{"allow_new":true,"directory":"/audit","keys":{"directory":"/keys","passphrase":"passphrase"},"node_id":"product-e2e-gate","trusted_head":null},"#,
                r#""listen":"127.0.0.1:18443","origin":"127.0.0.1:18080","#,
                r#""owner":{"kind":"unix","uid":65532},"rate_per_second":1000,"#,
                r#""routes":[{"method":"POST","path":"/effect","required":{"collector":false,"original_h2":false,"raw_header_order":true,"raw_tls":false,"rit":true}},"#,
                r#"{"method":"POST","path":"/tls","required":{"collector":false,"original_h2":false,"raw_header_order":true,"raw_tls":true,"rit":true}},"#,
                r#"{"method":"POST","path":"/collector","required":{"collector":true,"original_h2":false,"raw_header_order":true,"raw_tls":false,"rit":true}}],"#,
                r#""scope":{"authority":"owned.test","installation":"product-e2e","origin":"origin","revision":"r1","tenant":"lab"},"#,
                r#""session_capacity":64,"session_ttl_ms":60000,"timeout_ms":5000}"#
            )
        );
        assert!(base().get("legacy").is_none());
    }

    #[test]
    fn human_test_document_enables_every_legacy_surface() {
        let config = super::human_test_configuration(base(), super::HUMAN_TEST_WEB_DIR);
        let legacy = &config["legacy"];
        assert_eq!(legacy["pass"], true);
        assert_eq!(legacy["demo"], true);
        assert_eq!(legacy["static_assets"], true);
        assert!(Path::new(legacy["web_dir"].as_str().unwrap()).is_absolute());
        // Everything except the legacy section is the product e2e document.
        let mut stripped = config.clone();
        stripped.as_object_mut().unwrap().remove("legacy");
        assert_eq!(stripped, base());
        let parsed: Configuration = serde_json::from_value(config).unwrap();
        assert!(
            parsed.validate().is_ok(),
            "legacy paths collide with routes"
        );
    }

    /// The gateway legacy router mounts the surfaces from the repository's
    /// web directory, the source of the in-image copy.
    #[test]
    fn human_test_document_mounts_the_pass_surfaces() {
        let web = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../web");
        let web = web.canonicalize().unwrap();
        let config = super::human_test_configuration(base(), web.to_str().unwrap());
        let parsed: Configuration = serde_json::from_value(config).unwrap();
        let policy = parsed.legacy.as_ref().expect("legacy section");
        let legacy = Surfaces::new(policy, &Engine::default(), parsed.session_capacity)
            .unwrap_or_else(|_| panic!("legacy surfaces refused"));
        for path in [
            "/pass",
            "/api/pass/new",
            "/api/pass/solve",
            "/demo",
            "/static/js/demo.js",
        ] {
            assert!(legacy.mounted(path), "{path}");
        }
        for path in [
            "/effect",
            "/tls",
            "/collector",
            "/api/collect",
            "/api/report/x",
        ] {
            assert!(!legacy.mounted(path), "{path}");
        }
    }
}
