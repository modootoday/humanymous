use sha2::{Digest, Sha256};
use std::time::Duration;

/// Caller observations only; public fields cannot mint capture provenance.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    pub time: Duration,
    pub session_id: String,
    pub remote_addr: String,
    pub ja3: String,
    pub ja4: String,
    pub engine: String,
    pub ext_order: String,
    pub alpn: String,
    pub tls_version: String,
    pub proto: String,
    pub method: String,
    pub path: String,
    pub ua_hash: String,
    pub header_order: String,
    pub has_sec_fetch: bool,
}
impl Record {
    pub fn is_tls(&self) -> bool {
        !self.ja4.is_empty() && self.method.is_empty()
    }
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

pub fn hash_ua(ua: &str) -> String {
    if ua.is_empty() {
        return String::new();
    }
    hex::encode(&Sha256::digest(ua.as_bytes())[..6])
}
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
