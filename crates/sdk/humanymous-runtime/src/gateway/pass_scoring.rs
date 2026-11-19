//! Pass engine-fusion kernels (legacy cmd/server pass_scoring.go at baseline
//! 981d25c), ported under owner decision B1 with exact parity.
//!
//! Every weight, threshold, cutpoint and signal identity equals the Go
//! oracle; the signals come from the frozen registry and are scored only by
//! the frozen `humanymous-core` engine the caller runs. A Go-captured golden
//! (`tests/fixtures/gateway/pass-flow-go-golden.json`) pins each kernel.
//!
//! - Velocity: a per-session and per-fingerprint 30 s window (soft 4, hard 8)
//!   folds `l7.pass.velocity` (SUSPICIOUS) or `l7.pass.flood` (BOT) into the
//!   session once per new level. It never locks out; it raises risk and the
//!   proof-of-work cost, never the puzzle difficulty.
//! - Behavior: soft timing and mobile tells from the interaction proof.
//!   Accessibility conflict: these raise the risk of fast, regular input
//!   (switch and other assistive devices) and can hold a solved Pass short of
//!   ALLOW, so the served flow keeps them off unless explicitly configured.
//! - Difficulty and KPI labels follow the session risk; a red-team header
//!   always gets the hardest puzzle and reports its strategy name.
use crate::gateway::pass_proof::{PassProof, all_integer_ms, mean, stddev};
use crate::network::history::rate::Level;
use humanymous_core::signals::observation::{Signal, Source, Verdict};
use serde_json::Value;

/// Trust-upgrade signal of a cleared Pass.
pub const SOLVED_SIGNAL: &str = "l7.pass.solved";
/// Elevated Pass velocity (level 1).
pub const VELOCITY_SIGNAL: &str = "l7.pass.velocity";
/// Pass flood (level 2).
pub const FLOOD_SIGNAL: &str = "l7.pass.flood";
/// Velocity window in seconds (Go `abuse.NewLimiter(30*time.Second, 4, 8)`).
pub const VELOCITY_WINDOW_SECS: u64 = 30;
/// Velocity soft cutpoint.
pub const VELOCITY_SOFT: usize = 4;
/// Velocity hard cutpoint.
pub const VELOCITY_HARD: usize = 8;
/// Attestation issuance window in seconds.
pub const ATTEST_WINDOW_SECS: u64 = 30;
/// Attestation issuance budget: soft and hard cutpoint 3, denied at hard.
pub const ATTEST_BUDGET: usize = 3;
/// Base proof-of-work bits of a Pass instance.
pub const POW_BASE_BITS: u32 = 14;
/// Proof-of-work bit cap of a Pass instance.
pub const POW_MAX_BITS: u32 = 20;
/// Hardest puzzle difficulty.
pub const MAX_DIFFICULTY: u32 = 3;
/// Header naming a self-declared red-team strategy.
pub const RED_TEAM_HEADER: &str = "X-HM-Redteam";

/// Legacy `passSolvedSignal`.
pub fn solved_signal() -> Signal {
    Signal::new(
        SOLVED_SIGNAL,
        Value::Bool(true),
        Verdict::Ok,
        1.0,
        Source::Server,
        "humanymous Pass cleared",
    )
}

/// Go limiter level number: 0 normal, 1 soft, 2 hard.
pub fn level_number(level: Level) -> u8 {
    match level {
        Level::Normal => 0,
        Level::Soft => 1,
        Level::Hard => 2,
    }
}

/// Legacy `passFP`: stable JA4 prefix and client subnet, `|`-joined.
pub fn fingerprint(ja4_stable: &str, subnet: &str) -> String {
    format!("{ja4_stable}|{subnet}")
}

/// The velocity signal to fold for a combined `level` (1 or 2) when it
/// exceeds the session's `previous` flagged level, as in `applyPassVelocity`;
/// `None` for level 0 or an already-flagged level.
pub fn velocity_escalation(level: u8, previous: u8) -> Option<Signal> {
    if level == 0 || level <= previous {
        return None;
    }
    let (id, verdict) = if level == 2 {
        (FLOOD_SIGNAL, Verdict::Bot)
    } else {
        (VELOCITY_SIGNAL, Verdict::Suspicious)
    };
    Some(Signal::new(
        id,
        Value::Null,
        verdict,
        1.0,
        Source::Server,
        "Pass velocity (automation cadence)",
    ))
}

