//! Pure expectations from the sole collector fixture; never observed outcomes.
use serde_json::{Value, json};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub fn from_fixture(fixture: &Value) -> Result<Value> {
    let expected = fixture["expected"]
        .as_object()
        .ok_or("browser fixture expected object required")?;
    const FIELDS: [&str; 7] = [
        "before_report_status",
        "effect_requests",
        "before_report_verdict",
        "collect_status",
        "required_collector_status",
        "required_collector_ack",
        "origin_effects",
    ];
    const ROUTES: [&str; 4] = ["session", "collect", "effect", "required_collector"];
    let routes = fixture["routes"]
        .as_object()
        .ok_or("browser fixture routes required")?;
    if fixture["schema"] != "humanymous.browser-runtime-pilot/1"
        || expected.len() != FIELDS.len()
        || expected.keys().any(|k| !FIELDS.contains(&k.as_str()))
        || routes.len() != ROUTES.len()
        || routes.keys().any(|k| !ROUTES.contains(&k.as_str()))
    {
        return Err("browser fixture missing or unknown expectation/route fields".into());
    }
    let count = fixture["expected"]["effect_requests"]
        .as_u64()
        .filter(|n| (1..=32).contains(n))
        .ok_or("browser effect request count missing or invalid")?;
    let status = |key: &str| -> Result<u64> {
        fixture["expected"][key]
            .as_u64()
            .filter(|n| (100..=599).contains(n))
            .ok_or_else(|| "browser expected HTTP status invalid".into())
    };
    if fixture["expected"]["before_report_verdict"] != "CHALLENGE"
        || fixture["expected"]["required_collector_ack"] != false
        || fixture["expected"]["origin_effects"] != 0
    {
        return Err("browser fail-closed fixture expectations refused".into());
    }
    let mut records:Vec<_>=(1..=count).map(|seq|json!({"seq":seq,"event_type":"runtime.decision","verdict":fixture["expected"]["before_report_verdict"]})).collect();
    records.push(json!({"seq":count+1,"event_type":"runtime.collect","verdict":"COLLECTED"}));
    Ok(
        json!({"effect_requests":count,"request_counts":{"session":1,"effect":count,"collect":1,"required_collector":1},"responses":{"effect":{"status":status("before_report_status")?,"ack":true},"collect":{"status":status("collect_status")?,"ack":true},"required_collector":{"status":status("required_collector_status")?,"ack":fixture["expected"]["required_collector_ack"]}},"audit_sequence":count+1,"audit_records":records,"origin_effects":fixture["expected"]["origin_effects"]}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        serde_json::from_str(include_str!("../../crates/collector/pilot/fixture.json")).unwrap()
    }
    #[test]
    fn one_authority_controls_requests_responses_and_authenticated_audit_denominator() {
        for count in [1, 2] {
            let mut fixture = fixture();
            fixture["expected"]["effect_requests"] = json!(count);
            let value = from_fixture(&fixture).unwrap();
            assert_eq!(value["request_counts"]["effect"], count);
            assert_eq!(value["request_counts"]["collect"], 1);
            assert_eq!(value["audit_sequence"], count + 1);
            assert_eq!(
                value["audit_records"].as_array().unwrap().len(),
                count as usize + 1
            );
            assert_eq!(
                value["responses"]["effect"]["status"],
                fixture["expected"]["before_report_status"]
            );
        }
    }
    #[test]
    fn missing_zero_unknown_or_unbounded_expectations_never_fall_back_to_one() {
        for invalid in [
            Value::Null,
            json!(0),
            json!("unknown"),
            json!(33),
            json!(true),
        ] {
            let mut fixture = fixture();
            fixture["expected"]["effect_requests"] = invalid;
            assert!(from_fixture(&fixture).is_err());
        }
        let mut extra = fixture();
        extra["expected"]["unknown"] = json!(1);
        assert!(from_fixture(&extra).is_err());
        let mut extra = fixture();
        extra["routes"]["unknown"] = json!("/unknown");
        assert!(from_fixture(&extra).is_err());
        let mut missing = fixture();
        missing["expected"]
            .as_object_mut()
            .unwrap()
            .remove("effect_requests");
        assert!(from_fixture(&missing).is_err());
    }
}
