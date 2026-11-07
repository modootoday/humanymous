//! Passive event registration and callback lifetime, separate from pure reduction.
use super::reflection::{apply, get, truthy};
use crate::behavior::{KeySample, MAX_SAMPLES, PointerSample, Recorder};
use humanymous_core::signals::report::BehaviorSummary;
use js_sys::Function;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

const EVENTS: [&str; 6] = [
    "pointermove",
    "pointerdown",
    "keydown",
    "keyup",
    "wheel",
    "paste",
];
fn error(text: &str) -> JsValue {
    JsValue::from_str(text)
}
fn number(object: &JsValue, key: &str) -> Result<f64, JsValue> {
    get(object, key)?
        .as_f64()
        .filter(|n| n.is_finite() && n.abs() <= 9_007_199_254_740_991.0)
        .ok_or_else(|| error("invalid numeric event observation"))
}
#[derive(Clone)]
struct Clock {
    performance: JsValue,
    now: Function,
}
impl Clock {
    fn read(&self) -> Result<f64, JsValue> {
        apply(&self.now, &self.performance, &[])?
            .as_f64()
            .filter(|n| n.is_finite() && *n >= 0.0 && *n <= 9_007_199_254_740_991.0)
            .ok_or_else(|| error("invalid collection clock"))
    }
}
struct State {
    recorder: Option<Recorder>,
    frozen: Option<BehaviorSummary>,
    fault: Option<JsValue>,
    codes: Vec<JsValue>,
}
impl State {
    fn token(&mut self, event: &JsValue) -> Result<u32, JsValue> {
        let code = get(event, "code")?;
        let code = if truthy(&code) {
            code
        } else {
            JsValue::from_str("k")
        };
        if !code.is_string() || code.unchecked_ref::<js_sys::JsString>().length() > 128 {
            return Err(error("invalid physical key identity"));
        }
        if let Some(index) = self.codes.iter().position(|value| value == &code) {
            return Ok(index as u32 + 1);
        }
        if self.codes.len() >= MAX_SAMPLES * 2 {
            return Err(error("key pairing budget"));
        }
        self.codes.push(code);
        Ok(self.codes.len() as u32)
    }
    fn receive(&mut self, kind: usize, event: &JsValue, clock: &Clock) -> Result<(), JsValue> {
        if self.fault.is_some() || self.recorder.is_none() {
            return Ok(());
        }
        let token = if kind == 2 || kind == 3 {
            Some(self.token(event)?)
        } else {
            None
        };
        let r = self.recorder.as_mut().expect("recorder checked");
        let result = match kind {
            0 => {
                // Optional probe deliberately keeps the original catch-and-fallback.
                let coalesced = (|| {
                    let method = get(event, "getCoalescedEvents")?;
                    if !method.is_function() {
                        return Ok(1.0);
                    }
                    let values = apply(method.unchecked_ref(), event, &[])?;
                    let length = number(&values, "length")?;
                    Ok::<_, JsValue>(if length > 0.0 { length } else { 1.0 })
                })()
                .unwrap_or(1.0);
                let trusted = truthy(&get(event, "isTrusted")?);
                let sample = PointerSample {
                    x: number(event, "clientX")?,
                    y: number(event, "clientY")?,
                    time_ms: clock.read()?,
                };
                r.pointer_move(sample, clock.read()?, clock.read()?, coalesced, trusted)
            }
            1 => {
                let trusted = truthy(&get(event, "isTrusted")?);
                r.pointer_down(clock.read()?, clock.read()?, trusted)
            }
            2 => {
                let trusted = truthy(&get(event, "isTrusted")?);
                r.key_down(
                    KeySample {
                        token: token.expect("key token"),
                        time_ms: clock.read()?,
                    },
                    clock.read()?,
                    trusted,
                )
            }
            3 => r.key_up(KeySample {
                token: token.expect("key token"),
                time_ms: clock.read()?,
            }),
            4 => r.wheel(clock.read()?),
            5 => r.paste(),
            _ => return Err(error("unknown collection event")),
        };
        result.map_err(error)
    }
}
struct Listener {
    name: &'static str,
    callback: Closure<dyn FnMut(JsValue)>,
}
pub(crate) struct Collection {
    state: Rc<RefCell<State>>,
    active: Rc<Cell<bool>>,
    reentered: Rc<Cell<bool>>,
    clock: Clock,
    target: JsValue,
    remove: Function,
    listeners: Vec<Listener>,
}
impl Collection {
    pub(crate) fn start() -> Result<Self, JsValue> {
        let target: JsValue = js_sys::global().into();
        if !truthy(&get(&target, "document")?) {
            return Err(error("browser document required"));
        }
        let performance = get(&target, "performance")?;
        let clock = Clock {
            now: get(&performance, "now")?
                .dyn_into()
                .map_err(|_| error("performance clock unavailable"))?,
            performance,
        };
        let add: Function = get(&target, "addEventListener")?
            .dyn_into()
            .map_err(|_| error("event registration unavailable"))?;
        let remove: Function = get(&target, "removeEventListener")?
            .dyn_into()
            .map_err(|_| error("event removal unavailable"))?;
        let state = Rc::new(RefCell::new(State {
            recorder: Some(Recorder::new(clock.read()?).map_err(error)?),
            frozen: None,
            fault: None,
            codes: Vec::new(),
        }));
        let active = Rc::new(Cell::new(true));
        let reentered = Rc::new(Cell::new(false));
        let mut collection = Self {
            state,
            active,
            reentered,
            clock,
            target,
            remove,
            listeners: Vec::new(),
        };
        let options = js_sys::Object::new();
        if !js_sys::Reflect::set(&options, &JsValue::from_str("passive"), &JsValue::TRUE)? {
            return Err(error("passive option rejected"));
        }
        for (kind, name) in EVENTS.into_iter().enumerate() {
            let state = collection.state.clone();
            let active = collection.active.clone();
            let reentered = collection.reentered.clone();
            let clock = collection.clock.clone();
            let callback = Closure::wrap(Box::new(move |event: JsValue| {
                if !active.get() {
                    return;
                }
                if let Ok(mut state) = state.try_borrow_mut() {
                    if let Err(fault) = state.receive(kind, &event, &clock) {
                        state.fault = Some(fault);
                    }
                    if !active.get() {
                        state.recorder = None;
                        state.codes.clear();
                    }
                } else {
                    reentered.set(true);
                }
            }) as Box<dyn FnMut(JsValue)>);
            collection.listeners.push(Listener { name, callback });
            let listener = collection.listeners.last().expect("listener inserted");
            apply(
                &add,
                &collection.target,
                &[
                    JsValue::from_str(name),
                    listener.callback.as_ref().clone(),
                    options.clone().into(),
                ],
            )?;
        }
        Ok(collection)
    }
    pub(crate) fn summary(&self) -> Result<BehaviorSummary, JsValue> {
        let now = if self.active.get() {
            Some(self.clock.read()?)
        } else {
            None
        };
        if self.reentered.get() {
            return Err(error("reentrant event observation"));
        }
        let state = self
            .state
            .try_borrow()
            .map_err(|_| error("collection state borrowed"))?;
        if let Some(fault) = &state.fault {
            return Err(fault.clone());
        }
        if let Some(r) = &state.recorder {
            r.summary(now.ok_or_else(|| error("collection clock absent"))?)
                .map_err(error)
        } else {
            state
                .frozen
                .clone()
                .ok_or_else(|| error("collection stopped without summary"))
        }
    }
    pub(crate) fn stop(&mut self) -> Result<(), JsValue> {
        if !self.active.get() {
            if self.reentered.get() {
                return Err(error("reentrant event observation"));
            }
            return self
                .state
                .try_borrow()
                .map_err(|_| error("collection state borrowed"))?
                .fault
                .clone()
                .map_or(Ok(()), Err);
        }
        // Freeze before deactivation, but always detach even when summarization fails.
        let frozen = self.summary();
        self.active.set(false);
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.recorder = None;
            state.codes.clear();
            match frozen {
                Ok(summary) => state.frozen = Some(summary),
                Err(fault) => state.fault = Some(fault),
            }
        } else {
            self.reentered.set(true);
        }
        for listener in self.listeners.drain(..) {
            if let Err(fault) = apply(
                &self.remove,
                &self.target,
                &[
                    JsValue::from_str(listener.name),
                    listener.callback.as_ref().clone(),
                    JsValue::FALSE,
                ],
            ) {
                if let Ok(mut state) = self.state.try_borrow_mut() {
                    state.fault = Some(fault);
                } else {
                    self.reentered.set(true);
                }
                // The page may still hold this function. Its active flag is already false.
                listener.callback.forget();
            }
        }
        if self.reentered.get() {
            return Err(error("reentrant event observation"));
        }
        self.state
            .try_borrow()
            .map_err(|_| error("collection state borrowed"))?
            .fault
            .clone()
            .map_or(Ok(()), Err)
    }
}
impl Drop for Collection {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests;
