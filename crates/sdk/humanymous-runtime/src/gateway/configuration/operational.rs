use crate::gateway::Rejection;
use serde::{Deserialize, Serialize};

/// Bounded operational resource settings; no detection or identity authority.
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalSettings {
    /// Maximum volatile live sessions; no live eviction.
    pub session_capacity: usize,
    /// Fixed epoch lifetime in milliseconds.
    pub session_ttl_ms: u64,
    /// Whole request/connection deadline in milliseconds.
    pub timeout_ms: u64,
    /// Global pre-auth connection admissions per second.
    pub rate_per_second: u32,
    /// Authenticated volatile-session request admissions per second.
    pub session_rate_per_second: u32,
}
impl OperationalSettings {
    /// Validate only resource budgets. This does not durably apply a change.
    pub fn validate(&self) -> Result<(), Rejection> {
        if !(1..=4096).contains(&self.session_capacity)
            || !(1000..=3_600_000).contains(&self.session_ttl_ms)
            || !(1..=30_000).contains(&self.timeout_ms)
            || !(1..=10_000).contains(&self.rate_per_second)
            || !(1..=10_000).contains(&self.session_rate_per_second)
        {
            return Err(Rejection::new(400, "operational_settings"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn operational_schema_excludes_frozen_detection_policy_and_secrets() {
        let mut document = serde_json::json!({"session_capacity":16,"session_ttl_ms":60000,"timeout_ms":1000,"rate_per_second":1000,"session_rate_per_second":100});
        assert!(
            serde_json::from_value::<OperationalSettings>(document.clone())
                .unwrap()
                .validate()
                .is_ok()
        );
        for field in [
            "weights",
            "thresholds",
            "hard_rules",
            "verdict_bands",
            "passphrase",
        ] {
            document[field] = serde_json::json!({});
            assert!(serde_json::from_value::<OperationalSettings>(document.clone()).is_err());
            document.as_object_mut().unwrap().remove(field);
        }
    }
}
