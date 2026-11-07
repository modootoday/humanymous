//! Provisioned producer profiles; wire contents never select their own policy.

use super::ExpectedAdapter;

/// Supported protocol0.1 producer observation contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterProfile {
    /// Legacy Node raw HTTP headers or Fetch-normalized observations.
    Node,
    /// nginx parsed HTTP headers; no connection, TLS or original H2 claim.
    Nginx,
}

impl AdapterProfile {
    /// Exact producer identity/version required by this profile.
    pub const fn expected(self) -> ExpectedAdapter<'static> {
        ExpectedAdapter {
            id: match self {
                Self::Node => "humanymous.node",
                Self::Nginx => "humanymous.nginx",
            },
            version: "0.1.0-experimental.1",
        }
    }

    pub(super) fn header_order(self, mode: &str) -> Result<bool, &'static str> {
        match (self, mode) {
            (Self::Node, "node-raw") | (Self::Nginx, "nginx-raw") => Ok(true),
            (Self::Node, "fetch-normalized") | (Self::Nginx, "nginx-normalized") => Ok(false),
            _ => Err("invalid header provenance"),
        }
    }
}
