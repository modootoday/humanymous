//! Existing JS reducers are unchanged read-only migration oracles, never product glue.
use super::*;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen(module = "/../../web/js/features/mouse.js")]
extern "C" {
    #[wasm_bindgen(catch,js_name=mouseFeatures)]
    fn mouse_oracle(samples: &JsValue) -> Result<JsValue, JsValue>;
}
#[wasm_bindgen(module = "/../../web/js/features/keystroke.js")]
extern "C" {
    #[wasm_bindgen(catch,js_name=keyFeatures)]
    fn key_oracle(downs: &JsValue, ups: &JsValue, pastes: i32) -> Result<JsValue, JsValue>;
}
fn object(properties: &[(&str, f64)]) -> JsValue {
    let object = js_sys::Object::new();
    for (key, value) in properties {
        assert!(
            js_sys::Reflect::set(&object, &JsValue::from_str(key), &JsValue::from_f64(*value))
                .unwrap()
        );
    }
    object.into()
}
fn compare(actual: serde_json::Value, expected: JsValue) {
    let expected: serde_json::Value = serde_json::from_str(
        &js_sys::JSON::stringify(&expected)
            .unwrap()
            .as_string()
            .unwrap(),
    )
    .unwrap();
    let actual = actual.as_object().unwrap();
    let expected = expected.as_object().unwrap();
    assert_eq!(actual.len(), expected.len());
    for (key, value) in actual {
        let a = value.as_f64().unwrap();
        let b = expected[key].as_f64().unwrap();
        assert!(
            (a - b).abs() <= 1e-11 * b.abs().max(1.0),
            "{key}: {a} != {b}"
        );
    }
}
#[wasm_bindgen_test]
fn pointer_reducer_matches_unchanged_oracle_across_caps_and_geometry() {
    let mut cases: Vec<Vec<PointerSample>> = Vec::new();
    for n in [0, 1, 2, 3, 4, 30, 600] {
        let samples: Vec<_> = (0..n)
            .map(|i| PointerSample {
                x: ((i * 17) % 91) as f64,
                y: ((i * i) % 37) as f64,
                time_ms: (i * 127) as f64,
            })
            .collect();
        cases.push(samples);
    }
    cases.push(vec![
        PointerSample {
            x: 0.25,
            y: 0.5,
            time_ms: 0.0,
        },
        PointerSample {
            x: 1.5,
            y: 0.75,
            time_ms: 0.0,
        },
        PointerSample {
            x: 3.25,
            y: 1.125,
            time_ms: 0.5,
        },
        PointerSample {
            x: 4.5,
            y: 2.0,
            time_ms: 121.0,
        },
    ]);
    let height = (1.0_f64 / 0.995_f64.powi(2) - 1.0).sqrt();
    for h in [
        f64::from_bits(height.to_bits() - 1),
        height,
        f64::from_bits(height.to_bits() + 1),
    ] {
        cases.push(vec![
            PointerSample {
                x: 0.0,
                y: 0.0,
                time_ms: 0.0,
            },
            PointerSample {
                x: 1.0,
                y: h,
                time_ms: 1.0,
            },
            PointerSample {
                x: 2.0,
                y: 0.0,
                time_ms: 2.0,
            },
        ]);
    }
    for samples in cases {
        let array = js_sys::Array::new();
        for p in &samples {
            array.push(&object(&[("x", p.x), ("y", p.y), ("t", p.time_ms)]));
        }
        compare(
            serde_json::to_value(pointer::reduce(&samples)).unwrap(),
            mouse_oracle(&array).unwrap(),
        );
    }
}
#[wasm_bindgen_test]
fn key_reducer_matches_oracle_including_repeat_and_unknown_releases() {
    for n in [0, 1, 2, 20, 600] {
        let downs: Vec<_> = (0..n)
            .map(|i| KeySample {
                token: (i % 5) as u32,
                time_ms: (i * 100) as f64,
            })
            .collect();
        let ups: Vec<_> = (0..n)
            .map(|i| KeySample {
                token: (i % 7) as u32,
                time_ms: (i * 100 + 1) as f64,
            })
            .collect();
        let array = |values: &[KeySample]| {
            let a = js_sys::Array::new();
            for p in values {
                a.push(&object(&[("code", p.token as f64), ("t", p.time_ms)]));
            }
            a
        };
        compare(
            serde_json::to_value(keyboard::reduce(&downs, &ups, 3)).unwrap(),
            key_oracle(&array(&downs), &array(&ups), 3).unwrap(),
        );
    }
}
