use serde::Serialize;

const DAY_NS: i128 = 86_400_000_000_000;

/// Classification only; physical retirement requires checkpoint-preserving
/// storage transitions and must not be inferred from these labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Tier {
    /// Within the queryable hot window.
    Hot,
    /// Within the compressed warm window.
    Warm,
    /// Within the cold/WORM retention window.
    Cold,
    /// Beyond all configured windows; not an instruction to unlink files.
    Expired,
}

/// Cumulative age boundaries. The default is 90 days, 365 days, and 7*365
/// days; calendar years/leap years are not silently substituted.
pub struct RetentionPolicy {
    hot: i128,
    warm: i128,
    cold: i128,
}
impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            hot: 90 * DAY_NS,
            warm: 365 * DAY_NS,
            cold: 7 * 365 * DAY_NS,
        }
    }
}
impl RetentionPolicy {
    /// Configure ordered nonnegative nanosecond windows. Invalid policy is an
    /// error instead of misleading console labels.
    pub fn new(hot: i128, warm: i128, cold: i128) -> Result<Self, &'static str> {
        if hot < 0 || warm < hot || cold < warm {
            return Err("retention windows must be nonnegative and ordered");
        }
        Ok(Self { hot, warm, cold })
    }
    /// Inclusive historical boundaries. Negative ages remain HOT; the caller
    /// must separately report clock skew rather than treating future records as
    /// eligible for deletion.
    pub fn tier(&self, age_ns: i128) -> Tier {
        if age_ns <= self.hot {
            Tier::Hot
        } else if age_ns <= self.warm {
            Tier::Warm
        } else if age_ns <= self.cold {
            Tier::Cold
        } else {
            Tier::Expired
        }
    }
    /// Whole-day display windows, truncated toward zero like the historical
    /// console. Order is hot/warm/cold, not independent stage durations.
    pub fn days(&self) -> [i128; 3] {
        [self.hot / DAY_NS, self.warm / DAY_NS, self.cold / DAY_NS]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inclusive_boundaries_and_display_match_historical_policy() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.days(), [90, 365, 2555]);
        for (age, expected) in [
            (i128::MIN, Tier::Hot),
            (0, Tier::Hot),
            (90 * DAY_NS, Tier::Hot),
            (90 * DAY_NS + 1, Tier::Warm),
            (365 * DAY_NS, Tier::Warm),
            (365 * DAY_NS + 1, Tier::Cold),
            (2555 * DAY_NS, Tier::Cold),
            (2555 * DAY_NS + 1, Tier::Expired),
            (i128::MAX, Tier::Expired),
        ] {
            assert_eq!(policy.tier(age), expected);
        }
        assert!(RetentionPolicy::new(-1, 1, 2).is_err());
        assert!(RetentionPolicy::new(2, 1, 3).is_err());
        assert!(RetentionPolicy::new(1, 3, 2).is_err());
        let zero = RetentionPolicy::new(0, 0, 0).unwrap();
        assert_eq!(zero.tier(0), Tier::Hot);
        assert_eq!(zero.tier(1), Tier::Expired);
    }
}
