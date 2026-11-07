//! Finite source-reported failure categories; no raw tool text is returned.
//!
//! Literals trace to pinned wasm-bindgen-test0.3.80 rt/mod.rs and
//! wasm-bindgen-cli0.2.130 wasm_bindgen_test_runner/headless.rs.
const DRIVER_CONTEXT: &str = "webdriver POST /session request failed";
const NO_SUMMARY: &str = "Failed to detect test as having been run. It might have timed out.";
mod panic_site;

fn anyhow_context(line: &str) -> &str {
    let line = line.strip_prefix("Error: ").unwrap_or(line);
    match line.split_once(": ") {
        Some((number, context))
            if !number.is_empty()
                && number.len() <= 2
                && number.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            context
        }
        _ => line,
    }
}

pub(super) fn classify(output: &str, stderr: &str, name: &str) -> Option<&'static str> {
    let failed_test = format!("test {name} ... FAIL");
    let mut found = None;
    for line in output.lines().chain(stderr.lines()).map(str::trim) {
        let candidate = if line == failed_test {
            Some("browser_selected_test_failed")
        } else if line == NO_SUMMARY {
            Some("browser_runner_no_summary_reported")
        } else if anyhow_context(line) == DRIVER_CONTEXT {
            Some("browser_driver_session_request_failed")
        } else {
            None
        };
        if let Some(candidate) = candidate {
            if found.is_some() {
                return None;
            }
            found = Some(candidate);
        }
    }
    if found == Some("browser_selected_test_failed") {
        panic_site::classify(output, stderr, name).or(found)
    } else {
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_complete_lines_yield_only_fixed_source_reported_categories() {
        assert_eq!(
            classify("test owned::join ... FAIL", "", "owned::join"),
            Some("browser_selected_test_failed")
        );
        assert_eq!(
            classify(NO_SUMMARY, "", "owned::join"),
            Some("browser_runner_no_summary_reported")
        );
        for context in [
            DRIVER_CONTEXT.to_owned(),
            format!("Error: {DRIVER_CONTEXT}"),
            format!("   0: {DRIVER_CONTEXT}"),
            format!("Error: 12: {DRIVER_CONTEXT}"),
        ] {
            assert_eq!(
                classify("", &context, "owned::join"),
                Some("browser_driver_session_request_failed")
            );
        }
    }

    #[test]
    fn payloads_duplicate_and_ambiguous_reports_never_choose_a_cause() {
        for line in [
            format!("prefix {DRIVER_CONTEXT}"),
            format!("{DRIVER_CONTEXT} PRIVATE_BODY"),
            format!("100: {DRIVER_CONTEXT}"),
            format!("{NO_SUMMARY} PRIVATE_HEADER"),
            "test foreign::join ... FAIL".into(),
            "test owned::join ... FAILED".into(),
            "PRIVATE_PATH".into(),
        ] {
            assert_eq!(classify(&line, "", "owned::join"), None);
        }
        assert_eq!(
            classify(DRIVER_CONTEXT, DRIVER_CONTEXT, "owned::join"),
            None
        );
        assert_eq!(classify(NO_SUMMARY, DRIVER_CONTEXT, "owned::join"), None);
        assert_eq!(classify("", "", "owned::join"), None);
    }

    #[test]
    fn panic_site_refines_only_one_owned_selected_test_failure() {
        let name = "browser::transport::runtime_pilot::actual_runtime_session_report_and_required_capability_fail_closed";
        let panic = "panicked at crates/collector/src/browser/transport/runtime_pilot.rs:22:10:";
        let failed = format!("test {name} ... FAIL");
        assert_eq!(
            classify(&failed, panic, name),
            Some("browser_panic_bootstrap")
        );
        assert_eq!(classify("", panic, name), None);
        assert_eq!(
            classify(&failed, &format!("{panic} PRIVATE_BODY"), name),
            Some("browser_selected_test_failed")
        );
        assert_eq!(classify(&format!("{failed}\n{failed}"), panic, name), None);
    }
}
