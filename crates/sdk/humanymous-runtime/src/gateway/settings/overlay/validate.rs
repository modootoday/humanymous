//! Overlay bounds (SoT-39 §5.0), equal to the oracle `Validate`.
//!
//! Messages match the oracle byte for byte. When several entries are invalid
//! the oracle reports whichever its randomized map iteration meets first;
//! this port checks maps in sorted key order, one of the oracle's outcomes.
use super::{
    INTEGRITY_CRITICAL_GATES, KNOWN_GATES, Mode, NET_POLICY_CLASSES, Overlay, ScoringPatch,
    scoring_defaults,
};
use humanymous_core::signals::wire::{format_fixed_go, format_float_go, quote_go};

/// A refused overlay; the message is the oracle's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayValidationError(String);

impl std::fmt::Display for OverlayValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for OverlayValidationError {}

fn refuse<T>(message: String) -> Result<T, OverlayValidationError> {
    Err(OverlayValidationError(message))
}

/// Check bounds; `None` is valid. Does not compute the mutation class.
pub fn validate(overlay: Option<&Overlay>) -> Result<(), OverlayValidationError> {
    let Some(overlay) = overlay else {
        return Ok(());
    };
    if !overlay.schema_version.is_empty() && overlay.schema_version != "1.0.0" {
        return refuse(format!(
            "unsupported schemaVersion {}",
            quote_go(&overlay.schema_version)
        ));
    }
    for (id, mode) in &overlay.gates {
        gate_mode(id, mode)?;
    }
    for (id, mode) in &overlay.hard_rules {
        hard_rule_mode(id, mode)?;
    }
    for (id, mode) in &overlay.net_policy {
        net_mode(id, mode)?;
    }
    if let Some(scoring) = &overlay.scoring {
        scoring_bounds(scoring)?;
    }
    for (id, multiplier) in &overlay.weight_multipliers {
        if *multiplier < 0.0 || *multiplier > 2.0 {
            return refuse(format!(
                "weight multiplier {id}={} out of [0,2]",
                format_float_go(*multiplier)
            ));
        }
        if id.is_empty() || id.contains([' ', '\t', '\n']) {
            return refuse(format!("invalid signal id {}", quote_go(id)));
        }
    }
    if let Some(rate) = &overlay.rate_limit {
        if rate.window_sec.is_some_and(|value| value < 1) {
            return refuse("rateLimit.windowSec must be >= 1".into());
        }
        if rate.soft.is_some_and(|value| value < 1) {
            return refuse("rateLimit.soft must be >= 1".into());
        }
        if rate.hard.is_some_and(|value| value < 1) {
            return refuse("rateLimit.hard must be >= 1".into());
        }
        if let (Some(hard), Some(soft)) = (rate.hard, rate.soft)
            && hard < soft
        {
            return refuse("rateLimit.hard must be >= soft".into());
        }
    }
    if overlay
        .routes
        .get("/")
        .is_some_and(|preset| preset == "attested")
    {
        return refuse("attested refused on catch-all prefix /".into());
    }
    if overlay
        .routes
        .get("")
        .is_some_and(|preset| preset == "attested")
    {
        return refuse("attested refused on empty prefix".into());
    }
    Ok(())
}

fn gate_mode(id: &str, mode: &Mode) -> Result<(), OverlayValidationError> {
    if !KNOWN_GATES.contains(&id) {
        return refuse(format!("unknown gate module {}", quote_go(id)));
    }
    match mode {
        Mode::Enforce | Mode::Monitor | Mode::Shadow => Ok(()),
        Mode::Off if INTEGRITY_CRITICAL_GATES.contains(&id) => {
            refuse(format!("{id} cannot be off (integrity-critical)"))
        }
        Mode::Off => Ok(()),
        Mode::Other(_) => refuse(format!(
            "invalid mode {} for gate {id}",
            quote_go(mode.as_str())
        )),
    }
}

fn hard_rule_mode(id: &str, mode: &Mode) -> Result<(), OverlayValidationError> {
    if !id.starts_with("HR-") {
        return refuse(format!("invalid hard rule id {}", quote_go(id)));
    }
    match mode {
        Mode::Enforce | Mode::Monitor => Ok(()),
        Mode::Off => refuse(format!("engine hard rule {id} cannot be off (use monitor)")),
        _ => refuse(format!(
            "invalid mode {} for hard rule {id}",
            quote_go(mode.as_str())
        )),
    }
}

fn net_mode(id: &str, mode: &Mode) -> Result<(), OverlayValidationError> {
    if !NET_POLICY_CLASSES.contains(&id) {
        return refuse(format!("unknown netPolicy class {}", quote_go(id)));
    }
    if id == "net.tcp" && *mode == Mode::Enforce {
        return refuse("net.tcp cannot be enforce (Audit+Ban only)".into());
    }
    match mode {
        Mode::Enforce | Mode::Monitor | Mode::Shadow => Ok(()),
        Mode::Off => refuse(format!("netPolicy {id} cannot be off (use monitor)")),
        Mode::Other(_) => refuse(format!(
            "invalid mode {} for netPolicy {id}",
            quote_go(mode.as_str())
        )),
    }
}

fn scoring_bounds(patch: &ScoringPatch) -> Result<(), OverlayValidationError> {
    let (mut challenge, mut deny, _) = scoring_defaults();
    let out_of = |name: &str, value: f64, low: u8, high: u8| {
        refuse(format!(
            "{name} {} out of [{low},{high}]",
            format_fixed_go(value, 1)
        ))
    };
    if let Some(value) = patch.challenge_at {
        challenge = value;
        if value < 10.0 || value > 60.0 {
            return out_of("challengeAt", value, 10, 60);
        }
    }
    if let Some(value) = patch.deny_at {
        deny = value;
        if value < 40.0 || value > 95.0 {
            return out_of("denyAt", value, 40, 95);
        }
    }
    if let Some(value) = patch.layer_cap
        && (value < 30.0 || value > 80.0)
    {
        return out_of("layerCap", value, 30, 80);
    }
    if challenge >= deny {
        return refuse(format!(
            "challengeAt ({}) must be < denyAt ({})",
            format_fixed_go(challenge, 1),
            format_fixed_go(deny, 1)
        ));
    }
    Ok(())
}
