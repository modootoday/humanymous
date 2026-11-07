use super::*;
use js_sys::{Object, Reflect};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_test::wasm_bindgen_test;

// Unmodified legacy collector, imported only into this migration test image.
#[wasm_bindgen(module = "/../../web/js/advanced.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = collectAdvanced)]
    fn legacy_advanced() -> Result<Promise, JsValue>;
}

fn set(object: &JsValue, key: &str, value: &JsValue) {
    assert!(Reflect::set(object, &key.into(), value).unwrap());
}
fn fixture() -> (JsValue, JsValue) {
    let global: JsValue = Object::new().into();
    let navigator: JsValue = Object::new().into();
    set(&navigator, "language", &"ko-KR".into());
    set(&navigator, "userAgent", &"Fixture Android Mobi".into());
    set(&global, "navigator", &navigator);
    let actual: JsValue = js_sys::global().into();
    for method in ["setTimeout", "clearTimeout"] {
        let bound = call(
            &get(&actual, method).unwrap(),
            "bind",
            std::slice::from_ref(&actual),
        )
        .unwrap();
        set(&global, method, &bound);
    }
    (global, navigator)
}

#[wasm_bindgen_test(async)]
async fn missing_optional_advanced_apis_keep_original_wire_sentinels() {
    let (global, _) = fixture();
    let report = probe_with(&global).await.unwrap();
    assert!(report.probed && report.mobile_ua);
    assert_eq!(report.language, "ko-KR");
    assert_eq!(report.connection_rtt, -1.0);
    assert_eq!(report.battery_level, -1.0);
    assert!(!report.battery_present && !report.widevine_supported && !report.web_gpu_present);
    assert_eq!(report.voice_count, 0);
    assert!(report.web_rtc_host_addrs.is_none() && report.web_rtc_public_addrs.is_none());
}

#[wasm_bindgen_test(async)]
async fn advanced_device_kinds_gpu_battery_connection_and_memory_void() {
    let (global, navigator) = fixture();
    let devices = Object::new();
    let enumerate = Closure::<dyn FnMut() -> Promise>::new(|| {
        let values = js_sys::Array::new();
        for kind in ["audioinput", "videoinput", "audiooutput"] {
            let value: JsValue = Object::new().into();
            set(&value, "kind", &kind.into());
            values.push(&value);
        }
        Promise::resolve(&JsValue::from(values))
    });
    set(&devices, "enumerateDevices", enumerate.as_ref());
    set(&navigator, "mediaDevices", &devices);
    let gpu = Object::new();
    let adapter = Closure::<dyn FnMut() -> Promise>::new(|| {
        let adapter: JsValue = Object::new().into();
        let info: JsValue = Object::new().into();
        set(&info, "vendor", &"owned-vendor".into());
        set(&adapter, "info", &info);
        set(&adapter, "isFallbackAdapter", &JsValue::TRUE);
        Promise::resolve(&adapter)
    });
    set(&gpu, "requestAdapter", adapter.as_ref());
    set(&navigator, "gpu", &gpu);
    let battery = Closure::<dyn FnMut() -> Promise>::new(|| {
        let b: JsValue = Object::new().into();
        set(&b, "level", &0.5.into());
        set(&b, "charging", &JsValue::FALSE);
        Promise::resolve(&b)
    });
    set(&navigator, "getBattery", battery.as_ref());
    let connection: JsValue = Object::new().into();
    set(&connection, "rtt", &42.into());
    set(&navigator, "connection", &connection);
    let performance = Object::new();
    let memory = Closure::<dyn FnMut() -> Promise>::new(|| Promise::resolve(&JsValue::UNDEFINED));
    set(
        &performance,
        "measureUserAgentSpecificMemory",
        memory.as_ref(),
    );
    set(&performance, "memory", &Object::new().into());
    set(&global, "performance", &performance);
    let report = probe_with(&global).await.unwrap();
    assert_eq!(report.media_device_count, 3);
    assert!(report.has_audio_input && report.has_video_input);
    assert!(report.web_gpu_present && report.web_gpu_fallback);
    assert_eq!(report.web_gpu_vendor, "owned-vendor");
    assert!(report.battery_present && !report.battery_charging);
    assert_eq!(report.battery_level, 0.5);
    assert_eq!(report.connection_rtt, 42.0);
    assert!(report.connection_present && report.perf_memory_present);
    assert!(!report.measure_mem_throws);
    for (object, field) in [
        (&navigator, "mediaDevices"),
        (&navigator, "gpu"),
        (&navigator, "getBattery"),
        (&global, "performance"),
    ] {
        set(object, field, &JsValue::UNDEFINED);
    }
}

