//! Engine fusion for Pass attempts (legacy cmd/server/pass_scoring.go at
//! baseline 981d25c; owner decision 2026-11-17 approves a Go-identical port).
//!
//! Every value here (signal IDs, verdicts, confidences, notes, the 15/6 ms
//! timing cutpoints, the 0.01 motion floor, the risk-to-difficulty map and the
//! KPI bands) is copied from the Go oracle. Nothing registers a signal ID: each
//! emitted ID already exists in the shared registry, which is pinned by tests.
//!
//! These functions only fold risk into the shared engine. None of them blocks,
//! delays or locks out a Pass attempt: an accessible lane never gates on motor
//! richness or speed. The legacy `realEventOK` pre-filter, which did gate on
//! pointer and keyboard timing, is intentionally not ported (owner decision
//! 2026-11-17).
use crate::gateway::pass_proof::PassProof;
use crate::network::history::rate::Level;
use humanymous_core::signals::observation::{Signal, Source, Verdict};
use serde_json::Value;

/// Grants the Pass trust upgrade (OK verdict, zero weight).
pub const SOLVED_SIGNAL: &str = "l7.pass.solved";
/// Elevated Pass cadence.
pub const VELOCITY_SIGNAL: &str = "l7.pass.velocity";
/// Pass flood cadence.
pub const FLOOD_SIGNAL: &str = "l7.pass.flood";
/// Behavioral tells this module may emit, in Go emission order.
pub const BEHAVIOR_SIGNALS: [&str; 4] = [
    "l4.key.machine_speed",
    "l4.key.flight_std",
    "l4.event.perfect_timing",
    "l2.adv.mobile_inconsistent",
];
/// Self-declared red-team strategy header (wargame ground truth).
pub const REDTEAM_HEADER: &str = "X-HM-Redteam";
/// Base proof-of-work leading-zero bits for a Pass instance.
pub const POW_BASE: u32 = 14;
/// Proof-of-work ceiling that keeps the wargame solver responsive.
pub const POW_CEILING: u32 = 20;

/// Go limiter level as an integer: 0 normal, 1 soft, 2 hard.
pub fn rank(level: Level) -> u8 {
    match level {
        Level::Normal => 0,
        Level::Soft => 1,
        Level::Hard => 2,
    }
}

/// Stable per-fingerprint key, `ja4_stable|subnet` (Go `passFP`). The caller
/// supplies the stable JA4 prefix and the trust-gated client IP; the subnet is
/// derived exactly like Go `clientSubnet`: the first three dotted octets of
/// an IPv4 text, otherwise the whole address text.
pub fn fingerprint(ja4_stable: &str, client_ip: &str) -> String {
    let subnet = if client_ip.matches('.').count() == 3 {
        let mut parts = client_ip.split('.');
        let (a, b, c) = (
            parts.next().unwrap_or_default(),
            parts.next().unwrap_or_default(),
            parts.next().unwrap_or_default(),
        );
        format!("{a}.{b}.{c}")
    } else {
        client_ip.to_owned()
    };
    format!("{ja4_stable}|{subnet}")
}

/// The trust-upgrade signal (Go `passSolvedSignal`).
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

/// The velocity signal for a newly reached level (Go `applyPassVelocity`):
/// soft is `l7.pass.velocity` SUSPICIOUS, hard is `l7.pass.flood` BOT.
pub fn velocity_signal(level: Level) -> Option<Signal> {
    let (id, verdict) = match level {
        Level::Normal => return None,
        Level::Soft => (VELOCITY_SIGNAL, Verdict::Suspicious),
        Level::Hard => (FLOOD_SIGNAL, Verdict::Bot),
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

/// Soft behavioral tells from a submission (Go `applyPassBehavior`), each ID
/// at most once. The result only raises risk; it never rejects the attempt.
pub fn behavior_signals(proof: &PassProof) -> Vec<Signal> {
    let mut signals: Vec<Signal> = Vec::new();
    let mut add = |id: &str, verdict: Verdict, confidence: f64, note: &str| {
        if !signals.iter().any(|signal| signal.id == id) {
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

/// Blue difficulty controller (Go `passDifficulty`): a red-team probe always
/// gets 3; otherwise `trunc((risk - 20) / 15)` clamped to 0..=3.
pub fn difficulty(redteam: Option<&str>, risk: f64) -> u32 {
    if redteam.is_some_and(|value| !value.is_empty()) {
        return 3;
    }
    // `as` truncates toward zero like Go int(); NaN maps to 0.
    ((risk - 20.0) / 15.0).trunc().clamp(0.0, 3.0) as u32
}

/// Proof-of-work leading-zero bits: `14 + difficulty + 2 * velocity`, capped
/// at 20. Velocity taxes CPU cost only, never the puzzle.
pub fn pow_difficulty(difficulty: u32, velocity: Level) -> u32 {
    (POW_BASE + difficulty + u32::from(rank(velocity)) * 2).min(POW_CEILING)
}

/// KPI label (Go `passLabel`): the red-team strategy when declared, else the
/// weak risk band `bot` (>= 70), `human` (< 30) or `unknown`.
pub fn label(risk: f64, redteam: Option<&str>) -> &str {
    match redteam {
        Some(strategy) if !strategy.is_empty() => strategy,
        _ if risk >= 70.0 => "bot",
        _ if risk < 30.0 => "human",
        _ => "unknown",
    }
}

/// Whether a KPI label is an adversary (Go `isBotLabel`).
pub fn is_bot_label(label: &str) -> bool {
    label != "human" && label != "unknown"
}

/// Arithmetic mean; 0 for an empty slice (Go `meanFloats`).
pub fn mean(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.iter().sum::<f64>() / samples.len() as f64
}

/// Population standard deviation; 0 for an empty slice (Go `stddev`).
pub fn stddev(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let center = mean(samples);
    let variance = samples
        .iter()
        .map(|sample| (sample - center) * (sample - center))
        .sum::<f64>()
        / samples.len() as f64;
    variance.sqrt()
}

/// Whether every sample is a whole millisecond within 1e-6 (Go
/// `allIntegerMs`); false for an empty slice.
pub fn all_integer_ms(samples: &[f64]) -> bool {
    !samples.is_empty()
        && samples.iter().all(|sample| {
            let fraction = sample - sample.floor();
            !(fraction > 1e-6 && fraction < 1.0 - 1e-6)
        })
}
