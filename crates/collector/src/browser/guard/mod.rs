//! Measured partial guard baseline; no eval replacement or mutation observer yet.
use super::reflection::{call, get};
use crate::guard::{Epoch, NativeBaseline, PrototypeCounts, native_source_marker};
use humanymous_signals::report::Guard;
use wasm_bindgen::{JsCast, JsValue};

pub(crate) struct Baseline {
    global: JsValue,
    epoch: Epoch,
    initial: PrototypeCounts,
    stopped: bool,
}
impl Baseline {
    pub(crate) fn capture() -> Result<Self, JsValue> {
        Self::capture_with(js_sys::global().into())
    }
    fn capture_with(global: JsValue) -> Result<Self, JsValue> {
        let initial = counts(&global)?;
        let to_string = get(&get(&get(&global, "Function")?, "prototype")?, "toString")?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| JsValue::from_str("native toString unavailable"))?;
        let json = get(&global, "JSON")?;
        let console_log = get(&global, "console")
            .and_then(|console| get(&console, "log"))
            .and_then(|value| native(&to_string, &value))
            .unwrap_or(false);
        let observed = NativeBaseline {
            fetch: native(&to_string, &get(&global, "fetch")?)?,
            xhr: native(&to_string, &get(&global, "XMLHttpRequest")?)?,
            stringify: native(&to_string, &get(&json, "stringify")?)?,
            parse: native(&to_string, &get(&json, "parse")?)?,
            console_log,
        };
        Ok(Self {
            global,
            epoch: Epoch::new(initial, observed),
            initial,
            stopped: false,
        })
    }
    pub(crate) fn snapshot(&self) -> Result<Guard, JsValue> {
        let current = if self.stopped {
            self.initial
        } else {
            counts(&self.global)?
        };
        self.epoch.snapshot(current).map_err(JsValue::from_str)
    }
    pub(crate) fn stop(&mut self) -> Result<(), JsValue> {
        if !self.stopped {
            self.epoch
                .stop(counts(&self.global)?)
                .map_err(JsValue::from_str)?;
            self.stopped = true;
        }
        Ok(())
    }
}
fn native(to_string: &js_sys::Function, value: &JsValue) -> Result<bool, JsValue> {
    let Ok(text) = js_sys::Reflect::apply(to_string, value, &js_sys::Array::new()) else {
        return Ok(false);
    };
    if !text.is_string() || text.unchecked_ref::<js_sys::JsString>().length() > 32768 {
        return Err(JsValue::from_str("guard native source type/budget"));
    }
    Ok(native_source_marker(&text.as_string().unwrap_or_default()))
}
fn counts(global: &JsValue) -> Result<PrototypeCounts, JsValue> {
    let object = get(global, "Object")?;
    let count = |name: &str| -> Result<usize, JsValue> {
        let prototype = get(&get(global, name)?, "prototype")?;
        let keys = call(&object, "getOwnPropertyNames", &[prototype])?;
        let count = get(&keys, "length")?
            .as_f64()
            .ok_or_else(|| JsValue::from_str("guard enumeration length required"))?;
        if !count.is_finite() || !(0.0..=16384.0).contains(&count) || count.fract() != 0.0 {
            return Err(JsValue::from_str("guard enumeration budget"));
        }
        Ok(count as usize)
    };
    PrototypeCounts::new(count("Object")?, count("Array")?).map_err(JsValue::from_str)
}
#[cfg(test)]
mod tests;
