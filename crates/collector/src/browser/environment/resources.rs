use super::super::reflection::call;
pub(super) use super::super::resources::Abort;
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
