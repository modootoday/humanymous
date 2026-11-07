use super::{
    KeySample,
    statistics::{mean, variance},
};
use humanymous_core::signals::report::KeyFeatures;
use std::collections::HashMap;

pub(super) fn reduce(downs: &[KeySample], ups: &[KeySample], pastes: i64) -> KeyFeatures {
    // Preserve the oracle's window-wide latest-down pairing, including negative dwell.
    let mut pending = HashMap::new();
    for down in downs {
        pending.insert(down.token, down.time_ms);
    }
    let mut dwell = Vec::new();
    let mut zero = 0;
    for up in ups {
        if let Some(time) = pending.remove(&up.token) {
            let gap = up.time_ms - time;
            dwell.push(gap);
            if gap <= 1.0 {
                zero += 1;
            }
        }
    }
    let flight: Vec<_> = downs
        .windows(2)
        .map(|p| p[1].time_ms - p[0].time_ms)
        .collect();
    KeyFeatures {
        keystrokes: downs.len() as i64,
        mean_dwell_ms: mean(&dwell),
        dwell_std_dev_ms: variance(&dwell).sqrt(),
        mean_flight_ms: mean(&flight),
        flight_std_dev: variance(&flight).sqrt(),
        paste_events: pastes,
        zero_dwell_frac: if downs.is_empty() {
            0.0
        } else {
            zero as f64 / downs.len() as f64
        },
    }
}
