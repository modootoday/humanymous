//! Owned observer lifecycle. Only node type/marker are read; no script payload.
use super::super::reflection::{apply, get};
use crate::guard::{Epoch, Hook};
use js_sys::{Array, Function, Object, Reflect};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

pub(super) struct Observer {
    object: JsValue,
    disconnect: Function,
    take_records: Function,
    callback: Option<Closure<dyn Fn(JsValue, JsValue)>>,
    active: Rc<Cell<bool>>,
    fault: Rc<Cell<bool>>,
}

impl Observer {
    pub(super) fn start(
        global: &JsValue,
        epoch: Rc<RefCell<Epoch>>,
        fault: Rc<Cell<bool>>,
    ) -> Result<Self, JsValue> {
        let constructor = get(global, "MutationObserver")?
            .dyn_into::<Function>()
            .map_err(|_| error("guard observer unavailable"))?;
        let document = get(global, "document")?;
        if document.is_null() || document.is_undefined() {
            return Err(error("guard document unavailable"));
        }
        let active = Rc::new(Cell::new(false));
        let callback_active = active.clone();
        let callback_fault = fault.clone();
        let running = Cell::new(false);
        let callback_epoch = epoch.clone();
        let callback = Closure::<dyn Fn(JsValue, JsValue)>::new(move |records, _observer| {
            if !callback_active.get() || callback_fault.get() {
                return;
            }
            if running.replace(true) {
                callback_fault.set(true);
                return;
            }
            let result = count(&records).and_then(|n| {
                if !callback_active.get() || callback_fault.get() {
                    return Ok(());
                }
                record(&callback_epoch, n)
            });
            running.set(false);
            if result.is_err() {
                callback_fault.set(true);
            }
        });
        let args = Array::new();
        args.push(callback.as_ref());
        let object = match Reflect::construct(&constructor, &args) {
            Ok(object) => object,
            Err(_) => {
                // A substituted constructor may have retained its argument.
                // It remains inert because active was never enabled.
                callback.forget();
                return Err(error("guard observer construction failed"));
            }
        };
        let methods = (|| {
            let disconnect = function(&object, "disconnect")?;
            let take_records = function(&object, "takeRecords")?;
            let observe = function(&object, "observe")?;
            Ok::<_, JsValue>((disconnect, take_records, observe))
        })();
        let (disconnect, take_records, observe) = match methods {
            Ok(methods) => methods,
            Err(_) => {
                callback.forget();
                return Err(error("guard observer methods unavailable"));
            }
        };
        let mut owner = Self {
            object,
            disconnect,
            take_records,
            callback: Some(callback),
            active,
            fault,
        };
        let options = Object::new();
        Reflect::set(&options, &"childList".into(), &JsValue::TRUE)?;
        Reflect::set(&options, &"subtree".into(), &JsValue::TRUE)?;
        if apply(&observe, &owner.object, &[document, options.into()]).is_err() {
            owner.detach();
            return Err(error("guard observer registration failed"));
        }
        epoch
            .try_borrow_mut()
            .map_err(|_| error("guard epoch busy"))?
            .cover(Hook::Script);
        owner.active.set(true);
        Ok(owner)
    }

    pub(super) fn stop(&mut self, epoch: &Rc<RefCell<Epoch>>) -> Result<(), JsValue> {
        if !self.active.get() {
            return if self.fault.get() {
                Err(error("guard observer invalid"))
            } else {
                Ok(())
            };
        }
        let result = apply(&self.take_records, &self.object, &[])
            .and_then(|records| count(&records))
            .and_then(|n| record(epoch, n));
        if result.is_err() {
            self.fault.set(true);
        }
        self.detach();
        if self.fault.get() {
            Err(error("guard observer invalid"))
        } else {
            Ok(())
        }
    }

    fn detach(&mut self) {
        self.active.set(false);
        if apply(&self.disconnect, &self.object, &[]).is_err() {
            self.fault.set(true);
            if let Some(callback) = self.callback.take() {
                callback.forget();
            }
        }
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        self.detach();
    }
}

