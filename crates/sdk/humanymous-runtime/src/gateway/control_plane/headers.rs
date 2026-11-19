//! Gate header projection: legacy `internal/gate/control.go` `headerInfo` and
//! `protoVer` at baseline 981d25c, the L5 observation input of `scoreBeacon`.
//!
//! The input is the handler-visible header multimap ([`Request::headers`]).
//! Lookups follow Go `http.Header` semantics: names compare after
//! `textproto.CanonicalMIMEHeaderKey` and `Get` returns the first value, so an
//! empty first value reads as absent. `Host` is never a header name here
//! (Go moves it to `Request.Host` on every protocol). Other framing rewrites
//! net/http applies before a handler runs (`Transfer-Encoding` removal, the
//! `Pragma` to `Cache-Control` fix) belong to the caller's parser.
//!
//! The gate projection differs from the legacy Core adapter
//! (`reqToHeaderInfo`, ported separately under `network::http`) on purpose,
//! and both rules are kept as Go had them: HTTP/1.0 reports version `"11"`
//! and cookie names are never projected. Go collected names from map
//! iteration (random order); here they are the sorted canonical set, one of
//! the orders Go could produce, and neither order nor casing is claimed
//! reliable. Pinned against `header_projection` in
//! `tests/fixtures/gateway/control-plane-go-golden.json`.
use super::Request;
use crate::network::http::HeaderInfo;
use std::collections::BTreeSet;

fn token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

/// Go `textproto.CanonicalMIMEHeaderKey`: a name holding any non-token byte
/// is unchanged; otherwise the first letter and each letter after `-` are
/// uppercased and the rest lowercased.
pub(super) fn canonical_name(name: &str) -> String {
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

/// Go `http.Header.Get`: first value of the canonical name, else empty.
pub(super) fn first_value<'a>(headers: &'a [(String, String)], name: &str) -> &'a str {
    let canonical = canonical_name(name);
    headers
        .iter()
        .find(|(key, _)| canonical_name(key) == canonical)
        .map_or("", |(_, value)| value.as_str())
}

/// Go `protoVer`: `"20"` for HTTP/2, otherwise `"11"` (HTTP/1.0 included).
pub(super) fn proto_ver(proto_major: u8) -> &'static str {
    if proto_major == 2 { "20" } else { "11" }
}

/// Go `headerInfo(r)`.
pub(super) fn header_info(request: &Request<'_>) -> HeaderInfo {
    let names: BTreeSet<String> = request
        .headers
        .iter()
        .map(|(name, _)| canonical_name(name))
        .filter(|name| name != "Host")
        .collect();
    let get = |name: &str| request.header(name).to_owned();
    let x_cache = match request.header("X-Cache") {
        "" => get("X-Cache-Lookup"),
        value => value.to_owned(),
    };
    HeaderInfo {
        method: request.method.to_owned(),
        version: proto_ver(request.proto_major).into(),
        is_h2: request.proto_major == 2,
        names: names.into_iter().collect(),
        casing_reliable: false,
        order_reliable: false,
        has_cookie: !request.header("Cookie").is_empty(),
        has_referer: !request.header("Referer").is_empty(),
        accept_language: get("Accept-Language"),
        accept_encoding: get("Accept-Encoding"),
        cookie_names: Vec::new(),
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
