//! Exact local/internal routes. Host preflight must run before CEF LoadURL.
use url::Url;

/// Owner document, unrelated to Chrome's own new-tab implementation.
pub const NEW_TAB_URL: &str = "humanymous://newtab/";
/// Individually bounded, harmless diagnostic candidates for the pinned CEF tuple.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InternalPage {
    /// Engine/build version; raw paths are private.
    Version,
    /// Graphics diagnostics; device data is private.
    Gpu,
    /// Third-party notices.
    Credits,
    /// Engine license.
    License,
    /// Sandbox diagnostics; not a full OS policy attestation.
    Sandbox,
}
impl InternalPage {
    /// Curated candidate matrix; policy may narrow after exact-build validation.
    pub const ALL: [Self; 5] = [
        Self::Version,
        Self::Gpu,
        Self::Credits,
        Self::License,
        Self::Sandbox,
    ];
    /// Fixed canonical address. Never compose a host-controlled URL string.
    pub fn url(self) -> &'static str {
        match self {
            Self::Version => "chrome://version/",
            Self::Gpu => "chrome://gpu/",
            Self::Credits => "chrome://credits/",
            Self::License => "chrome://license/",
            Self::Sandbox => "chrome://sandbox/",
        }
    }
    /// Safe display name, independent of untrusted page text.
    pub fn label(self) -> &'static str {
        match self {
            Self::Version => "Version",
            Self::Gpu => "Graphics",
            Self::Credits => "Credits",
            Self::License => "License",
            Self::Sandbox => "Sandbox",
        }
    }
    /// Same-owner link action, classified before any privileged navigation.
    pub fn action(self) -> &'static str {
        match self {
            Self::Version => "/open/version",
            Self::Gpu => "/open/gpu",
            Self::Credits => "/open/credits",
            Self::License => "/open/license",
            Self::Sandbox => "/open/sandbox",
        }
    }
}
/// Only these typed routes may cross the trusted browser-initiated LoadURL boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// App-owned script-free landing page.
    NewTab,
    /// An individually allowlisted engine page.
    Internal(InternalPage),
}
impl Route {
    /// Canonical trusted address.
    pub fn url(self) -> &'static str {
        match self {
            Self::NewTab => NEW_TAB_URL,
            Self::Internal(page) => page.url(),
        }
    }
    /// Safe tab title derived from current committed route, never readiness.
    pub fn title(self) -> &'static str {
        match self {
            Self::NewTab => "New tab",
            Self::Internal(page) => page.label(),
        }
    }
}

/// Reject all variants except fixed route addresses and their slashless spelling.
pub fn classify(address: &str) -> Result<Route, &'static str> {
    if address.len() > 4096 || address.chars().any(char::is_control) {
        return Err("invalid address bounds");
    }
    if address == NEW_TAB_URL || address == "humanymous://newtab" {
        return Ok(Route::NewTab);
    }
    for page in InternalPage::ALL {
        if address == page.url() || address == page.url().trim_end_matches('/') {
            return Ok(Route::Internal(page));
        }
    }
    Err("Only the local new tab and admitted internal diagnostics are available")
}

/// Resolve a form/link action only when its caller separately verifies current
/// main-frame newtab origin and a user gesture. Never dispatch arbitrary schemes.
pub fn newtab_action(address: &str) -> Result<Route, &'static str> {
    let url = Url::parse(address).map_err(|_| "invalid newtab action")?;
    if url.scheme() != "humanymous"
        || url.host_str() != Some("newtab")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
        || address.len() > 4096
    {
        return Err("invalid newtab action authority");
    }
    for page in InternalPage::ALL {
        if address == format!("humanymous://newtab{}", page.action()) {
            return Ok(Route::Internal(page));
        }
    }
    if url.path() != "/open/" || !address.starts_with("humanymous://newtab/open/?") {
        return Err("unknown newtab action");
    }
    let pairs: Vec<_> = url.query_pairs().collect();
    if pairs.len() != 1 || pairs[0].0 != "address" {
        return Err("one address field required");
    }
    classify(&pairs[0].1)
}

/// Trusted WebUI assets are subresources only, scoped to an admitted main page.
pub fn resource_allowed(address: &str, navigation: bool, current: Option<Route>) -> bool {
    if navigation {
        return classify(address).is_ok();
    }
    let Some(Route::Internal(page)) = current else {
        return false;
    };
    let Ok(url) = Url::parse(address) else {
        return false;
    };
    if address.len() > 4096
        || address.chars().any(char::is_control)
        || url.scheme() != "chrome"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
    {
        return false;
    }
    let page_host = page
        .url()
        .trim_start_matches("chrome://")
        .trim_end_matches('/');
    matches!(url.host_str(), Some("resources" | "theme")) || url.host_str() == Some(page_host)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_routes_never_admit_privileged_aliases_or_credentials() {
        for page in InternalPage::ALL {
            assert_eq!(classify(page.url()), Ok(Route::Internal(page)));
        }
        for address in [
            "chrome://quit/",
            "chrome://restart/",
            "chrome://crash/",
            "chrome://hang/",
            "chrome://flags/",
            "chrome://settings/",
            "chrome://chrome-urls/",
            "chrome://inspect/",
            "chrome://resources/",
            "chrome://newtab/",
            "chrome://u@version/",
            "chrome://version:123/",
            "chrome://version/?q=1",
            "chrome://version/#x",
            "chrome://version/../quit",
            "chrome://%76ersion/",
            "about:version",
            "file:///C:/x",
            "https://example.invalid/",
            "javascript:void(0)",
            "data:text/html,x",
            "chrome://version/\n",
        ] {
            assert!(classify(address).is_err(), "{address}");
        }
    }
    #[test]
    fn assets_do_not_become_navigation_or_cross_origin_capabilities() {
        let page = Some(Route::Internal(InternalPage::Gpu));
        assert!(resource_allowed("chrome://resources/js/cr.js", false, page));
        assert!(resource_allowed(
            "chrome://gpu/gpu_internals.js",
            false,
            page
        ));
        assert!(!resource_allowed("chrome://resources/js/cr.js", true, page));
        assert!(!resource_allowed(
            "chrome://resources/js/cr.js",
            false,
            Some(Route::NewTab)
        ));
        for url in [
            "https://example.invalid/script.js",
            "file:///x",
            "chrome://settings/a.js",
            "chrome-untrusted://gpu/x",
        ] {
            assert!(!resource_allowed(url, false, page));
        }
    }
    #[test]
    fn native_form_and_links_only_resolve_exact_actions() {
        assert_eq!(
            newtab_action("humanymous://newtab/open/version"),
            Ok(Route::Internal(InternalPage::Version))
        );
        assert_eq!(
            newtab_action("humanymous://newtab/open/?address=chrome%3A%2F%2Fgpu"),
            Ok(Route::Internal(InternalPage::Gpu))
        );
        for url in [
            "humanymous://newtab/open/?address=chrome%3A%2F%2Fquit",
            "humanymous://newtab/open/?address=chrome%3A%2F%2Fgpu&address=x",
            "humanymous://evil/open/version",
            "humanymous://newtab/open/version?q=1",
            "humanymous://newtab/open/../open/version",
        ] {
            assert!(newtab_action(url).is_err(), "{url}");
        }
    }
}
