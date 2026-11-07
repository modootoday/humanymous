use super::*;

fn point(x: f64, y: f64, t: f64) -> PointerSample {
    PointerSample { x, y, time_ms: t }
}
fn key(token: u32, t: f64) -> KeySample {
    KeySample { token, time_ms: t }
}
fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-11 * expected.abs().max(1.0),
        "{actual} != {expected}"
    );
}

#[test]
fn analytic_pointer_geometry_population_variance_and_entropy() {
    let mut r = Recorder::new(0.0).unwrap();
    for (p, c) in [
        (point(0.0, 0.0, 0.0), 4.0),
        (point(3.0, 0.0, 1.0), 1.0),
        (point(6.0, 0.0, 2.0), 1.0),
        (point(6.0, 4.0, 4.0), 2.0),
    ] {
        r.pointer_move(p, p.time_ms, p.time_ms, c, true).unwrap();
    }
    let f = r.summary(1000.0).unwrap().mouse;
    assert_eq!(f.samples, 4);
    near(f.velocity_std_dev, (2.0_f64 / 9.0).sqrt());
    near(f.accel_entropy, 1.0);
    near(f.mean_jerk, 1.0);
    near(f.straight_line_frac, 0.5);
    near(f.mean_curvature, 0.005);
    near(f.max_teleport_px, 4.0);
    near(f.coalesced_ratio, 2.0);
}
#[test]
fn short_and_stationary_pointer_streams_keep_frozen_defaults() {
    let mut r = Recorder::new(0.0).unwrap();
    for t in [0.0, 120.0] {
        r.pointer_move(point(0.0, 0.0, t), t, t, 1.0, true).unwrap();
    }
    assert_eq!(r.summary(120.0).unwrap().mouse.pause_count, 0);
    r.pointer_move(point(0.0, 0.0, 240.001), 240.001, 240.001, 1.0, true)
        .unwrap();
    let f = r.summary(241.0).unwrap().mouse;
    assert_eq!(f.pause_count, 1);
    near(f.straight_line_frac, 0.0);
    near(f.mean_curvature, 0.0);
}
#[test]
fn repeat_key_preserves_negative_window_pairing_and_press_to_press_flight() {
    let mut r = Recorder::new(0.0).unwrap();
    r.key_down(key(7, 0.0), 0.0, true).unwrap();
    r.key_up(key(7, 10.0)).unwrap();
    r.key_down(key(7, 20.0), 20.0, true).unwrap();
    r.key_up(key(7, 30.0)).unwrap();
    r.key_up(key(99, 40.0)).unwrap();
    r.paste().unwrap();
    let f = r.summary(50.0).unwrap();
    assert_eq!(f.key.keystrokes, 2);
    near(f.key.mean_dwell_ms, -10.0);
    near(f.key.zero_dwell_frac, 0.5);
    near(f.key.mean_flight_ms, 20.0);
    assert_eq!(f.key.paste_events, 1);
    assert_eq!(f.events.total_events, 2);
    assert_eq!(f.mouse.samples, 0);
    let wire = serde_json::to_string(&f).unwrap();
    assert!(!wire.contains("token"));
    assert!(!wire.contains("code"));
    assert!(!wire.contains("time_ms"));
}
#[test]
fn cadence_uses_strict_long_boundary_and_population_variance() {
    let mut r = Recorder::new(0.0).unwrap();
    for time in [0.0, 1500.0, 1501.0, 3002.0, 3003.0] {
        r.wheel(time).unwrap();
    }
    let f = r.summary(3003.0).unwrap();
    assert_eq!(f.events.long_gap_count, 1);
    near(f.events.min_reaction_ms, 1.0);
    near(f.events.burst_var_ms, 2.0 * 1499.0_f64.powi(2) / 9.0);
    near(f.duration_s, 3.003);
}
#[test]
fn counts_and_trust_preserve_each_event_kind_and_no_move_boundary() {
    let mut r = Recorder::new(0.0).unwrap();
    r.pointer_down(0.0, 0.0, false).unwrap();
    r.pointer_move(point(1.0, 1.0, 1.0), 1.0, 1.0, 1.0, true)
        .unwrap();
    r.pointer_down(801.0, 801.0, true).unwrap();
    r.pointer_down(801.1, 801.1, true).unwrap();
    r.wheel(802.0).unwrap();
    r.paste().unwrap();
    r.key_up(key(0, 803.0)).unwrap();
    let f = r.summary(804.0).unwrap();
    assert_eq!(f.events.total_events, 5);
    assert_eq!(f.events.click_count, 3);
    assert_eq!(f.events.no_move_clicks, 2);
    near(f.events.untrusted_frac, 0.2);
    assert_eq!(f.events.synthetic_flags, 0);
}
#[test]
fn first_six_hundred_caps_do_not_roll_or_stop_aggregate_counts() {
    let mut r = Recorder::new(0.0).unwrap();
    for t in 0..1000 {
        r.pointer_move(
            point(t as f64, 0.0, t as f64),
            t as f64,
            t as f64,
            4.0,
            true,
        )
        .unwrap();
    }
    r.pointer_down(1700.0, 1700.0, true).unwrap();
    let f = r.summary(2000.0).unwrap();
    assert_eq!(f.mouse.samples, 600);
    assert_eq!(r.moves.last().unwrap().x, 599.0);
    assert_eq!(r.times.len(), 600);
    assert_eq!(f.events.total_events, 1001);
    assert_eq!(f.events.no_move_clicks, 0);
    near(f.mouse.coalesced_ratio, 4.0);
}
#[test]
fn malformed_or_unserializable_measurements_cannot_be_reported_as_success() {
    assert!(Recorder::new(f64::NAN).is_err());
    assert!(Recorder::new(-1.0).is_err());
    let mut r = Recorder::new(10.0).unwrap();
    assert!(r.key_down(key(1, 9.0), 10.0, true).is_err());
    assert!(
        r.pointer_move(point(f64::INFINITY, 0.0, 10.0), 10.0, 10.0, 1.0, true)
            .is_err()
    );
    assert!(
        r.pointer_move(point(0.0, 0.0, 10.0), 10.0, 10.0, 0.0, true)
            .is_err()
    );
    assert!(r.summary(f64::INFINITY).is_err());
    assert!(r.summary(9.0).is_err());
    r.events = MAX_NUMBER as i64;
    assert!(r.wheel(10.0).is_err());
}
