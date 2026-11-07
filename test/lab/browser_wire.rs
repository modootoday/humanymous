//! Counter completeness never replaces response, witness or effect verification.
use serde_json::Value;

fn complete(hits: &Value, expected: &Value, good: &str, bad: &str) -> bool {
    let Some(counts) = expected["request_counts"].as_object() else {
        return false;
    };
    let total = counts.values().try_fold(0_u64, |total, value| {
        let count = value.as_u64().filter(|count| *count > 0)?;
        total.checked_add(count)
    });
    let Some(total) = total.filter(|total| *total > 0) else {
        return false;
    };
    let Ok(contract) = serde_json::from_str::<Value>(include_str!("browser-wire-contract.json"))
    else {
        return false;
    };
    let (Some(good), Some(bad)) = (contract[good].as_str(), contract[bad].as_str()) else {
        return false;
    };
    hits[good].as_u64() == Some(total)
        && hits.get(bad).is_none_or(|value| value.as_u64() == Some(0))
}

pub fn origins_match(hits: &Value, expected: &Value) -> bool {
    complete(hits, expected, "origin_matched", "origin_other")
}

pub fn referers_absent(hits: &Value, expected: &Value) -> bool {
    complete(hits, expected, "referer_absent", "referer_present")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn denominator_follows_the_single_fixture_including_required_refusal() {
        let mut fixture: Value =
            serde_json::from_str(include_str!("../../crates/collector/pilot/fixture.json"))
                .unwrap();
        for count in [1, 2, 32] {
            fixture["expected"]["effect_requests"] = json!(count);
            let expected = crate::browser_expectations::from_fixture(&fixture).unwrap();
            let hits = json!({"wire_origin_matched":count+3,"wire_referer_absent":count+3});
            assert!(origins_match(&hits, &expected));
            assert!(referers_absent(&hits, &expected));
        }
    }

    #[test]
    fn missing_malformed_opposing_and_incomplete_counters_fail_closed() {
        let expected =
            json!({"request_counts":{"session":1,"effect":2,"collect":1,"required_collector":1}});
        for good in [
            Value::Null,
            json!(0),
            json!(4),
            json!(6),
            json!("5"),
            json!(true),
        ] {
            let hits = json!({"wire_origin_matched":good,"wire_referer_absent":good});
            assert!(!origins_match(&hits, &expected));
            assert!(!referers_absent(&hits, &expected));
        }
        for bad in [json!(1), json!("0"), json!(false), Value::Null] {
            let hits = json!({"wire_origin_matched":5,"wire_referer_absent":5,"wire_origin_other":bad,"wire_referer_present":bad});
            assert!(!origins_match(&hits, &expected));
            assert!(!referers_absent(&hits, &expected));
        }
        let hits = json!({"wire_origin_matched":5,"wire_referer_absent":5});
        for invalid in [
            json!({}),
            json!({"request_counts":{}}),
            json!({"request_counts":{"a":u64::MAX,"b":1}}),
        ] {
            assert!(!origins_match(&hits, &invalid));
            assert!(!referers_absent(&hits, &invalid));
        }
    }
}
