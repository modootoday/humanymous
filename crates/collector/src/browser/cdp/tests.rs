use super::*;
use std::cell::RefCell;
use wasm_bindgen_test::wasm_bindgen_test;

fn realm(console: &JsValue) -> JsValue {
    let global: JsValue = Object::new().into();
    let actual: JsValue = js_sys::global().into();
    for key in ["FinalizationRegistry", "Error", "Proxy"] {
        set(&global, key, &get(&actual, key).unwrap()).unwrap();
    }
    set(&global, "console", console).unwrap();
    global
}
#[wasm_bindgen_test]
fn synthetic_stack_and_proxy_callbacks_are_actual_observations() {
    let console: JsValue = Object::new().into();
    let debug =
        Closure::<dyn Fn(JsValue) -> Result<JsValue, JsValue>>::new(|value| get(&value, "stack"));
    let preview = Closure::<dyn Fn(JsValue) -> Result<JsValue, JsValue>>::new(|value: JsValue| {
        let prototype = Reflect::get_prototype_of(value.unchecked_ref::<Object>())?;
        Ok(Reflect::own_keys(&prototype)?.into())
    });
    set(&console, "debug", debug.as_ref()).unwrap();
    set(&console, "groupEnd", preview.as_ref()).unwrap();
    let observation = Observation::with_global(&realm(&console));
    assert_eq!(observation.flags(), Some([true, true]));
    let output: serde_json::Value = serde_json::from_str(&observation.json()).unwrap();
    assert_eq!(output["qualified"], false);
    assert_eq!(output["errorStack"]["status"], "attempted");
}
#[wasm_bindgen_test]
fn missing_and_throwing_console_methods_are_not_negative_probe_success() {
    let console: JsValue = Object::new().into();
    let global = realm(&console);
    let unsupported = Observation::with_global(&global);
    assert_eq!(unsupported.stack.status, "unsupported");
    assert_eq!(unsupported.flags(), None);
    let mut collector = crate::BrowserCollector::new().unwrap();
    collector.cdp = Some(unsupported);
    assert!(collector.snapshot_json().is_err());
    collector.stop().unwrap();
    let fail = Closure::<dyn Fn(JsValue) -> Result<JsValue, JsValue>>::new(|_| {
        Err(JsValue::from_str("private failure fixture"))
    });
    set(&console, "debug", fail.as_ref()).unwrap();
    set(&console, "groupEnd", fail.as_ref()).unwrap();
    let failed = Observation::with_global(&global);
    assert_eq!(failed.stack.status, "failed");
    assert_eq!(failed.prototype.status, "failed");
    assert_eq!(failed.flags(), None);
    assert!(!failed.json().contains("private failure fixture"));
    set(&global, "FinalizationRegistry", &JsValue::UNDEFINED).unwrap();
    assert_eq!(
        Observation::with_global(&global).stack.status,
        "unsupported"
    );
}
#[wasm_bindgen_test]
fn deferred_console_serialization_keeps_managed_callbacks_alive_but_inert() {
    let captured = Rc::new(RefCell::new(Vec::<JsValue>::new()));
    let values = captured.clone();
    let logger = Closure::<dyn Fn(JsValue)>::new(move |value| values.borrow_mut().push(value));
    let console: JsValue = Object::new().into();
    set(&console, "debug", logger.as_ref()).unwrap();
    set(&console, "groupEnd", logger.as_ref()).unwrap();
    let observation = Observation::with_global(&realm(&console));
    assert_eq!(observation.flags(), Some([false, false]));
    let values = captured.borrow();
    assert_eq!(
        get(&values[0], "stack").unwrap().as_string().as_deref(),
        Some("")
    );
    let prototype = Reflect::get_prototype_of(values[1].unchecked_ref::<Object>()).unwrap();
    assert_eq!(Reflect::own_keys(&prototype).unwrap().length(), 0);
    assert_eq!(observation.flags(), Some([false, false]));
}
#[wasm_bindgen_test]
fn actual_collector_probe_is_explicit_cached_and_cannot_run_after_stop() {
    let mut collector = crate::BrowserCollector::new().unwrap();
    assert!(collector.cdp.is_none());
    let first = collector.probe_cdp_json().unwrap();
    assert_eq!(collector.probe_cdp_json().unwrap(), first);
    let output: serde_json::Value = serde_json::from_str(&first).unwrap();
    assert_eq!(output["qualified"], false);
    // Actual console behavior is observed, not assumed to indicate CDP or absence.
    if collector.cdp.as_ref().unwrap().flags().is_some() {
        assert!(collector.snapshot_json().is_ok());
    }
    collector.stop().unwrap();
    assert!(collector.probe_cdp_json().is_err());
}
