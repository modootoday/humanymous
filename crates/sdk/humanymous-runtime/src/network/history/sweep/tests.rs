//! Ports of legacy internal/gate/round3_test.go TestSweepDetector and
//! TestSweepWindowExpiry (baseline 981d25c) plus bounds.
use super::*;

fn detector(window: u64, max: usize) -> Detector {
    Detector::new(Config::new(Duration::from_secs(window), max, 8).unwrap())
}
fn sweeping(result: Sweep) -> bool {
    matches!(result, Sweep::Counted { sweeping: true, .. })
}

#[test]
fn many_sessions_from_one_binding_trip_the_detector() {
    let mut d = detector(60, 3);
    let now = Duration::from_secs(1000);
    let mut flagged = false;
    for sid in ["a", "b", "c", "d", "e", "f"] {
        flagged |= sweeping(d.observe("bindX", sid, now).unwrap());
    }
    assert!(flagged);
    assert!(!sweeping(d.observe("bindY", "s1", now).unwrap()));
}

#[test]
fn sessions_spread_beyond_the_window_do_not_accumulate() {
    let mut d = detector(10, 3);
    let base = Duration::from_secs(1000);
    for (i, sid) in ["a", "b", "c"].iter().enumerate() {
        d.observe("b", sid, base + Duration::from_secs(11 * i as u64))
            .unwrap();
    }
    assert!(!sweeping(
        d.observe("b", "z", base + Duration::from_secs(100))
            .unwrap()
    ));
}

#[test]
fn repeat_session_is_not_a_new_session() {
    let mut d = detector(60, 1);
    let now = Duration::from_secs(5);
    for _ in 0..5 {
        assert_eq!(
            d.observe("b", "same", now).unwrap(),
            Sweep::Counted {
                sessions: 1,
                sweeping: false
            }
        );
    }
}

#[test]
fn bounds_clock_and_empty_inputs_fail_closed() {
    let mut d = Detector::new(Config::new(Duration::from_secs(60), 2, 1).unwrap());
    let now = Duration::from_secs(10);
    assert_eq!(d.observe("", "s", now), Ok(Sweep::IgnoredEmptyBinding));
    assert_eq!(
        d.observe("b", "", now),
        Err(Unavailable::InvalidMeasurement)
    );
    d.observe("b", "s", now).unwrap();
    assert_eq!(d.observe("c", "s", now), Err(Unavailable::Budget));
    assert_eq!(
        d.observe("b", "s", now - Duration::from_secs(1)),
        Err(Unavailable::TimeRegression)
    );
    for sid in 0..100 {
        d.observe("b", &sid.to_string(), now).unwrap();
    }
    assert_eq!(
        d.observe("b", "x", now).unwrap(),
        Sweep::Counted {
            sessions: 3,
            sweeping: true
        }
    );
    assert_eq!(d.gc(now + Duration::from_secs(121)), Ok(1));
    assert!(d.observe("c", "s", now + Duration::from_secs(121)).is_ok());
    assert!(Config::new(Duration::ZERO, 1, 1).is_err());
}

/// Sweep rounds from legacy internal/gate wargame_escalation_r81 and r121
/// (baseline 981d25c), with the Go window and threshold values.
mod wargame {
    use super::*;

    /// Go TestWargameR101_SweepDoesNotFlagSingleSession.
    #[test]
    fn wargame_r101_sweep_does_not_flag_single_session() {
        let mut d = detector(60, 8);
        let now = Duration::from_secs(3000);
        assert!(!sweeping(d.observe("bind", "sid1", now).unwrap()));
    }

    /// Go TestWargameR102_SweepFlagsBurst.
    #[test]
    fn wargame_r102_sweep_flags_burst() {
        let mut d = detector(60, 3);
        let now = Duration::from_secs(3000);
        let mut flagged = false;
        for sid in ["sa", "sb", "sc", "sd", "se"] {
            flagged |= sweeping(d.observe("bind", sid, now).unwrap());
        }
        assert!(flagged);
    }

    /// Go TestWargameR103_SweepWindowResets.
    #[test]
    fn wargame_r103_sweep_window_resets() {
        let mut d = detector(5, 3);
        let base = Duration::from_secs(4000);
        for (i, sid) in ["sa", "sb", "sc"].iter().enumerate() {
            d.observe("b", sid, base + Duration::from_secs(6 * i as u64))
                .unwrap();
        }
        assert!(!sweeping(
            d.observe("b", "sZ", base + Duration::from_secs(30))
                .unwrap()
        ));
    }

    /// Go TestWargameR152_SweepGC: an expired session leaves the window.
    #[test]
    fn wargame_r152_sweep_gc() {
        let mut d = detector(1, 2);
        let now = Duration::from_secs(6000);
        d.observe("b", "s1", now).unwrap();
        assert_eq!(
            d.observe("b", "s2", now + Duration::from_secs(2)),
            Ok(Sweep::Counted {
                sessions: 1,
                sweeping: false
            })
        );
    }
}
