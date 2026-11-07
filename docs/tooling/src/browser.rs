//! Hosted WebDriver keyboard pilot; no execute-script or authored JS payloads.
//! Protocol: https://www.w3.org/TR/webdriver2/ (structured commands only).
use serde::Serialize;
use serde_json::{Value, json};

use std::{
    path::Path,
    time::{Duration, Instant},
};

mod webdriver;
use webdriver::{Driver, local_url, page, segment, served_artifacts};

/// One measured page/viewport; failure never becomes a skipped success.
#[derive(Serialize)]
pub struct Case {
    page: String,
    window_width: u32,
    window_height: u32,
    passed: bool,
    error: Option<String>,
    measurements: Value,
}

/// Keyboard/DOM/CSS evidence only, not screen-reader or WCAG certification.
#[derive(Serialize)]
pub struct BrowserReport {
    scope: &'static str,
    status: &'static str,
    full_accessibility_qualified: bool,
    build_input_sha256: String,
    browser_name: String,
    browser_version: String,
    cases: Vec<Case>,
    expected_cases: usize,
    performed_cases: usize,
    not_run: usize,
    passed: usize,
    failed: usize,
    session_deleted: bool,
    input_unchanged: bool,
    served_artifacts_verified: usize,
    served_artifacts_sha256: String,
    errors: Vec<String>,
    assistive_technology: &'static str,
}
impl BrowserReport {
    /// All required measurements and cleanup must succeed.
    pub fn passed(&self) -> bool {
        !self.cases.is_empty()
            && self.cases.len() == self.expected_cases
            && self.passed == self.expected_cases
            && self.performed_cases == self.expected_cases
            && self.not_run == 0
            && self.failed == 0
            && self.session_deleted
            && self.input_unchanged
            && self.served_artifacts_verified > 0
            && self.errors.is_empty()
    }
}

