use super::super::reflection::call;
use wasm_bindgen::JsValue;

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
