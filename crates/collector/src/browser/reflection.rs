use wasm_bindgen::{JsCast, JsValue};

pub(super) fn get(object: &JsValue, key: &str) -> Result<JsValue, JsValue> {
    js_sys::Reflect::get(object, &JsValue::from_str(key))
}
pub(super) fn truthy(value: &JsValue) -> bool {
    value.is_truthy()
}
pub(super) fn string(value: &JsValue) -> Result<String, JsValue> {
    let text = value.as_string().unwrap_or_default();
    if text.len() > 32_768 {
        Err(JsValue::from_str("string observation budget"))
    } else {
        Ok(text)
    }
}
pub(super) fn int(value: &JsValue) -> Result<i64, JsValue> {
    let n = value
        .as_f64()
        .ok_or_else(|| JsValue::from_str("numeric observation required"))?;
    if !n.is_finite() || !(-9_007_199_254_740_991.0..=9_007_199_254_740_991.0).contains(&n) {
        return Err(JsValue::from_str("invalid integer observation"));
    }
    Ok(n.trunc() as i64)
}
pub(super) fn call(object: &JsValue, key: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    let function = get(object, key)?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| JsValue::from_str("browser method unavailable"))?;
    let values = js_sys::Array::new();
    for value in args {
        values.push(value);
    }
    function.apply(object, &values)
}

pub(super) fn go_string(value: &JsValue) -> Result<String, JsValue> {
    let text = if let Some(text) = value.as_string() {
        text
    } else if value.is_undefined() {
        "<undefined>".into()
    } else if value.is_null() {
        "<null>".into()
    } else if value.is_function() {
        "<function>".into()
    } else if value.is_symbol() {
        "<symbol>".into()
    } else if value.is_object() {
        "<object>".into()
    } else if value.as_bool().is_some() || value.as_f64().is_some() {
        let global: JsValue = js_sys::global().into();
        let converted = call(&global, "String", std::slice::from_ref(value))?;
        let text = converted
            .as_string()
            .ok_or_else(|| JsValue::from_str("invalid primitive string conversion"))?;
        format!(
            "<{}: {text}>",
            if value.as_bool().is_some() {
                "boolean"
            } else {
                "number"
            }
        )
    } else {
        return Err(JsValue::from_str("unsupported browser value type"));
    };
    if text.len() > 32_768 {
        Err(JsValue::from_str("string observation budget"))
    } else {
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn primitive_tags_truthiness_and_numeric_budgets() {
        for (value, expected) in [
            (JsValue::UNDEFINED, "<undefined>"),
            (JsValue::NULL, "<null>"),
            (JsValue::TRUE, "<boolean: true>"),
            (JsValue::from_f64(-0.0), "<number: 0>"),
            (JsValue::from_f64(f64::NAN), "<number: NaN>"),
            (JsValue::from_f64(f64::INFINITY), "<number: Infinity>"),
            (JsValue::from_str("SWİFTSHADER"), "SWİFTSHADER"),
            (js_sys::Object::new().into(), "<object>"),
        ] {
            assert_eq!(go_string(&value).unwrap(), expected);
        }
        assert!(!truthy(&JsValue::from_str("")));
        assert!(truthy(&js_sys::Array::new().into()));
        assert_eq!(int(&JsValue::from_f64(-2.9)).unwrap(), -2);
        for value in [
            JsValue::NULL,
            JsValue::from_str("1"),
            JsValue::from_f64(f64::NAN),
            JsValue::from_f64(f64::INFINITY),
            JsValue::from_f64(9_007_199_254_740_992.0),
        ] {
            assert!(int(&value).is_err());
        }
        let limit = JsValue::from_str(&"a".repeat(32_768));
        assert!(string(&limit).is_ok());
        assert!(go_string(&limit).is_ok());
        let excess = JsValue::from_str(&"a".repeat(32_769));
        assert!(string(&excess).is_err());
        assert!(go_string(&excess).is_err());
    }
}
