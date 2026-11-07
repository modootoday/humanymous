use super::*;
use crate::browser::resources::Settlement;
use js_sys::Uint8Array;

pub(super) fn set(object: &JsValue, name: &str, value: &JsValue) -> Result<(), JsValue> {
    if js_sys::Reflect::set(object, &name.into(), value)? {
        Ok(())
    } else {
        Err(error("transport property rejected"))
    }
}
pub(super) fn construct(
    global: &JsValue,
    name: &str,
    args: &[JsValue],
) -> Result<JsValue, JsValue> {
    let constructor = get(global, name)?
        .dyn_into::<Function>()
        .map_err(|_| error("transport API unavailable"))?;
    let arguments = Array::new();
    for arg in args {
        arguments.push(arg);
    }
    js_sys::Reflect::construct(&constructor, &arguments)
        .map_err(|_| error("transport construction failed"))
}
pub(super) fn now(global: &JsValue) -> Result<u64, JsValue> {
    call(&get(global, "Date")?, "now", &[])?
        .as_f64()
        .filter(|n| {
            n.is_finite()
                && *n >= 0.0
                && *n <= humanymous_core::integrity::request::browser::MAX_SAFE_INTEGER as f64
                && n.fract() == 0.0
        })
        .map(|n| n as u64)
        .ok_or_else(|| error("invalid transport clock"))
}
pub(super) async fn wait(timer: &Timer, promise: JsValue) -> Result<JsValue, JsValue> {
    let promise = promise
        .dyn_into::<Promise>()
        .map_err(|_| error("transport promise required"))?;
    match timer.race(&promise).await {
        Settlement::Fulfilled(value) => Ok(value),
        Settlement::Rejected => Err(error("transport operation rejected")),
        Settlement::TimedOut => Err(error("transport deadline exceeded")),
    }
}
struct Reader {
    object: JsValue,
    finished: bool,
}
impl Drop for Reader {
    fn drop(&mut self) {
        if !self.finished {
            let _ = call(&self.object, "cancel", &[]);
        }
        let _ = call(&self.object, "releaseLock", &[]);
    }
}
pub(super) async fn body(
    response: &JsValue,
    timer: &Timer,
    limit: usize,
) -> Result<Zeroizing<Vec<u8>>, JsValue> {
    let stream = get(response, "body")?;
    if stream.is_null() {
        return Ok(Zeroizing::new(Vec::new()));
    }
    let mut reader = Reader {
        object: call(&stream, "getReader", &[])?,
        finished: false,
    };
    let mut bytes = Zeroizing::new(Vec::new());
    loop {
        let item = wait(timer, call(&reader.object, "read", &[])?).await?;
        match get(&item, "done")?.as_bool() {
            Some(true) => {
                reader.finished = true;
                return Ok(bytes);
            }
            Some(false) => {}
            None => return Err(error("malformed response stream")),
        }
        let chunk = get(&item, "value")?
            .dyn_into::<Uint8Array>()
            .map_err(|_| error("byte response chunk required"))?;
        let start = bytes.len();
        let end = start
            .checked_add(chunk.length() as usize)
            .filter(|n| *n <= limit)
            .ok_or_else(|| error("response body exceeds limit"))?;
        bytes.resize(end, 0);
        chunk.copy_to(&mut bytes[start..]);
    }
}
pub(super) fn status(response: &JsValue) -> Result<u16, JsValue> {
    if get(response, "redirected")?.as_bool() != Some(false) {
        return Err(error("redirected response refused"));
    }
    get(response, "status")?
        .as_f64()
        .filter(|n| n.is_finite() && (200.0..=599.0).contains(n) && n.fract() == 0.0)
        .map(|n| n as u16)
        .ok_or_else(|| error("invalid response status"))
}
pub(super) fn header(headers: &JsValue, name: &str) -> Result<String, JsValue> {
    let value = call(headers, "get", &[name.into()])?;
    if value.is_null() {
        return Ok(String::new());
    }
    text(&value, 32768)
}
pub(super) fn text(value: &JsValue, limit: usize) -> Result<String, JsValue> {
    if !value.is_string() || value.unchecked_ref::<js_sys::JsString>().length() as usize > limit {
        return Err(error("string observation exceeds limit"));
    }
    let value = value
        .as_string()
        .ok_or_else(|| error("invalid string observation"))?;
    if value.len() > limit {
        return Err(error("string observation exceeds limit"));
    }
    Ok(value)
}
pub(super) fn exposed_headers(headers: &JsValue) -> Result<String, JsValue> {
    let iterator = call(headers, "entries", &[])?;
    let mut pairs = Vec::new();
    let mut total = 0usize;
    for _ in 0..=256 {
        let item = call(&iterator, "next", &[])?;
        match get(&item, "done")?.as_bool() {
            Some(true) => {
                return serde_json::to_string(&pairs)
                    .map_err(|_| error("header serialization failed"));
            }
            Some(false) => {}
            None => return Err(error("invalid header iterator")),
        }
        let pair = get(&item, "value")?
            .dyn_into::<Array>()
            .map_err(|_| error("invalid header pair"))?;
        if pair.length() != 2 {
            return Err(error("invalid header pair"));
        }
        let name = text(&pair.get(0), 1024)?;
        let value = text(&pair.get(1), 32768)?;
        total = total
            .checked_add(name.len() + value.len())
            .filter(|n| *n <= 65536)
            .ok_or_else(|| error("response header budget"))?;
        if !name.to_ascii_lowercase().starts_with("x-humanymous-") {
            pairs.push([name, value]);
        }
    }
    Err(error("response header count budget"))
}
