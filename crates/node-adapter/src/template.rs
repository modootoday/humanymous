//! Explicit application template integration, with no automatic HTML mutation.
use napi_derive::napi;

/// Markup and matching CSP source for an operator-provisioned browser script.
#[napi(object)]
pub struct HtmlHook {
    /// Insert only into an eligible application-owned HTML template.
    pub markup: String,
    /// Add to the same response's explicit CSP script policy.
    pub csp_source: String,
}
/// Return validated template markup without serving a script or altering CSP,
/// compressed content, ETags, integrity headers, caching or response streaming.
#[napi]
pub fn html_hook(nonce: String, script_path: String) -> napi::Result<HtmlHook> {
    if !(16..=128).contains(&nonce.len())
        || !nonce
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"+/_=-".contains(&b))
        || !script_path.starts_with('/')
        || script_path.starts_with("//")
        || !script_path.ends_with(".js")
        || script_path.len() > 256
        || !script_path[..script_path.len() - 3]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/_-".contains(&b))
    {
        return Err(crate::envelope::error("invalid explicit template hook"));
    }
    Ok(HtmlHook {
        markup: format!("<script nonce=\"{nonce}\" src=\"{script_path}\" defer></script>"),
        csp_source: format!("'nonce-{nonce}'"),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_without_rewriting_response() {
        let hook = html_hook("a".repeat(24), "/_humanymous/collector.js".into()).unwrap();
        assert!(hook.markup.contains("defer"));
        assert_eq!(hook.csp_source, format!("'nonce-{}'", "a".repeat(24)));
        for path in ["//external/a.js", "/x\".js", "/../x.js"] {
            assert!(html_hook("a".repeat(24), path.into()).is_err());
        }
        assert!(html_hook("\" onload=evil".into(), "/a.js".into()).is_err());
    }
}
