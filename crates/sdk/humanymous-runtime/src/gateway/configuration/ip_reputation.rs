//! Operator IP reputation settings (`sot:root/ip-reputation`).
use crate::gateway::Rejection;
use crate::network::intel::{IntelCategory, store::StorePolicy};
use serde::Deserialize;
use std::path::PathBuf;

const HOUR: u64 = 3_600;
const DAY: u64 = 24 * HOUR;

/// Snapshot location, freshness bounds, enabled categories and the trusted
/// client-address source. Absent configuration keeps today's behaviour: no
/// classification and `is_datacenter_ip` stays false.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IpReputation {
    /// Snapshot file written by `humanymous ip-intel update`.
    pub snapshot: PathBuf,
    /// How the original client address reaches this loopback listener.
    pub client_address: ClientAddressSource,
    /// Age after which the snapshot is reported stale (default 7 days).
    #[serde(default = "default_max_age")]
    pub max_age_s: u64,
    /// Age after which the snapshot classifies nothing (default 30 days).
    #[serde(default = "default_expire_after")]
    pub expire_after_s: u64,
    /// Loaded categories; only `datacenter` reaches scoring. Default: all.
    #[serde(default = "default_categories")]
    pub categories: Vec<IntelCategory>,
    /// Deadline for the edge's PROXY v2 header after accept, in milliseconds
    /// (default [`DEFAULT_PROXY_HEADER_TIMEOUT_MS`], bounds 10..=30 000). The
    /// edge writes the header with the connection, so a short deadline keeps a
    /// silent peer from holding the serialized accept loop. The request
    /// deadline (`timeout_ms`) still applies when it is shorter.
    #[serde(default = "default_proxy_header_timeout")]
    pub proxy_header_timeout_ms: u64,
}

/// Default PROXY v2 header deadline: one second.
pub const DEFAULT_PROXY_HEADER_TIMEOUT_MS: u64 = 1_000;

/// Trusted transport source of the original client address. HTTP forwarding
/// headers are never trusted: native request integrity binds every header, and
/// a header is client-controllable whenever the edge forgets to strip it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientAddressSource {
    /// Every accepted loopback connection starts with a PROXY protocol v2
    /// header from the trusted edge; a connection without one is dropped.
    ProxyV2,
}

fn default_max_age() -> u64 {
    7 * DAY
}
fn default_expire_after() -> u64 {
    30 * DAY
}
fn default_proxy_header_timeout() -> u64 {
    DEFAULT_PROXY_HEADER_TIMEOUT_MS
}
fn default_categories() -> Vec<IntelCategory> {
    IntelCategory::ALL.to_vec()
}

impl IpReputation {
    /// Fail closed on an unusable setting before binding.
    pub fn validate(&self) -> Result<(), Rejection> {
        let mut seen = std::collections::BTreeSet::new();
        if self.snapshot.as_os_str().is_empty()
            || !(HOUR..=90 * DAY).contains(&self.max_age_s)
            || !(self.max_age_s..=365 * DAY).contains(&self.expire_after_s)
            || !(10..=30_000).contains(&self.proxy_header_timeout_ms)
            || self.categories.is_empty()
            || !self.categories.iter().all(|c| seen.insert(*c))
        {
            return Err(Rejection::new(503, "configuration"));
        }
        Ok(())
    }

    /// Store parameters for the snapshot owner.
    pub fn store_policy(&self) -> StorePolicy {
        StorePolicy {
            path: self.snapshot.clone(),
            max_age_s: self.max_age_s,
            expire_after_s: self.expire_after_s,
            categories: self.categories.clone(),
        }
    }

    /// Static readiness facts; live freshness is in the management status.
    pub fn readiness(&self) -> serde_json::Value {
        serde_json::json!({
            "client_address": self.client_address,
            "proxy_header_timeout_ms": self.proxy_header_timeout_ms,
            "max_age_s": self.max_age_s,
            "expire_after_s": self.expire_after_s,
            "scoring_categories": self.categories.iter().filter(|c| c.feeds_scoring()).collect::<Vec<_>>(),
            "shadow_categories": self.categories.iter().filter(|c| !c.feeds_scoring()).collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(value: serde_json::Value) -> IpReputation {
        serde_json::from_value(value).unwrap()
    }
    #[test]
    fn defaults_and_bounds() {
        let ok = parse(
            serde_json::json!({"snapshot":"/var/lib/hy/ip.json","client_address":"proxy_v2"}),
        );
        assert!(ok.validate().is_ok());
        assert_eq!((ok.max_age_s, ok.expire_after_s), (7 * DAY, 30 * DAY));
        assert_eq!(ok.proxy_header_timeout_ms, DEFAULT_PROXY_HEADER_TIMEOUT_MS);
        assert_eq!(ok.store_policy().categories, IntelCategory::ALL);
        assert_eq!(
            ok.readiness()["scoring_categories"],
            serde_json::json!(["datacenter"])
        );
        assert_eq!(
            ok.readiness()["shadow_categories"],
            serde_json::json!(["tor_exit", "blocklist"])
        );
        for bad in [
            serde_json::json!({"snapshot":""}),
            serde_json::json!({"max_age_s":60}),
            serde_json::json!({"max_age_s":DAY, "expire_after_s":HOUR}),
            serde_json::json!({"categories":[]}),
            serde_json::json!({"proxy_header_timeout_ms":0}),
            serde_json::json!({"proxy_header_timeout_ms":30_001}),
            serde_json::json!({"categories":["tor_exit","tor_exit"]}),
        ] {
            let mut value = serde_json::json!({"snapshot":"/s.json","client_address":"proxy_v2"});
            for (key, field) in bad.as_object().unwrap() {
                value[key] = field.clone();
            }
            assert!(parse(value.clone()).validate().is_err(), "{value}");
        }
        for source in ["x-forwarded-for", "header"] {
            assert!(
                serde_json::from_value::<IpReputation>(
                    serde_json::json!({"snapshot":"/s","client_address":source})
                )
                .is_err()
            );
        }
        assert!(
            serde_json::from_value::<IpReputation>(serde_json::json!({"snapshot":"/s","client_address":"proxy_v2","scoring":["tor_exit"]}))
                .is_err()
        );
    }
}
