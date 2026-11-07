use super::*;
use wasm_bindgen_test::wasm_bindgen_test;

fn realm() -> JsValue {
    let actual: JsValue = js_sys::global().into();
    let global: JsValue = Object::new().into();
    for key in ["Proxy", "FinalizationRegistry", "eval", "Function"] {
        set(&global, key, &get(&actual, key).unwrap()).unwrap();
    }
    global
}
fn state() -> (Rc<RefCell<Epoch>>, Rc<Cell<bool>>) {
    use crate::guard::{NativeBaseline, PrototypeCounts};
    (
        Rc::new(RefCell::new(Epoch::new(
            PrototypeCounts::new(0, 0).unwrap(),
            NativeBaseline {
                fetch: true,
                xhr: true,
                stringify: true,
                parse: true,
                console_log: true,
            },
        ))),
        Rc::new(Cell::new(false)),
    )
}
fn report(epoch: &Rc<RefCell<Epoch>>) -> humanymous_core::signals::report::Guard {
    epoch
        .borrow()
        .snapshot(crate::guard::PrototypeCounts::new(0, 0).unwrap())
        .unwrap()
}

#[wasm_bindgen_test]
fn call_receiver_return_and_constructor_new_target_are_forwarded() {
    let global = realm();
    let actual: JsValue = js_sys::global().into();
    let array = get(&actual, "Array").unwrap();
    let push = get(&get(&array, "prototype").unwrap(), "push").unwrap();
    set(&global, "eval", &push).unwrap();
    set(&global, "Function", &array).unwrap();
    let (epoch, fault) = state();
    let mut hooks = Hooks::install(&global, epoch.clone(), fault.clone()).unwrap();
    let eval = get(&global, "eval")
        .unwrap()
        .dyn_into::<Function>()
        .unwrap();
    let function = get(&global, "Function")
        .unwrap()
        .dyn_into::<Function>()
        .unwrap();
    let receiver = Array::new();
    let args = Array::new();
    args.push(&17.into());
    assert_eq!(
        Reflect::apply(&eval, &receiver, &args).unwrap().as_f64(),
        Some(1.0)
    );
    assert_eq!(receiver.get(0).as_f64(), Some(17.0));
    let constructed = Reflect::construct(&function, &args).unwrap();
    assert!(Array::is_array(&constructed));
    assert_eq!(get(&constructed, "length").unwrap().as_f64(), Some(17.0));
    let object = get(&actual, "Object")
        .unwrap()
        .dyn_into::<Function>()
        .unwrap();
    let alternate = Reflect::construct_with_new_target(&function, &args, &object).unwrap();
    assert_eq!(
        JsValue::from(Reflect::get_prototype_of(alternate.unchecked_ref::<Object>()).unwrap()),
        get(&object, "prototype").unwrap()
    );
    assert_eq!((report(&epoch).eval_used, report(&epoch).func_ctor), (1, 2));
    hooks.stop().unwrap();
    assert_eq!(get(&global, "eval").unwrap(), push);
    assert_eq!(get(&global, "Function").unwrap(), array);
    drop(hooks);
    // External holders can keep calling the inactive managed proxy safely.
    Reflect::apply(&eval, &receiver, &args).unwrap();
    assert_eq!(report(&epoch).eval_used, 1);
    assert!(!fault.get());
}

#[wasm_bindgen_test]
fn thrown_value_is_identical_and_failed_second_install_rolls_back_first() {
    let global = realm();
    let thrown: JsValue = Object::new().into();
    let value = thrown.clone();
    let target = Closure::<dyn Fn() -> Result<JsValue, JsValue>>::new(move || Err(value.clone()));
    set(&global, "eval", target.as_ref()).unwrap();
    let (epoch, fault) = state();
    let mut hooks = Hooks::install(&global, epoch.clone(), fault).unwrap();
    let proxy = get(&global, "eval")
        .unwrap()
        .dyn_into::<Function>()
        .unwrap();
    assert_eq!(
        Reflect::apply(&proxy, &JsValue::UNDEFINED, &Array::new()).unwrap_err(),
        thrown
    );
    assert_eq!(report(&epoch).eval_used, 1);
    hooks.stop().unwrap();
    drop(hooks);
    let descriptor =
        Reflect::get_own_property_descriptor(global.unchecked_ref::<Object>(), &"Function".into())
            .unwrap();
    set(&descriptor, "writable", &JsValue::FALSE).unwrap();
    assert!(
        Reflect::define_property(
            global.unchecked_ref::<Object>(),
            &"Function".into(),
            descriptor.unchecked_ref()
        )
        .unwrap()
    );
    let (epoch, fault) = state();
    assert!(Hooks::install(&global, epoch, fault).is_err());
    assert_eq!(get(&global, "eval").unwrap(), *target.as_ref());
}

