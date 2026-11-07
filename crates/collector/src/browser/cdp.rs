//! Explicit legacy console-preview observations, never universal CDP detection.
use super::reflection::{apply, get};
use js_sys::{Array, Function, Object, Reflect};
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

#[derive(Clone, Copy)]
pub(crate) struct Attempt {
    pub(crate) status: &'static str,
    pub(crate) triggered: bool,
}
#[derive(Clone, Copy)]
pub(crate) struct Observation {
    stack: Attempt,
    prototype: Attempt,
}
impl Observation {
    pub(crate) fn probe() -> Self {
        Self::with_global(&js_sys::global().into())
    }
    fn with_global(global: &JsValue) -> Self {
        let managed = get(global, "FinalizationRegistry").is_ok_and(|value| value.is_function());
        Self {
            stack: attempt(global, managed, false),
            prototype: attempt(global, managed, true),
        }
    }
    pub(crate) fn flags(&self) -> Option<[bool; 2]> {
        if self.stack.status == "attempted" && self.prototype.status == "attempted" {
            Some([self.stack.triggered, self.prototype.triggered])
        } else {
            None
        }
    }
    pub(crate) fn json(&self) -> String {
        serde_json::json!({"errorStack":{"status":self.stack.status,"triggered":self.stack.triggered},"prototypePreview":{"status":self.prototype.status,"triggered":self.prototype.triggered},"sampling":"synchronous-after-console-call","qualified":false}).to_string()
    }
}
fn attempt(global: &JsValue, managed: bool, prototype: bool) -> Attempt {
    let unsupported = Attempt {
        status: "unsupported",
        triggered: false,
    };
    if !managed {
        return unsupported;
    }
    let console = match get(global, "console") {
        Ok(value) if !value.is_null() && !value.is_undefined() => value,
        _ => return unsupported,
    };
    let method = match get(&console, if prototype { "groupEnd" } else { "debug" })
        .and_then(|value| value.dyn_into::<Function>())
    {
        Ok(value) => value,
        Err(_) => return unsupported,
    };
    let active = Rc::new(Cell::new(true));
    let triggered = Rc::new(Cell::new(false));
    let probe = if prototype {
        prototype_object(global, active.clone(), triggered.clone())
    } else {
        stack_object(global, active.clone(), triggered.clone())
    };
    let result = probe.and_then(|value| apply(&method, &console, &[value]));
    active.set(false);
    Attempt {
        status: if result.is_ok() {
            "attempted"
        } else {
            "failed"
        },
        triggered: result.is_ok() && triggered.get(),
    }
}
fn stack_object(
    global: &JsValue,
    active: Rc<Cell<bool>>,
    triggered: Rc<Cell<bool>>,
) -> Result<JsValue, JsValue> {
    let constructor = get(global, "Error")?.dyn_into::<Function>()?;
    let error = Reflect::construct(&constructor, &Array::new())?;
    let getter = Closure::<dyn Fn() -> JsValue>::new(move || {
        if active.get() {
            triggered.set(true);
        }
        JsValue::from_str("")
    })
    .into_js_value();
    let descriptor = Object::new();
    set(&descriptor, "get", &getter)?;
    if !Reflect::define_property(
        error.unchecked_ref::<Object>(),
        &"stack".into(),
        &descriptor,
    )? {
        return Err(JsValue::from_str("preview descriptor unavailable"));
    }
    Ok(error)
}
fn prototype_object(
    global: &JsValue,
    active: Rc<Cell<bool>>,
    triggered: Rc<Cell<bool>>,
) -> Result<JsValue, JsValue> {
    let constructor = get(global, "Proxy")?.dyn_into::<Function>()?;
    let trap = Closure::<dyn Fn(JsValue) -> Array>::new(move |_| {
        if active.get() {
            triggered.set(true);
        }
        Array::new()
    })
    .into_js_value();
    let handler = Object::new();
    set(&handler, "ownKeys", &trap)?;
    let args = Array::new();
    args.push(&Object::new());
    args.push(&handler);
    let proxy = Reflect::construct(&constructor, &args)?;
    Ok(Object::create(proxy.unchecked_ref::<Object>()).into())
}
fn set(object: &JsValue, key: &str, value: &JsValue) -> Result<(), JsValue> {
    if Reflect::set(object, &key.into(), value)? {
        Ok(())
    } else {
        Err(JsValue::from_str("preview descriptor unavailable"))
    }
}

#[cfg(test)]
mod tests;
