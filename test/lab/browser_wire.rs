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

/// Browser-local proxy observations only; not Gate/Runtime header attestation.
pub fn bootstrap_origin(hits: &Value, fixture: &Value) -> Option<&'static str> {
    let session = fixture["routes"]["session"].as_str()?;
    let response = format!("response:{session}:403:ack_false");
    if session != "/_humanymous/browser/session"
        || hits.get(session)?.as_u64() != Some(1)
        || hits.get(response.as_str())?.as_u64() != Some(1)
    {
        return None;
    }
    for route in fixture["routes"].as_object()?.values() {
        let route = route.as_str()?;
        if route != session
            && hits
                .get(route)
                .is_some_and(|count| count.as_u64() != Some(0))
        {
            return None;
        }
    }
    let prefix = format!("response:{session}:");
    if hits.as_object()?.iter().any(|(key, count)| {
        key.starts_with(&prefix) && key != &response && count.as_u64() != Some(0)
    }) {
        return None;
    }
    let contract: Value = serde_json::from_str(include_str!("browser-wire-contract.json")).ok()?;
    let number = |key: &str| hits.get(contract[key].as_str()?)?.as_u64();
    let matched = number("origin_matched")?;
    let other = number("origin_other")?;
    let absent = number("origin_absent")?;
    let null = number("origin_null")?;
    if matched.checked_add(other)? != 1 {
        return None;
    }
    let other = other.checked_sub(absent)?.checked_sub(null)?;
    [
        (absent, "browser_bootstrap_origin_browser_proxy_absent"),
        (null, "browser_bootstrap_origin_browser_proxy_null"),
        (matched, "browser_bootstrap_origin_browser_proxy_matched"),
        (other, "browser_bootstrap_origin_browser_proxy_other"),
    ]
    .into_iter()
    .find_map(|(count, code)| (count == 1).then_some(code))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn origin_hits(matched: u64, other: u64, absent: u64, null: u64) -> Value {
        json!({"wire_origin_matched":matched,"wire_origin_other":other,
            "wire_origin_absent":absent,"wire_origin_null":null,
            "/_humanymous/browser/session":1,
            "response:/_humanymous/browser/session:403:ack_false":1})
    }

    #[test]
    fn one_owned_bootstrap_response_binds_four_finite_browser_proxy_classes() {
        let fixture = json!({"routes":{"session":"/_humanymous/browser/session"}});
        let catalog: Value =
            serde_json::from_str(include_str!("runner/safe-diagnostics.json")).unwrap();
        for (hits, code) in [
            (
                origin_hits(0, 1, 1, 0),
                "browser_bootstrap_origin_browser_proxy_absent",
            ),
            (
                origin_hits(0, 1, 0, 1),
                "browser_bootstrap_origin_browser_proxy_null",
            ),
            (
                origin_hits(1, 0, 0, 0),
                "browser_bootstrap_origin_browser_proxy_matched",
            ),
            (
                origin_hits(0, 1, 0, 0),
                "browser_bootstrap_origin_browser_proxy_other",
            ),
        ] {
            assert_eq!(bootstrap_origin(&hits, &fixture), Some(code));
            assert!(
                catalog["error_codes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|value| value.as_str() == Some(code))
            );
        }
    }

    #[test]
    fn unbound_malformed_ambiguous_and_overflowing_origin_counts_cannot_refine() {
        let fixture = json!({"routes":{"session":"/_humanymous/browser/session"}});
        for hits in [
            origin_hits(0, 0, 0, 0),
            origin_hits(1, 1, 0, 0),
            origin_hits(0, 1, 1, 1),
            origin_hits(1, 0, 1, 0),
            origin_hits(u64::MAX, 1, 0, 0),
            origin_hits(0, 1, u64::MAX, 1),
        ] {
            assert_eq!(bootstrap_origin(&hits, &fixture), None);
        }
        for key in [
            "wire_origin_matched",
            "wire_origin_other",
            "wire_origin_absent",
            "wire_origin_null",
            "/_humanymous/browser/session",
            "response:/_humanymous/browser/session:403:ack_false",
        ] {
            let mut hits = origin_hits(0, 1, 1, 0);
            hits.as_object_mut().unwrap().remove(key);
            assert_eq!(bootstrap_origin(&hits, &fixture), None);
            for bad in [
                Value::Null,
                json!("1"),
                json!(true),
                json!(-1),
                json!(1.5),
                json!(2),
            ] {
                let mut hits = origin_hits(0, 1, 1, 0);
                hits[key] = bad;
                assert_eq!(bootstrap_origin(&hits, &fixture), None);
            }
        }
        for fixture in [json!({}), json!({"routes":{"session":"/foreign"}})] {
            assert_eq!(bootstrap_origin(&origin_hits(0, 1, 1, 0), &fixture), None);
        }
        for key in [
            "response:/_humanymous/browser/session:200:ack_false",
            "response:/_humanymous/browser/session:403:ack_true",
        ] {
            let mut hits = origin_hits(0, 1, 1, 0);
            hits[key] = json!(1);
            assert_eq!(bootstrap_origin(&hits, &fixture), None);
        }
        let fixture =
            json!({"routes":{"session":"/_humanymous/browser/session","effect":"/effect"}});
        let mut hits = origin_hits(0, 1, 1, 0);
        hits["/effect"] = json!(1);
        assert_eq!(bootstrap_origin(&hits, &fixture), None);
    }

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
