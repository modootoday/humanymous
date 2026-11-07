use super::super::reflection::call;
use super::*;
use wasm_bindgen_test::wasm_bindgen_test;

fn event(name: &str, constructor: &str, properties: &[(&str, JsValue)]) -> JsValue {
    let options = js_sys::Object::new();
    for (key, value) in properties {
        assert!(js_sys::Reflect::set(&options, &JsValue::from_str(key), value).unwrap());
    }
    let global: JsValue = js_sys::global().into();
    let args = js_sys::Array::new();
    args.push(&JsValue::from_str(name));
    args.push(&options);
    let constructor: Function = get(&global, constructor).unwrap().dyn_into().unwrap();
    js_sys::Reflect::construct(&constructor, &args).unwrap()
}
fn dispatch(e: &JsValue) {
    let global: JsValue = js_sys::global().into();
    call(&global, "dispatchEvent", std::slice::from_ref(e)).unwrap();
}
fn trap_property(
    event: &JsValue,
    key: &str,
    read: Rc<Cell<bool>>,
) -> Closure<dyn FnMut() -> Result<JsValue, JsValue>> {
    let trap = Closure::wrap(Box::new(move || {
        read.set(true);
        Err(error("forbidden private value read"))
    }) as Box<dyn FnMut() -> Result<JsValue, JsValue>>);
    let descriptor = js_sys::Object::new();
    assert!(js_sys::Reflect::set(&descriptor, &JsValue::from_str("get"), trap.as_ref()).unwrap());
    let global: JsValue = js_sys::global().into();
    call(
        &get(&global, "Object").unwrap(),
        "defineProperty",
        &[event.clone(), JsValue::from_str(key), descriptor.into()],
    )
    .unwrap();
    trap
}

#[wasm_bindgen_test]
fn passive_dispatch_preserves_counts_privacy_and_frozen_stop() {
    let mut collector = crate::BrowserCollector::new().unwrap();
    let read = Rc::new(Cell::new(false));
    let down = event(
        "keydown",
        "KeyboardEvent",
        &[
            ("code", JsValue::from_str("KeyA")),
            ("cancelable", JsValue::TRUE),
        ],
    );
    let _key = trap_property(&down, "key", read.clone());
    dispatch(&down);
    assert!(!get(&down, "defaultPrevented").unwrap().as_bool().unwrap());
    dispatch(&event(
        "keyup",
        "KeyboardEvent",
        &[("code", JsValue::from_str("KeyA"))],
    ));
    dispatch(&event("pointerdown", "PointerEvent", &[]));
    dispatch(&event("wheel", "WheelEvent", &[]));
    let paste = event("paste", "Event", &[]);
    let _clipboard = trap_property(&paste, "clipboardData", read.clone());
    dispatch(&paste);
    let wire = collector.behavior_json().unwrap();
    let summary: BehaviorSummary = serde_json::from_str(&wire).unwrap();
    assert_eq!(summary.events.total_events, 3);
    assert_eq!(summary.events.click_count, 1);
    assert_eq!(summary.events.no_move_clicks, 1);
    assert_eq!(summary.events.untrusted_frac, 2.0 / 3.0);
    assert_eq!(summary.key.keystrokes, 1);
    assert_eq!(summary.key.paste_events, 1);
    assert!(!read.get());
    assert!(!wire.contains("KeyA"));
    assert!(!wire.contains("token"));
    let report: serde_json::Value =
        serde_json::from_str(&collector.snapshot_json().unwrap()).unwrap();
    assert_eq!(report["behavior"]["events"]["totalEvents"], 3);
    assert_eq!(report["environment"]["probed"], false);
    assert_eq!(report["sessionId"], "");
    collector.stop().unwrap();
    let frozen = collector.behavior_json().unwrap();
    dispatch(&event("wheel", "WheelEvent", &[]));
    assert_eq!(collector.behavior_json().unwrap(), frozen);
    assert!(collector.collection.state.borrow().recorder.is_none());
    assert_eq!(collector.collection.state.borrow().codes.len(), 0);
    collector.stop().unwrap();
}

