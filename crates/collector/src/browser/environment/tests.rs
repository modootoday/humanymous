use super::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::closure::Closure;
use wasm_bindgen_test::wasm_bindgen_test;

fn fake_global(real: &JsValue) -> JsValue {
    let fake: JsValue = Object::new().into();
    for name in ["setTimeout", "clearTimeout", "getComputedStyle"] {
        let bound = call(
            &get(real, name).unwrap(),
            "bind",
            std::slice::from_ref(real),
        )
        .unwrap();
        set(&fake, name, &bound).unwrap();
    }
    for name in ["document", "AbortController"] {
        set(&fake, name, &get(real, name).unwrap()).unwrap();
    }
    fake
}

#[wasm_bindgen_test(async)]
async fn environment_export_observes_owned_browser_and_first_party_http() {
    // The Docker component runner owns this loopback origin; a missing honeypot
    // resolves as HTTP404 and must not be labeled a network/content-block failure.
    let report: Environment =
        serde_json::from_str(&crate::environment_json().await.unwrap()).unwrap();
    assert!(report.probed);
    assert!(!report.tracking_protection && !report.collector_blocked);
}

#[wasm_bindgen_test(async)]
async fn environment_privacy_bait_fetch_and_timeout_cleanup() {
    let real: JsValue = js_sys::global().into();
    let fake = fake_global(&real);
    let document = get(&real, "document").unwrap();
    let count = || {
        let nodes = call(&document, "querySelectorAll", &[".adsbox".into()]).unwrap();
        int(&get(&nodes, "length").unwrap()).unwrap()
    };
    let baseline = count();
    assert!(!ad_block(&fake).await.unwrap());
    assert_eq!(count(), baseline);
    let style = Bait(call(&document, "createElement", &["style".into()]).unwrap());
    set(
        &style.0,
        "textContent",
        &".adsbox { display: none !important; }".into(),
    )
    .unwrap();
    call(
        &get(&document, "head").unwrap(),
        "appendChild",
        std::slice::from_ref(&style.0),
    )
    .unwrap();
    assert!(ad_block(&fake).await.unwrap());
    style.remove().unwrap();
    assert_eq!(count(), baseline);

    let captured = Rc::new(RefCell::new(None));
    let output = captured.clone();
    let fetch = Closure::<dyn FnMut(JsValue, JsValue) -> Promise>::new(
        move |url: JsValue, options: JsValue| {
            assert_eq!(url.as_string().unwrap(), "/static/honeypot/analytics.js");
            for (key, value) in [
                ("cache", "no-store"),
                ("credentials", "omit"),
                ("mode", "same-origin"),
                ("redirect", "error"),
            ] {
                assert_eq!(get(&options, key).unwrap().as_string().unwrap(), value);
            }
            *output.borrow_mut() = Some(get(&options, "signal").unwrap());
            let response: JsValue = Object::new().into();
            set(&response, "status", &500.into()).unwrap();
            Promise::resolve(&response)
        },
    );
    set(&fake, "fetch", fetch.as_ref()).unwrap();
    assert!(!tracking_protection(&fake).await.unwrap());
    assert_eq!(
        get(captured.borrow().as_ref().unwrap(), "aborted").unwrap(),
        JsValue::TRUE
    );

    let navigator: JsValue = Object::new().into();
    set(&navigator, "doNotTrack", &"1".into()).unwrap();
    set(&navigator, "globalPrivacyControl", &Object::new().into()).unwrap();
    set(&navigator, "brave", &JsValue::FALSE).unwrap();
    set(&fake, "navigator", &navigator).unwrap();
    let result = probe_with(&fake).await.unwrap();
    assert!(result.probed && result.gpc);
    assert_eq!(result.do_not_track, "1");
    assert!(!result.brave_shields && !result.collector_blocked && !result.tracking_protection);
    assert_eq!(count(), baseline);

    let rejected = Closure::<dyn FnMut() -> Promise>::new(|| {
        Promise::reject(&"owned fixture rejection".into())
    });
    set(&fake, "fetch", rejected.as_ref()).unwrap();
    assert!(tracking_protection(&fake).await.unwrap());
    let pending = Closure::<dyn FnMut() -> Promise>::new(|| Promise::new(&mut |_, _| {}));
    set(&fake, "fetch", pending.as_ref()).unwrap();
    assert!(tracking_protection(&fake).await.unwrap());
    // All closures remain owned until requests/timers have settled or been cancelled.
    set(&fake, "fetch", &JsValue::UNDEFINED).unwrap();
}

#[wasm_bindgen_test(async)]
async fn environment_required_dom_failure_cleans_bait_and_rejects() {
    let real: JsValue = js_sys::global().into();
    let fake = fake_global(&real);
    // A missing timer is an observation failure, never a clean/probed report.
    set(&fake, "setTimeout", &JsValue::UNDEFINED).unwrap();
    let document = get(&real, "document").unwrap();
    let before = call(&document, "querySelectorAll", &[".adsbox".into()]).unwrap();
    assert!(ad_block(&fake).await.is_err());
    let after = call(&document, "querySelectorAll", &[".adsbox".into()]).unwrap();
    assert_eq!(
        get(&before, "length").unwrap(),
        get(&after, "length").unwrap()
    );
    assert!(probe_with(&fake).await.is_err());
}

#[wasm_bindgen_test(async)]
async fn environment_timer_cancel_and_invalid_privacy_are_explicit() {
    use std::cell::Cell;
    let real: JsValue = js_sys::global().into();
    let fake = fake_global(&real);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let actual = real.clone();
    let clear = Closure::<dyn FnMut(JsValue) -> Result<JsValue, JsValue>>::new(move |handle| {
        observed.set(observed.get() + 1);
        call(&actual, "clearTimeout", &[handle])
    });
    set(&fake, "clearTimeout", clear.as_ref()).unwrap();
    assert!(!ad_block(&fake).await.unwrap());
    assert_eq!(calls.get(), 1);
    let fetch =
        Closure::<dyn FnMut() -> Promise>::new(|| Promise::reject(&"owned rejection".into()));
    set(&fake, "fetch", fetch.as_ref()).unwrap();
    assert!(tracking_protection(&fake).await.unwrap());
    assert_eq!(calls.get(), 2);
    let navigator: JsValue = Object::new().into();
    set(&navigator, "doNotTrack", &Object::new().into()).unwrap();
    set(&fake, "navigator", &navigator).unwrap();
    assert!(probe_with(&fake).await.is_err());
    set(&navigator, "doNotTrack", &"x".repeat(257).into()).unwrap();
    assert!(probe_with(&fake).await.is_err());
    // A throwing getter rejects before bait/timer allocation; generated glue only.
    let getter = Closure::<dyn FnMut() -> Result<JsValue, JsValue>>::new(|| {
        Err(JsValue::from_str("owned privacy getter failure"))
    });
    let descriptor = Object::new();
    set(&descriptor, "get", getter.as_ref()).unwrap();
    assert!(
        Reflect::define_property(
            &navigator.unchecked_into::<Object>(),
            &"doNotTrack".into(),
            &descriptor
        )
        .unwrap()
    );
    assert!(probe_with(&fake).await.is_err());
    assert_eq!(calls.get(), 2);
    set(&fake, "fetch", &JsValue::UNDEFINED).unwrap();
    set(&fake, "clearTimeout", &JsValue::UNDEFINED).unwrap();
}
