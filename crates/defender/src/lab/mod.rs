//! Private owned-child lab boundary, distinct from remote target authentication.
mod client;
mod isolation;
mod process;
mod snapshot;

pub use client::{Response, Session};
pub use humanymous_runtime::gateway::http::Request;
pub use process::{OwnedRuntime, ProcessReceipt};
pub use snapshot::{AuditEvent, SnapshotDelta, VerifiedSnapshot};

/// Sanitized boundary failure. Never embeds request bodies or capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabError(pub &'static str);
impl std::fmt::Display for LabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for LabError {}

pub(crate) fn now_ms() -> Result<u64, LabError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|value| u64::try_from(value.as_millis()).ok())
        .ok_or(LabError("clock"))
}

#[cfg(test)]
fn config() -> humanymous_runtime::gateway::configuration::Configuration {
    serde_json::from_value(serde_json::json!({
        "listen":"127.0.0.1:0","origin":"127.0.0.1:8081",
        "scope":{"installation":"owned-lab","tenant":"t","origin":"o","revision":"r1","authority":"owned.test"},
        "routes":[{"method":"POST","path":"/effect","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}}],
        "audit":{"directory":"/private/audit","node_id":"lab","keys":{"directory":"/private/keys","passphrase":"synthetic"},"allow_new":true,"trusted_head":null},
        "owner":{"kind":"unix","uid":65532},"session_capacity":16,"session_ttl_ms":60000,"timeout_ms":5000,"rate_per_second":1000
    })).unwrap()
}