/// Legacy `applyPassBehavior` tells, in Go order and deduplicated by id.
/// Accessibility conflict: see the module notes.
pub fn behavior_signals(proof: &PassProof) -> Vec<Signal> {
    let mut signals: Vec<Signal> = Vec::new();
    let mut add = |id: &str, verdict: Verdict, confidence: f64, note: &str| {
        if signals.iter().all(|signal| signal.id != id) {
            signals.push(Signal::new(
                id,
                Value::Null,
                verdict,
                confidence,
                Source::Server,
                note,
            ));
        }
    };
    if proof.key_durs.len() >= 3 {
        if mean(&proof.key_durs) < 15.0 {
            add(
                "l4.key.machine_speed",
                Verdict::Bot,
                0.7,
                "machine-speed inter-key latency (<15ms)",
            );
        }
        if stddev(&proof.key_durs) < 6.0 {
            add(
                "l4.key.flight_std",
                Verdict::Suspicious,
                0.5,
                "near-zero keystroke flight variance",
            );
        }
        if all_integer_ms(&proof.key_durs) {
            add(
                "l4.event.perfect_timing",
                Verdict::Suspicious,
                0.6,
                "zero sub-ms jitter (synthetic timing)",
            );
        }
    }
    if proof.durations.len() >= 5 && all_integer_ms(&proof.durations) {
        add(
            "l4.event.perfect_timing",
            Verdict::Suspicious,
            0.6,
            "zero sub-ms jitter (synthetic pointer timing)",
        );
    }
    if (proof.pointer_type == "touch" || !proof.pressures.is_empty())
        && (proof.motion.len() < 3 || stddev(&proof.motion) < 0.01)
    {
        add(
            "l2.adv.mobile_inconsistent",
            Verdict::Suspicious,
            0.5,
            "touch/mobile claim without device-motion micro-jitter",
        );
    }
    signals
}

/// Go `int(x)` on amd64: truncation toward zero, with NaN and values outside
/// the int64 range yielding the minimum int64.
fn go_int(value: f64) -> i64 {
    const LIMIT: f64 = 9_223_372_036_854_775_808.0;
    if value.is_nan() || !(-LIMIT..LIMIT).contains(&value) {
        i64::MIN
    } else {
        value.trunc() as i64
    }
}

/// Legacy `passDifficulty`: `int((risk-20)/15)` clamped to 0..=3; a
/// red-team request always gets 3.
pub fn difficulty(risk: f64, red_team: bool) -> u32 {
    if red_team {
        return MAX_DIFFICULTY;
    }
    go_int((risk - 20.0) / 15.0).clamp(0, i64::from(MAX_DIFFICULTY)) as u32
}

/// Proof-of-work bits of an instance: 14 + difficulty + 2 per velocity
/// level, capped at 20 (legacy `handlePassNew`).
pub fn pow_difficulty(difficulty: u32, velocity_level: u8) -> u32 {
    (POW_BASE_BITS + difficulty + 2 * u32::from(velocity_level)).min(POW_MAX_BITS)
}

/// Legacy `passLabel`: a red-team strategy name, else `bot` at risk 70 or
/// more, `human` under 30 and `unknown` between (and for NaN).
pub fn label(risk: f64, red_team: Option<&str>) -> String {
    if let Some(strategy) = red_team.filter(|strategy| !strategy.is_empty()) {
        return strategy.to_owned();
    }
    if risk >= 70.0 {
        "bot".into()
    } else if risk < 30.0 {
        "human".into()
    } else {
        "unknown".into()
    }
}

/// Legacy `isBotLabel`: everything but `human` and `unknown`.
pub fn is_bot_label(label: &str) -> bool {
    label != "human" && label != "unknown"
}

#[cfg(test)]
mod tests;