#[wasm_bindgen_test]
fn replacement_is_not_overwritten_and_descriptor_changes_are_preserved() {
    let global = realm();
    let (epoch, fault) = state();
    let original = get(&global, "Function").unwrap();
    let mut hooks = Hooks::install(&global, epoch, fault.clone()).unwrap();
    let external: JsValue = Object::new().into();
    set(&global, "eval", &external).unwrap();
    let descriptor =
        Reflect::get_own_property_descriptor(global.unchecked_ref::<Object>(), &"Function".into())
            .unwrap();
    set(&descriptor, "enumerable", &JsValue::FALSE).unwrap();
    assert!(
        Reflect::define_property(
            global.unchecked_ref::<Object>(),
            &"Function".into(),
            descriptor.unchecked_ref()
        )
        .unwrap()
    );
    assert!(hooks.check().is_err());
    assert!(fault.get());
    hooks.stop().unwrap();
    assert_eq!(get(&global, "eval").unwrap(), external);
    assert_eq!(get(&global, "Function").unwrap(), original);
    let restored =
        Reflect::get_own_property_descriptor(global.unchecked_ref::<Object>(), &"Function".into())
            .unwrap();
    assert_eq!(get(&restored, "enumerable").unwrap().as_bool(), Some(false));
}

#[wasm_bindgen_test]
fn real_collector_requires_explicit_opt_in_and_restores_intrinsic_references() {
    let global: JsValue = js_sys::global().into();
    let original_eval = get(&global, "eval").unwrap();
    let original_function = get(&global, "Function").unwrap();
    let mut collector = crate::BrowserCollector::new().unwrap();
    assert_eq!(get(&global, "eval").unwrap(), original_eval);
    collector.enable_guard_hooks().unwrap();
    collector.enable_guard_hooks().unwrap();
    let current = get(&global, "eval")
        .unwrap()
        .dyn_into::<Function>()
        .unwrap();
    let args = Array::new();
    args.push(&7.into());
    assert_eq!(
        Reflect::apply(&current, &JsValue::UNDEFINED, &args)
            .unwrap()
            .as_f64(),
        Some(7.0)
    );
    assert_eq!(collector.guard.snapshot().unwrap().eval_used, 1);
    assert!(collector.guard.snapshot().unwrap().reported);
    collector.stop().unwrap();
    assert_eq!(get(&global, "eval").unwrap(), original_eval);
    assert_eq!(get(&global, "Function").unwrap(), original_function);
    assert!(collector.enable_guard_hooks().is_err());
}

#[wasm_bindgen_test]
fn unrestoreable_proxy_stays_inert_and_continues_forwarding_safely() {
    let global = realm();
    let (epoch, fault) = state();
    let mut hooks = Hooks::install(&global, epoch.clone(), fault.clone()).unwrap();
    let proxy = get(&global, "eval")
        .unwrap()
        .dyn_into::<Function>()
        .unwrap();
    let descriptor =
        Reflect::get_own_property_descriptor(global.unchecked_ref::<Object>(), &"eval".into())
            .unwrap();
    set(&descriptor, "writable", &JsValue::FALSE).unwrap();
    set(&descriptor, "configurable", &JsValue::FALSE).unwrap();
    assert!(
        Reflect::define_property(
            global.unchecked_ref::<Object>(),
            &"eval".into(),
            descriptor.unchecked_ref()
        )
        .unwrap()
    );
    assert!(hooks.stop().is_err());
    drop(hooks);
    assert!(fault.get());
    let args = Array::new();
    args.push(&9.into());
    assert_eq!(
        Reflect::apply(&proxy, &JsValue::UNDEFINED, &args)
            .unwrap()
            .as_f64(),
        Some(9.0)
    );
    assert_eq!(report(&epoch).eval_used, 0);
}

#[wasm_bindgen_test]
fn nested_forwarded_calls_do_not_hold_an_epoch_borrow_across_user_code() {
    let global = realm();
    let (epoch, fault) = state();
    let proxy_slot = Rc::new(RefCell::new(JsValue::UNDEFINED));
    let slot = proxy_slot.clone();
    let nesting = Cell::new(false);
    let target = Closure::<dyn Fn() -> Result<JsValue, JsValue>>::new(move || {
        if nesting.replace(true) {
            return Ok(11.into());
        }
        let proxy = slot.borrow().clone();
        let result = Reflect::apply(
            proxy.unchecked_ref::<Function>(),
            &JsValue::UNDEFINED,
            &Array::new(),
        );
        nesting.set(false);
        result
    });
    set(&global, "eval", target.as_ref()).unwrap();
    let mut hooks = Hooks::install(&global, epoch.clone(), fault.clone()).unwrap();
    let proxy = get(&global, "eval").unwrap();
    *proxy_slot.borrow_mut() = proxy.clone();
    assert_eq!(
        Reflect::apply(
            proxy.unchecked_ref::<Function>(),
            &JsValue::UNDEFINED,
            &Array::new()
        )
        .unwrap()
        .as_f64(),
        Some(11.0)
    );
    assert_eq!(report(&epoch).eval_used, 2);
    assert!(!fault.get());
    hooks.stop().unwrap();
    *proxy_slot.borrow_mut() = JsValue::UNDEFINED;
}

#[wasm_bindgen_test]
fn overlapping_same_realm_activation_is_rejected_until_owned_stop() {
    let global = realm();
    let (first, fault) = state();
    let original = get(&global, "eval").unwrap();
    let mut hooks = Hooks::install(&global, first, fault).unwrap();
    let installed = get(&global, "eval").unwrap();
    let (second, fault) = state();
    assert!(Hooks::install(&global, second.clone(), fault.clone()).is_err());
    assert_eq!(get(&global, "eval").unwrap(), installed);
    hooks.stop().unwrap();
    assert_eq!(get(&global, "eval").unwrap(), original);
    let mut next = Hooks::install(&global, second, fault).unwrap();
    next.stop().unwrap();
    assert_eq!(get(&global, "eval").unwrap(), original);
}