fn function(object: &JsValue, name: &str) -> Result<Function, JsValue> {
    get(object, name)?
        .dyn_into::<Function>()
        .map_err(|_| error("guard observer method invalid"))
}
fn error(value: &str) -> JsValue {
    JsValue::from_str(value)
}
fn length(value: &JsValue, limit: usize) -> Result<usize, JsValue> {
    get(value, "length")?
        .as_f64()
        .filter(|n| n.is_finite() && *n >= 0.0 && *n <= limit as f64 && n.fract() == 0.0)
        .map(|n| n as usize)
        .ok_or_else(|| error("guard mutation budget"))
}
fn count(records: &JsValue) -> Result<usize, JsValue> {
    let mut visited = 0;
    let mut scripts = 0;
    for i in 0..length(records, 4096)? {
        let mutation = Reflect::get(records, &(i as u32).into())?;
        let nodes = get(&mutation, "addedNodes")?;
        let n = length(&nodes, 16384)?;
        visited += n;
        if visited > 16384 {
            return Err(error("guard mutation budget"));
        }
        for j in 0..n {
            let node = Reflect::get(&nodes, &(j as u32).into())?;
            if get(&node, "tagName")?.as_string().as_deref() != Some("SCRIPT") {
                continue;
            }
            let dataset = get(&node, "dataset")?;
            let marked = if dataset.is_null() || dataset.is_undefined() {
                false
            } else {
                get(&dataset, "hm")?.is_truthy()
            };
            if !marked {
                scripts += 1;
            }
        }
    }
    Ok(scripts)
}
fn record(epoch: &Rc<RefCell<Epoch>>, count: usize) -> Result<(), JsValue> {
    let mut epoch = epoch
        .try_borrow_mut()
        .map_err(|_| error("guard epoch busy"))?;
    for _ in 0..count {
        epoch.record(Hook::Script).map_err(JsValue::from_str)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn set(object: &JsValue, key: &str, value: &JsValue) {
        assert!(Reflect::set(object, &key.into(), value).unwrap());
    }
    fn records(nodes: &JsValue) -> JsValue {
        let mutation: JsValue = Object::new().into();
        set(&mutation, "addedNodes", nodes);
        let batch = Array::new();
        batch.push(&mutation);
        batch.into()
    }

    #[wasm_bindgen_test]
    fn marker_truthiness_and_delivery_budgets_do_not_inspect_script_payload() {
        let nodes = Array::new();
        for marker in [
            JsValue::UNDEFINED,
            "".into(),
            JsValue::FALSE,
            JsValue::from_f64(0.0),
            "owned".into(),
            JsValue::TRUE,
        ] {
            let node: JsValue = Object::new().into();
            set(&node, "tagName", &"SCRIPT".into());
            let dataset: JsValue = Object::new().into();
            set(&dataset, "hm", &marker);
            set(&node, "dataset", &dataset);
            nodes.push(&node);
        }
        assert_eq!(count(&records(&nodes.into())).unwrap(), 4);
        let oversized: JsValue = Object::new().into();
        set(&oversized, "length", &JsValue::from_f64(4097.0));
        assert!(count(&oversized).is_err());
        for size in [16385.0, -1.0, 0.5, f64::NAN] {
            let nodes: JsValue = Object::new().into();
            set(&nodes, "length", &size.into());
            assert!(count(&records(&nodes)).is_err());
        }
    }

    #[wasm_bindgen_test]
    fn malformed_delivery_invalidates_snapshot_but_still_detaches() {
        let mut baseline = super::super::Baseline::capture().unwrap();
        let callback = baseline
            .observer
            .as_ref()
            .unwrap()
            .callback
            .as_ref()
            .unwrap()
            .as_ref()
            .clone();
        apply(
            callback.unchecked_ref(),
            &JsValue::UNDEFINED,
            &[records(&JsValue::UNDEFINED), JsValue::UNDEFINED],
        )
        .unwrap();
        assert!(baseline.snapshot().is_err());
        assert!(baseline.stop().is_err());
        assert!(!baseline.observer.as_ref().unwrap().active.get());
    }

    #[wasm_bindgen_test]
    fn disconnect_failure_preserves_an_inert_reachable_callback() {
        let mut baseline = super::super::Baseline::capture().unwrap();
        let observer = baseline.observer.as_mut().unwrap();
        let callback = observer.callback.as_ref().unwrap().as_ref().clone();
        let original = observer.disconnect.clone();
        let failure = Closure::<dyn Fn() -> Result<JsValue, JsValue>>::new(|| {
            Err(error("owned disconnect fixture"))
        });
        observer.disconnect = failure.as_ref().unchecked_ref::<Function>().clone();
        assert!(baseline.stop().is_err());
        let observer = baseline.observer.as_mut().unwrap();
        assert!(!observer.active.get());
        assert!(observer.callback.is_none());
        // Leaked callback remains callable and inert even with malformed input.
        apply(
            callback.unchecked_ref(),
            &JsValue::UNDEFINED,
            &[JsValue::UNDEFINED, JsValue::UNDEFINED],
        )
        .unwrap();
        observer.disconnect = original;
    }

    #[wasm_bindgen_test]
    fn reentrant_observation_invalidates_without_a_refcell_or_closure_panic() {
        let mut baseline = super::super::Baseline::capture().unwrap();
        let callback = baseline
            .observer
            .as_ref()
            .unwrap()
            .callback
            .as_ref()
            .unwrap()
            .as_ref()
            .clone();
        let nested_callback = callback.clone();
        let getter = Closure::<dyn Fn() -> JsValue>::new(move || {
            apply(
                nested_callback.unchecked_ref(),
                &JsValue::UNDEFINED,
                &[Array::new().into(), JsValue::UNDEFINED],
            )
            .unwrap();
            "SCRIPT".into()
        });
        let node: JsValue = Object::new().into();
        let descriptor: JsValue = Object::new().into();
        set(&descriptor, "get", getter.as_ref());
        let global: JsValue = js_sys::global().into();
        let object = get(&global, "Object").unwrap();
        let define = function(&object, "defineProperty").unwrap();
        apply(
            &define,
            &object,
            &[node.clone(), "tagName".into(), descriptor],
        )
        .unwrap();
        let nodes = Array::new();
        nodes.push(&node);
        apply(
            callback.unchecked_ref(),
            &JsValue::UNDEFINED,
            &[records(&nodes.into()), JsValue::UNDEFINED],
        )
        .unwrap();
        assert!(baseline.snapshot().is_err());
        assert!(baseline.stop().is_err());
    }
}