#[wasm_bindgen_test(async)]
async fn advanced_eme_audio_display_and_timezone_vectors() {
    use std::{cell::Cell, rc::Rc};
    let (global, navigator) = fixture();
    let eme =
        Closure::<dyn FnMut(JsValue, JsValue) -> Promise>::new(|key: JsValue, configs: JsValue| {
            assert_eq!(key.as_string().unwrap(), "com.widevine.alpha");
            let config = get(&configs, "0").unwrap();
            assert_eq!(
                get(&get(&config, "initDataTypes").unwrap(), "0").unwrap(),
                JsValue::from_str("cenc")
            );
            for (name, content) in [
                ("audioCapabilities", "audio/mp4;codecs=\"mp4a.40.2\""),
                ("videoCapabilities", "video/mp4;codecs=\"avc1.42E01E\""),
            ] {
                let item = get(&get(&config, name).unwrap(), "0").unwrap();
                assert_eq!(
                    get(&item, "contentType").unwrap().as_string().unwrap(),
                    content
                );
            }
            Promise::resolve(&JsValue::UNDEFINED)
        });
    set(&navigator, "requestMediaKeySystemAccess", eme.as_ref());
    let closed = Rc::new(Cell::new(0));
    let closes = closed.clone();
    let close = Closure::<dyn FnMut() -> Promise>::new(move || {
        closes.set(closes.get() + 1);
        Promise::resolve(&JsValue::UNDEFINED)
    });
    let context: JsValue = Object::new().into();
    set(&context, "sampleRate", &48000.into());
    set(&context, "close", close.as_ref());
    // Bound native Object remains constructible; Rust callback wrappers do not.
    let actual: JsValue = js_sys::global().into();
    let constructor = call(
        &get(&actual, "Object").unwrap(),
        "bind",
        &[JsValue::NULL, context],
    )
    .unwrap();
    set(&global, "AudioContext", &constructor);
    let media = Closure::<dyn FnMut(JsValue) -> JsValue>::new(|query: JsValue| {
        let value: JsValue = Object::new().into();
        set(
            &value,
            "matches",
            &(query.as_string().unwrap() != "(color-gamut: p3)").into(),
        );
        value
    });
    set(&global, "matchMedia", media.as_ref());
    let options: JsValue = Object::new().into();
    set(&options, "timeZone", &"Asia/Seoul".into());
    let resolved = Closure::<dyn FnMut() -> JsValue>::new(move || options.clone());
    let formatter: JsValue = Object::new().into();
    set(&formatter, "resolvedOptions", resolved.as_ref());
    let date_time = Closure::<dyn FnMut() -> JsValue>::new(move || formatter.clone());
    let intl: JsValue = Object::new().into();
    set(&intl, "DateTimeFormat", date_time.as_ref());
    set(&global, "Intl", &intl);
    let actual: JsValue = js_sys::global().into();
    set(&global, "Date", &get(&actual, "Date").unwrap());
    let report = probe_with(&global).await.unwrap();
    assert!(report.widevine_supported && report.pointer_coarse);
    assert_eq!(report.audio_sample_rate, 48000.0);
    assert_eq!(closed.get(), 1);
    assert_eq!(report.color_gamut, "srgb");
    assert_eq!(report.dynamic_range, "high");
    assert_eq!(report.timezone_iana, "Asia/Seoul");
    for key in ["AudioContext", "matchMedia", "Intl"] {
        set(&global, key, &JsValue::UNDEFINED);
    }
    set(
        &navigator,
        "requestMediaKeySystemAccess",
        &JsValue::UNDEFINED,
    );
}

