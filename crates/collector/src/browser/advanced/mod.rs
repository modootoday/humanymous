//! Optional capability observations; shared wire/scoring, never automation proof.
mod capabilities;
mod media;
mod profile;
#[cfg(test)]
mod tests;
use super::{
    reflection::{call, get, int, truthy},
    resources::{Settlement, settle},
};
use humanymous_core::signals::report::Advanced;
use js_sys::Promise;
use wasm_bindgen::{JsCast, JsValue};

fn optional(object: &JsValue, key: &str) -> Option<JsValue> {
    get(object, key).ok().filter(truthy)
}
fn text(value: &JsValue) -> Result<String, JsValue> {
    if !truthy(value) {
        return Ok(String::new());
    }
    if !value.is_string() || value.unchecked_ref::<js_sys::JsString>().length() > 4096 {
        return Err(JsValue::from_str("advanced string type/budget"));
    }
    Ok(value.as_string().unwrap_or_default())
}
fn number(value: &JsValue) -> Result<f64, JsValue> {
    value
        .as_f64()
        .filter(|v| v.is_finite() && v.abs() <= 9_007_199_254_740_991.0)
        .ok_or_else(|| JsValue::from_str("advanced numeric type/budget"))
}
fn count(value: &JsValue) -> Result<i64, JsValue> {
    let n = int(&get(value, "length")?)?;
    if !(0..=4096).contains(&n) {
        return Err(JsValue::from_str("advanced enumeration budget"));
    }
    Ok(n)
}
async fn completion(
    global: &JsValue,
    result: Result<JsValue, JsValue>,
) -> Result<Option<JsValue>, JsValue> {
    let Ok(value) = result else {
        return Ok(None);
    };
    let promise = value
        .dyn_into::<Promise>()
        .map_err(|_| JsValue::from_str("advanced Promise required"))?;
    Ok(match settle(global, &promise, 800).await? {
        Settlement::Fulfilled(value) => Some(value),
        Settlement::Rejected | Settlement::TimedOut => None,
    })
}
pub(crate) async fn probe() -> Result<Advanced, JsValue> {
    probe_with(&js_sys::global().into()).await
}
async fn probe_with(global: &JsValue) -> Result<Advanced, JsValue> {
    let navigator = get(global, "navigator")?;
    let mut report = Advanced {
        connection_rtt: -1.0,
        battery_level: -1.0,
        language: text(&get(&navigator, "language")?)?,
        ..Default::default()
    };
    if let Some(touch) = optional(&navigator, "maxTouchPoints") {
        report.max_touch_points = int(&touch)?;
        if !(0..=4096).contains(&report.max_touch_points) {
            return Err(JsValue::from_str("touch point budget"));
        }
    }
    let ua = text(&get(&navigator, "userAgent")?)?.to_ascii_lowercase();
    report.mobile_ua = ["mobi", "android", "iphone", "ipad"]
        .iter()
        .any(|word| ua.contains(word));
    if let Some(performance) = optional(global, "performance") {
        report.perf_memory_present = optional(&performance, "memory").is_some();
    }
    media::collect(global, &navigator, &mut report).await?;
    capabilities::collect(global, &navigator, &mut report).await?;
    profile::collect(global, &navigator, &mut report).await?;
    report.probed = true;
    Ok(report)
}
