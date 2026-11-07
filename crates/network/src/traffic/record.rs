use sha2::{Digest, Sha256};
use std::time::Duration;

/// Caller observations only; public fields cannot mint capture provenance.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    /// Ordered caller ticks, not an independently read wall clock.
    pub time: Duration,
    /// Caller-supplied session key; this value does not authenticate it.
    pub session_id: String,
    /// Legacy address text, not a verified transport peer.
    pub remote_addr: String,
    /// Observed JA3 fingerprint text.
    pub ja3: String,
    /// Observed complete JA4 text.
    pub ja4: String,
    /// Observed TLS engine-family classification.
    pub engine: String,
    /// Extension-order fingerprint, never enabling disabled permutation rules.
    pub ext_order: String,
    /// Protocol metadata supplied by the caller.
    pub alpn: String,
    /// TLS version metadata supplied by the caller.
    pub tls_version: String,
    /// Legacy HTTP protocol string, such as1.1 or2.
    pub proto: String,
    /// HTTP method; empty distinguishes a TLS-only record.
    pub method: String,
    /// Request path retained in transient history.
    pub path: String,
    /// Short stable User-Agent hash.
    pub ua_hash: String,
    /// Legacy ordered-name/set hash, without order provenance.
    pub header_order: String,
    /// Caller-observed Sec-Fetch header presence.
    pub has_sec_fetch: bool,
}
impl Record {
    /// JA4 exists and no HTTP method was recorded.
    pub fn is_tls(&self) -> bool {
        !self.ja4.is_empty() && self.method.is_empty()
    }
    /// An HTTP method was recorded.
    pub fn is_http(&self) -> bool {
        !self.method.is_empty()
    }
    pub(super) fn fields(&self) -> [&str; 13] {
        [
            &self.session_id,
            &self.remote_addr,
            &self.ja3,
            &self.ja4,
            &self.engine,
            &self.ext_order,
            &self.alpn,
            &self.tls_version,
            &self.proto,
            &self.method,
            &self.path,
            &self.ua_hash,
            &self.header_order,
        ]
    }
    pub(super) fn bytes(&self) -> usize {
        self.fields().iter().map(|field| field.len()).sum::<usize>() + size_of::<Duration>() + 1
    }
}

/// First six SHA256 bytes as hex; empty input retains the absent marker.
pub fn hash_ua(ua: &str) -> String {
    if ua.is_empty() {
        return String::new();
    }
    hex::encode(&Sha256::digest(ua.as_bytes())[..6])
}
/// Hash the exact comma-joined names; this does not authenticate wire order.
pub fn hash_order(names: &[&str]) -> String {
    if names.is_empty() {
        return String::new();
    }
    hex::encode(&Sha256::digest(names.join(",").as_bytes())[..6])
}

/// Frozen textual Go transform, NOT a general IPv6 /48 parser or peer validator.
pub fn subnet_key(remote: &str) -> String {
    let mut host = remote;
    if host.bytes().filter(|byte| *byte == b':').count() == 1
        && let Some(index) = host.rfind(':').filter(|index| *index > 0)
    {
        host = &host[..index];
    }
    host = host.trim_matches(['[', ']']);
    if host.contains('.') {
        let parts: Vec<_> = host.split('.').collect();
        if parts.len() == 4 {
            return parts[..3].join(".");
        }
    }
    if host.contains(':') {
        let parts: Vec<_> = host.split(':').collect();
        if parts.len() >= 3 {
            return parts[..3].join(":");
        }
    }
    host.into()
}