#[wasm_bindgen_test(async)]
async fn advanced_voice_event_is_owned_and_preserves_existing_handler() {
    use std::{cell::Cell, rc::Rc};
    let (global, _) = fixture();
    let actual: JsValue = js_sys::global().into();
    let target = call(
        &get(&actual, "document").unwrap(),
        "createElement",
        &["div".into()],
    )
    .unwrap();
    let calls = Rc::new(Cell::new(0));
    let reads = calls.clone();
    let voices = Closure::<dyn FnMut() -> JsValue>::new(move || {
        let result = js_sys::Array::new();
        if reads.get() > 0 {
            result.push(&Object::new());
        }
        reads.set(reads.get() + 1);
        result.into()
    });
    set(&target, "getVoices", voices.as_ref());
    let old_handler: JsValue = Object::new().into();
    set(&target, "onvoiceschanged", &old_handler);
    let add = get(&target, "addEventListener").unwrap();
    let remove = get(&target, "removeEventListener").unwrap();
    let event_target = target.clone();
    let register = Closure::<dyn FnMut(JsValue, JsValue) -> Result<JsValue, JsValue>>::new(
        move |kind, callback| {
            call(&add, "call", &[event_target.clone(), kind, callback])?;
            let constructor = get(&js_sys::global().into(), "Event")?
                .dyn_into::<js_sys::Function>()
                .unwrap();
            let args = js_sys::Array::new();
            args.push(&"voiceschanged".into());
            let event = Reflect::construct(&constructor, &args)?;
            call(&event_target, "dispatchEvent", &[event])
        },
    );
    let removed = Rc::new(Cell::new(0));
    let removals = removed.clone();
    let event_target = target.clone();
    let unregister = Closure::<dyn FnMut(JsValue, JsValue) -> Result<JsValue, JsValue>>::new(
        move |kind, callback| {
            removals.set(removals.get() + 1);
            call(&remove, "call", &[event_target.clone(), kind, callback])
        },
    );
    set(&target, "addEventListener", register.as_ref());
    set(&target, "removeEventListener", unregister.as_ref());
    set(&global, "speechSynthesis", &target);
    let report = probe_with(&global).await.unwrap();
    assert_eq!(report.voice_count, 1);
    assert_eq!(calls.get(), 2);
    assert!(removed.get() >= 1);
    assert_eq!(get(&target, "onvoiceschanged").unwrap(), old_handler);
    set(&global, "speechSynthesis", &JsValue::UNDEFINED);
}

#[wasm_bindgen_test(async)]
async fn advanced_rejections_timeouts_and_oversized_devices_do_not_become_success_values() {
    let (global, navigator) = fixture();
    let rejected =
        Closure::<dyn FnMut() -> Promise>::new(|| Promise::reject(&"owned rejection".into()));
    let performance: JsValue = Object::new().into();
    set(
        &performance,
        "measureUserAgentSpecificMemory",
        rejected.as_ref(),
    );
    set(&global, "performance", &performance);
    set(&navigator, "getBattery", rejected.as_ref());
    set(&navigator, "requestMediaKeySystemAccess", rejected.as_ref());
    let report = probe_with(&global).await.unwrap();
    assert!(report.measure_mem_throws);
    assert!(!report.battery_present && !report.widevine_supported);
    assert_eq!(report.battery_level, -1.0);
    let pending = Closure::<dyn FnMut() -> Promise>::new(|| Promise::new(&mut |_, _| {}));
    set(
        &performance,
        "measureUserAgentSpecificMemory",
        pending.as_ref(),
    );
    assert!(!probe_with(&global).await.unwrap().measure_mem_throws);
    let oversized = Closure::<dyn FnMut() -> Promise>::new(|| {
        let values = js_sys::Array::new();
        values.set_length(4097);
        Promise::resolve(&JsValue::from(values))
    });
    let devices: JsValue = Object::new().into();
    set(&devices, "enumerateDevices", oversized.as_ref());
    set(&navigator, "mediaDevices", &devices);
    assert!(probe_with(&global).await.is_err());
    set(&navigator, "mediaDevices", &JsValue::UNDEFINED);
    set(&navigator, "getBattery", &JsValue::UNDEFINED);
    set(
        &navigator,
        "requestMediaKeySystemAccess",
        &JsValue::UNDEFINED,
    );
    set(&global, "performance", &JsValue::UNDEFINED);
}

#[wasm_bindgen_test(async)]
async fn advanced_export_attempts_actual_owned_browser_surfaces() {
    let global: JsValue = js_sys::global().into();
    let speech = get(&global, "speechSynthesis").unwrap();
    let handler = get(&speech, "onvoiceschanged").unwrap();
    let original = wasm_bindgen_futures::JsFuture::from(legacy_advanced().unwrap()).await;
    set(&speech, "onvoiceschanged", &handler);
    let original: Advanced = serde_json::from_str(
        &js_sys::JSON::stringify(&original.unwrap())
            .unwrap()
            .as_string()
            .unwrap(),
    )
    .unwrap();
    let report: Advanced = serde_json::from_str(&crate::advanced_json().await.unwrap()).unwrap();
    assert_eq!(report, original);
    assert!(report.probed);
    assert!(!report.language.is_empty());
    assert!(report.media_device_count >= 0 && report.voice_count >= 0);
    assert!(report.audio_sample_rate.is_finite());
    assert!(report.web_rtc_host_addrs.is_none() && report.web_rtc_public_addrs.is_none());
}

