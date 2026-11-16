use super::*;
use crate::model::correction::test_support::{feature_vector, go_rng};

#[test]
fn feature_monitor_freezes_reference_before_evaluating() {
    let mut rng = go_rng(1);
    let monitor = FeatureMonitor::new(200).unwrap();
    assert!(!monitor.ready());
    for _ in 0..200 {
        assert_eq!(
            monitor.observe(&feature_vector(&mut rng, 0.0)).unwrap(),
            None
        );
    }
    assert!(monitor.ready());
}

#[test]
fn feature_monitor_keeps_stable_traffic_below_shift_threshold() {
    let mut rng = go_rng(2);
    let monitor = FeatureMonitor::new(600).unwrap();
    for _ in 0..600 {
        monitor.observe(&feature_vector(&mut rng, 0.0)).unwrap();
    }
    let mut reading = None;
    for _ in 0..600 {
        if let Some(value) = monitor.observe(&feature_vector(&mut rng, 0.0)).unwrap() {
            reading = Some(value);
        }
    }
    let reading = reading.expect("a full current window must close");
    assert!(reading < 0.25, "same-distribution PSI {reading} >= 0.25");
}

#[test]
fn feature_monitor_reports_high_psi_for_shifted_traffic() {
    let mut rng = go_rng(3);
    let monitor = FeatureMonitor::new(300).unwrap();
    for _ in 0..300 {
        monitor.observe(&feature_vector(&mut rng, 0.0)).unwrap();
    }
    let mut reading = 0.0;
    for _ in 0..300 {
        if let Some(value) = monitor.observe(&feature_vector(&mut rng, 4.0)).unwrap() {
            reading = value;
        }
    }
    assert!(reading >= 0.25, "shifted PSI {reading} < 0.25");
    assert_eq!(monitor.last_max_psi(), reading);
}

#[test]
fn feature_monitor_rejects_invalid_windows_shapes_and_values() {
    assert_eq!(FeatureMonitor::new(0).unwrap_err(), CorrectionError::Limit);
    assert_eq!(
        FeatureMonitor::new(MAX_WINDOW + 1).unwrap_err(),
        CorrectionError::Limit
    );
    assert_eq!(FeatureMonitor::default().window(), DEFAULT_WINDOW);
    let monitor = FeatureMonitor::new(1).unwrap();
    assert_eq!(
        monitor.observe(&[0.0; 3]).unwrap_err(),
        CorrectionError::Shape
    );
    let mut vector = [0.0_f32; FEATURE_DIM];
    vector[5] = f32::NAN;
    assert_eq!(
        monitor.observe(&vector).unwrap_err(),
        CorrectionError::Nonfinite
    );
    assert!(
        !monitor.ready(),
        "rejected vectors must not enter the reference"
    );
    vector[5] = 1.0;
    assert_eq!(monitor.observe(&vector).unwrap(), None);
    assert!(monitor.ready());
    assert_eq!(monitor.observe(&vector).unwrap(), Some(0.0));
}

#[test]
fn quantile_edges_and_bucket_lookup_follow_legacy_clamping() {
    let sorted: Vec<f64> = (0..9).map(f64::from).collect();
    let edges = quantile_edges(&sorted);
    assert_eq!(edges, [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    assert_eq!(bucket_of(&edges, -5.0), 0);
    assert_eq!(bucket_of(&edges, 3.5), 3);
    assert_eq!(bucket_of(&edges, 8.0), PSI_BUCKETS - 1);
    assert_eq!(bucket_of(&edges, 100.0), PSI_BUCKETS - 1);
    assert_eq!(quantile_edges(&[]), [0.0; PSI_BUCKETS + 1]);
}
