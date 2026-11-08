//! Parse only Root's private bootstrap unit enum's exact Debug spelling.
const PREFIX: &str = "actual runtime bootstrap: ";
const STAGES: &[(&str, &str)] = &[
    ("TimeoutProfile", "browser_bootstrap_timeout_profile"),
    ("Origin", "browser_bootstrap_origin"),
    ("AbortController", "browser_bootstrap_abort_controller"),
    ("Request", "browser_bootstrap_request"),
    ("Timer", "browser_bootstrap_timer"),
    ("FetchCall", "browser_bootstrap_fetch_call"),
    ("FetchAwait", "browser_bootstrap_fetch_await"),
    ("ResponseStatus", "browser_bootstrap_response_status"),
    ("HttpForbidden", "browser_bootstrap_http_forbidden"),
    (
        "HttpForbiddenOrigin",
        "browser_bootstrap_http_forbidden_origin",
    ),
    (
        "HttpForbiddenFetchMetadata",
        "browser_bootstrap_http_forbidden_fetch_metadata",
    ),
    ("HttpUnsupported", "browser_bootstrap_http_unsupported"),
    ("HttpClient", "browser_bootstrap_http_client"),
    ("HttpServer", "browser_bootstrap_http_server"),
    ("HttpOther", "browser_bootstrap_http_other"),
    ("Body", "browser_bootstrap_body"),
    ("TimerCancel", "browser_bootstrap_timer_cancel"),
    ("Abort", "browser_bootstrap_abort"),
    ("Clock", "browser_bootstrap_clock"),
    ("SessionScope", "browser_bootstrap_session_scope"),
    ("SessionReply", "browser_bootstrap_session_reply"),
];

pub(super) fn refine(block: &[&str], location: usize) -> Option<&'static str> {
    let mut messages = block
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim().starts_with("actual runtime bootstrap"));
    let Some((index, message)) = messages.next() else {
        return Some("browser_panic_bootstrap");
    };
    if messages.next().is_some() || index != location + 1 {
        return None;
    }
    let variant = message.trim().strip_prefix(PREFIX)?;
    STAGES
        .iter()
        .find_map(|(name, code)| (*name == variant).then_some(*code))
}

#[cfg(test)]
mod tests {
    use super::super::tests::{NAME, PATH, classify, report};
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn exact_bootstrap_variants_require_pinned_location_and_public_codes() {
        let catalog: serde_json::Value =
            serde_json::from_str(include_str!("../../../../safe-diagnostics.json")).unwrap();
        let mut variants = HashSet::new();
        let mut codes = HashSet::new();
        for (variant, code) in STAGES {
            assert!(variants.insert(variant));
            assert!(codes.insert(code));
            assert!(
                catalog["error_codes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|value| value.as_str() == Some(*code))
            );
            let body = format!("panicked at {PATH}:22:10:\n{PREFIX}{variant}");
            assert_eq!(classify(&report(&body, &body), NAME), Some(*code));
        }
    }

    #[test]
    fn unknown_ambiguous_or_unbound_bootstrap_messages_do_not_refine() {
        let location = format!("panicked at {PATH}:22:10:");
        for body in [
            format!("{location}\n{PREFIX}Unknown"),
            format!("{location}\n{PREFIX}HttpForbidden PRIVATE"),
            format!("{location}\n{PREFIX}HttpForbidden(403)"),
            format!("{location}\n{PREFIX}httpForbidden"),
            format!("{location}\nactual runtime bootstrap:HttpForbidden"),
            format!("{location}\nextra\n{PREFIX}HttpForbidden"),
            format!("{PREFIX}HttpForbidden\n{location}"),
            format!("{location}\n{PREFIX}HttpForbidden\n{PREFIX}Body"),
        ] {
            assert_eq!(classify(&report(&body, ""), NAME), None);
        }
        assert_eq!(
            classify(&report(&location, &format!("{PREFIX}HttpForbidden")), NAME),
            Some("browser_panic_bootstrap")
        );
        assert_eq!(
            classify(&report(&format!("{PREFIX}HttpForbidden"), ""), NAME),
            None
        );
    }

    #[test]
    fn forbidden_reasons_cannot_be_forged_by_payload_or_console_replay() {
        let location = format!("panicked at {PATH}:22:10:");
        for variant in ["HttpForbiddenOrigin", "HttpForbiddenFetchMetadata"] {
            for message in [
                format!("{PREFIX}{variant} PRIVATE"),
                format!("{PREFIX}{variant}(\"private\")"),
                format!("{PREFIX}{variant}Extra"),
                format!("{PREFIX}{variant}\n{PREFIX}HttpForbidden"),
            ] {
                let body = format!("{location}\n{message}");
                assert_eq!(classify(&report(&body, ""), NAME), None);
            }
            let message = format!("{PREFIX}{variant}");
            assert_eq!(
                classify(&report(&location, &message), NAME),
                Some("browser_panic_bootstrap")
            );
            let wrong_site = format!("panicked at {PATH}:18:5:\n{message}");
            assert_eq!(
                classify(&report(&wrong_site, ""), NAME),
                Some("browser_panic_owned_origin")
            );
        }
    }
}
