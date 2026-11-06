use humanymous_signals::{
    observation::{Signal, Source, Verdict},
    report::Environment,
};
use serde_json::json;

/// Derived environment observations and the privacy mitigation flag.
#[derive(Debug, Default)]
pub struct EnvironmentResult {
    /// Ordered observations.
    pub signals: Vec<Signal>,
    /// Whether privacy tooling was observed or claimed under the existing policy.
    pub privacy: bool,
}

/// Evaluate the existing environment probe without changing its confidence policy.
pub fn environment_signals(environment: &Environment, has_client: bool) -> EnvironmentResult {
    let mut result = EnvironmentResult::default();
    let mut add = |id, value, verdict, confidence, notes| {
        result.signals.push(Signal::new(
            id,
            value,
            verdict,
            confidence,
            Source::Js,
            notes,
        ))
    };
    if has_client && !environment.probed {
        add(
            "l2.env.probe_dead",
            json!(false),
            Verdict::Suspicious,
            0.5,
            "client present but environment probe did not run",
        );
        return result;
    }
    if !environment.probed {
        return result;
    }
    if environment.ad_block {
        add(
            "l2.env.adblock",
            json!(true),
            Verdict::Ok,
            1.0,
            "ad-block active (privacy user)",
        );
        result.privacy = true;
    }
    if environment.tracking_protection
        || environment.gpc
        || environment.do_not_track == "1"
        || environment.brave_shields
    {
        add(
            "l2.env.tracking_protection",
            json!(true),
            Verdict::Ok,
            1.0,
            "tracking protection active",
        );
        result.privacy = true;
    }
    let claims = environment.gpc || environment.do_not_track == "1" || environment.brave_shields;
    let blocks = environment.ad_block || environment.tracking_protection;
    if claims && !blocks {
        add(
            "l2.env.privacy_inconsistent",
            json!(true),
            Verdict::Suspicious,
            0.6,
            "privacy flags set but no blocking behavior observed",
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_probe_and_privacy_claim_precedence() {
        let environment = Environment {
            gpc: true,
            ..Default::default()
        };
        let result = environment_signals(&environment, true);
        assert_eq!(result.signals[0].id, "l2.env.probe_dead");
        assert!(!result.privacy);
        let result = environment_signals(
            &Environment {
                probed: true,
                ..environment
            },
            true,
        );
        assert!(result.privacy);
        assert_eq!(
            result.signals.last().unwrap().id,
            "l2.env.privacy_inconsistent"
        );
    }
}
