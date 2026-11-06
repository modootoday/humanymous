// Translated from the frozen Go wire contract by scripts/migration/contracts.

/// BehaviorSummary is the reduced feature vector from the behavioral collector.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BehaviorSummary {
    /// Wire field `mouse`.
    #[serde(
        rename = "mouse",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub mouse: MouseFeatures,
    /// Wire field `key`.
    #[serde(rename = "key", deserialize_with = "crate::wire::decode::null_default")]
    pub key: KeyFeatures,
    /// Wire field `events`.
    #[serde(
        rename = "events",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub events: EventFeatures,
    /// observation window seconds
    #[serde(
        rename = "durationS",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub duration_s: f64,
}

/// MouseFeatures summarises pointer movement (SoT-03 §1).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MouseFeatures {
    /// Wire field `samples`.
    #[serde(
        rename = "samples",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub samples: i64,
    /// 0=straight lines
    #[serde(
        rename = "meanCurvature",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub mean_curvature: f64,
    /// human: nonzero variance
    #[serde(
        rename = "velocityStdDev",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub velocity_std_dev: f64,
    /// shannon entropy of accel bins
    #[serde(
        rename = "accelEntropy",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub accel_entropy: f64,
    /// frac of near-perfect lines
    #[serde(
        rename = "straightLineFrac",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub straight_line_frac: f64,
    /// Wire field `pauseCount`.
    #[serde(
        rename = "pauseCount",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub pause_count: i64,
    /// largest instantaneous jump
    #[serde(
        rename = "maxTeleportPx",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub max_teleport_px: f64,
    /// higher-order smoothness
    #[serde(
        rename = "meanJerk",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub mean_jerk: f64,
    /// CoalescedRatio is the mean getCoalescedEvents() count per pointermove.
    /// Real mice/trackpads emit many coalesced sub-events at high frequency;
    /// synthetic (CDP/OS-injected) input emits ~1 (SoT-14). A movement-rich
    /// session with CoalescedRatio≈1 is a strong synthetic-input tell.
    #[serde(
        rename = "coalescedRatio",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub coalesced_ratio: f64,
}

/// KeyFeatures summarises keystroke dynamics (SoT-03 §2).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct KeyFeatures {
    /// Wire field `keystrokes`.
    #[serde(
        rename = "keystrokes",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub keystrokes: i64,
    /// key press->release
    #[serde(
        rename = "meanDwellMs",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub mean_dwell_ms: f64,
    /// human: nonzero variance
    #[serde(
        rename = "dwellStdDevMs",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub dwell_std_dev_ms: f64,
    /// release->next press
    #[serde(
        rename = "meanFlightMs",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub mean_flight_ms: f64,
    /// Wire field `flightStdDevMs`.
    #[serde(
        rename = "flightStdDevMs",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub flight_std_dev: f64,
    /// Wire field `pasteEvents`.
    #[serde(
        rename = "pasteEvents",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub paste_events: i64,
    /// synthetic keys often 0
    #[serde(
        rename = "zeroDwellFrac",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub zero_dwell_frac: f64,
}

/// EventFeatures captures trust/dispatch artifacts + AI-agent cadence (SoT-03 §5,
/// SoT-16). Per FP-Agent (arXiv 2605.01247), behavioral cadence separates AI
/// browsing agents from humans almost perfectly.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct EventFeatures {
    /// Wire field `totalEvents`.
    #[serde(
        rename = "totalEvents",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub total_events: i64,
    /// isTrusted===false ratio
    #[serde(
        rename = "untrustedFrac",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub untrusted_frac: f64,
    /// clicks with no preceding move
    #[serde(
        rename = "noMoveClicks",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub no_move_clicks: i64,
    /// CDP dispatch artifacts seen
    #[serde(
        rename = "syntheticFlags",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub synthetic_flags: i64,
    /// pointerdown count
    #[serde(
        rename = "clickCount",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub click_count: i64,
    /// LLM inference-loop cadence (SoT-16): long "thinking" gaps punctuating
    /// bursts of near-instant, low-variance actions.
    #[serde(
        rename = "longGapCount",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub long_gap_count: i64,
    /// variance of intra-burst gaps
    #[serde(
        rename = "burstVarMs",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub burst_var_ms: f64,
    /// fastest action after a long gap
    #[serde(
        rename = "minReactionMs",
        deserialize_with = "crate::wire::decode::null_default"
    )]
    pub min_reaction_ms: f64,
}
