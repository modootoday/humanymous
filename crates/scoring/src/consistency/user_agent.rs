//! Frozen coarse User-Agent classification; not a general browser parser.

pub(crate) use humanymous_signals::wire::simple_lowercase as lowercase;

/// Claimed engine family: chrome, firefox, safari or other.
pub fn claimed_engine(user_agent: &str) -> &'static str {
    let value = lowercase(user_agent);
    if value.contains("firefox") {
        "firefox"
    } else if ["chrome", "chromium", "edg/"]
        .iter()
        .any(|token| value.contains(token))
    {
        "chrome"
    } else if value.contains("safari") && !value.contains("chrome") {
        "safari"
    } else {
        "other"
    }
}

/// Whether the User-Agent claims the Chromium family.
pub fn claims_chromium(user_agent: &str) -> bool {
    claimed_engine(user_agent) == "chrome"
}

/// Whether the string claims a browser and lacks the known library-default tokens.
pub fn is_browser_ua(user_agent: &str) -> bool {
    let value = lowercase(user_agent);
    if value.is_empty()
        || [
            "go-http-client",
            "python-requests",
            "curl/",
            "okhttp",
            "axios",
            "node-fetch",
            "java/",
            "libwww",
        ]
        .iter()
        .any(|token| value.contains(token))
    {
        return false;
    }
    value.contains("mozilla")
}

/// Unknown observed engines provide no contradiction; other observations must match.
pub fn engine_consistent(claimed: &str, observed: &str) -> bool {
    observed.is_empty() || observed == "unknown" || claimed == observed
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classification_order_and_absence_are_preserved() {
        assert_eq!(claimed_engine("Mozilla Firefox Chrome Safari"), "firefox");
        assert_eq!(claimed_engine("Mozilla Edg/ Safari"), "chrome");
        assert_eq!(claimed_engine("Mozilla CHROMİUM"), "chrome");
        assert!(!is_browser_ua("Mozilla curl/1"));
        assert!(is_browser_ua("Mozilla/5.0"));
        assert!(engine_consistent("chrome", "unknown"));
        assert!(!engine_consistent("chrome", "firefox"));
    }
}
