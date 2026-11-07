//! Browser L1-L3 observation, not transport authentication or human attestation.
pub mod behavior;
#[cfg(target_arch = "wasm32")]
mod browser;
pub mod detection;
pub mod guard;

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

/// Explicitly run the bounded first-party privacy probe, without session attestation.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn environment_json() -> Result<String, wasm_bindgen::JsValue> {
    serde_json::to_string(&browser::environment::probe().await?)
        .map_err(|_| wasm_bindgen::JsValue::from_str("environment serialization failed"))
}

/// Attempt optional advanced observations; this is not a qualified session report.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn advanced_json() -> Result<String, wasm_bindgen::JsValue> {
    serde_json::to_string(&browser::advanced::probe().await?)
        .map_err(|_| wasm_bindgen::JsValue::from_str("advanced serialization failed"))
}

/// An explicit collection epoch owning passive event listeners and bounded buffers.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub struct BrowserCollector {
    collection: browser::behavior::Collection,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
impl BrowserCollector {
    /// Register passive listeners in the current browser document.
    #[wasm_bindgen::prelude::wasm_bindgen(constructor)]
    pub fn new() -> Result<BrowserCollector, wasm_bindgen::JsValue> {
        Ok(Self {
            collection: browser::behavior::Collection::start()?,
        })
    }
    /// Return reduced behavior only; no input text, key identities or raw samples.
    pub fn behavior_json(&self) -> Result<String, wasm_bindgen::JsValue> {
        serde_json::to_string(&self.collection.summary()?)
            .map_err(|_| wasm_bindgen::JsValue::from_str("behavior serialization failed"))
    }
    /// Combine actual L1-L3 and measured behavior; all other planes remain unqualified.
    pub fn snapshot_json(&self) -> Result<String, wasm_bindgen::JsValue> {
        let mut report =
            detection::detect(&browser::capture()?).map_err(wasm_bindgen::JsValue::from_str)?;
        report.behavior = self.collection.summary()?;
        serde_json::to_string(&report)
            .map_err(|_| wasm_bindgen::JsValue::from_str("report serialization failed"))
    }
    /// Freeze the summary, clear raw pairing/buffers and detach all owned callbacks.
    pub fn stop(&mut self) -> Result<(), wasm_bindgen::JsValue> {
        self.collection.stop()
    }
}
