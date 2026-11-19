//! One full manifest schema; expected configuration is not runtime attestation.
use crate::gateway::configuration::Configuration;
use serde_json::{Value, json};

/// Expected full profile for the inherited-pipe standalone executable. Consumers
/// must still validate actual process, listener, source and complete identity.
pub fn standalone_profile(config: &Configuration) -> Value {
    profile(config, None, None, false, None)
}
/// TLS listener readiness from the ingress configuration actually served.
/// `original_h2` is true exactly when the server offers `h2` by ALPN: the
/// ingress then serves R14 H2 and a route requiring `original_h2` is admitted
/// over an H2 connection (issue #153 finding: readiness reported false while
/// the binary negotiated h2). The capability gate itself is unchanged: an
/// H1 request to such a route still answers 422 `required_capability`.
///
/// The loopback profile keeps its stable name `experimental-loopback-tls-h1`
/// for existing consumers; `alpn` lists the protocols actually offered.
pub(super) fn tls_listener(
    profile: &mut Value,
    config: &Configuration,
    tls: &rustls::ServerConfig,
) {
    let offers = |protocol: &[u8]| tls.alpn_protocols.iter().any(|p| p == protocol);
    let h2 = offers(b"h2");
    profile["profile"] = json!(if config.public_listen {
        "experimental-public-tls"
    } else {
        "experimental-loopback-tls-h1"
    });
    if config.public_listen {
        // Concurrent receive bounds of the public listener (PR #156 review).
        profile["public_receive"] = json!({
            "max_receiving": config.public_receive.max_receiving,
            "receive_ms": config.public_receive.receive_ms.min(config.timeout_ms),
        });
    }
    profile["raw_tls"] = json!(true);
    profile["original_h2"] = json!(h2);
    profile["alpn"] = json!(
        tls.alpn_protocols
            .iter()
            .map(|protocol| String::from_utf8_lossy(protocol).into_owned())
            .collect::<Vec<_>>()
    );
    profile["routes"] = json!(
        config
            .routes
            .iter()
            .map(|route| json!({
                "method":route.method,"path":route.path,
                "ready":(h2 || !route.required.original_h2) && !route.required.collector,
                "rit_profile":route.rit_profile,"response_profile":route.response_profile
            }))
            .collect::<Vec<_>>()
    );
}
/// Startup IP reputation state. Live freshness, counters and shadow hits are
/// served by the management status route, not by this static manifest.
pub(super) fn ip_reputation(
    profile: &mut Value,
    status: &crate::network::intel::store::StoreStatus,
) {
    profile["ip_reputation"]["startup_state"] = json!(status.state);
    profile["ip_reputation"]["startup_datacenter_scoring_active"] =
        json!(status.datacenter_scoring_active);
    profile["ip_reputation"]["startup_reload_error"] = json!(status.last_reload_error);
    profile["ip_reputation"]["startup_degraded_reasons"] = json!(status.degraded_reasons);
}
pub(super) fn profile(
    config: &Configuration,
    witness: Option<&str>,
    recovery: Option<Value>,
    management: bool,
    settings_revision: Option<u64>,
) -> Value {
    let mut profile = config.readiness();
    profile["audit_external_witness"] = json!(witness.is_some());
    profile["audit_witness_required_durability"] = json!(witness);
    profile["audit_witness_independence_qualified"] = json!(false);
    profile["audit_startup_recovery"] = json!(recovery);
    profile["administration"] = json!(management);
    profile["administration_writable"] = json!(false);
    profile["operational_revision"] = json!(settings_revision);
    profile
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standalone_expected_profile_has_complete_owned_fact_defaults() {
        let config = crate::gateway::tests::config();
        let expected = standalone_profile(&config);
        assert_eq!(expected, profile(&config, None, None, false, None));
        assert_ne!(expected, config.readiness());
        assert_eq!(expected["audit_external_witness"], false);
        assert!(expected["audit_witness_required_durability"].is_null());
        assert!(expected["audit_startup_recovery"].is_null());
        assert!(expected["operational_revision"].is_null());
        let managed = profile(&config, Some("file_and_directory"), None, true, Some(1));
        assert_ne!(expected, managed);
        assert_eq!(managed["product_qualified"], false);
        assert_eq!(managed["audit_witness_independence_qualified"], false);
        let mut config = config;
        config.routes[0].required.raw_tls = true;
        let mut captured = standalone_profile(&config);
        assert_eq!(captured["routes"][0]["ready"], false);
        let h1 = server(&[b"http/1.1"]);
        tls_listener(&mut captured, &config, &h1);
        assert_eq!(captured["routes"][0]["ready"], true);
        assert_eq!(captured["raw_tls"], true);
        assert_eq!(captured["original_h2"], false);
        assert_eq!(captured["product_qualified"], false);
        config.routes[0].required.collector = true;
        tls_listener(&mut captured, &config, &h1);
        assert_eq!(captured["routes"][0]["ready"], false);
        config.routes[0].required.collector = false;
        config.routes[0].required.original_h2 = true;
        tls_listener(&mut captured, &config, &h1);
        assert_eq!(captured["routes"][0]["ready"], false);
    }

    fn server(alpn: &[&[u8]]) -> rustls::ServerConfig {
        let certified = rcgen::generate_simple_self_signed(vec!["owned.test".into()]).unwrap();
        let key =
            rustls::pki_types::PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
        let mut server = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![certified.cert.der().clone()], key.into())
            .unwrap();
        server.alpn_protocols = alpn.iter().map(|p| p.to_vec()).collect();
        server
    }

    /// Issue #153: an ingress that offers h2 serves R14 H2, so readiness
    /// reports `original_h2` and a route requiring it is ready; an H1-only
    /// ingress still reports neither. Gating semantics are unchanged.
    #[test]
    fn tls_readiness_reports_the_served_h2_offer() {
        let mut config = crate::gateway::tests::config();
        config.routes[0].required.original_h2 = true;
        let mut profile = standalone_profile(&config);
        assert_eq!(profile["original_h2"], false);
        tls_listener(&mut profile, &config, &server(&[b"h2", b"http/1.1"]));
        assert_eq!(profile["original_h2"], true);
        assert_eq!(profile["alpn"], json!(["h2", "http/1.1"]));
        assert_eq!(profile["routes"][0]["ready"], true);
        assert_eq!(profile["profile"], "experimental-loopback-tls-h1");
        tls_listener(&mut profile, &config, &server(&[b"h2"]));
        assert_eq!(profile["original_h2"], true);
        tls_listener(&mut profile, &config, &server(&[b"http/1.1"]));
        assert_eq!(profile["original_h2"], false);
        assert_eq!(profile["routes"][0]["ready"], false);
        // A collector requirement stays unmet over TLS either way.
        config.routes[0].required.collector = true;
        tls_listener(&mut profile, &config, &server(&[b"h2", b"http/1.1"]));
        assert_eq!(profile["routes"][0]["ready"], false);
        // A public edge has its own profile name.
        config.public_listen = true;
        tls_listener(&mut profile, &config, &server(&[b"h2", b"http/1.1"]));
        assert_eq!(profile["profile"], "experimental-public-tls");
    }
}
