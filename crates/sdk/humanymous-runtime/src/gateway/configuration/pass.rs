//! Opt-in gateway Pass for the attestation-floor step-up proof (owner
//! decision #4, option 1; see `application::pass`). Legacy reference:
//! internal/gate control.go `handleStepUp` (`hmn_su`, 30 minutes) at baseline
//! 981d25c.
//!
//! Without a `pass` object the Pass API answers as any other reserved route
//! (403) and no route may use the `attested` preset. Present, the gateway
//! serves the Pass API to RIT-authenticated sessions and a solve records a
//! step-up proof on the server for `step_up_ttl_ms`. No key is configured:
//! the Pass master key is random per process, as Go's Core key was.
use serde::Deserialize;

/// Go's step-up proof lifetime (`now.Add(30*time.Minute)`), also the maximum.
pub const DEFAULT_STEP_UP_TTL_MS: u64 = 30 * 60 * 1000;

/// `pass` section of the private startup document.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pass {
    /// Step-up proof lifetime; default and maximum Go's 30 minutes.
    #[serde(default = "ttl")]
    pub step_up_ttl_ms: u64,
}

fn ttl() -> u64 {
    DEFAULT_STEP_UP_TTL_MS
}

impl Pass {
    /// Whether the lifetime is usable.
    pub fn valid(&self) -> bool {
        (1..=DEFAULT_STEP_UP_TTL_MS).contains(&self.step_up_ttl_ms)
    }

    /// Nonsecret readiness view. Motor evidence is always off on a served
    /// path (accessibility hard rule, owner decision #9).
    pub fn readiness(&self) -> serde_json::Value {
        serde_json::json!({"step_up_ttl_ms":self.step_up_ttl_ms,"motor_evidence":"off"})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_is_go_and_the_lifetime_is_bounded() {
        let pass: Pass = serde_json::from_value(json!({})).unwrap();
        assert_eq!(pass.step_up_ttl_ms, 1_800_000);
        assert!(pass.valid());
        assert_eq!(
            pass.readiness(),
            json!({"step_up_ttl_ms":1_800_000,"motor_evidence":"off"})
        );
        for bad in [0, DEFAULT_STEP_UP_TTL_MS + 1] {
            let pass: Pass = serde_json::from_value(json!({"step_up_ttl_ms":bad})).unwrap();
            assert!(!pass.valid(), "{bad}");
        }
        assert!(serde_json::from_value::<Pass>(json!({"master_key":"x"})).is_err());
    }
}
