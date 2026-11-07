//! Frozen numeric session policy and its wire verdict vocabulary.
use serde::{Deserialize, Serialize};

/// Session-level outcome, separate from an individual signal's verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionVerdict {
    /// Serve the request subject to route policy.
    #[serde(rename = "ALLOW")]
    Allow,
    /// Require the configured verification.
    #[serde(rename = "CHALLENGE")]
    Challenge,
    /// Deny under the current policy.
    #[serde(rename = "DENY")]
    Deny,
}

impl SessionVerdict {
    /// Stable wire label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "ALLOW",
            Self::Challenge => "CHALLENGE",
            Self::Deny => "DENY",
        }
    }
}

/// Frozen L7 numeric policy; explicit overrides are supplied by configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Policy {
    /// Policy version.
    #[serde(rename = "Version")]
    pub version: String,
    /// Score at or above which verification is required.
    #[serde(rename = "ChallengeAt")]
    pub challenge_at: f64,
    /// Score at or above which requests are denied.
    #[serde(rename = "DenyAt")]
    pub deny_at: f64,
    /// Maximum contribution of any individual layer before combination.
    #[serde(rename = "LayerCap")]
    pub layer_cap: f64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            version: "1.0.0".into(),
            challenge_at: 30.0,
            deny_at: 70.0,
            layer_cap: 60.0,
        }
    }
}

impl Policy {
    /// Map numeric risk using the frozen inclusive thresholds, before hard rules.
    pub fn verdict_for(&self, score: f64) -> SessionVerdict {
        if score >= self.deny_at {
            SessionVerdict::Deny
        } else if score >= self.challenge_at {
            SessionVerdict::Challenge
        } else {
            SessionVerdict::Allow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thresholds_are_inclusive_and_frozen() {
        let policy = Policy::default();
        for (risk, expected) in [
            (29.999, SessionVerdict::Allow),
            (30.0, SessionVerdict::Challenge),
            (69.999, SessionVerdict::Challenge),
            (70.0, SessionVerdict::Deny),
            (f64::NAN, SessionVerdict::Allow),
        ] {
            assert_eq!(policy.verdict_for(risk), expected);
        }
        assert_eq!(policy.layer_cap, 60.0);
    }
}
