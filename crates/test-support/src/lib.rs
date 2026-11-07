//! Shared cross-language contract assertions for migration and integration tests.
//! Runtime crates use this only as a dev-dependency.
use serde_json::{Number, Value};
pub mod json;

#[cfg(not(target_arch = "wasm32"))]
pub mod process;

fn numbers_equal(first: &Number, second: &Number) -> bool {
    if let (Some(first), Some(second)) = (first.as_i64(), second.as_i64()) {
        return first == second;
    }
    if let (Some(first), Some(second)) = (first.as_u64(), second.as_u64()) {
        return first == second;
    }
    let integer_and_float = |integer: &Number, float: &Number| {
        let Some(float) = float.as_f64() else {
            return false;
        };
        if float.fract() != 0.0 {
            return false;
        }
        if let Some(integer) = integer.as_i64() {
            (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&float)
                && float as i64 == integer
        } else if let Some(integer) = integer.as_u64() {
            (0.0..18_446_744_073_709_551_616.0).contains(&float) && float as u64 == integer
        } else {
            false
        }
    };
    match (first.is_f64(), second.is_f64()) {
        (true, true) => first.as_f64() == second.as_f64(),
        (false, true) => integer_and_float(first, second),
        (true, false) => integer_and_float(second, first),
        _ => false,
    }
}

/// Assert equal JSON shapes and values, allowing `1` and `1.0` without losing
/// integer precision. Field omissions, array ordering and numeric differences fail.
pub fn assert_json_equivalent(actual: &Value, expected: &Value) {
    fn compare(actual: &Value, expected: &Value, path: &str) {
        match (actual, expected) {
            (Value::Number(actual), Value::Number(expected)) => assert!(
                numbers_equal(actual, expected),
                "{path}: {actual} != {expected}"
            ),
            (Value::Object(actual), Value::Object(expected)) => {
                assert_eq!(
                    actual.keys().collect::<Vec<_>>(),
                    expected.keys().collect::<Vec<_>>(),
                    "{path}"
                );
                for (key, value) in expected {
                    compare(&actual[key], value, &format!("{path}.{key}"));
                }
            }
            (Value::Array(actual), Value::Array(expected)) => {
                assert_eq!(actual.len(), expected.len(), "{path}");
                for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                    compare(actual, expected, &format!("{path}[{index}]"));
                }
            }
            _ => assert_eq!(actual, expected, "{path}"),
        }
    }
    compare(actual, expected, "$");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn integral_float_spellings_are_equivalent() {
        assert_json_equivalent(&json!({"v":[1.0,-2.0]}), &json!({"v":[1,-2]}));
    }
    #[test]
    fn large_integers_are_never_compared_via_rounded_floats() {
        assert!(!numbers_equal(
            &Number::from(u64::MAX),
            &Number::from_f64(18_446_744_073_709_551_616.0).unwrap()
        ));
        assert!(!numbers_equal(
            &Number::from(9_007_199_254_740_993_u64),
            &Number::from_f64(9_007_199_254_740_992.0).unwrap()
        ));
        assert!(numbers_equal(
            &Number::from(i64::MIN),
            &Number::from_f64(i64::MIN as f64).unwrap()
        ));
    }
    #[test]
    #[should_panic]
    fn missing_and_null_fields_are_different() {
        assert_json_equivalent(&json!({}), &json!({"v":null}));
    }
}
