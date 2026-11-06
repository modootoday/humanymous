use super::FeatureVector;
use humanymous_signals::report::BehaviorSummary;

fn log1p(value: f64) -> f32 {
    let value = if value < 0.0 { 0.0 } else { value };
    value.ln_1p() as f32
}

/// Whether an observed pointer stream is usable; absence is not a bot verdict.
pub fn has_pointer(behavior: &BehaviorSummary) -> bool {
    behavior.mouse.samples > 0
}
/// Whether the session carries keystrokes.
pub fn has_key(behavior: &BehaviorSummary) -> bool {
    behavior.key.keystrokes > 0
}

/// Transform a summary without changing the frozen feature order or scaling.
pub fn extract(behavior: &BehaviorSummary) -> FeatureVector {
    let (mouse, key, events) = (&behavior.mouse, &behavior.key, &behavior.events);
    [
        log1p(mouse.samples as f64),
        mouse.mean_curvature as f32,
        mouse.velocity_std_dev as f32,
        mouse.accel_entropy as f32,
        mouse.straight_line_frac as f32,
        log1p(mouse.pause_count as f64),
        log1p(mouse.max_teleport_px),
        mouse.mean_jerk as f32,
        mouse.coalesced_ratio as f32,
        log1p(key.keystrokes as f64),
        log1p(key.mean_dwell_ms),
        log1p(key.dwell_std_dev_ms),
        log1p(key.mean_flight_ms),
        log1p(key.flight_std_dev),
        log1p(key.paste_events as f64),
        key.zero_dwell_frac as f32,
        log1p(events.total_events as f64),
        events.untrusted_frac as f32,
        log1p(events.no_move_clicks as f64),
        log1p(events.synthetic_flags as f64),
        log1p(events.click_count as f64),
        log1p(events.long_gap_count as f64),
        log1p(events.burst_var_ms),
        log1p(events.min_reaction_ms),
        log1p(behavior.duration_s),
        u8::from(has_pointer(behavior)) as f32,
        u8::from(has_key(behavior)) as f32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use humanymous_signals::report::KeyFeatures;

    #[test]
    fn keyboard_only_presence_is_first_class() {
        let behavior = BehaviorSummary {
            key: KeyFeatures {
                keystrokes: 20,
                mean_dwell_ms: 90.0,
                ..Default::default()
            },
            duration_s: 5.0,
            ..Default::default()
        };
        assert!(!has_pointer(&behavior));
        assert!(has_key(&behavior));
        let vector = extract(&behavior);
        assert_eq!(&vector[25..], &[0.0, 1.0]);
        assert_eq!(vector, extract(&behavior));
    }

    #[test]
    fn empty_summary_is_finite_and_negative_log_inputs_clamp() {
        assert!(
            extract(&BehaviorSummary::default())
                .iter()
                .all(|value| value.is_finite())
        );
        assert_eq!(log1p(-1.0), 0.0);
        assert!(log1p(f64::NAN).is_nan());
    }
}
