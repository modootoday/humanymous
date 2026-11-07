use super::{
    PointerSample,
    statistics::{mean, variance},
};
use humanymous_core::signals::report::MouseFeatures;

pub(super) fn reduce(samples: &[PointerSample]) -> MouseFeatures {
    let mut out = MouseFeatures {
        samples: samples.len() as i64,
        ..Default::default()
    };
    if samples.len() < 3 {
        return out;
    }
    let mut velocities = Vec::new();
    for pair in samples.windows(2) {
        let gap = pair[1].time_ms - pair[0].time_ms;
        let distance = (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y);
        out.max_teleport_px = out.max_teleport_px.max(distance);
        velocities.push(distance / gap.max(1.0));
        if gap > 120.0 {
            out.pause_count += 1;
        }
    }
    let accelerations: Vec<_> = velocities.windows(2).map(|p| p[1] - p[0]).collect();
    let jerks: Vec<_> = accelerations
        .windows(2)
        .map(|p| (p[1] - p[0]).abs())
        .collect();
    let (mut straight, mut segments) = (0, 0);
    for p in samples.windows(3) {
        let chord = (p[2].x - p[0].x).hypot(p[2].y - p[0].y);
        let arc =
            (p[2].x - p[1].x).hypot(p[2].y - p[1].y) + (p[1].x - p[0].x).hypot(p[1].y - p[0].y);
        if arc > 0.0 {
            segments += 1;
            if chord / arc > 0.995 {
                straight += 1;
            }
        }
    }
    out.mean_jerk = mean(&jerks);
    out.velocity_std_dev = variance(&velocities).sqrt();
    out.accel_entropy = entropy(&accelerations);
    if segments > 0 {
        out.straight_line_frac = straight as f64 / segments as f64;
        out.mean_curvature = (1.0 - out.straight_line_frac) * 0.01;
    }
    out
}
fn entropy(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let maximum = values.iter().map(|v| v.abs()).fold(0.0, f64::max);
    let maximum = if maximum == 0.0 { 1.0 } else { maximum };
    let mut bins = [0usize; 12];
    for v in values {
        bins[((v.abs() / maximum * 11.0).floor() as usize).min(11)] += 1;
    }
    let mut entropy = 0.0;
    for count in bins.into_iter().filter(|n| *n > 0) {
        let probability = count as f64 / values.len() as f64;
        entropy -= probability * probability.log2();
    }
    entropy
}
