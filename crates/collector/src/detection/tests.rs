use super::*;
use humanymous_core::signals::observation::{Source, Verdict, lookup};
use serde_json::json;

fn baseline() -> Snapshot {
    Snapshot {
        user_agent: "Mozilla/5.0 Chrome/156.0 Safari/537.36".into(),
        chrome_truthy: true,
        widths: Some((1200, 1100)),
        plugins: Some(5),
        languages: Some(1),
        memory: Some(32.0),
        screen: Some((900, 860)),
        getter: Getter::Native,
        iframe: Some((false, false)),
        ..Default::default()
    }
}
fn ids(s: &Snapshot) -> Vec<String> {
    detect(s)
        .unwrap()
        .signals
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect()
}
#[test]
fn benign_partial_report_never_claims_other_planes_or_session() {
    let r = detect(&baseline()).unwrap();
    assert_eq!(r.signals.as_ref().unwrap().len(), 1);
    assert_eq!(r.signals.as_ref().unwrap()[0].verdict, Verdict::Ok);
    assert_eq!(r.signals.as_ref().unwrap()[0].score, 0.0);
    assert!(!r.environment.probed);
    assert_eq!(r.behavior.events.total_events, 0);
    assert!(r.session_id.is_empty());
    assert_eq!(r.engine_version, "rust-wasm-0.1.0");
}
#[test]
fn all_seventeen_frozen_ids_values_and_confidences_use_shared_registry() {
    let s = Snapshot {
        webdriver: true,
        user_agent: "HeadlessChrome".into(),
        plugins: Some(0),
        languages: Some(0),
        notification_denied: true,
        widths: Some((100, 100)),
        cdp_flags: [true, true],
        document_keys: vec!["a_cdc_token".into()],
        playwright: [true, false, false, false],
        phantom: [false, true],
        webgl_vendor: "Google".into(),
        webgl_renderer: "SwiftShader".into(),
        memory: Some(3.0),
        screen: Some((900, 900)),
        getter: Getter::Patched,
        iframe: Some((true, false)),
        ..Default::default()
    };
    let expected = [
        ("l1.navigator.webdriver", json!(true), Verdict::Bot, 1.0),
        ("l1.ua.headless_token", json!(true), Verdict::Bot, 1.0),
        (
            "l1.navigator.plugins_empty",
            json!(0),
            Verdict::Suspicious,
            1.0,
        ),
        (
            "l1.navigator.languages_empty",
            json!(true),
            Verdict::Bot,
            1.0,
        ),
        (
            "l1.window.chrome_missing",
            json!(true),
            Verdict::Suspicious,
            1.0,
        ),
        ("l1.permissions.mismatch", json!(true), Verdict::Bot, 1.0),
        (
            "l1.window.outer_eq_inner",
            json!(true),
            Verdict::Suspicious,
            1.0,
        ),
        ("l1.cdp.runtime_enable", json!(true), Verdict::Bot, 1.0),
        ("l1.cdp.proxy_leak", json!(true), Verdict::Bot, 1.0),
        ("l1.artifact.selenium", json!(true), Verdict::Bot, 1.0),
        ("l1.artifact.playwright", json!(true), Verdict::Bot, 1.0),
        ("l1.artifact.phantom", json!(true), Verdict::Bot, 1.0),
        (
            "l2.webgl.swiftshader",
            json!("SwiftShader"),
            Verdict::Bot,
            1.0,
        ),
        (
            "l2.hardware.devicememory",
            json!(3.0),
            Verdict::Suspicious,
            0.8,
        ),
        (
            "l2.screen.metrics",
            json!({"height":900,"availHeight":900}),
            Verdict::Suspicious,
            0.5,
        ),
        (
            "l3.integrity.native_tostring",
            json!(false),
            Verdict::Bot,
            1.0,
        ),
        (
            "l3.integrity.iframe_recheck",
            json!(false),
            Verdict::Bot,
            1.0,
        ),
    ];
    let report = detect(&s).unwrap();
    let signals = report.signals.unwrap();
    assert_eq!(signals.len(), expected.len());
    for (actual, (id, value, verdict, confidence)) in signals.iter().zip(expected) {
        assert_eq!(actual.id, id);
        assert_eq!(actual.value, value);
        assert_eq!(actual.verdict, verdict);
        assert_eq!(actual.confidence, confidence);
        assert_eq!(actual.weight, lookup(id).unwrap().weight);
        assert_eq!(actual.collected, Source::Wasm);
    }
}
#[test]
fn case_zero_and_absence_do_not_become_stronger_tells() {
    let mut s = baseline();
    s.user_agent = "headlesschrome".into();
    s.chrome_truthy = false;
    s.plugins = None;
    s.languages = None;
    s.widths = Some((0, 0));
    s.screen = Some((0, 0));
    s.notification_denied = true;
    s.getter = Getter::Absent;
    s.iframe = None;
    assert_eq!(ids(&s), vec!["l1.navigator.webdriver"]);
    s.user_agent = "HeadlessEdg".into();
    assert!(ids(&s).contains(&"l1.ua.headless_token".into()));
}
#[test]
fn all_memory_boundaries_and_software_renderer_case_match_frozen_conditions() {
    let mut s = baseline();
    for value in [
        0.25,
        0.5,
        1.0,
        2.0,
        4.0,
        8.0,
        16.0,
        32.0,
        64.0,
        0.0,
        -1.0,
        f64::NAN,
    ] {
        s.memory = Some(value);
        assert!(!ids(&s).contains(&"l2.hardware.devicememory".into()));
    }
    for value in [0.125, 3.0, 6.0, 128.0] {
        s.memory = Some(value);
        assert!(ids(&s).contains(&"l2.hardware.devicememory".into()));
    }
    s.memory = None;
    for renderer in ["SwiftShader", "LLVMPIPE", "Mesa Offscreen", "SWİFTSHADER"] {
        s.webgl_renderer = renderer.into();
        assert!(ids(&s).contains(&"l2.webgl.swiftshader".into()));
    }
    s.webgl_renderer = String::new();
    s.webgl_vendor = "SwiftShader".into();
    assert!(!ids(&s).contains(&"l2.webgl.swiftshader".into()));
}
#[test]
fn artifact_enumeration_and_fresh_realm_values_are_exact() {
    let mut s = baseline();
    for marker in [
        "$cdc_",
        "cdc_",
        "__webdriver_evaluate",
        "__selenium_evaluate",
        "__webdriver_script_function",
        "__fxdriver_evaluate",
        "__driver_evaluate",
        "domAutomation",
        "domAutomationController",
    ] {
        s.window_keys = vec![format!("prefix{marker}suffix")];
        assert!(ids(&s).contains(&"l1.artifact.selenium".into()));
    }
    s.window_keys.clear();
    s.iframe = Some((true, true));
    assert!(!ids(&s).contains(&"l3.integrity.iframe_recheck".into()));
    s.iframe = Some((false, true));
    let r = detect(&s).unwrap();
    assert_eq!(r.signals.unwrap().last().unwrap().value, json!(true));
}
#[test]
fn incomplete_and_nonserializable_observations_cannot_return_empty_success() {
    assert!(detect(&Snapshot::default()).is_err());
    let mut s = baseline();
    s.memory = Some(f64::INFINITY);
    assert!(detect(&s).is_err());
}
