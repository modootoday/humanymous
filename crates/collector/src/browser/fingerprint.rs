use super::reflection::{call, get, go_string, truthy};
use wasm_bindgen::JsValue;
pub(super) fn webgl(document: &JsValue) -> Result<(String, String), JsValue> {
    match webgl_values(document, WebGlScope::Fingerprint)? {
        Some((vendor, renderer)) => Ok((go_string(&vendor)?, go_string(&renderer)?)),
        None => Ok(Default::default()),
    }
}
pub(super) enum WebGlScope {
    Fingerprint,
    Advanced,
}
pub(super) fn webgl_values(
    document: &JsValue,
    scope: WebGlScope,
) -> Result<Option<(JsValue, JsValue)>, JsValue> {
    let canvas = call(document, "createElement", &[JsValue::from_str("canvas")])?;
    let mut gl = call(&canvas, "getContext", &[JsValue::from_str("webgl")])?;
    if !truthy(&gl) && matches!(scope, WebGlScope::Fingerprint) {
        gl = call(
            &canvas,
            "getContext",
            &[JsValue::from_str("experimental-webgl")],
        )?;
    }
    if !truthy(&gl) {
        return Ok(None);
    }
    let info = call(
        &gl,
        "getExtension",
        &[JsValue::from_str("WEBGL_debug_renderer_info")],
    )?;
    if !truthy(&info) {
        return Ok(None);
    }
    let vendor = call(&gl, "getParameter", &[get(&info, "UNMASKED_VENDOR_WEBGL")?])?;
    let renderer = if matches!(scope, WebGlScope::Fingerprint) {
        call(
            &gl,
            "getParameter",
            &[get(&info, "UNMASKED_RENDERER_WEBGL")?],
        )?
    } else {
        JsValue::UNDEFINED
    };
    Ok(Some((vendor, renderer)))
}
