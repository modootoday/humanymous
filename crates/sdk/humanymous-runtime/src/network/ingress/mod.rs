//! Bounded decoded-header projection. DATA alone never proves transport origin.
use crate::network::http::HeaderInfo;
use std::collections::BTreeSet;

/// Parsed application claims to compare with captured bytes, never a receipt.
pub struct H2Claim<'a> {
    /// Client stream identifier on the retained connection.
    pub stream: u32,
    /// Exact parsed method, without normalization.
    pub method: &'a str,
    /// Exact parsed authority.
    pub authority: &'a str,
    /// Exact path and query bytes represented as UTF-8.
    pub target: &'a str,
    /// Application-consumed body byte count; this is not a body digest.
    pub body_bytes: u64,
}

/// Missing or ambiguous input is a refusal, never a clean empty observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Required same-connection owner is cancelled, expired or failed.
    OwnerUnavailable,
    /// Actual negotiation does not establish h2; offered ALPN is insufficient.
    Protocol,
    /// Invalid/unknown stream or missing completed initial header section.
    MissingStream,
    /// Input has not completed END_STREAM, or was reset.
    IncompleteStream,
    /// Captured DATA count differs from the application count/Content-Length.
    BodyLength,
    /// Trailer fields are not represented by this initial-header-only profile.
    TrailersUnsupported,
    /// A required pseudo-header is missing.
    MissingField,
    /// Repeated singleton or contradictory host/authority representation.
    AmbiguousField,
    /// Parsed method, target or authority differs from the actual header section.
    RequestMismatch,
    /// Unsupported encoding/value/profile shape.
    InvalidField,
    /// Fixed field/byte/cookie-name budget exceeded.
    Budget,
}

/// Pure DATA projection. Reliability flags stay FALSE. Only the actual capture
/// owner may establish its own raw-header observation in a separate live view.
pub fn project_h2(
    fields: &[(Vec<u8>, Vec<u8>)],
    claim: &H2Claim<'_>,
) -> Result<HeaderInfo, Refusal> {
    if claim.stream == 0 || claim.stream.is_multiple_of(2) {
        return Err(Refusal::MissingStream);
    }
    if fields.len() > 128 {
        return Err(Refusal::Budget);
    }
    let mut total = 0usize;
    for (name, value) in fields {
        total = total
            .checked_add(name.len())
            .and_then(|n| n.checked_add(value.len()))
            .ok_or(Refusal::Budget)?;
    }
    if total > 65_536 {
        return Err(Refusal::Budget);
    }
    let mut header = HeaderInfo {
        version: "20".into(),
        is_h2: true,
        ..HeaderInfo::default()
    };
    let mut seen = BTreeSet::new();
    let (mut method, mut authority, mut path, mut scheme, mut host) =
        (None, None, None, None, None);
    for (name, bytes) in fields {
        let name = std::str::from_utf8(name).map_err(|_| Refusal::InvalidField)?;
        let value = std::str::from_utf8(bytes).map_err(|_| Refusal::InvalidField)?;
        if name.is_empty()
            || name.bytes().any(|b| b.is_ascii_uppercase())
            || value.bytes().any(|b| matches!(b, 0 | b'\r' | b'\n'))
        {
            return Err(Refusal::InvalidField);
        }
        let singleton = matches!(
            name,
            ":method"
                | ":authority"
                | ":path"
                | ":scheme"
                | "host"
                | "content-length"
                | "cookie"
                | "referer"
                | "accept-language"
                | "accept-encoding"
                | "user-agent"
                | "via"
                | "proxy-connection"
                | "x-cache"
                | "x-squid-error"
                | "x-forwarded-for"
                | "forwarded"
                | "cf-connecting-ip"
                | "true-client-ip"
                | "x-client-ip"
                | "x-original-forwarded-for"
                | "x-bluecoat-via"
        );
        if singleton && !seen.insert(name) {
            return Err(Refusal::AmbiguousField);
        }
        if !name.starts_with(':') {
            header.names.push(name.into());
        }
        match name {
            ":method" => method = Some(value),
            ":authority" => authority = Some(value),
            ":path" => path = Some(value),
            ":scheme" => scheme = Some(value),
            "host" => host = Some(value),
            "content-length" => {
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(Refusal::InvalidField);
                }
                if value.parse::<u64>().map_err(|_| Refusal::InvalidField)? != claim.body_bytes {
                    return Err(Refusal::BodyLength);
                }
            }
            "cookie" => {
                header.has_cookie = true;
                for cookie in value.split(';') {
                    let (name, _) = cookie.trim().split_once('=').ok_or(Refusal::InvalidField)?;
                    if name.is_empty() {
                        return Err(Refusal::InvalidField);
                    }
                    if header.cookie_names.len() >= 128 {
                        return Err(Refusal::Budget);
                    }
                    header.cookie_names.push(name.into());
                }
            }
            "referer" => header.has_referer = true,
            _ => {
                let slot = match name {
                    "accept-language" => Some(&mut header.accept_language),
                    "accept-encoding" => Some(&mut header.accept_encoding),
                    "user-agent" => Some(&mut header.user_agent),
                    "via" => Some(&mut header.via),
                    "proxy-connection" => Some(&mut header.proxy_connection),
                    "x-cache" => Some(&mut header.x_cache),
                    "x-squid-error" => Some(&mut header.x_squid_error),
                    "x-forwarded-for" => Some(&mut header.x_forwarded_for),
                    "forwarded" => Some(&mut header.forwarded),
                    "cf-connecting-ip" => Some(&mut header.cf_connecting_ip),
                    "true-client-ip" => Some(&mut header.true_client_ip),
                    "x-client-ip" => Some(&mut header.x_client_ip),
                    "x-original-forwarded-for" => Some(&mut header.x_original_forwarded_for),
                    "x-bluecoat-via" => Some(&mut header.x_blue_coat_via),
                    _ if name.starts_with(':') => return Err(Refusal::InvalidField),
                    _ => None,
                };
                if let Some(slot) = slot {
                    *slot = value.into();
                }
            }
        }
    }
    let (method, authority, path, scheme) = match (method, authority, path, scheme) {
        (Some(m), Some(a), Some(p), Some(s)) if !m.is_empty() && !a.is_empty() && !p.is_empty() => {
            (m, a, p, s)
        }
        _ => return Err(Refusal::MissingField),
    };
    if scheme != "https" {
        return Err(Refusal::Protocol);
    }
    if host.is_some_and(|host| host != authority) {
        return Err(Refusal::AmbiguousField);
    }
    if method != claim.method || authority != claim.authority || path != claim.target {
        return Err(Refusal::RequestMismatch);
    }
    header.method = method.into();
    Ok(header)
}

#[cfg(test)]
mod tests;
