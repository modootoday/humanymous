// Translated from the frozen Go wire contract by scripts/migration/contracts.
use crate::observation::Signal;

/// NetworkReport is the server-captured portion (L5).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct NetworkReport {
    /// Wire field `ja3`.
    #[serde(rename = "ja3", deserialize_with = "crate::wire::decode::null_default")]
    pub ja3: String,
    /// Wire field `ja3Text`.
    #[serde(
        rename = "ja3Text",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub ja3_text: String,
    /// Wire field `ja4`.
    #[serde(rename = "ja4", deserialize_with = "crate::wire::decode::null_default")]
    pub ja4: String,
    /// Wire field `ja4Raw`.
    #[serde(
        rename = "ja4Raw",
        deserialize_with = "crate::wire::decode::null_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub ja4_raw: String,
    /// JA4Engine / H2Engine are the browser-engine families the network layer
    /// resolved from the fingerprints ("chrome"|"firefox"|"safari"|"go"|
    /// "openssl"|"unknown"). Used by the L6 cross-check (SoT-02 §L6).
    #[serde(
        rename = "ja4Engine",
        deserialize_with = "crate::wire::decode::null_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub ja4_engine: String,
    /// Wire field `h2Engine`.
    #[serde(
        rename = "h2Engine",
        deserialize_with = "crate::wire::decode::null_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub h2_engine: String,
    /// Wire field `akamaiH2`.
    #[serde(
        rename = "akamaiH2",
        deserialize_with = "crate::wire::decode::null_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub akamai_h2: String,
    /// SecFetchPresent / SecCHUAPresent record header presence for cross-checks.
    #[serde(
        rename = "secFetchPresent",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub sec_fetch_present: bool,
    /// Wire field `secChUaPresent`.
    #[serde(
        rename = "secChUaPresent",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub sec_ch_ua_present: bool,
    /// Wire field `h2Settings`.
    #[serde(
        rename = "h2Settings",
        deserialize_with = "crate::wire::decode::nullable_map",
        skip_serializing_if = "crate::wire::decode::optional_map_empty"
    )]
    pub h2_settings: Option<std::collections::BTreeMap<String, u32>>,
    /// Wire field `pseudoHeaderOrder`.
    #[serde(
        rename = "pseudoHeaderOrder",
        deserialize_with = "crate::wire::decode::nullable_vec",
        skip_serializing_if = "crate::wire::decode::optional_vec_empty"
    )]
    pub pseudo_order: Option<Vec<String>>,
    /// Wire field `headerOrder`.
    #[serde(
        rename = "headerOrder",
        deserialize_with = "crate::wire::decode::nullable_vec",
        skip_serializing_if = "crate::wire::decode::optional_vec_empty"
    )]
    pub header_order: Option<Vec<String>>,
    /// Wire field `tlsVersion`.
    #[serde(
        rename = "tlsVersion",
        deserialize_with = "crate::wire::decode::null_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub tls_version: String,
    /// Wire field `negotiatedAlpn`.
    #[serde(
        rename = "negotiatedAlpn",
        deserialize_with = "crate::wire::decode::null_default",
        skip_serializing_if = "String::is_empty"
    )]
    pub negotiated_alpn: String,
    /// Wire field `signals`.
    #[serde(
        rename = "signals",
        deserialize_with = "crate::wire::decode::nullable_vec"
    )]
    pub signals: Option<Vec<Signal>>,
}