#[wasm_bindgen_test]
fn malformed_event_fails_report_but_always_detaches() {
    let mut collection = Collection::start().unwrap();
    dispatch(&event("pointermove", "Event", &[]));
    assert!(collection.summary().is_err());
    assert!(collection.stop().is_err());
    assert!(collection.state.borrow().recorder.is_none());
    assert!(collection.listeners.is_empty());
    let mut clean = Collection::start().unwrap();
    dispatch(&event("wheel", "WheelEvent", &[]));
    assert_eq!(clean.summary().unwrap().events.total_events, 1);
    clean.stop().unwrap();
}

#[wasm_bindgen_test]
fn drop_releases_callbacks_and_pairing_buffers() {
    let collection = Collection::start().unwrap();
    let state = collection.state.clone();
    drop(collection);
    assert_eq!(Rc::strong_count(&state), 1);
    assert!(state.borrow().recorder.is_none());
    dispatch(&event("wheel", "WheelEvent", &[]));
    assert_eq!(
        state.borrow().frozen.as_ref().unwrap().events.total_events,
        0
    );
}

#[wasm_bindgen_test]
fn failing_remove_is_reported_and_retained_callbacks_are_inactive() {
    let mut collection = Collection::start().unwrap();
    let native = collection.remove.clone();
    let callbacks: Vec<_> = collection
        .listeners
        .iter()
        .map(|l| (l.name, l.callback.as_ref().clone()))
        .collect();
    let fail = Closure::wrap(Box::new(|| -> Result<JsValue, JsValue> {
        Err(error("owned removal failure"))
    }) as Box<dyn FnMut() -> Result<JsValue, JsValue>>);
    collection.remove = fail.as_ref().clone().unchecked_into();
    assert!(collection.stop().is_err());
    assert!(!collection.active.get());
    assert!(collection.state.borrow().recorder.is_none());
    assert_eq!(collection.state.borrow().codes.len(), 0);
    dispatch(&event("wheel", "WheelEvent", &[]));
    assert!(collection.summary().is_err());
    // Restore actual lab listeners independently; never leave global test pollution.
    for (name, callback) in callbacks {
        apply(
            &native,
            &collection.target,
            &[JsValue::from_str(name), callback, JsValue::FALSE],
        )
        .unwrap();
    }
}

#[wasm_bindgen_test]
fn optional_coalesced_probe_failure_keeps_the_frozen_fallback() {
    let mut collection = Collection::start().unwrap();
    let read = Rc::new(Cell::new(false));
    for x in [0.0, 10.0, 20.0] {
        let e = event(
            "pointermove",
            "PointerEvent",
            &[("clientX", JsValue::from_f64(x))],
        );
        let _trap = trap_property(&e, "getCoalescedEvents", read.clone());
        dispatch(&e);
    }
    let summary = collection.summary().unwrap();
    assert!(read.get());
    assert_eq!(summary.mouse.samples, 3);
    assert_eq!(summary.mouse.coalesced_ratio, 1.0);
    collection.stop().unwrap();
}

#[wasm_bindgen_test]
fn recursive_event_dispatch_is_a_collection_error_not_a_borrow_panic() {
    let mut collection = Collection::start().unwrap();
    let e = event("pointermove", "PointerEvent", &[]);
    let nested = event("wheel", "WheelEvent", &[]);
    let getter = Closure::wrap(Box::new(move || {
        dispatch(&nested);
        JsValue::from_f64(1.0)
    }) as Box<dyn FnMut() -> JsValue>);
    let descriptor = js_sys::Object::new();
    js_sys::Reflect::set(&descriptor, &JsValue::from_str("get"), getter.as_ref()).unwrap();
    let global: JsValue = js_sys::global().into();
    call(
        &get(&global, "Object").unwrap(),
        "defineProperty",
        &[e.clone(), JsValue::from_str("clientX"), descriptor.into()],
    )
    .unwrap();
    dispatch(&e);
    assert!(collection.summary().is_err());
    assert!(collection.stop().is_err());
    assert!(collection.listeners.is_empty());
    assert!(collection.state.borrow().recorder.is_none());
}

