//! Explicit opt-in proxy forwarding. Caller source remains opaque and transient.
#[cfg(test)]
mod tests;
use super::super::reflection::get;
use crate::guard::{Epoch, Hook};
use js_sys::{Array, Function, Object, Reflect};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

pub(super) struct Hooks {
    eval: Owner,
    function: Owner,
    lease: Lease,
}
thread_local! {
    static REALMS: RefCell<Vec<(JsValue, Weak<Cell<bool>>)>> = const { RefCell::new(Vec::new()) };
}
struct Lease(Rc<Cell<bool>>);
impl Lease {
    fn acquire(global: &JsValue) -> Result<Self, JsValue> {
        REALMS.with(|realms| {
            let mut realms = realms
                .try_borrow_mut()
                .map_err(|_| error("guard realm registry busy"))?;
            realms.retain(|(_, owner)| owner.upgrade().is_some_and(|active| active.get()));
            if realms.iter().any(|(realm, _)| realm == global) {
                return Err(error("guard realm already instrumented"));
            }
            if realms.len() >= 64 {
                return Err(error("guard realm budget"));
            }
            let active = Rc::new(Cell::new(true));
            realms.push((global.clone(), Rc::downgrade(&active)));
            Ok(Self(active))
        })
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
impl Hooks {
    pub(super) fn check(&self) -> Result<(), JsValue> {
        for owner in [&self.eval, &self.function] {
            let descriptor = Reflect::get_own_property_descriptor(&owner.global, &owner.key)?;
            if !owner.active.get()
                || descriptor.is_undefined()
                || get(&descriptor, "value")? != owner.proxy
            {
                owner.fault.set(true);
                return Err(error("guard hook ownership lost"));
            }
        }
        Ok(())
    }
    pub(super) fn install(
        global: &JsValue,
        epoch: Rc<RefCell<Epoch>>,
        fault: Rc<Cell<bool>>,
    ) -> Result<Self, JsValue> {
        let lease = Lease::acquire(global)?;
        if !get(global, "FinalizationRegistry")?.is_function() {
            return Err(error("managed hook lifetime unavailable"));
        }
        let constructor = get(global, "Proxy")?
            .dyn_into::<Function>()
            .map_err(|_| error("Proxy unavailable"))?;
        let eval = Owner::install(
            global,
            "eval",
            Hook::Eval,
            &constructor,
            epoch.clone(),
            fault.clone(),
        )?;
        let function = Owner::install(
            global,
            "Function",
            Hook::Function,
            &constructor,
            epoch.clone(),
            fault,
        )?;
        {
            let mut state = epoch
                .try_borrow_mut()
                .map_err(|_| error("guard epoch busy"))?;
            state.cover(Hook::Eval);
            state.cover(Hook::Function);
        }
        Ok(Self {
            eval,
            function,
            lease,
        })
    }
    pub(super) fn stop(&mut self) -> Result<(), JsValue> {
        self.eval.active.set(false);
        self.function.active.set(false);
        // Both must be attempted even if the first restoration fails.
        let first = self.eval.stop();
        let second = self.function.stop();
        self.lease.0.set(false);
        first.and(second)
    }
}
struct Owner {
    global: Object,
    key: JsValue,
    target: Function,
    proxy: JsValue,
    active: Rc<Cell<bool>>,
    fault: Rc<Cell<bool>>,
}
impl Owner {
    fn install(
        global: &JsValue,
        name: &str,
        hook: Hook,
        proxy_constructor: &Function,
        epoch: Rc<RefCell<Epoch>>,
        fault: Rc<Cell<bool>>,
    ) -> Result<Self, JsValue> {
        let global = global
            .clone()
            .dyn_into::<Object>()
            .map_err(|_| error("hook realm unavailable"))?;
        let key = JsValue::from_str(name);
        let descriptor = Reflect::get_own_property_descriptor(&global, &key)?;
        if descriptor.is_undefined()
            || get(&descriptor, "writable")?.as_bool() != Some(true)
            || !get(&descriptor, "get")?.is_undefined()
            || !get(&descriptor, "set")?.is_undefined()
        {
            return Err(error("writable own hook data property required"));
        }
        let target = get(&descriptor, "value")?
            .dyn_into::<Function>()
            .map_err(|_| error("hook target unavailable"))?;
        let active = Rc::new(Cell::new(false));
        let handler = Object::new();
        let apply_target = target.clone();
        let apply_epoch = epoch.clone();
        let apply_active = active.clone();
        let apply_fault = fault.clone();
        let apply = Closure::<dyn Fn(JsValue, JsValue, Array) -> Result<JsValue, JsValue>>::new(
            move |_, receiver, args| {
                observe(&apply_epoch, &apply_active, &apply_fault, hook)?;
                Reflect::apply(&apply_target, &receiver, &args)
            },
        )
        .into_js_value();
        set(&handler, "apply", &apply)?;
        let construct_target = target.clone();
        let construct_active = active.clone();
        let construct_fault = fault.clone();
        let construct =
            Closure::<dyn Fn(JsValue, Array, Function) -> Result<JsValue, JsValue>>::new(
                move |_, args, new_target| {
                    observe(&epoch, &construct_active, &construct_fault, hook)?;
                    Reflect::construct_with_new_target(&construct_target, &args, &new_target)
                },
            )
            .into_js_value();
        set(&handler, "construct", &construct)?;
        let args = Array::new();
        args.push(&target);
        args.push(&handler);
        let proxy = Reflect::construct(proxy_constructor, &args)?;
        let mut owner = Self {
            global,
            key,
            target,
            proxy,
            active,
            fault,
        };
        set(&descriptor, "value", &owner.proxy)?;
        match Reflect::define_property(&owner.global, &owner.key, descriptor.unchecked_ref()) {
            Ok(true) => {
                owner.active.set(true);
                Ok(owner)
            }
            _ => {
                let _ = owner.stop();
                Err(error("hook installation failed"))
            }
        }
    }
    fn stop(&mut self) -> Result<(), JsValue> {
        self.active.set(false);
        let result = (|| {
            let current = Reflect::get_own_property_descriptor(&self.global, &self.key)?;
            if current.is_undefined() || get(&current, "value")? != self.proxy {
                return Ok(());
            }
            set(&current, "value", &self.target)?;
            if !Reflect::define_property(&self.global, &self.key, current.unchecked_ref())? {
                return Err(error("hook restoration failed"));
            }
            Ok(())
        })();
        if result.is_err() {
            self.fault.set(true);
        }
        result
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
fn observe(
    epoch: &Rc<RefCell<Epoch>>,
    active: &Cell<bool>,
    fault: &Cell<bool>,
    hook: Hook,
) -> Result<(), JsValue> {
    if !active.get() {
        return Ok(());
    }
    if fault.get() {
        return Err(error("guard epoch invalid"));
    }
    let result = epoch
        .try_borrow_mut()
        .map_err(|_| error("guard epoch busy"))
        .and_then(|mut epoch| epoch.record(hook).map_err(JsValue::from_str));
    if result.is_err() {
        fault.set(true);
    }
    result
}
fn set(object: &JsValue, name: &str, value: &JsValue) -> Result<(), JsValue> {
    if Reflect::set(object, &name.into(), value)? {
        Ok(())
    } else {
        Err(error("hook descriptor unavailable"))
    }
}
fn error(value: &str) -> JsValue {
    JsValue::from_str(value)
}
