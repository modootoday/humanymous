use crate::network::http::HeaderInfo;
use humanymous_core::signals::wire::simple_lowercase;

pub(super) fn browser(ua: &str) -> bool {
    let ua = simple_lowercase(ua);
    ua.contains("mozilla/")
        && ["chrome", "firefox", "safari"]
            .iter()
            .any(|token| ua.contains(token))
}
pub(super) fn chromium(ua: &str) -> bool {
    simple_lowercase(ua).contains("chrome/")
}
pub(super) fn chrome(ua: &str) -> bool {
    let ua = simple_lowercase(ua);
    ua.contains("chrome") && !ua.contains("edg/")
}
fn version_after(ua: &str, token: &str) -> i64 {
    let Some((_, rest)) = ua.split_once(token) else {
        return 0;
    };
    let end = rest.bytes().take_while(u8::is_ascii_digit).count();
    if end == 0 {
        return 0;
    }
    // strconv.Atoi returns saturated MaxInt plus an error on overflow. The
    // frozen Go predicate ignores the error; preserve that observable boundary.
    rest[..end].parse().unwrap_or(i64::MAX)
}
fn versioned_browser(ua: &str, chrome: i64, firefox: i64) -> bool {
    let ua = simple_lowercase(ua);
    !ua.contains("edg/")
        && (version_after(&ua, "chrome/") >= chrome || version_after(&ua, "firefox/") >= firefox)
}
pub(super) fn pq_browser(ua: &str) -> bool {
    versioned_browser(ua, 131, 132)
}
pub(super) fn ech_browser(ua: &str) -> bool {
    versioned_browser(ua, 117, 118)
}
pub(super) fn chrome_order_anomaly(header: &HeaderInfo) -> bool {
    if !chrome(&header.user_agent) {
        return false;
    }
    let names = header.order();
    matches!((names.iter().position(|n|n == "user-agent"),names.iter().position(|n|n == "sec-ch-ua")),(Some(ua),Some(ch)) if ua < ch)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_boundaries_edge_exclusion_and_go_integer_overflow_are_preserved() {
        for (ua, pq, ech) in [
            ("Chrome/116", false, false),
            ("Chrome/117", false, true),
            ("Chrome/131", true, true),
            ("Firefox/131", false, true),
            ("Firefox/132", true, true),
            ("Chrome/149 Edg/149", false, false),
            ("Chrome/999999999999999999999999", true, true),
            ("Chrome/+149", false, false),
        ] {
            assert_eq!(pq_browser(ua), pq);
            assert_eq!(ech_browser(ua), ech);
        }
    }
}
