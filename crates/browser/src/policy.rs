//! Owned-origin navigation policy, independent of detector scoring.
use std::collections::BTreeSet;
use url::Url;

/// Capabilities requested by untrusted target content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    /// Filesystem access, including file pickers and downloads.
    Filesystem,
    /// Microphone or camera capture.
    Capture,
    /// New windows or external protocol handlers.
    Popup,
    /// Application management APIs.
    Administration,
    /// Authority credentials.
    Bearer,
    /// Arbitrary native function invocation.
    NativeBridge,
}

/// Exact owned HTTPS origins; constructor input must come from trusted config.
#[derive(Debug, Default)]
pub struct NavigationPolicy {
    origins: BTreeSet<String>,
}

impl NavigationPolicy {
    /// Reject malformed, credentialed or non-origin configuration entries.
    pub fn owned(origins: &[&str]) -> Result<Self, &'static str> {
        let mut policy = Self::default();
        for origin in origins {
            let url = checked_url(origin)?;
            if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
                return Err("configuration must contain origins only");
            }
            policy.origins.insert(url.origin().ascii_serialization());
        }
        Ok(policy)
    }

    /// Parse with the URL standard; never use prefix matching for authority.
    pub fn navigation(&self, address: &str) -> Result<Url, &'static str> {
        let url = checked_url(address)?;
        if !self.origins.contains(&url.origin().ascii_serialization()) {
            return Err("origin is not an explicitly configured owned fixture");
        }
        Ok(url)
    }

    /// This slice grants no target-content capability, even on an owned origin.
    pub fn allows(&self, _permission: Permission) -> bool {
        false
    }
}

fn checked_url(address: &str) -> Result<Url, &'static str> {
    if address.len() > 4096 || address.chars().any(char::is_control) {
        return Err("invalid address length or control character");
    }
    let url = Url::parse(address).map_err(|_| "invalid URL")?;
    if url.scheme() != "https" || url.host_str().is_none() {
        return Err("HTTPS fixture required");
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("URL credentials forbidden");
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_owned_origin_and_deny_all_privileged_capabilities() {
        let policy = NavigationPolicy::owned(&["https://fixture.invalid"]).unwrap();
        assert!(
            policy
                .navigation("https://fixture.invalid/path?q=synthetic")
                .is_ok()
        );
        for address in [
            "https://fixture.invalid.evil.invalid/",
            "https://fixture.invalid:444/",
            "https://fixture.invalid@evil.invalid/",
            "https://user@fixture.invalid/",
            "file:///C:/secret",
            "javascript:alert(1)",
            "http://fixture.invalid/",
            "https://fixture.invalid/\nadmin",
        ] {
            assert!(policy.navigation(address).is_err(), "{address}");
        }
        for permission in [
            Permission::Filesystem,
            Permission::Capture,
            Permission::Popup,
            Permission::Administration,
            Permission::Bearer,
            Permission::NativeBridge,
        ] {
            assert!(!policy.allows(permission));
        }
        assert!(
            NavigationPolicy::default()
                .navigation("https://fixture.invalid")
                .is_err()
        );
    }

    #[test]
    fn configuration_is_an_origin_not_a_path_or_credential() {
        for origin in [
            "https://fixture.invalid/a",
            "https://fixture.invalid/?a",
            "https://fixture.invalid/#a",
            "https://u:p@fixture.invalid/",
        ] {
            assert!(NavigationPolicy::owned(&[origin]).is_err());
        }
    }
}
