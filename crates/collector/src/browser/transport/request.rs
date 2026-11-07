use super::*;
use io::{construct, set};

pub(super) fn origin(global: &JsValue, scope: &Scope) -> Result<String, JsValue> {
    let location = get(global, "location")?;
    let origin = io::text(&get(&location, "origin")?, 2048)?;
    let host = io::text(&get(&location, "host")?, 1024)?;
    let protocol = io::text(&get(&location, "protocol")?, 16)?;
    let hostname = io::text(&get(&location, "hostname")?, 1024)?;
    if origin.len() > 2048
        || host != scope.authority
        || !(protocol == "https:"
            || protocol == "http:"
                && ["127.0.0.1", "[::1]", "localhost"].contains(&hostname.as_str()))
    {
        return Err(error("secure same-origin scope required"));
    }
    Ok(origin)
}
pub(super) fn build(
    global: &JsValue,
    origin: &str,
    method: &str,
    target: &str,
    headers_json: &str,
    body: &[u8],
    abort: &Abort,
) -> Result<JsValue, JsValue> {
    if method.len() > 32
        || humanymous_core::integrity::request::browser::validate_target(target).is_err()
        || target.len() > 8192
        || !target.is_ascii()
        || target.bytes().any(|b| b <= 32 || b == 127)
        || !target.starts_with('/')
        || target.starts_with("//")
        || target.contains(['#', '\\'])
        || target
            .split('?')
            .next()
            .is_some_and(|path| path.contains('%'))
        || body.len() > 65536
        || headers_json.len() > 32768
    {
        return Err(error("unsupported browser request profile"));
    }
    let pairs: Zeroizing<Vec<[String; 2]>> = Zeroizing::new(
        serde_json::from_str(headers_json).map_err(|_| error("invalid application headers"))?,
    );
    if pairs.len() > 100 {
        return Err(error("application header count budget"));
    }
    let headers = construct(global, "Headers", &[])?;
    let mut singleton = std::collections::HashSet::new();
    for [name, value] in pairs.iter() {
        let lower = name.to_ascii_lowercase();
        if ["authorization", "content-type"].contains(&lower.as_str())
            && !singleton.insert(lower.clone())
        {
            return Err(error("duplicate signed application header"));
        }
        if lower.starts_with("x-humanymous-")
            || lower.starts_with("sec-")
            || lower.starts_with("proxy-")
            || lower.starts_with("x-forwarded-")
            || [
                "cookie",
                "cookie2",
                "host",
                "content-length",
                "connection",
                "origin",
                "referer",
                "user-agent",
                "transfer-encoding",
                "te",
                "trailer",
                "upgrade",
                "expect",
                "keep-alive",
                "accept-encoding",
                "accept-charset",
                "date",
                "dnt",
                "permissions-policy",
                "set-cookie",
                "via",
                "forwarded",
                "x-real-ip",
                "cf-connecting-ip",
                "true-client-ip",
                "x-http-method",
                "x-http-method-override",
                "x-method-override",
                "access-control-request-method",
                "access-control-request-headers",
            ]
            .contains(&lower.as_str())
        {
            return Err(error("reserved application header"));
        }
        call(
            &headers,
            "append",
            &[name.as_str().into(), value.as_str().into()],
        )?;
    }
    let options = js_sys::Object::new();
    for (name, value) in [
        ("method", method),
        ("mode", "same-origin"),
        ("credentials", "omit"),
        ("redirect", "error"),
        ("cache", "no-store"),
        ("referrer", ""),
        ("referrerPolicy", "same-origin"),
    ] {
        set(&options, name, &value.into())?;
    }
    set(&options, "headers", &headers)?;
    set(&options, "signal", &get(&abort.0, "signal")?)?;
    if !body.is_empty() {
        set(&options, "body", &js_sys::Uint8Array::from(body).into())?;
    }
    construct(
        global,
        "Request",
        &[format!("{origin}{target}").into(), options.into()],
    )
}
pub(super) fn final_target(
    global: &JsValue,
    origin: &str,
    request: &JsValue,
) -> Result<String, JsValue> {
    let serialized = io::text(&get(request, "url")?, 10300)?;
    let url = construct(global, "URL", &[JsValue::from_str(&serialized)])?;
    if io::text(&get(&url, "origin")?, 2048)? != origin {
        return Err(error("cross-origin request refused"));
    }
    let path = io::text(&get(&url, "pathname")?, 8192)?;
    if path.contains('%') {
        return Err(error("encoded path unsupported"));
    }
    // URL.search loses a present-but-empty query marker. Sign the actual
    // serialized Request URL suffix, preserving /api? versus /api.
    let target = serialized
        .strip_prefix(origin)
        .ok_or_else(|| error("request origin prefix mismatch"))?;
    humanymous_core::integrity::request::browser::validate_target(target)
        .map_err(|_| error("invalid final target"))?;
    Ok(target.to_string())
}
