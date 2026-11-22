//! Front-of-path edge gate settings: the legacy `gate.ban` and
//! `gate.spoof_header` module modes (internal/gate gates.go at baseline
//! 981d25c), the opt-in HSTS field and the origin-cloak key
//! (`sot:root/runtime-application`, owner decision C10).
use crate::gateway::Rejection;
use crate::gateway::ban::gate::Mode;
use crate::gateway::guard::OriginKey;
use serde::Deserialize;
use zeroize::Zeroizing;

/// Edge gate section of the private startup document. Absent, both gates
/// enforce (Go's default without a settings store), HSTS is off and no
/// origin-cloak header is attached. Not Debug: it carries the origin key.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    /// `gate.spoof_header` mode. Only `enforce` denies; every other mode
    /// records the HR-27b finding with mode `monitor` and continues, as Go did.
    #[serde(default = "enforce")]
    pub spoof_header: Mode,
    /// `gate.ban` mode; the gate runs only with an installed ban store.
    #[serde(default = "enforce")]
    pub ban: Mode,
    /// Add `Strict-Transport-Security` to proxied responses. TLS listener only.
    #[serde(default)]
    pub hsts: bool,
    /// Origin-cloak HMAC key as lowercase or uppercase hex, at least
    /// `MIN_ORIGIN_KEY` bytes. Present means cloaking is enabled.
    #[serde(default)]
    pub origin_key_hex: Option<Zeroizing<String>>,
}

fn enforce() -> Mode {
    Mode::Enforce
}

impl Default for Edge {
    fn default() -> Self {
        Self {
            spoof_header: Mode::Enforce,
            ban: Mode::Enforce,
            hsts: false,
            origin_key_hex: None,
        }
    }
}

pub(super) fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Off => "off",
        Mode::Monitor => "monitor",
        Mode::Shadow => "shadow",
        Mode::Enforce => "enforce",
    }
}

impl Edge {
    /// The configured origin key. Enabled cloaking without a valid key is a
    /// configuration refusal (fail closed, owner decision C10).
    pub fn origin_key(&self) -> Result<Option<OriginKey>, Rejection> {
        let Some(text) = &self.origin_key_hex else {
            return Ok(None);
        };
        let bytes = Zeroizing::new(
            hex::decode(text.as_bytes()).map_err(|_| Rejection::new(503, "configuration"))?,
        );
        OriginKey::new(bytes.to_vec())
            .map(Some)
            .map_err(|_| Rejection::new(503, "configuration"))
    }

    /// Nonsecret view: modes and whether cloaking is enabled, never the key.
    pub fn readiness(&self) -> serde_json::Value {
        serde_json::json!({"spoof_header":mode_name(self.spoof_header),"ban":mode_name(self.ban),
            "hsts":self.hsts,"origin_cloak":self.origin_key_hex.is_some()})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_enforce_and_short_or_malformed_keys_are_refused() {
        let edge: Edge = serde_json::from_value(json!({})).unwrap();
        assert_eq!(
            (edge.spoof_header, edge.ban),
            (Mode::Enforce, Mode::Enforce)
        );
        assert!(!edge.hsts);
        assert!(edge.origin_key().unwrap().is_none());
        let key = |hex: &str| {
            serde_json::from_value::<Edge>(json!({"origin_key_hex":hex}))
                .unwrap()
                .origin_key()
                .map(|key| key.is_some())
        };
        assert_eq!(key(&"ab".repeat(32)), Ok(true));
        assert!(key(&"ab".repeat(31)).is_err());
        assert!(key("not hex").is_err());
        assert!(key("").is_err());
        let edge: Edge =
            serde_json::from_value(json!({"spoof_header":"monitor","ban":"off","hsts":true}))
                .unwrap();
        assert_eq!(
            edge.readiness(),
            json!({"spoof_header":"monitor","ban":"off","hsts":true,"origin_cloak":false})
        );
        assert!(serde_json::from_value::<Edge>(json!({"ban":"block"})).is_err());
        assert!(serde_json::from_value::<Edge>(json!({"origin_key":"ab"})).is_err());
    }
}
