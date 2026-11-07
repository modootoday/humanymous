//! Browser L1-L3 observation, not transport authentication or human attestation.
#[cfg(target_arch = "wasm32")]
mod browser;
pub mod detection;

/// Read actual browser APIs and return the partial frozen report wire shape.
/// Any collection failure rejects rather than returning an empty valid report.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn detect_json() -> Result<String, wasm_bindgen::JsValue> {
    let snapshot = browser::capture()?;
    let report = detection::detect(&snapshot).map_err(wasm_bindgen::JsValue::from_str)?;
    serde_json::to_string(&report)
        .map_err(|_| wasm_bindgen::JsValue::from_str("report serialization failed"))
}
