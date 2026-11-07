use super::*;

#[wasm_bindgen_test(async)]
async fn reused_deadline_stays_expired_for_later_stages() {
    let global: JsValue = js_sys::global().into();
    let mut timer = Timer::start(&global, 1).unwrap();
    assert!(matches!(
        timer.race(&Promise::resolve(&JsValue::TRUE)).await,
        Settlement::Fulfilled(_)
    ));
    wasm_bindgen_futures::JsFuture::from(timer.promise.clone())
        .await
        .unwrap();
    assert!(matches!(
        timer.race(&Promise::new(&mut |_, _| {})).await,
        Settlement::TimedOut
    ));
    timer.cancel().unwrap();
}
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test(async)]
async fn native_promise_deadline_keeps_void_false_null_rejection_and_timeout_distinct() {
    let global: JsValue = js_sys::global().into();
    for value in [
        JsValue::UNDEFINED,
        JsValue::NULL,
        JsValue::FALSE,
        Object::new().into(),
    ] {
        match settle(&global, &Promise::resolve(&value), 25)
            .await
            .unwrap()
        {
            Settlement::Fulfilled(actual) => assert_eq!(actual, value),
            _ => panic!("fulfilled value incorrectly classified"),
        }
    }
    assert!(matches!(
        settle(&global, &Promise::reject(&"owned rejection".into()), 25)
            .await
            .unwrap(),
        Settlement::Rejected
    ));
    assert!(matches!(
        settle(&global, &Promise::new(&mut |_, _| {}), 1)
            .await
            .unwrap(),
        Settlement::TimedOut
    ));
    assert!(
        settle(&Object::new().into(), &Promise::resolve(&JsValue::TRUE), 1)
            .await
            .is_err()
    );
}
