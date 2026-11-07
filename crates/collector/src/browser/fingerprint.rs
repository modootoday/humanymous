use super::reflection::{call, get, go_string, truthy};
use wasm_bindgen::JsValue;
pub(super) fn webgl(document: &JsValue) -> Result<(String, String), JsValue> {
    let canvas = call(document, "createElement", &[JsValue::from_str("canvas")])?;
    let mut gl = call(&canvas, "getContext", &[JsValue::from_str("webgl")])?;
    if !truthy(&gl) {
        gl = call(
            &canvas,
            "getContext",
            &[JsValue::from_str("experimental-webgl")],
        )?;
    }
    if !truthy(&gl) {
        return Ok(Default::default());
    }
    let info = call(
        &gl,
        "getExtension",
        &[JsValue::from_str("WEBGL_debug_renderer_info")],
    )?;
    if !truthy(&info) {
        return Ok(Default::default());
    }
    let vendor = go_string(&call(
        &gl,
        "getParameter",
        &[get(&info, "UNMASKED_VENDOR_WEBGL")?],
    )?)?;
    let renderer = go_string(&call(
        &gl,
        "getParameter",
        &[get(&info, "UNMASKED_RENDERER_WEBGL")?],
    )?)?;
    Ok((vendor, renderer))
}
