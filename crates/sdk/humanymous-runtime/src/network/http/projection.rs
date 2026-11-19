//! Handler-level header projection (legacy `reqToHeaderInfo`, Go oracle 981d25c).
//!
//! The input is the header multimap a request handler sees, as ordered
//! `(name, value)` pairs. Lookups follow Go `http.Header` semantics: names are
//! compared after `textproto.CanonicalMIMEHeaderKey` and `Get` returns the first
//! value. The framing layer's own rewrites (net/http drops Host, Transfer-Encoding
//! and some Content-Length/Trailer fields before a handler runs) are not part of
//! this projection. Names come out as a sorted canonical SET with
//! `order_reliable = false`; this projection never claims wire order or casing.
use super::HeaderInfo;
use std::collections::BTreeSet;

/// Go `net/http` default cookie-count ceiling (`httpcookiemaxnum`, Go 1.24+):
/// above it `Request.Cookies` yields nothing.
const COOKIE_LIMIT: usize = 3000;

fn token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

/// Go `textproto.CanonicalMIMEHeaderKey`: a name holding any non-token byte is
/// returned unchanged; otherwise the first letter and each letter after `-` are
/// uppercased and the rest lowercased.
pub fn canonical_name(name: &str) -> String {
    if !name.bytes().all(token) {
        return name.to_owned();
    }
    let mut upper = true;
    name.bytes()
        .map(|byte| {
            let byte = if upper {
                byte.to_ascii_uppercase()
            } else {
                byte.to_ascii_lowercase()
            };
            upper = byte == b'-';
            char::from(byte)
        })
        .collect()
}

fn values<'a>(
    fields: &'a [(String, String)],
    canonical: &str,
) -> impl Iterator<Item = &'a str> + use<'a> {
    let canonical = canonical.to_owned();
    fields
        .iter()
        .filter(move |(name, _)| canonical_name(name) == canonical)
        .map(|(_, value)| value.as_str())
}

/// Go `http.Header.Get`: first value of the canonical name, else empty.
pub fn first_value<'a>(fields: &'a [(String, String)], name: &str) -> &'a str {
    values(fields, &canonical_name(name)).next().unwrap_or("")
}

// Go textproto.TrimString: ASCII space, tab, CR and LF only.
fn trim_ascii(value: &str) -> &str {
    value.trim_matches([' ', '\t', '\r', '\n'])
}

// Go parseCookieValue(raw, allowDoubleQuote=true): the value with one pair of
// surrounding quotes removed, or None when any byte is not a cookie octet.
fn cookie_value_parsed(raw: &str) -> Option<&str> {
    let unquoted = if raw.len() > 1 && raw.starts_with('"') && raw.ends_with('"') {
        &raw[1..raw.len() - 1]
    } else {
        raw
    };
    unquoted
        .bytes()
        .all(|b| (0x20..0x7f).contains(&b) && !matches!(b, b'"' | b';' | b'\\'))
        .then_some(unquoted)
}

/// Go `readCookies`: every valid `(name, value)` pair in request order, or
/// nothing when the cookie-count ceiling is exceeded.
fn cookies(fields: &[(String, String)]) -> Vec<(&str, &str)> {
    let lines: Vec<&str> = values(fields, "Cookie").collect();
    let count: usize = lines.iter().map(|line| line.matches(';').count() + 1).sum();
    if lines.is_empty() || count > COOKIE_LIMIT {
        return Vec::new();
    }
    let mut pairs = Vec::new();
    for line in lines {
        let mut line = trim_ascii(line);
        while !line.is_empty() {
            let (part, rest) = line.split_once(';').unwrap_or((line, ""));
            line = rest;
            let part = trim_ascii(part);
            if part.is_empty() {
                continue;
            }
            let (name, value) = part.split_once('=').unwrap_or((part, ""));
            let name = trim_ascii(name);
            if name.is_empty() || !name.bytes().all(token) {
                continue;
            }
            if let Some(value) = cookie_value_parsed(value) {
                pairs.push((name, value));
            }
        }
    }
    pairs
}

/// Cookie names in the order Go `Request.Cookies` returns them.
fn cookie_names(fields: &[(String, String)]) -> Vec<String> {
    cookies(fields)
        .into_iter()
        .map(|(name, _)| name.to_owned())
        .collect()
}

/// Legacy Core `cookieValue` (Go `Request.Cookie(name).Value`): the value of
/// the first valid cookie with exactly this name, one pair of surrounding
/// double quotes removed; empty when absent or invalid.
pub fn cookie_value(fields: &[(String, String)], name: &str) -> String {
    cookies(fields)
        .into_iter()
        .find(|(found, _)| *found == name)
        .map_or_else(String::new, |(_, value)| value.to_owned())
}

/// Project a handler-visible request into [`HeaderInfo`] exactly as the legacy
/// Core adapter did. Reliability flags stay false.
pub fn project_request(
    method: &str,
    proto_major: u8,
    proto_minor: u8,
    fields: &[(String, String)],
) -> HeaderInfo {
    let names: BTreeSet<String> = fields
        .iter()
        .map(|(name, _)| canonical_name(name))
        .collect();
    let version = match (proto_major, proto_minor) {
        (2, _) => "20",
        (1, 0) => "10",
        _ => "11",
    };
    let get = |name| first_value(fields, name).to_owned();
    let x_cache = match first_value(fields, "X-Cache") {
        "" => get("X-Cache-Lookup"),
        value => value.to_owned(),
    };
    HeaderInfo {
        method: method.to_owned(),
        version: version.into(),
        is_h2: proto_major == 2,
        names: names.into_iter().collect(),
        casing_reliable: false,
        order_reliable: false,
        has_cookie: !first_value(fields, "Cookie").is_empty(),
        has_referer: !first_value(fields, "Referer").is_empty(),
        accept_language: get("Accept-Language"),
        accept_encoding: get("Accept-Encoding"),
        cookie_names: cookie_names(fields),
        user_agent: get("User-Agent"),
        via: get("Via"),
        proxy_connection: get("Proxy-Connection"),
        x_cache,
        x_squid_error: get("X-Squid-Error"),
        x_forwarded_for: get("X-Forwarded-For"),
        forwarded: get("Forwarded"),
        cf_connecting_ip: get("CF-Connecting-IP"),
        true_client_ip: get("True-Client-IP"),
        x_client_ip: get("X-Client-IP"),
        x_original_forwarded_for: get("X-Original-Forwarded-For"),
        x_blue_coat_via: get("X-BlueCoat-Via"),
    }
}
