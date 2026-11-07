//! Browser-controllable request binding; no transport or producer qualification.
#[cfg(test)]
mod tests;
mod validation;

use sha2::{Digest, Sha256};
use std::fmt::Write;

/// Separate domain from legacy native request canonicalization.
pub const PROTOCOL: &str = "humanymous.browser/0.1";
/// Largest exactly representable JavaScript integer.
pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Final browser request representation plus provisioned session scope.
/// This value intentionally has no Debug implementation: headers can be secret.
pub struct Binding<'a> {
    /// Provisioned installation identifier.
    pub installation: &'a str,
    /// Provisioned tenant identifier.
    pub tenant: &'a str,
    /// Provisioned origin identity, not an arbitrary URL.
    pub origin_identity: &'a str,
    /// Immutable policy revision.
    pub revision: &'a str,
    /// Provisioned authority, independently checked against server observation.
    pub authority: &'a str,
    /// Canonical base64url encoding of32 session capability bytes.
    pub session: &'a str,
    /// Final uppercase HTTP method.
    pub method: &'a str,
    /// Final ASCII origin-form path and query, without fragment.
    pub target: &'a str,
    /// Monotonic request counter, beginning at1.
    pub counter: u64,
    /// Ten-second time bucket; window acceptance belongs to runtime.
    pub bucket: u64,
    /// UTC Unix millisecond deadline; freshness belongs to runtime.
    pub deadline: u64,
    /// Final Content-Type header value, or empty when absent.
    pub content_type: &'a str,
    /// Final application Authorization value, never the session capability.
    pub application_authorization: &'a str,
    /// Exact buffered request body; streaming is not supported by this profile.
    pub body: &'a [u8],
}

/// Static, credential-free validation category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingError {
    /// Invalid provisioned scope or authority.
    Scope,
    /// Invalid session encoding or byte length.
    Session,
    /// Unsupported method or ambiguous target.
    Request,
    /// Counter, bucket or deadline outside representable bounds.
    Integer,
    /// Invalid or oversized header value.
    Header,
    /// Buffered body exceeds the bounded profile.
    Body,
}

impl std::fmt::Display for BindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Scope => "invalid browser binding scope",
            Self::Session => "invalid browser binding session",
            Self::Request => "invalid browser binding request",
            Self::Integer => "invalid browser binding integer",
            Self::Header => "invalid browser binding header",
            Self::Body => "browser binding body exceeds limit",
        })
    }
}
impl std::error::Error for BindingError {}

/// Encode exactly14 length-prefixed fields. Returned bytes include credentials:
/// sign transiently and never log, persist or return them as diagnostics.
pub fn canonical(binding: &Binding<'_>) -> Result<String, BindingError> {
    validation::validate(binding)?;
    let content_type = validation::header(binding.content_type, 1024)?;
    let authorization = validation::header(binding.application_authorization, 8192)?;
    let counter = binding.counter.to_string();
    let bucket = binding.bucket.to_string();
    let deadline = binding.deadline.to_string();
    let digest = hex::encode(Sha256::digest(binding.body));
    let fields = [
        binding.installation,
        binding.tenant,
        binding.origin_identity,
        binding.revision,
        binding.authority,
        binding.session,
        binding.method,
        binding.target,
        &counter,
        &bucket,
        &deadline,
        content_type,
        authorization,
        &digest,
    ];
    let mut result = String::with_capacity(512);
    result.push_str(PROTOCOL);
    result.push('\n');
    for field in fields {
        // Writing to a String is infallible; no user-controlled format string.
        writeln!(result, "{}:{}", field.len(), field).expect("String formatting");
    }
    Ok(result)
}
