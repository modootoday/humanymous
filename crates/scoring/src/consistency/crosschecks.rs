use super::user_agent::{claimed_engine, claims_chromium, engine_consistent, is_browser_ua};
use humanymous_signals::{
    report::{CrossCheck, SessionReport},
    wire::{quote_go, simple_lowercase},
};

const WEIGHTS: [(&str, f64); 7] = [
    ("x.ua_vs_ja4", 50.0),
    ("x.ua_vs_h2", 45.0),
    ("x.uach_present", 40.0),
    ("x.uach_vs_js", 35.0),
    ("x.ua_vs_header", 35.0),
    ("x.non_browser_ua", 45.0),
    ("x.browser_no_js", 45.0),
];

fn make_check(
    id: &str,
    inputs: &[&str],
    claim: &str,
    observed: &str,
    consistent: bool,
    notes: &str,
) -> CrossCheck {
    let weight = WEIGHTS
        .iter()
        .find(|(key, _)| *key == id)
        .map(|(_, weight)| *weight)
        .unwrap_or_default();
    CrossCheck {
        id: id.into(),
        inputs: Some(inputs.iter().map(|value| (*value).into()).collect()),
        claim: claim.into(),
        observed: observed.into(),
        consistent,
        weight,
        score: if consistent { 0.0 } else { weight },
        notes: notes.into(),
    }
}

fn hints_match(report: &SessionReport) -> bool {
    let platform = simple_lowercase(report.client.ua_client_hints.platform.trim());
    if platform.is_empty() {
        return false;
    }
    let platform = platform.trim_matches('"');
    let user_agent = simple_lowercase(&report.client.user_agent);
    if user_agent.contains("android") {
        platform == "android"
    } else if ["iphone", "ipad", "ipod"]
        .iter()
        .any(|token| user_agent.contains(token))
    {
        platform == "ios" || platform == "macos"
    } else if user_agent.contains("windows") {
        platform == "windows"
    } else if user_agent.contains("mac os") || user_agent.contains("macintosh") {
        platform == "macos" || platform == "mac os"
    } else if user_agent.contains("cros") || user_agent.contains("chromium os") {
        platform == "chrome os" || platform == "chromium os"
    } else if user_agent.contains("linux") {
        platform == "linux" || platform == "chrome os" || platform == "chromium os"
    } else {
        true
    }
}

/// Derive the frozen ordered L6 checks from claimed identity and independent evidence.
pub fn cross_checks(report: &SessionReport) -> Vec<CrossCheck> {
    let user_agent = &report.client.user_agent;
    let claimed = claimed_engine(user_agent);
    let mut result = Vec::new();
    if !user_agent.is_empty() && !is_browser_ua(user_agent) {
        result.push(make_check(
            "x.non_browser_ua",
            &["client.userAgent"],
            "UA looks like a browser",
            "UA matches a known HTTP library",
            false,
            &format!("UA={} not a browser", quote_go(user_agent)),
        ));
    }
    if is_browser_ua(user_agent) {
        let no_js = report.client.signals.as_ref().is_none_or(Vec::is_empty)
            && !report.client.advanced.probed
            && report.client.behavior.mouse.samples == 0
            && report.client.behavior.events.total_events == 0;
        result.push(make_check(
            "x.browser_no_js",
            &[
                "client.userAgent",
                "client.signals",
                "client.advanced",
                "client.behavior",
            ],
            "browser UA implies JS/WASM execution",
            "no client-side execution evidence delivered",
            !no_js,
            "browser UA but zero JS/WASM execution evidence (HTTP parrot)",
        ));
    }
    let has_browser_claim = matches!(claimed, "chrome" | "firefox" | "safari");
    if has_browser_claim && !report.network.ja4_engine.is_empty() {
        result.push(make_check(
            "x.ua_vs_ja4",
            &["client.userAgent", "network.ja4"],
            &format!("UA={claimed}"),
            &format!("JA4 engine={}", report.network.ja4_engine),
            engine_consistent(claimed, &report.network.ja4_engine),
            "TLS stack vs claimed browser",
        ));
    }
    if has_browser_claim && !report.network.h2_engine.is_empty() {
        result.push(make_check(
            "x.ua_vs_h2",
            &["client.userAgent", "network.akamaiH2"],
            &format!("UA={claimed}"),
            &format!("H2 engine={}", report.network.h2_engine),
            engine_consistent(claimed, &report.network.h2_engine),
            "HTTP/2 profile vs claimed browser",
        ));
    }
    if claims_chromium(user_agent) {
        result.push(make_check(
            "x.uach_present",
            &["client.userAgent", "network.secChUa"],
            "Chromium always sends Sec-CH-UA",
            "Sec-CH-UA present",
            report.network.sec_ch_ua_present,
            "UA-CH presence",
        ));
        result.push(make_check(
            "x.ua_vs_header",
            &["client.userAgent", "network.secFetch"],
            "Chromium sends sec-fetch-*",
            "sec-fetch present",
            report.network.sec_fetch_present,
            "sec-fetch presence",
        ));
    }
    if report
        .client
        .ua_client_hints
        .brands
        .as_ref()
        .is_some_and(|brands| !brands.is_empty())
        && report.network.sec_ch_ua_present
    {
        result.push(make_check(
            "x.uach_vs_js",
            &["network.secChUa", "client.uaClientHints"],
            "header UA-CH",
            "JS userAgentData",
            hints_match(report),
            "UA-CH header vs JS",
        ));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strings_and_environment_claims_do_not_replace_execution_evidence() {
        let mut report = SessionReport::default();
        report.client.user_agent = "Mozilla Chrome Windows".into();
        report.client.engine_version = "120".into();
        report.client.environment.probed = true;
        assert!(
            !cross_checks(&report)
                .iter()
                .find(|check| check.id == "x.browser_no_js")
                .unwrap()
                .consistent
        );
        report.client.advanced.probed = true;
        assert!(
            cross_checks(&report)
                .iter()
                .find(|check| check.id == "x.browser_no_js")
                .unwrap()
                .consistent
        );
    }
}
