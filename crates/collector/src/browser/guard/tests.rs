use super::*;
use js_sys::{Object, Reflect};
use wasm_bindgen::closure::Closure;
use wasm_bindgen_test::wasm_bindgen_test;

fn set(object: &JsValue, key: &str, value: &JsValue) {
    assert!(Reflect::set(object, &key.into(), value).unwrap());
}
fn fixture() -> (JsValue, JsValue) {
    let actual: JsValue = js_sys::global().into();
    let global: JsValue = Object::new().into();
    let prototype: JsValue = Object::new().into();
    let object: JsValue = Object::new().into();
    set(&object, "prototype", &prototype);
    set(
        &object,
        "getOwnPropertyNames",
        &get(&get(&actual, "Object").unwrap(), "getOwnPropertyNames").unwrap(),
    );
    let array: JsValue = Object::new().into();
    set(&array, "prototype", &Object::new().into());
    set(&global, "Object", &object);
    set(&global, "Array", &array);
    for key in ["Function", "fetch", "XMLHttpRequest", "JSON", "console"] {
        set(&global, key, &get(&actual, key).unwrap());
    }
    (global, prototype)
}

#[wasm_bindgen_test]
fn guard_native_baseline_and_read_time_counts_never_fabricate_dynamic_coverage() {
    let (global, prototype) = fixture();
    set(&global, "fetch", &JsValue::FALSE);
    set(&global, "console", &JsValue::UNDEFINED);
    let mut baseline = Baseline::capture_with(global.clone()).unwrap();
    let first = baseline.snapshot().unwrap();
    assert!(
        !first.reported && first.native_hooked && first.console_disabled && !first.proto_polluted
    );
    set(&prototype, "owned_count_change", &JsValue::TRUE);
    assert!(baseline.snapshot().unwrap().proto_polluted);
    assert!(
        Reflect::delete_property(
            prototype.unchecked_ref::<Object>(),
            &"owned_count_change".into()
        )
        .unwrap()
    );
    assert!(!baseline.snapshot().unwrap().proto_polluted);
    baseline.stop().unwrap();
    // Frozen guard reads do not touch newly broken global acquisition methods.
    set(&global, "Object", &JsValue::UNDEFINED);
    assert_eq!(baseline.snapshot().unwrap(), first);
    baseline.stop().unwrap();
}

#[wasm_bindgen_test]
fn malformed_guard_enumeration_and_native_text_fail_capture() {
    let (global, _) = fixture();
    set(&global, "Object", &JsValue::UNDEFINED);
    assert!(Baseline::capture_with(global).is_err());
    let (global, _) = fixture();
    let to_string =
        Closure::<dyn FnMut() -> JsValue>::new(|| JsValue::from_str(&"x".repeat(32769)));
    let prototype: JsValue = Object::new().into();
    set(&prototype, "toString", to_string.as_ref());
    let function: JsValue = Object::new().into();
    set(&function, "prototype", &prototype);
    set(&global, "Function", &function);
    assert!(Baseline::capture_with(global.clone()).is_err());
    set(&global, "Function", &JsValue::UNDEFINED);
}

#[wasm_bindgen_test]
fn actual_collector_snapshot_carries_only_partial_guard_measurement() {
    let mut collector = crate::BrowserCollector::new().unwrap();
    let report: humanymous_core::signals::report::ClientReport =
        serde_json::from_str(&collector.snapshot_json().unwrap()).unwrap();
    assert!(!report.guard.reported);
    assert_eq!(
        (
            report.guard.eval_used,
            report.guard.func_ctor,
            report.guard.script_injected
        ),
        (0, 0, 0)
    );
    collector.stop().unwrap();
    assert!(!collector.guard.snapshot().unwrap().reported);
}

#[wasm_bindgen_test]
fn native_marker_matches_legacy_browser_regexp_on_character_boundaries() {
    // A regular-expression data pattern, not executable JavaScript source.
    let legacy = js_sys::RegExp::new(r"\{\s*\[native code\]\s*\}", "");
    let codes = (0..=255).chain(0x2000..=0x200b).chain([
        0x1680, 0x180e, 0x2028, 0x2029, 0x202f, 0x205f, 0x3000, 0xfeff,
    ]);
    for code in codes {
        let c = char::from_u32(code).unwrap();
        let text = format!("prefix{{{c}[native code]{c}}}suffix");
        assert_eq!(
            native_source_marker(&text),
            legacy.test(&text),
            "codepoint {code}"
        );
    }
}

fn script(document: &JsValue, marker: Option<&str>) -> JsValue {
    let script = call(document, "createElement", &["script".into()]).unwrap();
    // Inert MIME type and no content/src: observe structure without running code.
    set(&script, "type", &"application/octet-stream".into());
    if let Some(marker) = marker {
        set(&get(&script, "dataset").unwrap(), "hm", &marker.into());
    }
    script
}

#[wasm_bindgen_test(async)]
async fn actual_mutations_count_only_direct_unmarked_scripts_and_stop_drains() {
    let global: JsValue = js_sys::global().into();
    let document = get(&global, "document").unwrap();
    let body = get(&document, "body").unwrap();
    let mut baseline = Baseline::capture().unwrap();
    let container = call(&document, "createElement", &["div".into()]).unwrap();
    // A detached descendant is not independently added to an observed target.
    call(&container, "appendChild", &[script(&document, None)]).unwrap();
    call(&body, "appendChild", std::slice::from_ref(&container)).unwrap();
    let mut nodes = vec![container];
    for marker in [None, Some(""), Some("owned")] {
        let node = script(&document, marker);
        call(&body, "appendChild", std::slice::from_ref(&node)).unwrap();
        nodes.push(node);
    }
    wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(&JsValue::UNDEFINED))
        .await
        .unwrap();
    assert_eq!(baseline.snapshot().unwrap().script_injected, 2);
    assert!(!baseline.snapshot().unwrap().reported);
    let pending = script(&document, None);
    call(&body, "appendChild", std::slice::from_ref(&pending)).unwrap();
    nodes.push(pending);
    baseline.stop().unwrap();
    assert_eq!(baseline.snapshot().unwrap().script_injected, 3);
    let frozen = baseline.snapshot().unwrap();
    let after = script(&document, None);
    call(&body, "appendChild", std::slice::from_ref(&after)).unwrap();
    nodes.push(after);
    wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(&JsValue::UNDEFINED))
        .await
        .unwrap();
    baseline.stop().unwrap();
    assert_eq!(baseline.snapshot().unwrap(), frozen);
    for node in nodes {
        call(&body, "removeChild", &[node]).unwrap();
    }
}

#[wasm_bindgen_test(async)]
async fn dropped_guard_detaches_and_failed_registration_is_rejected() {
    let global: JsValue = js_sys::global().into();
    let document = get(&global, "document").unwrap();
    let body = get(&document, "body").unwrap();
    let baseline = Baseline::capture().unwrap();
    let epoch = baseline.epoch.clone();
    let initial = baseline.initial;
    drop(baseline);
    let node = script(&document, None);
    call(&body, "appendChild", std::slice::from_ref(&node)).unwrap();
    wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(&JsValue::UNDEFINED))
        .await
        .unwrap();
    assert_eq!(epoch.borrow().snapshot(initial).unwrap().script_injected, 0);
    call(&body, "removeChild", &[node]).unwrap();
    let fake: JsValue = Object::new().into();
    set(
        &fake,
        "MutationObserver",
        &get(&global, "MutationObserver").unwrap(),
    );
    set(&fake, "document", &Object::new().into());
    assert!(observer::Observer::start(&fake, epoch, Rc::new(Cell::new(false))).is_err());
}
