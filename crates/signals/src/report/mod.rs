//! Shared client, network and merged-session wire reports.
mod aggregation;
mod behavior;
mod client;
mod network;
mod scoring;
mod session;
mod timestamp;

pub use behavior::{BehaviorSummary, EventFeatures, KeyFeatures, MouseFeatures};
pub use client::{Advanced, ClientReport, Environment, Guard, UaBrand, UaClientHints};
pub use network::NetworkReport;
pub use scoring::{Contributor, CrossCheck, ScoreResult};
pub use session::SessionReport;
pub use timestamp::Timestamp;
