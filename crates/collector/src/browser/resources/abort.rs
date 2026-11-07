use super::super::reflection::call;
use wasm_bindgen::JsValue;

/// Owned abort lifetime, shared by environment probes and session transport.
pub(crate) struct Abort(pub(crate) JsValue);
impl Abort {
    pub(crate) fn abort(&self) -> Result<(), JsValue> {
        call(&self.0, "abort", &[]).map(|_| ())
    }
}
impl Drop for Abort {
    fn drop(&mut self) {
        let _ = self.abort();
    }
}
