//! Private adapter protocol validation. Parsing grants no decision authority.
#[cfg(test)]
mod tests;
mod wire;
use base64::{Engine, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};
use wire::Request;

/// Exact adapter identity provisioned by the authority, never selected from
/// untrusted envelope contents. This grants no additional protocol capability.
#[derive(Clone, Copy, Debug)]
pub struct ExpectedAdapter<'a> {
    /// Exact provisioned producer identifier.
    pub id: &'a str,
    /// Exact supported producer version, without wildcard matching.
    pub version: &'a str,
}

/// Schema-checked, complete request bytes. Credential scope, real IPC peer,
/// route inventory, replay and durable audit must still be checked by authority.
pub struct ValidatedRequest {
    request: Request,
    body: Vec<u8>,
}
impl ValidatedRequest {
    /// Envelope identity; authentication and provisioned scope remain required.
    pub fn adapter_identity(&self) -> (&str, &str) {
        (&self.request.adapter.id, &self.request.adapter.version)
    }
    /// Validated observation mode, not proof of original client provenance.
    pub fn header_mode(&self) -> &str {
        &self.request.header_mode
    }
    /// Adapter-claimed connection UUID, not an authenticated transport binding.
    pub fn connection_id(&self) -> Option<&str> {
        self.request.connection_id.0.as_deref()
    }
    /// Adapter-generated UUIDv4 nonce; not client trace identity.
    pub fn id(&self) -> &str {
        &self.request.id
    }
    /// Installation scope claimed by the authenticated adapter.
    pub fn installation(&self) -> &str {
        &self.request.adapter.installation
    }
    /// Configured tenant scope, never original-client attribution.
    pub fn tenant(&self) -> &str {
        &self.request.tenant
    }
    /// Exact method and original origin-form target.
    pub fn request_target(&self) -> (&str, &str) {
        (&self.request.method, &self.request.target)
    }
    /// Ordered sanitized headers, preserving duplicates and original casing.
    pub fn headers(&self) -> &[[String; 2]] {
        &self.request.headers
    }
    /// Verified transfer-decoded, content-encoded entity bytes.
    pub fn body(&self) -> &[u8] {
        &self.body
    }
    /// Configured origin, revision and received authority, requiring scope match.
    pub fn origin_binding(&self) -> (&str, &str, &str) {
        (
            &self.request.origin,
            &self.request.revision,
            &self.request.authority,
        )
    }
    /// Nonce lifetime upper bound in UTC Unix milliseconds.
    pub fn deadline(&self) -> u64 {
        self.request.deadline
    }
}

/// Reject malformed JSON, duplicate/unknown keys, missing fields, unsupported
/// capabilities and byte mismatches. No permissive map intermediate is used:
/// derived fixed-schema visitors reject duplicate keys, including escaped keys.
/// Static errors cannot disclose bearer credentials, cookies or body contents.
pub fn parse(bytes: &[u8], now_ms: u64) -> Result<ValidatedRequest, &'static str> {
    parse_for(
        bytes,
        now_ms,
        ExpectedAdapter {
            id: "humanymous.node",
            version: "0.1.0-experimental.1",
        },
    )
}

