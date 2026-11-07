use super::super::reflection::call;
use js_sys::{Array, Object, Promise};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

/// Browser-owned callback is a native Promise resolver, never a Rust closure.
pub(crate) struct Timer {
    global: JsValue,
    handle: Option<JsValue>,
    marker: JsValue,
    pub(crate) promise: Promise,
}
impl Timer {
    pub(crate) fn start(global: &JsValue, millis: u32) -> Result<Self, JsValue> {
        let mut handle = None;
        let mut error = None;
        let marker: JsValue = Object::new().into();
        let promise = Promise::new(&mut |resolve, _| match call(
            global,
            "setTimeout",
            &[resolve.into(), millis.into(), marker.clone()],
        ) {
            Ok(value) => handle = Some(value),
            Err(value) => error = Some(value),
        });
        if let Some(error) = error {
            return Err(error);
        }
        Ok(Self {
            global: global.clone(),
            handle,
            marker,
            promise,
        })
    }
    pub(crate) fn cancel(&mut self) -> Result<(), JsValue> {
        if let Some(handle) = &self.handle {
            call(&self.global, "clearTimeout", std::slice::from_ref(handle))?;
            self.handle = None;
        }
        Ok(())
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        let _ = self.cancel();
    }
}

pub(crate) enum Settlement {
    Fulfilled(JsValue),
    Rejected,
    TimedOut,
}

/// A void result is still completion; it must never be mistaken for timeout.
pub(crate) async fn settle(
    global: &JsValue,
    promise: &Promise,
    millis: u32,
) -> Result<Settlement, JsValue> {
    let mut timer = Timer::start(global, millis)?;
    let candidates = Array::new();
    candidates.push(promise);
    candidates.push(&timer.promise);
    let settled = JsFuture::from(Promise::race(&candidates)).await;
    timer.cancel()?;
    Ok(match settled {
        Ok(value) if value == timer.marker => Settlement::TimedOut,
        Ok(value) => Settlement::Fulfilled(value),
        Err(_) => Settlement::Rejected,
    })
}

#[cfg(test)]
mod tests;