#[wasm_bindgen_test]
fn reentrant_stop_clears_raw_state_after_the_active_callback_returns() {
    let collection = Rc::new(RefCell::new(Collection::start().unwrap()));
    let e = event("pointermove", "PointerEvent", &[]);
    let owner = collection.clone();
    let getter = Closure::wrap(Box::new(move || {
        assert!(owner.borrow_mut().stop().is_err());
        JsValue::from_f64(1.0)
    }) as Box<dyn FnMut() -> JsValue>);
    let descriptor = js_sys::Object::new();
    js_sys::Reflect::set(&descriptor, &JsValue::from_str("get"), getter.as_ref()).unwrap();
    let global: JsValue = js_sys::global().into();
    call(
        &get(&global, "Object").unwrap(),
        "defineProperty",
        &[e.clone(), JsValue::from_str("clientX"), descriptor.into()],
    )
    .unwrap();
    dispatch(&e);
    let collection = collection.borrow();
    assert!(collection.summary().is_err());
    assert!(collection.listeners.is_empty());
    assert!(collection.state.borrow().recorder.is_none());
    assert_eq!(collection.state.borrow().codes.len(), 0);
}

#[wasm_bindgen_test]
fn partial_registration_failure_removes_every_attempted_callback() {
    let global: JsValue = js_sys::global().into();
    let add: Function = get(&global, "addEventListener")
        .unwrap()
        .dyn_into()
        .unwrap();
    let remove: Function = get(&global, "removeEventListener")
        .unwrap()
        .dyn_into()
        .unwrap();
    let additions = Rc::new(Cell::new(0));
    let removals = Rc::new(Cell::new(0));
    let count = additions.clone();
    let target = global.clone();
    let native = add.clone();
    let wrapper = Closure::wrap(Box::new(
        move |name: JsValue, callback: JsValue, options: JsValue| -> Result<JsValue, JsValue> {
            count.set(count.get() + 1);
            if count.get() == 3 {
                return Err(error("owned third registration failure"));
            }
            apply(&native, &target, &[name, callback, options])
        },
    )
        as Box<dyn FnMut(JsValue, JsValue, JsValue) -> Result<JsValue, JsValue>>);
    let count = removals.clone();
    let target = global.clone();
    let native = remove.clone();
    let removal = Closure::wrap(Box::new(
        move |name: JsValue, callback: JsValue, options: JsValue| -> Result<JsValue, JsValue> {
            count.set(count.get() + 1);
            apply(&native, &target, &[name, callback, options])
        },
    )
        as Box<dyn FnMut(JsValue, JsValue, JsValue) -> Result<JsValue, JsValue>>);
    assert!(
        js_sys::Reflect::set(
            &global,
            &JsValue::from_str("addEventListener"),
            wrapper.as_ref()
        )
        .unwrap()
    );
    assert!(
        js_sys::Reflect::set(
            &global,
            &JsValue::from_str("removeEventListener"),
            removal.as_ref()
        )
        .unwrap()
    );
    let result = Collection::start();
    js_sys::Reflect::set(&global, &JsValue::from_str("addEventListener"), &add).unwrap();
    js_sys::Reflect::set(&global, &JsValue::from_str("removeEventListener"), &remove).unwrap();
    assert!(result.is_err());
    assert_eq!(additions.get(), 3);
    assert_eq!(removals.get(), 3);
    let mut clean = Collection::start().unwrap();
    clean.stop().unwrap();
}
