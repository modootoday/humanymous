//! Go HTTP origin deployment contracts consuming the shared Rust Gate.
//! No Go SDK, authority implementation or fabricated capture provenance.
pub mod probe;
mod qualification;
pub mod scenario;
use humanymous_runtime::{Rejection, configuration::Configuration};
pub use qualification::{CASES, Case, ExpectedTuple, Outcome, QUALIFICATION_PROFILE, Receipt};
use serde::{Deserialize, Serialize};

/// Host families have independent lifecycle qualification, even behind one Gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Host {
    /// Standard library HTTP handler; request body and context cancellation.
    NetHttp,
    /// chi router using the standard library handler interface.
    Chi,
    /// Gin engine using net/http, installed before application parsers/effects.
    Gin,
    /// Echo using net/http, with framework middleware routing semantics.
    Echo,
    /// Fiber with recycled fasthttp contexts; not a net/http wrapper.
    Fiber,
    /// fasthttp server with borrowed request buffer lifetimes.
    FastHttp,
}
impl Host {
    /// Whether this framework recycles its own context after handler return.
    /// This does not grant permission to retain ordinary net/http request bodies.
    pub const fn pooled_context(self) -> bool {
        matches!(self, Self::Gin | Self::Echo | Self::Fiber | Self::FastHttp)
    }
    /// fasthttp-style borrowed entity slices need copying before retention.
    /// Separate from pooled Gin/Echo framework contexts over net/http streams.
    pub const fn borrows_entity_slices(self) -> bool {
        matches!(self, Self::Fiber | Self::FastHttp)
    }
}

/// Validated universal Gate configuration for an external Go HTTP origin.
/// The runtime remains sole owner of parsing, authorization, audit and forwarding.
pub struct Deployment {
    host: Host,
    configuration: Configuration,
}
impl Deployment {
    /// Validate using the actual shared runtime, without binding any socket.
    /// Origin network isolation must additionally be proved by the hosted lab.
    pub fn new(host: Host, configuration: Configuration) -> Result<Self, Rejection> {
        configuration.validate()?;
        Ok(Self {
            host,
            configuration,
        })
    }
    /// Exact selected host family, never inferred from public HTTP headers.
    pub const fn host(&self) -> Host {
        self.host
    }
    /// Transfer configuration into the existing runtime composition root.
    pub fn into_configuration(self) -> Configuration {
        self.configuration
    }
}
