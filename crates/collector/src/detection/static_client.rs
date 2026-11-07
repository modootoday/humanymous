use super::{Snapshot, add};
use humanymous_signals::observation::{Signal, Verdict};
use serde_json::json;

pub(super) fn collect(s: &Snapshot, out: &mut Vec<Signal>) {
    let mut emit = |id, value, verdict, notes| add(out, id, value, verdict, 1.0, notes);
    emit(
        "l1.navigator.webdriver",
        json!(s.webdriver),
        if s.webdriver {
            Verdict::Bot
        } else {
            Verdict::Ok
        },
        "navigator.webdriver",
    );
    if s.user_agent.contains("Headless") {
        emit(
            "l1.ua.headless_token",
            json!(true),
            Verdict::Bot,
            "Headless token in UA",
        );
    }
    if s.plugins == Some(0) {
        emit(
            "l1.navigator.plugins_empty",
            json!(0),
            Verdict::Suspicious,
            "empty plugins",
        );
    }
    if s.languages == Some(0) {
        emit(
            "l1.navigator.languages_empty",
            json!(true),
            Verdict::Bot,
            "empty languages",
        );
    }
    if s.user_agent.contains("Chrome") && !s.chrome_truthy {
        emit(
            "l1.window.chrome_missing",
            json!(true),
            Verdict::Suspicious,
            "window.chrome absent for Chrome UA",
        );
    }
    if s.notification_denied && s.webdriver {
        emit(
            "l1.permissions.mismatch",
            json!(true),
            Verdict::Bot,
            "Notification vs permissions.query mismatch",
        );
    }
    if s.widths
        .is_some_and(|(outer, inner)| outer != 0 && outer == inner)
    {
        emit(
            "l1.window.outer_eq_inner",
            json!(true),
            Verdict::Suspicious,
            "outerWidth==innerWidth",
        );
    }
    if s.cdp_flags[0] {
        emit(
            "l1.cdp.runtime_enable",
            json!(true),
            Verdict::Bot,
            "CDP Runtime.enable leak",
        );
    }
    if s.cdp_flags[1] {
        emit(
            "l1.cdp.proxy_leak",
            json!(true),
            Verdict::Bot,
            "CDP prototype-Proxy preview leak",
        );
    }
    const MARKERS: [&str; 9] = [
        "$cdc_",
        "cdc_",
        "__webdriver_evaluate",
        "__selenium_evaluate",
        "__webdriver_script_function",
        "__fxdriver_evaluate",
        "__driver_evaluate",
        "domAutomation",
        "domAutomationController",
    ];
    if s.document_keys
        .iter()
        .chain(&s.window_keys)
        .any(|key| MARKERS.iter().any(|marker| key.contains(marker)))
    {
        emit(
            "l1.artifact.selenium",
            json!(true),
            Verdict::Bot,
            "selenium/cdc_ artifact present",
        );
    }
    if s.playwright.iter().any(|v| *v) {
        emit(
            "l1.artifact.playwright",
            json!(true),
            Verdict::Bot,
            "playwright global present",
        );
    }
    if s.phantom.iter().any(|v| *v) {
        emit(
            "l1.artifact.phantom",
            json!(true),
            Verdict::Bot,
            "phantomjs global present",
        );
    }
}
