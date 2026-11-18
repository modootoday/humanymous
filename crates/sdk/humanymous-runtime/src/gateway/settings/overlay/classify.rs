//! Server-computed mutation class (SoT-39 §5.1), equal to the oracle `Classify`.
//!
//! The class only ever rises (`C > B > D > A`), so map iteration order cannot
//! change the result.
use super::{
    Effective, INTEGRITY_CRITICAL_ENGINE_HRS, INTEGRITY_CRITICAL_GATES, Mode, MutationClass,
    Overlay, scoring_defaults,
};

fn rank(class: MutationClass) -> u8 {
    match class {
        MutationClass::A => 1,
        MutationClass::D => 2,
        MutationClass::B => 3,
        MutationClass::C => 4,
    }
}

/// Class of `proposed` relative to code defaults and the current effective
/// posture. Never trust a client-supplied class.
pub fn classify(current: &Effective, proposed: Option<&Overlay>) -> MutationClass {
    let mut class = MutationClass::A;
    let Some(overlay) = proposed else {
        return class;
    };
    let mut raise = |candidate: MutationClass| {
        if rank(candidate) > rank(class) {
            class = candidate;
        }
    };
    let (default_challenge, default_deny, default_cap) = scoring_defaults();
    if let Some(scoring) = &overlay.scoring {
        if let Some(value) = scoring.challenge_at {
            if value > current.challenge_at || value > default_challenge {
                raise(if value > default_challenge + 15.0 {
                    MutationClass::C
                } else {
                    MutationClass::B
                });
            }
            if value < current.challenge_at {
                raise(MutationClass::A);
            }
        }
        if let Some(value) = scoring.deny_at {
            if value > current.deny_at || value > default_deny {
                raise(if value > default_deny + 15.0 {
                    MutationClass::C
                } else {
                    MutationClass::B
                });
            }
            if value < current.deny_at {
                raise(MutationClass::A);
            }
        }
        if let Some(value) = scoring.layer_cap {
            if value < current.layer_cap || value < default_cap {
                raise(MutationClass::B);
            }
            if value > current.layer_cap {
                raise(MutationClass::A);
            }
        }
    }

    let mut zero_count = 0;
    for multiplier in overlay.weight_multipliers.values() {
        if *multiplier == 0.0 {
            zero_count += 1;
            raise(MutationClass::B);
        } else if *multiplier < 1.0 {
            raise(MutationClass::B);
        } else if *multiplier > 1.0 {
            raise(MutationClass::A);
        }
    }
    if zero_count >= 5 {
        raise(MutationClass::C);
    }

    for (id, mode) in &overlay.hard_rules {
        let current_mode = current.hr_mode(id);
        if *mode == Mode::Monitor && current_mode == Mode::Enforce {
            raise(if INTEGRITY_CRITICAL_ENGINE_HRS.contains(&id.as_str()) {
                MutationClass::C
            } else {
                MutationClass::B
            });
        }
        if *mode == Mode::Enforce && current_mode == Mode::Monitor {
            raise(MutationClass::A);
        }
    }

    for (id, mode) in &overlay.gates {
        if *mode != Mode::Enforce && current.gate_mode(id) == Mode::Enforce {
            raise(if INTEGRITY_CRITICAL_GATES.contains(&id.as_str()) {
                MutationClass::C
            } else {
                MutationClass::B
            });
        }
    }

    if overlay.global_monitor == Some(true) && !current.global_monitor {
        raise(MutationClass::C);
    }

    // Residual classes default to monitor except net.correlation (HR-19 family).
    for (id, mode) in &overlay.net_policy {
        let current_mode = match current.net_policy.get(id) {
            Some(mode) => mode.clone(),
            None if id == "net.correlation" => Mode::Enforce,
            None => Mode::Monitor,
        };
        if *mode == Mode::Monitor && current_mode == Mode::Enforce {
            raise(if id == "net.correlation" {
                MutationClass::C
            } else {
                MutationClass::B
            });
        }
    }

    for (prefix, preset) in &overlay.routes {
        let current_preset = current.routes.get(prefix).map_or("", String::as_str);
        if matches!(preset.as_str(), "monitor" | "off") {
            match current_preset {
                "attested" | "strict" => raise(MutationClass::D),
                "balanced" => raise(MutationClass::B),
                _ => {}
            }
        }
        if matches!(preset.as_str(), "attested" | "strict")
            && matches!(current_preset, "monitor" | "off" | "")
        {
            raise(MutationClass::A);
        }
    }

    if let Some(rate) = &overlay.rate_limit {
        if let Some(window) = rate.window_sec {
            if window < current.rate_window_sec {
                raise(MutationClass::B);
            }
            if window > current.rate_window_sec {
                raise(MutationClass::A);
            }
        }
        if rate.hard.is_some_and(|hard| hard > current.rate_hard) {
            raise(MutationClass::B);
        }
        if rate.soft.is_some_and(|soft| soft > current.rate_soft) {
            raise(MutationClass::B);
        }
        if rate.hard.is_some_and(|hard| hard < current.rate_hard) {
            raise(MutationClass::A);
        }
        if rate.soft.is_some_and(|soft| soft < current.rate_soft) {
            raise(MutationClass::A);
        }
    }

    class
}
