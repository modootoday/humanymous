//! Checked browser I/O. All executable glue is generated upstream by bindgen.
pub(crate) mod advanced;
pub(crate) mod behavior;
pub(crate) mod cdp;
pub(crate) mod environment;
mod fingerprint;
pub(crate) mod guard;
mod integrity;
mod reflection;
mod resources;
pub(crate) mod transport;
use crate::detection::Snapshot;
use humanymous_signals::report::{UaBrand, UaClientHints};
use reflection::{call, get, go_string, int, string, truthy};
use wasm_bindgen::JsValue;

fn length(value: &JsValue) -> Result<i64, JsValue> {
    int(&get(value, "length")?)
}
fn count(value: &JsValue) -> Result<Option<i64>, JsValue> {
    if truthy(value) {
        Ok(Some(length(value)?))
    } else {
        Ok(None)
    }
}
fn keys(global: &JsValue, value: &JsValue) -> Result<Vec<String>, JsValue> {
    let object = get(global, "Object")?;
    let keys = call(&object, "keys", std::slice::from_ref(value))?;
    let n = length(&keys)?;
    if n > 16_384 {
        return Err(JsValue::from_str("property enumeration budget"));
    }
    let mut out = Vec::new();
    for i in 0..n {
        out.push(go_string(&get(&keys, &i.to_string())?)?);
    }
    Ok(out)
}
fn hints(navigator: &JsValue) -> Result<UaClientHints, JsValue> {
    let data = get(navigator, "userAgentData")?;
    if !truthy(&data) {
        return Ok(Default::default());
    }
    let mut hints = UaClientHints {
        mobile: truthy(&get(&data, "mobile")?),
        platform: string(&get(&data, "platform")?)?,
        ..Default::default()
    };
    let brands = get(&data, "brands")?;
    if truthy(&brands) {
        let n = length(&brands)?;
        if n > 128 {
            return Err(JsValue::from_str("client hints budget"));
        }
        let mut values = Vec::new();
        for i in 0..n {
            let brand = get(&brands, &i.to_string())?;
            values.push(UaBrand {
                brand: go_string(&get(&brand, "brand")?)?,
                version: go_string(&get(&brand, "version")?)?,
            });
        }
        if !values.is_empty() {
            hints.brands = Some(values);
        }
    }
    Ok(hints)
}
pub(super) fn capture() -> Result<Snapshot, JsValue> {
    let global: JsValue = js_sys::global().into();
    let navigator = get(&global, "navigator")?;
    let document = get(&global, "document")?;
    if !truthy(&navigator) || !truthy(&document) {
        return Err(JsValue::from_str("browser document and navigator required"));
    }
    let notification = get(&global, "Notification")?;
    let denied = if truthy(&notification) {
        get(&notification, "permission")?
            .as_string()
            .is_some_and(|s| s == "denied")
    } else {
        false
    };
    let memory = get(&navigator, "deviceMemory")?.as_f64();
    let screen = get(&global, "screen")?;
    let screen = if truthy(&screen) {
        Some((
            int(&get(&screen, "height")?)?,
            int(&get(&screen, "availHeight")?)?,
        ))
    } else {
        None
    };
    let (vendor, renderer) = fingerprint::webgl(&document)?;
    let mut playwright = [false; 4];
    for (i, key) in [
        "__playwright",
        "__pw_manual",
        "__pwInitScripts",
        "__playwright__binding__",
    ]
    .iter()
    .enumerate()
    {
        playwright[i] = truthy(&get(&global, key)?);
    }
    Ok(Snapshot {
        webdriver: get(&navigator, "webdriver")?.as_bool().unwrap_or(false),
        user_agent: string(&get(&navigator, "userAgent")?)?,
        hints: hints(&navigator)?,
        plugins: count(&get(&navigator, "plugins")?)?,
        languages: count(&get(&navigator, "languages")?)?,
        chrome_truthy: truthy(&get(&global, "chrome")?),
        notification_denied: denied,
        widths: Some((
            int(&get(&global, "outerWidth")?)?,
            int(&get(&global, "innerWidth")?)?,
        )),
        cdp_flags: [
            truthy(&get(&global, "__hmCDP")?),
            truthy(&get(&global, "__hmCDP2")?),
        ],
        document_keys: keys(&global, &document)?,
        window_keys: keys(&global, &global)?,
        playwright,
        phantom: [
            truthy(&get(&global, "_phantom")?),
            truthy(&get(&global, "callPhantom")?),
        ],
        webgl_vendor: vendor,
        webgl_renderer: renderer,
        memory,
        screen,
        getter: integrity::getter(&global, &navigator)?,
        iframe: integrity::iframe(&navigator, &document)?,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);
    #[wasm_bindgen_test]
    fn actual_browser_report_and_iframe_cleanup() {
        let global: JsValue = js_sys::global().into();
        let document = get(&global, "document").unwrap();
        let before = call(
            &document,
            "getElementsByTagName",
            &[JsValue::from_str("iframe")],
        )
        .unwrap();
        let before = length(&before).unwrap();
        let s = capture().unwrap();
        assert!(!s.user_agent.is_empty());
        let actual_ua =
            string(&get(&get(&global, "navigator").unwrap(), "userAgent").unwrap()).unwrap();
        assert_eq!(s.user_agent, actual_ua);
        let report = crate::detection::detect(&s).unwrap();
        assert!(!report.environment.probed);
        assert!(report.session_id.is_empty());
        let wd = report
            .signals
            .unwrap()
            .into_iter()
            .find(|s| s.id == "l1.navigator.webdriver")
            .unwrap();
        assert_eq!(wd.value, serde_json::json!(s.webdriver));
        let after = call(
            &document,
            "getElementsByTagName",
            &[JsValue::from_str("iframe")],
        )
        .unwrap();
        assert_eq!(length(&after).unwrap(), before);
        assert!(!crate::detect_json().unwrap().is_empty());
    }
}
