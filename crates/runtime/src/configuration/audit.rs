use serde::Deserialize;
use std::path::PathBuf;
use zeroize::Zeroizing;

/// Audit metadata and an explicit preprovisioned sealed key source.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Audit {
    /// Private durable WAL directory.
    pub directory: PathBuf,
    /// Stable writer node identifier.
    pub node_id: String,
    /// Existing sealed custody; never initialized by runtime startup.
    pub keys: SealedKeys,
    /// Permission to initialize audit history, not key material.
    pub allow_new: bool,
    /// Independently retained anchor, when provisioned.
    pub trusted_head: Option<humanymous_audit::checkpoint::Checkpoint>,
}
/// Secret startup source; neither Debug nor Serialize is implemented.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedKeys {
    /// Existing private custody directory.
    pub directory: PathBuf,
    /// Private inherited passphrase, consumed before listening.
    pub passphrase: Zeroizing<String>,
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn inline_keys_and_unknown_sources_are_rejected() {
        let mut value = json!({"directory":"private-audit","node_id":"node","keys":{"directory":"private-keys","passphrase":"fixture"},"allow_new":true,"trusted_head":null});
        assert!(serde_json::from_value::<Audit>(value.clone()).is_ok());
        value["hmac_key_hex"] = json!("11");
        assert!(serde_json::from_value::<Audit>(value.clone()).is_err());
        value.as_object_mut().unwrap().remove("hmac_key_hex");
        value["keys"]["allow_create"] = json!(true);
        assert!(serde_json::from_value::<Audit>(value).is_err());
    }
}
