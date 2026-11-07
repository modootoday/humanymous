use super::super::reflection::call;
use js_sys::Promise;
use wasm_bindgen::JsValue;

/// Browser-owned callback is a native Promise resolver, never a Rust closure.
pub(super) struct Timer {
    global: JsValue,
    handle: Option<JsValue>,
    pub(super) promise: Promise,
}
impl Timer {
    pub(super) fn start(global: &JsValue, millis: u32) -> Result<Self, JsValue> {
        let mut handle = None;
        let mut error = None;
        let promise = Promise::new(&mut |resolve, _| match call(
            global,
            "setTimeout",
            &[resolve.into(), millis.into()],
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
            promise,
        })
    }
    pub(super) fn cancel(&mut self) -> Result<(), JsValue> {
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

pub(super) struct Bait(pub(super) JsValue);
impl Bait {
    pub(super) fn remove(&self) -> Result<(), JsValue> {
        call(&self.0, "remove", &[]).map(|_| ())
    }
}
impl Drop for Bait {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

pub(super) struct Abort(pub(super) JsValue);
impl Abort {
    pub(super) fn abort(&self) -> Result<(), JsValue> {
        call(&self.0, "abort", &[]).map(|_| ())
    }
}
impl Drop for Abort {
    fn drop(&mut self) {
        let _ = self.abort();
    }
}