/// Run against caller-owned hosted services; never starts a browser/server locally.
/// `site` is the current built output, `base` its literal-loopback serving prefix.
pub fn run(
    root: &Path,
    site: &Path,
    baseurl: &str,
    endpoint: &str,
    base: &str,
) -> Result<BrowserReport, String> {
    let mut endpoint = local_url(endpoint)?;
    if !endpoint.path().ends_with('/') {
        endpoint.set_path(&format!("{}/", endpoint.path()));
    }
    let base = local_url(base)?;
    if base.path() != format!("{baseurl}/") {
        return Err("served URL prefix must match the checked baseurl".into());
    }
    let output = crate::check_output(root, site, baseurl)?;
    if !output.errors.is_empty() {
        return Err("browser pilot requires a passing current output check".into());
    }
    let expected_cases = output
        .rendered_html
        .ok_or("missing rendered denominator")?
        .checked_mul(2)
        .ok_or("browser case denominator overflow")?;
    let before = crate::inputs::digest(root)?;
    let driver = Driver {
        endpoint,
        deadline: Instant::now() + Duration::from_secs(900),
        agent: ureq::Agent::config_builder()
            .proxy(None)
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(20)))
            .build()
            .into(),
    };
    let (served_count, served_digest) = served_artifacts(&driver, &base, site)?;
    let created = driver.command(
        "POST",
        "session",
        json!({"capabilities":{"alwaysMatch":{
            "browserName":"firefox","moz:firefoxOptions":{"args":["-headless"],"prefs":{
                "browser.cache.disk.enable":false,"browser.cache.memory.enable":false
            }},
            "timeouts":{"implicit":0,"pageLoad":10000,"script":0}
        }}}),
    )?;
    let session = segment(&created, "sessionId")?;
    let mut report = BrowserReport {
        scope: "pages-keyboard-component",
        status: "failed",
        full_accessibility_qualified: false,
        build_input_sha256: before.clone(),
        browser_name: created["capabilities"]["browserName"]
            .as_str()
            .unwrap_or("")
            .into(),
        browser_version: created["capabilities"]["browserVersion"]
            .as_str()
            .unwrap_or("")
            .into(),
        cases: Vec::new(),
        expected_cases,
        performed_cases: 0,
        not_run: expected_cases,
        passed: 0,
        failed: 0,
        session_deleted: false,
        input_unchanged: false,
        served_artifacts_verified: served_count,
        served_artifacts_sha256: served_digest.clone(),
        errors: Vec::new(),
        assistive_technology: "NOT_RUN",
    };
    let result = (|| -> Result<(), String> {
        let mut paths = Vec::new();
        for path in crate::files::walk(site, false)? {
            if path.extension().and_then(|v| v.to_str()) == Some("html") {
                let relative = crate::files::relative(site, &path);
                paths.push(
                    crate::files::rendered_url(&relative)
                        .trim_start_matches('/')
                        .to_owned(),
                );
            }
        }
        for path in paths {
            let url = base.join(&path).map_err(|_| "invalid rendered URL")?;
            for (width, height) in [(1280, 900), (390, 844)] {
                if Instant::now() >= driver.deadline {
                    continue;
                }
                let mut measurements = json!({"skip_css":{}});
                let result = page(&driver, &session, &url, width, height, &mut measurements);
                let passed = result.is_ok();
                report.performed_cases += 1;
                report.passed += usize::from(passed);
                report.failed += usize::from(!passed);
                report.cases.push(Case {
                    page: path.clone(),
                    window_width: width,
                    window_height: height,
                    passed,
                    error: result.err(),
                    measurements,
                });
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        report.errors.push(error);
    }
    report.not_run = report.expected_cases.saturating_sub(report.performed_cases);
    if report.not_run > 0 {
        report
            .errors
            .push("required browser cases were not run".into());
    }
    report.session_deleted = driver
        .command("DELETE", &format!("session/{session}"), Value::Null)
        .is_ok();
    if !report.session_deleted {
        report
            .errors
            .push("WebDriver session deletion failed".into());
    }
    if !served_artifacts(&driver, &base, site)
        .is_ok_and(|after| after == (served_count, served_digest))
    {
        report
            .errors
            .push("served output changed during browser execution".into());
    }
    report.input_unchanged = crate::inputs::digest(root).is_ok_and(|after| after == before);
    if !report.input_unchanged {
        report
            .errors
            .push("authored inputs changed during browser execution".into());
    }
    if report.browser_name != "firefox" || report.browser_version.is_empty() {
        report
            .errors
            .push("missing or unexpected actual browser identity".into());
    }
    report.status = if report.passed() { "passed" } else { "failed" };
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::webdriver::usable_css;
    use super::*;

    #[test]
    fn driver_and_site_require_literal_local_urls_without_ambient_authority() {
        for valid in ["http://127.0.0.1:4444/", "http://[::1]:8080/humanymous/"] {
            assert!(local_url(valid).is_ok(), "{valid}");
        }
        for invalid in [
            "https://127.0.0.1/",
            "http://localhost/",
            "http://example.test/",
            "http://127.0.0.1@192.0.2.1/",
            "http://user:secret@127.0.0.1/",
            "http://127.0.0.1/?x=1",
            "http://127.0.0.1/#fragment",
        ] {
            assert!(local_url(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn driver_id_cannot_change_command_path_or_query() {
        assert_eq!(
            segment(&json!({"id":"f.ab.d.cd.e.1"}), "id").unwrap(),
            "f.ab.d.cd.e.1"
        );
        for invalid in ["", "../session", "a/b", "a?x", "a#fragment", "a%2fb", "é"] {
            assert!(segment(&json!({"id":invalid}), "id").is_err());
        }
        assert!(segment(&json!({"id":null}), "id").is_err());
    }

    #[test]
    fn css_measurements_reject_invisible_and_malformed_computed_values() {
        for (property, value) in [
            ("display", "block"),
            ("visibility", "visible"),
            ("opacity", "0.5"),
        ] {
            assert!(usable_css(property, &json!(value)));
        }
        for (property, value) in [
            ("display", "none"),
            ("visibility", "collapse"),
            ("opacity", "0.0"),
            ("opacity", "NaN"),
            ("opacity", "-1"),
            ("opacity", "2"),
            ("display", ""),
        ] {
            assert!(!usable_css(property, &json!(value)));
        }
        assert!(!usable_css("opacity", &Value::Null));
    }

    #[test]
    fn report_cannot_accept_missing_cases_cleanup_or_changed_inputs() {
        let mut report = BrowserReport {
            scope: "pages-keyboard-component",
            status: "failed",
            full_accessibility_qualified: false,
            build_input_sha256: "fixture".into(),
            browser_name: "firefox".into(),
            browser_version: "fixture".into(),
            cases: vec![Case {
                page: "fixture".into(),
                window_width: 1280,
                window_height: 900,
                passed: true,
                error: None,
                measurements: json!({}),
            }],
            expected_cases: 1,
            performed_cases: 1,
            not_run: 0,
            passed: 1,
            failed: 0,
            session_deleted: true,
            input_unchanged: true,
            served_artifacts_verified: 1,
            served_artifacts_sha256: "fixture".into(),
            errors: vec![],
            assistive_technology: "NOT_RUN",
        };
        assert!(report.passed());
        report.performed_cases = 0;
        report.not_run = 1;
        assert!(!report.passed());
        report.performed_cases = 1;
        report.not_run = 0;
        report.expected_cases = 2;
        assert!(!report.passed());
        report.expected_cases = 1;
        report.session_deleted = false;
        assert!(!report.passed());
        report.session_deleted = true;
        report.input_unchanged = false;
        assert!(!report.passed());
        report.input_unchanged = true;
        report
            .errors
            .push("fixture: post-execution artifact mismatch".into());
        assert!(!report.passed());
        assert!(!report.full_accessibility_qualified);
    }
}
