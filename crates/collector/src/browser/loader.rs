//! Go oracle `web/js/loader.js` report planes (981d25c): the explicit async
//! report attaches the probed environment, advanced surfaces and the
//! `advanced.js` `computeFingerprintId` value to the collector's measured
//! L1-L3, behavior and guard observations. Nothing here is scored, and guard
//! coverage is reported exactly as measured: forwarding hooks stay opt-in.
use super::reflection::{call, get, truthy};
use super::resources::{Settlement, settle};
use crate::fingerprint_id::{digest_id, fallback_id};
use humanymous_core::signals::report::ClientReport;
use js_sys::{Array, JsString, Promise, Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};

/// Upper bound on the joined surface text, in UTF-16 code units.
const SURFACES_MAX: u32 = 1_048_576;
/// Bound for the WebCrypto digest Promise; the oracle had none.
const DIGEST_MILLIS: u32 = 800;

fn set(object: &JsValue, key: &str, value: &JsValue) -> Result<(), JsValue> {
    if Reflect::set(object, &key.into(), value)? {
        Ok(())
    } else {
        Err(JsValue::from_str("fingerprint property write rejected"))
    }
}

/// `try { ... } catch (_) {}` in the oracle: a throwing surface is skipped.
fn optional(parts: &Array, surface: Result<Option<JsValue>, JsValue>) {
    if let Ok(Some(value)) = surface {
        parts.push(&value);
    }
}

/// Fixed canvas drawing; the data URL is hashed, never exported.
fn canvas(document: &JsValue) -> Result<Option<JsValue>, JsValue> {
    let canvas = call(document, "createElement", &["canvas".into()])?;
    set(&canvas, "width", &240.into())?;
    set(&canvas, "height", &60.into())?;
    let context = call(&canvas, "getContext", &["2d".into()])?;
    set(&context, "textBaseline", &"top".into())?;
    set(&context, "font", &"14px Arial".into())?;
    set(&context, "fillStyle", &"#f60".into())?;
    call(
        &context,
        "fillRect",
        &[2.into(), 2.into(), 100.into(), 20.into()],
    )?;
    set(&context, "fillStyle", &"#069".into())?;
    call(
        &context,
        "fillText",
        &["humanymous-fp-\u{1F512}".into(), 4.into(), 4.into()],
    )?;
    Ok(Some(call(&canvas, "toDataURL", &[])?))
}

/// `gl.getParameter(VENDOR) + '/' + gl.getParameter(RENDERER)` from `webgl` only.
fn webgl(global: &JsValue, document: &JsValue) -> Result<Option<JsValue>, JsValue> {
    let canvas = call(document, "createElement", &["canvas".into()])?;
    let gl = call(&canvas, "getContext", &["webgl".into()])?;
    if !truthy(&gl) {
        return Ok(None);
    }
    let info = call(&gl, "getExtension", &["WEBGL_debug_renderer_info".into()])?;
    if !truthy(&info) {
        return Ok(None);
    }
    let vendor = call(&gl, "getParameter", &[get(&info, "UNMASKED_VENDOR_WEBGL")?])?;
    let renderer = call(
        &gl,
        "getParameter",
        &[get(&info, "UNMASKED_RENDERER_WEBGL")?],
    )?;
    let text = |value: &JsValue| -> Result<JsString, JsValue> {
        call(global, "String", std::slice::from_ref(value))?
            .dyn_into::<JsString>()
            .map_err(|_| JsValue::from_str("string conversion required"))
    };
    Ok(Some(
        text(&vendor)?
            .concat(&"/".into())
            .concat(&text(&renderer)?.into())
            .into(),
    ))
}

fn time_zone(global: &JsValue) -> Result<Option<JsValue>, JsValue> {
    let format = call(&get(global, "Intl")?, "DateTimeFormat", &[])?;
    let options = call(&format, "resolvedOptions", &[])?;
    Ok(Some(get(&options, "timeZone")?))
}

/// The oracle's surface list, joined with JavaScript `Array.prototype.join`
/// semantics so undefined/null/number conversions match exactly.
fn surfaces(global: &JsValue) -> Result<JsString, JsValue> {
    let document = get(global, "document")?;
    let navigator = get(global, "navigator")?;
    let screen = get(global, "screen")?;
    let parts = Array::new();
    optional(&parts, canvas(&document));
    optional(&parts, webgl(global, &document));
    let display = Array::new();
    for key in ["width", "height", "colorDepth"] {
        display.push(&get(&screen, key)?);
    }
    display.push(&get(global, "devicePixelRatio")?);
    parts.push(&display.join("x").into());
    optional(&parts, time_zone(global));
    let languages = get(&navigator, "languages")?;
    parts.push(&if truthy(&languages) {
        call(&languages, "join", &[",".into()])?
    } else {
        "".into()
    });
    let threads = get(&navigator, "hardwareConcurrency")?;
    let threads = if truthy(&threads) { threads } else { 0.into() };
    parts.push(&call(global, "String", &[threads])?);
    let platform = get(&navigator, "platform")?;
    parts.push(&if truthy(&platform) {
        platform
    } else {
        "".into()
    });
    let joined = parts.join("|");
    if joined.length() > SURFACES_MAX {
        return Err(JsValue::from_str("fingerprint surface budget"));
    }
    Ok(joined)
}

/// WebCrypto SHA-256 as the oracle used it; a throwing or rejecting digest
/// takes the oracle's fallback hash. A timeout is an error, not a value.
async fn digest(global: &JsValue, joined: &JsString) -> Result<String, JsValue> {
    let fallback = || fallback_id(&joined.iter().collect::<Vec<u16>>());
    let started = (|| -> Result<Promise, JsValue> {
        let subtle = get(&get(global, "crypto")?, "subtle")?;
        let encoder = Reflect::construct(
            &get(global, "TextEncoder")?
                .dyn_into::<js_sys::Function>()
                .map_err(|_| JsValue::from_str("TextEncoder unavailable"))?,
            &Array::new(),
        )?;
        let bytes = call(&encoder, "encode", &[joined.into()])?;
        call(&subtle, "digest", &["SHA-256".into(), bytes])?
            .dyn_into::<Promise>()
            .map_err(|_| JsValue::from_str("digest Promise required"))
    })();
    let Ok(promise) = started else {
        return Ok(fallback());
    };
    match settle(global, &promise, DIGEST_MILLIS).await? {
        Settlement::Fulfilled(buffer) => {
            if !buffer.is_instance_of::<js_sys::ArrayBuffer>() {
                return Err(JsValue::from_str("digest ArrayBuffer required"));
            }
            digest_id(&Uint8Array::new(&buffer).to_vec()).map_err(JsValue::from_str)
        }
        Settlement::Rejected => Ok(fallback()),
        Settlement::TimedOut => Err(JsValue::from_str("fingerprint digest timeout")),
    }
}

/// `computeFingerprintId()`; surfaces the oracle read outside `try` reject.
pub(crate) async fn fingerprint_id(global: &JsValue) -> Result<String, JsValue> {
    let joined = surfaces(global)?;
    digest(global, &joined).await
}

/// Attach the loader planes to an already measured collector report.
pub(crate) async fn complete(mut report: ClientReport) -> Result<String, JsValue> {
    let global: JsValue = js_sys::global().into();
    report.environment = super::environment::probe().await?;
    report.advanced = super::advanced::probe().await?;
    report.fingerprint_id = fingerprint_id(&global).await?;
    serde_json::to_string(&report).map_err(|_| JsValue::from_str("report serialization failed"))
}
