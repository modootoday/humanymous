//! Serving decision matrix for a resource tier under a session verdict (SoT-10 §2).
use super::Tier;
use std::fmt;

/// Serving outcome for one resource request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Decision {
    /// Serve the full resource.
    Serve,
    /// Serve a preview, poster or lower-quality substitute.
    Downgrade,
    /// Serve a placeholder or block (traffic saved).
    DenyServe,
    /// Require verification before serving the heavy resource.
    Challenge,
}

impl Decision {
    /// Stable label used in the legacy `X-HM-Gate` header.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Serve => "serve",
            Self::Downgrade => "downgrade",
            Self::DenyServe => "deny",
            Self::Challenge => "challenge",
        }
    }
}

impl fmt::Display for Decision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Decide how to serve a resource.
///
/// `verdict` is the session verdict wire label (`ALLOW`, `CHALLENGE`, `DENY`,
/// matched exactly and case-sensitively); any other value is treated as
/// unknown trust, which challenges heavy and embed tiers and serves the rest.
/// Essential resources are always served so signal collection keeps working.
/// RIT validity and behavioral liveness only matter for heavy resources under
/// `ALLOW`.
pub fn gate(tier: Tier, verdict: &str, rit_ok: bool, liveness: bool) -> Decision {
    if tier == Tier::Essential {
        return Decision::Serve;
    }
    match verdict {
        "ALLOW" => match tier {
            Tier::Heavy if !(rit_ok && liveness) => Decision::Challenge,
            // Embed inspection is reported separately as signals.
            _ => Decision::Serve,
        },
        "CHALLENGE" => match tier {
            Tier::Heavy | Tier::Media => Decision::Downgrade,
            Tier::Embed => Decision::Challenge,
            _ => Decision::Serve,
        },
        "DENY" => match tier {
            // Cheap; not worth a placeholder.
            Tier::Light => Decision::Serve,
            _ => Decision::DenyServe,
        },
        _ => match tier {
            Tier::Heavy | Tier::Embed => Decision::Challenge,
            _ => Decision::Serve,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::policy::SessionVerdict;

    const TIERS: [Tier; 5] = [
        Tier::Essential,
        Tier::Light,
        Tier::Media,
        Tier::Heavy,
        Tier::Embed,
    ];

    #[test]
    fn verdict_labels_are_the_session_wire_vocabulary() {
        for (verdict, label) in [
            (SessionVerdict::Allow, "ALLOW"),
            (SessionVerdict::Challenge, "CHALLENGE"),
            (SessionVerdict::Deny, "DENY"),
        ] {
            assert_eq!(verdict.as_str(), label);
        }
    }

    #[test]
    fn full_matrix_matches_legacy_rules() {
        use Decision::{Challenge, DenyServe, Downgrade, Serve};
        // Rows: tier order of TIERS; columns: (rit, liveness) in
        // (false,false), (false,true), (true,false), (true,true).
        let expected: [(&str, [[Decision; 4]; 5]); 5] = [
            (
                "ALLOW",
                [
                    [Serve; 4],
                    [Serve; 4],
                    [Serve; 4],
                    [Challenge, Challenge, Challenge, Serve],
                    [Serve; 4],
                ],
            ),
            (
                "CHALLENGE",
                [
                    [Serve; 4],
                    [Serve; 4],
                    [Downgrade; 4],
                    [Downgrade; 4],
                    [Challenge; 4],
                ],
            ),
            (
                "DENY",
                [
                    [Serve; 4],
                    [Serve; 4],
                    [DenyServe; 4],
                    [DenyServe; 4],
                    [DenyServe; 4],
                ],
            ),
            (
                "allow",
                [
                    [Serve; 4],
                    [Serve; 4],
                    [Serve; 4],
                    [Challenge; 4],
                    [Challenge; 4],
                ],
            ),
            (
                "",
                [
                    [Serve; 4],
                    [Serve; 4],
                    [Serve; 4],
                    [Challenge; 4],
                    [Challenge; 4],
                ],
            ),
        ];
        for (verdict, rows) in expected {
            for (tier, row) in TIERS.into_iter().zip(rows) {
                for (index, want) in row.into_iter().enumerate() {
                    let (rit, live) = (index & 2 != 0, index & 1 != 0);
                    assert_eq!(
                        gate(tier, verdict, rit, live),
                        want,
                        "{tier} {verdict:?} rit={rit} live={live}"
                    );
                }
            }
        }
    }

    #[test]
    fn labels_are_stable() {
        assert_eq!(Decision::Serve.to_string(), "serve");
        assert_eq!(Decision::Downgrade.as_str(), "downgrade");
        assert_eq!(Decision::DenyServe.as_str(), "deny");
        assert_eq!(Decision::Challenge.as_str(), "challenge");
    }
}
