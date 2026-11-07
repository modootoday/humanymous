use super::*;
use js_sys::{Array, Object, Reflect};

fn property(object: &JsValue, key: &str, value: &JsValue) -> Result<(), JsValue> {
    if Reflect::set(object, &key.into(), value)? {
        Ok(())
    } else {
        Err(JsValue::from_str("advanced config property rejected"))
    }
}
fn capability(content: &str) -> Result<JsValue, JsValue> {
    let object: JsValue = Object::new().into();
    property(&object, "contentType", &content.into())?;
    let values = Array::new();
    values.push(&object);
    Ok(values.into())
}
pub(super) async fn collect(
    global: &JsValue,
    navigator: &JsValue,
    report: &mut Advanced,
) -> Result<(), JsValue> {
    if optional(navigator, "requestMediaKeySystemAccess").is_some() {
        let config: JsValue = Object::new().into();
        let types = Array::new();
        types.push(&"cenc".into());
        property(&config, "initDataTypes", &types)?;
        property(
            &config,
            "audioCapabilities",
            &capability("audio/mp4;codecs=\"mp4a.40.2\"")?,
        )?;
        property(
            &config,
            "videoCapabilities",
            &capability("video/mp4;codecs=\"avc1.42E01E\"")?,
        )?;
        let configs = Array::new();
        configs.push(&config);
        report.widevine_supported = completion(
            global,
            call(
                navigator,
                "requestMediaKeySystemAccess",
                &["com.widevine.alpha".into(), configs.into()],
            ),
        )
        .await?
        .is_some();
    }
    // Reuse the existing frozen WebGL acquisition rather than a second detector.
    if let Some(document) = optional(global, "document") {
        use super::super::fingerprint::{WebGlScope, webgl_values};
        if let Ok(Some((vendor, _))) = webgl_values(&document, WebGlScope::Advanced) {
            report.web_gl_vendor = text(&vendor)?;
        }
    }
    if let Some(gpu) = optional(navigator, "gpu")
        && let Some(adapter) = completion(global, call(&gpu, "requestAdapter", &[]))
            .await?
            .filter(truthy)
    {
        report.web_gpu_present = true;
        report.web_gpu_fallback = optional(&adapter, "isFallbackAdapter").is_some();
        let mut info = optional(&adapter, "info");
        if info.is_none() && optional(&adapter, "requestAdapterInfo").is_some() {
            info = completion(global, call(&adapter, "requestAdapterInfo", &[])).await?;
        }
        if let Some(info) = info.filter(truthy) {
            report.web_gpu_vendor = text(&get(&info, "vendor")?)?;
        }
    }
    if let Some(constructor) =
        optional(global, "AudioContext").or_else(|| optional(global, "webkitAudioContext"))
        && let Ok(constructor) = constructor.dyn_into::<js_sys::Function>()
        && let Ok(context) = Reflect::construct(&constructor, &Array::new())
    {
        let measured = get(&context, "sampleRate").and_then(|v| number(&v));
        // No play/start or permission request. Close is attempted even if reading failed.
        let closed = completion(global, call(&context, "close", &[])).await?;
        if closed.is_none() {
            return Err(JsValue::from_str("AudioContext close unconfirmed"));
        }
        report.audio_sample_rate = measured?;
    }
    Ok(())
}