#[wasm_bindgen_test(async)]
async fn advanced_actual_media_stage() {
    let global: JsValue = js_sys::global().into();
    let nav = get(&global, "navigator").unwrap();
    media::collect(&global, &nav, &mut Advanced::default())
        .await
        .unwrap();
}
#[wasm_bindgen_test(async)]
async fn advanced_actual_capability_stage() {
    let global: JsValue = js_sys::global().into();
    let nav = get(&global, "navigator").unwrap();
    capabilities::collect(&global, &nav, &mut Advanced::default())
        .await
        .unwrap();
}
#[wasm_bindgen_test(async)]
async fn advanced_actual_profile_stage() {
    let global: JsValue = js_sys::global().into();
    let nav = get(&global, "navigator").unwrap();
    profile::collect(&global, &nav, &mut Advanced::default())
        .await
        .unwrap();
}

#[wasm_bindgen_test(async)]
async fn advanced_legacy_gpu_info_and_eme_timeout_keep_frozen_fallbacks() {
    let (global, nav) = fixture();
    let info = Closure::<dyn FnMut() -> Promise>::new(|| {
        let result: JsValue = Object::new().into();
        set(&result, "vendor", &"legacy-vendor".into());
        Promise::resolve(&result)
    });
    let adapter: JsValue = Object::new().into();
    set(&adapter, "requestAdapterInfo", info.as_ref());
    let request = Closure::<dyn FnMut() -> Promise>::new(move || Promise::resolve(&adapter));
    let gpu: JsValue = Object::new().into();
    set(&gpu, "requestAdapter", request.as_ref());
    set(&nav, "gpu", &gpu);
    let pending = Closure::<dyn FnMut() -> Promise>::new(|| Promise::new(&mut |_, _| {}));
    set(&nav, "requestMediaKeySystemAccess", pending.as_ref());
    let report = probe_with(&global).await.unwrap();
    assert!(report.web_gpu_present && !report.web_gpu_fallback);
    assert_eq!(report.web_gpu_vendor, "legacy-vendor");
    assert!(!report.widevine_supported);
    set(&nav, "gpu", &JsValue::UNDEFINED);
    set(&nav, "requestMediaKeySystemAccess", &JsValue::UNDEFINED);
}

#[wasm_bindgen_test(async)]
async fn advanced_webgl_uses_only_vendor_without_experimental_fallback() {
    use std::{cell::Cell, rc::Rc};
    let (global, _) = fixture();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let context = Closure::<dyn FnMut(JsValue) -> JsValue>::new(move |name: JsValue| {
        assert_eq!(name.as_string().unwrap(), "webgl");
        observed.set(observed.get() + 1);
        JsValue::NULL
    });
    let canvas: JsValue = Object::new().into();
    set(&canvas, "getContext", context.as_ref());
    let create = Closure::<dyn FnMut() -> JsValue>::new(move || canvas.clone());
    let document: JsValue = Object::new().into();
    set(&document, "createElement", create.as_ref());
    set(&global, "document", &document);
    assert_eq!(probe_with(&global).await.unwrap().web_gl_vendor, "");
    assert_eq!(calls.get(), 1);
    set(&global, "document", &JsValue::UNDEFINED);
}

#[wasm_bindgen_test(async)]
async fn advanced_audio_read_failure_still_closes_and_unclosed_audio_rejects() {
    use std::{cell::Cell, rc::Rc};
    let (global, _) = fixture();
    let closed = Rc::new(Cell::new(0));
    let observed = closed.clone();
    let close = Closure::<dyn FnMut() -> Promise>::new(move || {
        observed.set(observed.get() + 1);
        Promise::resolve(&JsValue::UNDEFINED)
    });
    let context: JsValue = Object::new().into();
    set(&context, "sampleRate", &JsValue::from_f64(f64::NAN));
    set(&context, "close", close.as_ref());
    let actual: JsValue = js_sys::global().into();
    let constructor = call(
        &get(&actual, "Object").unwrap(),
        "bind",
        &[JsValue::NULL, context.clone()],
    )
    .unwrap();
    set(&global, "AudioContext", &constructor);
    assert!(probe_with(&global).await.is_err());
    assert_eq!(closed.get(), 1);
    let failure =
        Closure::<dyn FnMut() -> Promise>::new(|| Promise::reject(&"owned close failure".into()));
    set(&context, "sampleRate", &48000.into());
    set(&context, "close", failure.as_ref());
    assert!(probe_with(&global).await.is_err());
    set(&global, "AudioContext", &JsValue::UNDEFINED);
}