/// Validate protocol0.1 against an explicitly provisioned adapter. Callers must
/// not construct `expected` from the request being validated. All existing body,
/// provenance and unsupported capture/stream restrictions remain in effect.
pub fn parse_for(
    bytes: &[u8],
    now_ms: u64,
    expected: ExpectedAdapter<'_>,
) -> Result<ValidatedRequest, &'static str> {
    if !identity(expected.id) || !identity(expected.version) {
        return Err("invalid provisioned adapter identity");
    }
    if bytes.len() > 1_600_000 {
        return Err("adapter envelope exceeds limit");
    }
    let request: Request =
        serde_json::from_slice(bytes).map_err(|_| "invalid adapter JSON schema")?;
    let r = &request;
    if r.protocol != "humanymous.adapter/0.1"
        || r.adapter.id != expected.id
        || r.adapter.version != expected.version
    {
        return Err("unsupported adapter protocol or version");
    }
    if [&r.tenant, &r.origin, &r.revision, &r.adapter.installation]
        .iter()
        .any(|s| !identity(s))
        || !uuid4(&r.id)
        || r.connection_id.0.as_ref().is_some_and(|id| !uuid4(id))
    {
        return Err("invalid adapter identity");
    }
    if r.deadline > 9_007_199_254_740_991 || r.deadline <= now_ms || r.deadline - now_ms > 30_000 {
        return Err("invalid adapter deadline");
    }
    if r.method.is_empty()
        || r.method.len() > 32
        || !r
            .method
            .bytes()
            .all(|b| token(b) && !b.is_ascii_lowercase())
        || r.method == "CONNECT"
    {
        return Err("unsupported adapter method");
    }
    validate_target(&r.target)?;
    if r.authority.is_empty()
        || r.authority.len() > 1024
        || !r
            .authority
            .bytes()
            .all(|b| (33..=126).contains(&b) && !b"@/#?\\".contains(&b))
    {
        return Err("invalid adapter authority");
    }
    if r.peer.0.as_ref().is_some_and(|peer| {
        peer.is_empty() || peer.len() > 128 || peer.chars().any(char::is_control)
    }) {
        return Err("invalid application peer observation");
    }
    let raw = match r.header_mode.as_str() {
        "node-raw" => true,
        "fetch-normalized" => false,
        _ => return Err("invalid header provenance"),
    };
    if r.capabilities.raw_tls
        || r.capabilities.original_h2
        || !r.capabilities.complete_body
        || r.capabilities.raw_header_order != raw
        || (!raw && (r.connection_id.0.is_some() || r.peer.0.is_some()))
    {
        return Err("unsupported or contradictory capture capabilities");
    }
    if r.body.layer != "transfer-decoded-content-encoded"
        || !r.body.complete
        || r.body.length > 1_048_576
    {
        return Err("unsupported body layer or completeness");
    }
    let body = STANDARD
        .decode(&r.body.base64)
        .map_err(|_| "invalid body base64")?;
    if body.len() as u64 != r.body.length
        || STANDARD.encode(&body) != r.body.base64
        || hex::encode(Sha256::digest(&body)) != r.body.sha256
    {
        return Err("body length or digest mismatch");
    }
    validate_headers(r, body.len())?;
    Ok(ValidatedRequest { request, body })
}
fn identity(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
fn uuid4(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, &c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == b'-'
            } else {
                c.is_ascii_digit() || (b'a'..=b'f').contains(&c)
            }
        })
        && b[14] == b'4'
        && b"89ab".contains(&b[19])
}
fn token(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}
fn validate_target(s: &str) -> Result<(), &'static str> {
    if !s.starts_with('/')
        || s.starts_with("//")
        || s.len() > 8192
        || s.bytes()
            .any(|b| b <= 32 || b == 127 || b"#\\".contains(&b))
    {
        return Err("ambiguous adapter target");
    }
    let mut decoded = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let pair = bytes.get(i + 1..i + 3).ok_or("invalid target escape")?;
            let h = |b: u8| (b as char).to_digit(16).map(|n| n as u8);
            let value = h(pair[0])
                .zip(h(pair[1]))
                .map(|(a, b)| a * 16 + b)
                .ok_or("invalid target escape")?;
            if [0, b'/', b'\\'].contains(&value) {
                return Err("ambiguous target escape");
            }
            decoded.push(value);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    let decoded = std::str::from_utf8(&decoded).map_err(|_| "invalid target UTF-8")?;
    if decoded
        .split('?')
        .next()
        .unwrap_or("")
        .split('/')
        .any(|part| part == "." || part == "..")
    {
        return Err("ambiguous target dot segment");
    }
    Ok(())
}
fn validate_headers(r: &Request, body_len: usize) -> Result<(), &'static str> {
    if r.headers.len() > 100
        || r.headers
            .iter()
            .map(|h| h[0].len() + h[1].len())
            .sum::<usize>()
            > 32768
    {
        return Err("adapter header limit");
    }
    let mut lengths = Vec::new();
    let mut transfers = Vec::new();
    let mut hosts = Vec::new();
    for [name, value] in &r.headers {
        let lower = name.to_ascii_lowercase();
        if name.is_empty()
            || !name.bytes().all(token)
            || value.bytes().any(|b| [0, 10, 13].contains(&b))
        {
            return Err("invalid adapter header");
        }
        if lower.starts_with("x-humanymous-")
            || lower.starts_with("x-forwarded-")
            || [
                "forwarded",
                "cf-connecting-ip",
                "true-client-ip",
                "x-real-ip",
                "upgrade",
                "trailer",
            ]
            .contains(&lower.as_str())
        {
            return Err("untrusted internal header or unsupported stream");
        }
        match lower.as_str() {
            "content-length" => lengths.push(value),
            "transfer-encoding" => transfers.push(value),
            "host" => hosts.push(value),
            "content-type" if value.to_ascii_lowercase().starts_with("application/grpc") => {
                return Err("unsupported gRPC stream");
            }
            _ => {}
        }
    }
    if lengths.len() > 1
        || transfers.len() > 1
        || hosts.len() > 1
        || (!lengths.is_empty() && !transfers.is_empty())
        || hosts
            .first()
            .is_some_and(|host| host.as_str() != r.authority)
        || transfers
            .first()
            .is_some_and(|v| !v.eq_ignore_ascii_case("chunked"))
    {
        return Err("ambiguous adapter framing");
    }
    if let Some(length) = lengths.first()
        && (length.is_empty()
            || !length.bytes().all(|b| b.is_ascii_digit())
            || (length.len() > 1 && length.starts_with('0'))
            || length.parse::<usize>().ok() != Some(body_len))
    {
        return Err("adapter content length mismatch");
    }
    Ok(())
}
