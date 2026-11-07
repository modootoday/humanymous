use super::reflection::{call, get, go_string, truthy};
use crate::detection::Getter;
use wasm_bindgen::JsValue;

pub(super) fn getter(global: &JsValue, navigator: &JsValue) -> Result<Getter, JsValue> {
    let prototype = get(&get(navigator, "constructor")?, "prototype")?;
    let descriptor = call(
        &get(global, "Object")?,
        "getOwnPropertyDescriptor",
        &[prototype, JsValue::from_str("webdriver")],
    )?;
    if !truthy(&descriptor) {
        return Ok(Getter::Absent);
    }
    let getter = get(&descriptor, "get")?;
    if !truthy(&getter) {
        return Ok(Getter::Absent);
    }
    if !getter.is_function() {
        return Ok(Getter::Patched);
    }
    let prototype = get(&get(global, "Function")?, "prototype")?;
    let to_string = get(&prototype, "toString")?;
    let text = go_string(&call(&to_string, "call", &[getter])?)?;
    Ok(if text.contains("[native code]") {
        Getter::Native
    } else {
        Getter::Patched
    })
}
struct Frame {
    body: JsValue,
    element: JsValue,
}
impl Drop for Frame {
    fn drop(&mut self) {
        let _ = call(
            &self.body,
            "removeChild",
            std::slice::from_ref(&self.element),
        );
    }
}
pub(super) fn iframe(
    navigator: &JsValue,
    document: &JsValue,
) -> Result<Option<(bool, bool)>, JsValue> {
    let body = get(document, "body")?;
    if !truthy(&body) {
        return Ok(None);
    }
    let element = call(document, "createElement", &[JsValue::from_str("iframe")])?;
    let style = get(&element, "style")?;
    if !js_sys::Reflect::set(
        &style,
        &JsValue::from_str("display"),
        &JsValue::from_str("none"),
    )? {
        return Err(JsValue::from_str("hidden frame style rejected"));
    }
    call(&body, "appendChild", std::slice::from_ref(&element))?;
    let frame = Frame { body, element };
    let window = get(&frame.element, "contentWindow")?;
    if !truthy(&window) {
        return Ok(None);
    }
    let inner = get(&window, "navigator")?;
    if !truthy(&inner) {
        return Ok(None);
    }
    Ok(Some((
        get(navigator, "webdriver")?.as_bool().unwrap_or(false),
        get(&inner, "webdriver")?.as_bool().unwrap_or(false),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    use wasm_bindgen::{JsCast, closure::Closure};
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn failing_navigator_getter_propagates_and_removes_real_frame() {
        let global: JsValue = js_sys::global().into();
        let document = get(&global, "document").unwrap();
        let count = || {
            let frames = call(
                &document,
                "getElementsByTagName",
                &[JsValue::from_str("iframe")],
            )
            .unwrap();
            get(&frames, "length").unwrap().as_f64().unwrap()
        };
        let before = count();
        let observed = Rc::new(Cell::new(false));
        let seen = observed.clone();
        let doc = document.clone();
        let getter = Closure::wrap(Box::new(move || -> Result<JsValue, JsValue> {
            let frames = call(&doc, "getElementsByTagName", &[JsValue::from_str("iframe")])?;
            seen.set(get(&frames, "length")?.as_f64() == Some(before + 1.0));
            Err(JsValue::from_str("owned getter failure"))
        }) as Box<dyn FnMut() -> Result<JsValue, JsValue>>);
        let navigator = js_sys::Object::new();
        let descriptor = js_sys::Object::new();
        assert!(
            js_sys::Reflect::set(&descriptor, &JsValue::from_str("get"), getter.as_ref()).unwrap()
        );
        call(
            &get(&global, "Object").unwrap(),
            "defineProperty",
            &[
                navigator.clone().into(),
                JsValue::from_str("webdriver"),
                descriptor.into(),
            ],
        )
        .unwrap();
        let error = iframe(navigator.unchecked_ref(), &document).unwrap_err();
        assert_eq!(error.as_string().as_deref(), Some("owned getter failure"));
        assert!(observed.get());
        assert_eq!(count(), before);
    }
}
