//! First-party weak privacy observations, with owned DOM/timer/request lifetime.
mod resources;
#[cfg(test)]
mod tests;
use super::reflection::{call, get, int, string, truthy};
use super::resources::{Settlement, Timer, settle};
use humanymous_signals::report::Environment;
use js_sys::{Array, Object, Promise, Reflect};
use resources::{Abort, Bait};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

fn set(object: &JsValue, key: &str, value: &JsValue) -> Result<(), JsValue> {
    if Reflect::set(object, &key.into(), value)? {
        Ok(())
    } else {
        Err(JsValue::from_str("environment property write rejected"))
    }
}

pub(crate) async fn probe() -> Result<Environment, JsValue> {
    let global: JsValue = js_sys::global().into();
    probe_with(&global).await
}

async fn probe_with(global: &JsValue) -> Result<Environment, JsValue> {
    let navigator = get(global, "navigator")?;
    let dnt = get(&navigator, "doNotTrack")?;
    if truthy(&dnt) && !dnt.is_string() {
        return Err(JsValue::from_str("doNotTrack string required"));
    }
    if dnt.is_string() && dnt.unchecked_ref::<js_sys::JsString>().length() > 256 {
        return Err(JsValue::from_str("doNotTrack observation budget"));
    }
    let mut environment = Environment {
        do_not_track: if truthy(&dnt) {
            string(&dnt)?
        } else {
            String::new()
        },
        gpc: truthy(&get(&navigator, "globalPrivacyControl")?),
        brave_shields: truthy(&get(&navigator, "brave")?),
        ..Default::default()
    };
    environment.ad_block = ad_block(global).await?;
    environment.tracking_protection = tracking_protection(global).await?;
    environment.probed = true;
    Ok(environment)
}

async fn ad_block(global: &JsValue) -> Result<bool, JsValue> {
    let document = get(global, "document")?;
    let bait = Bait(call(&document, "createElement", &["div".into()])?);
    set(
        &bait.0,
        "className",
        &"adsbox ad-banner pub_300x250 banner_ad".into(),
    )?;
    set(
        &get(&bait.0, "style")?,
        "cssText",
        &"position:absolute;left:-9999px;height:10px;width:10px;".into(),
    )?;
    call(
        &get(&document, "body")?,
        "appendChild",
        std::slice::from_ref(&bait.0),
    )?;
    let measured = async {
        let mut timer = Timer::start(global, 60)?;
        let waited = JsFuture::from(timer.promise.clone()).await;
        let cleanup = timer.cancel();
        waited?;
        cleanup?;
        if int(&get(&bait.0, "offsetHeight")?)? == 0 {
            return Ok(true);
        }
        let style = call(global, "getComputedStyle", std::slice::from_ref(&bait.0))?;
        Ok(string(&get(&style, "display")?)? == "none")
    }
    .await;
    let cleanup = bait.remove();
    measured.and_then(|value| cleanup.map(|()| value))
}

async fn tracking_protection(global: &JsValue) -> Result<bool, JsValue> {
    let constructor = get(global, "AbortController")?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| JsValue::from_str("AbortController unavailable"))?;
    let abort = Abort(Reflect::construct(&constructor, &Array::new())?);
    let options: JsValue = Object::new().into();
    for (key, value) in [
        ("cache", "no-store"),
        ("credentials", "omit"),
        ("mode", "same-origin"),
        ("redirect", "error"),
    ] {
        set(&options, key, &value.into())?;
    }
    set(&options, "signal", &get(&abort.0, "signal")?)?;
    let measured = async {
        let response = match call(
            global,
            "fetch",
            &["/static/honeypot/analytics.js".into(), options],
        ) {
            Ok(value) => value
                .dyn_into::<Promise>()
                .map_err(|_| JsValue::from_str("fetch promise required"))?,
            Err(_) => return Ok(true),
        };
        // HTTP status is deliberately not inspected: fetch resolves on 404/500.
        Ok(match settle(global, &response, 400).await? {
            Settlement::Fulfilled(value) => value.is_undefined(),
            Settlement::Rejected | Settlement::TimedOut => true,
        })
    }
    .await;
    let request_cleanup = abort.abort();
    measured.and_then(|value| request_cleanup.map(|()| value))
}
