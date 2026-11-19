//! Operator IP reputation: public datacenter ranges, Tor exits and blocklists.
//!
//! Governed by `sot:root/ip-reputation`. Only [`IntelCategory::Datacenter`] feeds
//! scoring, and only through the existing frozen `is_datacenter_ip` observation
//! input. Tor exits and blocklists are shadow-only: they are counted and exposed
//! to operators, never passed to scoring or verdicts.
//!
//! Boundaries: [`prefix_set`] (pure CIDR lookup), [`snapshot`] (pure versioned
//! file format), [`source`] (pure registry and upstream format normalizers),
//! [`store`] (snapshot file I/O, atomic swap, staleness and the ops gauge) and
//! [`shadow`] (shadow counters). The serving path consumes only [`IntelReader`].
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

pub mod prefix_set;
pub mod shadow;
pub mod snapshot;
pub mod source;
pub mod store;

/// Reputation category of a prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntelCategory {
    /// Cloud/hosting ranges. The only category that reaches scoring.
    Datacenter,
    /// Tor exit relays. Shadow-only.
    TorExit,
    /// Abuse/hijacked-range blocklists. Shadow-only.
    Blocklist,
}

impl IntelCategory {
    /// Every category in stable order.
    pub const ALL: [Self; 3] = [Self::Datacenter, Self::TorExit, Self::Blocklist];

    /// Whether this category may feed the frozen scoring input. Promotion of any
    /// other category is an owner decision recorded in the SoT, not a setting.
    pub fn feeds_scoring(self) -> bool {
        self == Self::Datacenter
    }
}

/// Category membership of one client address.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Classification {
    /// Address is in a datacenter/hosting range: sets `is_datacenter_ip`.
    pub datacenter: bool,
    /// Address is a Tor exit. Shadow-only.
    pub tor_exit: bool,
    /// Address is in a blocklisted range. Shadow-only.
    pub blocklist: bool,
}

impl Classification {
    /// Membership in one category.
    pub fn contains(&self, category: IntelCategory) -> bool {
        match category {
            IntelCategory::Datacenter => self.datacenter,
            IntelCategory::TorExit => self.tor_exit,
            IntelCategory::Blocklist => self.blocklist,
        }
    }
}

/// The one read-side contract the serving path uses.
pub trait IntelReader: Send + Sync {
    /// Classify a trusted client address at `now_s` (Unix seconds). `None`
    /// means the request carried no usable trusted address; it is counted as
    /// unattributed and classifies as nothing. An unavailable or expired feed
    /// also classifies as nothing, and is visible in the operator status.
    fn classify(&self, client: Option<IpAddr>, now_s: u64) -> Classification;
}

/// Parse a trusted client-address field: exactly one IP literal, no port, list
/// or whitespace. IPv4-mapped IPv6 is normalized to IPv4.
pub fn client_address(value: &str) -> Option<IpAddr> {
    match value.parse::<IpAddr>().ok()? {
        IpAddr::V6(v6) => Some(v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4)),
        v4 => Some(v4),
    }
}

/// Addresses that are never classified: loopback, private/unique-local,
/// link-local and unspecified (Go oracle parity; a LAN or proxy hop is not a
/// datacenter accusation).
pub fn is_classifiable(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !(v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified())
        }
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_classifiable(IpAddr::V4(v4)),
            None => {
                !(v6.is_loopback()
                    || v6.is_unique_local()
                    || v6.is_unicast_link_local()
                    || v6.is_unspecified())
            }
        },
    }
}

#[cfg(test)]
mod tests;
