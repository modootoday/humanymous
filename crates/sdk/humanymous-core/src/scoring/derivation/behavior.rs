use crate::behavior::features::extract;
use crate::model::inference::Runtime;
use crate::signals::wire::float32_json_value;
use crate::signals::{
    observation::{Signal, Source, Verdict},
    report::BehaviorSummary,
};
use serde_json::json;

fn sample_confidence(samples: i64, minimum: i64) -> f64 {
    if samples <= minimum {
        return 0.5;
    }
    (0.5 + 0.5 * (samples - minimum) as f64 / (minimum * 4) as f64).min(1.0)
}

/// Derive the frozen L4 heuristics and optional score-exempt model observation.
/// The model runtime is shared with its owner rather than duplicated in the scorer.
pub fn behavior_signals(behavior: &BehaviorSummary, model: &Runtime) -> Vec<Signal> {
    let (mouse, key, events) = (&behavior.mouse, &behavior.key, &behavior.events);
    let mut result = Vec::new();
    let mut add = |id, value, verdict, confidence, notes| {
        result.push(Signal::new(
            id,
            value,
            verdict,
            confidence,
            Source::Wasm,
            notes,
        ))
    };
    if mouse.samples < 4 && key.keystrokes == 0 && behavior.duration_s > 1.5 {
        add(
            "l4.event.no_interaction",
            json!(mouse.samples),
            Verdict::Suspicious,
            0.6,
            "no meaningful human input over observation window",
        );
    }
    if events.total_events > 0 {
        if events.untrusted_frac > 0.0 {
            add(
                "l4.event.untrusted",
                json!(events.untrusted_frac),
                Verdict::Bot,
                1.0,
                "synthetic dispatchEvent detected",
            );
        } else {
            add("l4.event.untrusted", json!(0), Verdict::Ok, 1.0, "");
        }
        if events.no_move_clicks > 0 {
            add(
                "l4.event.no_move_click",
                json!(events.no_move_clicks),
                Verdict::Suspicious,
                0.7,
                "clicks without preceding movement",
            );
        }
        if events.synthetic_flags > 0 {
            add(
                "l4.event.perfect_timing",
                json!(events.synthetic_flags),
                Verdict::Suspicious,
                0.6,
                "zero-jitter timing artifacts",
            );
        }
    }
    if events.long_gap_count >= 2
        && events.total_events >= 8
        && events.min_reaction_ms > 0.0
        && events.min_reaction_ms < 150.0
        && events.burst_var_ms < 2500.0
    {
        add(
            "l4.agent.burst_silence",
            json!({"gaps":events.long_gap_count,"react":events.min_reaction_ms}),
            Verdict::Bot,
            0.6,
            "LLM inference-loop cadence (pause->instant burst)",
        );
    }
    if events.click_count >= 2 && mouse.samples < events.click_count.wrapping_mul(3) {
        add(
            "l4.mouse.click_no_trajectory",
            json!(mouse.samples),
            Verdict::Bot,
            0.6,
            "clicks without an approach trajectory (agent teleport-click)",
        );
    }
    if key.keystrokes >= 4 && key.mean_flight_ms > 0.0 && key.mean_flight_ms < 25.0 {
        add(
            "l4.key.machine_speed",
            json!(key.mean_flight_ms),
            Verdict::Bot,
            0.7,
            "machine-speed inter-key latency",
        );
    }
    if mouse.samples >= 12 {
        let confidence = sample_confidence(mouse.samples, 12);
        if mouse.straight_line_frac > 0.8
            || (mouse.mean_curvature > 0.0 && mouse.mean_curvature < 0.001)
        {
            add(
                "l4.mouse.straightness",
                json!(mouse.straight_line_frac),
                Verdict::Suspicious,
                confidence,
                "near-straight-line trajectories",
            );
        }
        if mouse.velocity_std_dev < 0.02 {
            add(
                "l4.mouse.velocity_std",
                json!(mouse.velocity_std_dev),
                Verdict::Suspicious,
                confidence,
                "near-zero velocity variance (too regular)",
            );
        }
        if mouse.max_teleport_px > 400.0 {
            add(
                "l4.mouse.max_teleport",
                json!(mouse.max_teleport_px),
                Verdict::Suspicious,
                confidence,
                "large instantaneous cursor jump",
            );
        }
        if mouse.coalesced_ratio > 0.0 && mouse.coalesced_ratio <= 1.05 {
            add(
                "l4.mouse.coalesced_synthetic",
                json!(mouse.coalesced_ratio),
                Verdict::Suspicious,
                0.5,
                "pointermoves carry no coalesced sub-events (synthetic input)",
            );
        }
    } else if mouse.samples == 0 && events.no_move_clicks > 0 {
        add(
            "l4.mouse.samples",
            json!(0),
            Verdict::Suspicious,
            0.6,
            "interaction without any mouse movement",
        );
    }
    if key.keystrokes >= 6 {
        let confidence = sample_confidence(key.keystrokes, 6);
        if key.dwell_std_dev_ms < 3.0 {
            add(
                "l4.key.dwell_std",
                json!(key.dwell_std_dev_ms),
                Verdict::Suspicious,
                confidence,
                "near-zero dwell variance (fixed-delay typing)",
            );
        }
        if key.flight_std_dev < 5.0 {
            add(
                "l4.key.flight_std",
                json!(key.flight_std_dev),
                Verdict::Suspicious,
                confidence,
                "near-zero flight variance",
            );
        }
        if key.zero_dwell_frac > 0.5 {
            add(
                "l4.key.zero_dwell_frac",
                json!(key.zero_dwell_frac),
                Verdict::Bot,
                confidence,
                "majority synthetic zero-dwell keys",
            );
        }
    }
    if key.paste_events > 0 && key.keystrokes == 0 {
        add(
            "l4.key.paste",
            json!(key.paste_events),
            Verdict::Suspicious,
            0.6,
            "field populated by paste with no keystrokes",
        );
    }
    let prediction = model.score(&extract(behavior));
    if !prediction.abstain {
        let verdict = if prediction.p_bot >= model.fire_threshold() {
            Verdict::Suspicious
        } else {
            Verdict::Ok
        };
        let mut distance = prediction.p_bot - 0.5;
        if distance < 0.0 {
            distance = -distance;
        }
        add(
            "l4.ml.behavioral",
            float32_json_value(prediction.p_bot),
            verdict,
            f64::from(2.0 * distance),
            "behavioral model p(bot)",
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sample_confidence_and_strict_boundary_cases() {
        assert_eq!(sample_confidence(12, 12), 0.5);
        assert_eq!(sample_confidence(60, 12), 1.0);
        let mut behavior = BehaviorSummary {
            duration_s: 1.5,
            ..Default::default()
        };
        assert!(behavior_signals(&behavior, &Runtime::default()).is_empty());
        behavior.duration_s = 1.50001;
        assert_eq!(
            behavior_signals(&behavior, &Runtime::default())[0].id,
            "l4.event.no_interaction"
        );
        behavior.events.click_count = i64::MAX;
        assert!(
            behavior_signals(&behavior, &Runtime::default())
                .iter()
                .any(|signal| signal.id == "l4.mouse.click_no_trajectory")
        );
        behavior.events.click_count = i64::MAX / 2;
        assert!(
            !behavior_signals(&behavior, &Runtime::default())
                .iter()
                .any(|signal| signal.id == "l4.mouse.click_no_trajectory")
        );
    }
}
