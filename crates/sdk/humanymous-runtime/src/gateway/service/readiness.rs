//! One full manifest schema; expected configuration is not runtime attestation.
use crate::gateway::configuration::Configuration;
use serde_json::{Value, json};

/// Expected full profile for the inherited-pipe standalone executable. Consumers
/// must still validate actual process, listener, source and complete identity.
pub fn standalone_profile(config: &Configuration) -> Value {
    profile(config, None, None, false, None)
}
pub(super) fn tls_h1(profile: &mut Value, config: &Configuration) {
    profile["profile"] = json!("experimental-loopback-tls-h1");
    profile["raw_tls"] = json!(true);
    profile["routes"] = json!(
        config
            .routes
            .iter()
            .map(|route| json!({
                "method":route.method,"path":route.path,
                "ready":!route.required.original_h2 && !route.required.collector,
                "rit_profile":route.rit_profile,"response_profile":route.response_profile
            }))
            .collect::<Vec<_>>()
    );
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
        tls_h1(&mut captured, &config);
        assert_eq!(captured["routes"][0]["ready"], true);
        assert_eq!(captured["raw_tls"], true);
        assert_eq!(captured["product_qualified"], false);
        config.routes[0].required.collector = true;
        tls_h1(&mut captured, &config);
        assert_eq!(captured["routes"][0]["ready"], false);
        config.routes[0].required.collector = false;
        config.routes[0].required.original_h2 = true;
        tls_h1(&mut captured, &config);
        assert_eq!(captured["routes"][0]["ready"], false);
    }
}
