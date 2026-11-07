//! Private consumer bookkeeping tests; no provider authority.
use super::*;

#[test]
fn caught_panic_with_surviving_frame_cannot_finish_pending_as_success() {
    let mut tracker = Tracker::new();
    let mut frame = tracker.begin(tick(1)).unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = frame.traffic_tls(|| panic!("injected pending call"));
        }))
        .is_err()
    );
    assert!(matches!(frame.finish(), Err(Error::Interrupted)));
    let view = tracker.view(tick(1));
    assert!(view.abandoned);
    assert_eq!(
        view.losses[Domain::Traffic.index()].unwrap().reason,
        Gap::Interrupted
    );
    tracker.begin(tick(2)).unwrap().finish().unwrap();
    assert_eq!(tracker.view(tick(2)).losses, view.losses);
}

#[test]
fn later_refusal_preserves_prior_commit_and_gap_survives_success() {
    let calls = Cell::new(0);
    let mut tracker = Tracker::new();
    let mut frame = tracker.begin(tick(1)).unwrap();
    frame
        .rate(|| {
            calls.set(calls.get() + 1);
            Ok(counted(calls.get(), rate::Coverage::Retained))
        })
        .unwrap();
    assert!(matches!(
        frame.correlation(false, || Err(Unavailable::Budget)),
        Err(Error::Provider(Unavailable::Budget))
    ));
    let receipt = frame.finish().unwrap();
    assert_eq!(
        receipt.records[Operation::RateObserve.index()]
            .unwrap()
            .effect,
        Effect::Committed
    );
    let first = tracker.view(tick(1)).losses[Domain::Correlation.index()];
    let mut frame = tracker.begin(tick(2)).unwrap();
    frame
        .rate(|| {
            calls.set(calls.get() + 1);
            Ok(counted(calls.get(), rate::Coverage::Retained))
        })
        .unwrap();
    frame
        .correlation(false, || Ok(correlation(correlation::Scope::LocalOnly)))
        .unwrap();
    frame.finish().unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(
        tracker.view(tick(2)).losses[Domain::Correlation.index()],
        first
    );
}

#[test]
fn duplicate_operation_is_refused_before_lower_callback_runs_again() {
    let calls = Cell::new(0);
    let mut tracker = Tracker::new();
    let mut frame = tracker.begin(tick(1)).unwrap();
    frame
        .rate(|| {
            calls.set(1);
            Ok(counted(1, rate::Coverage::Retained))
        })
        .unwrap();
    assert!(matches!(
        frame.rate(|| {
            calls.set(2);
            Ok(counted(2, rate::Coverage::Retained))
        }),
        Err(Error::Duplicate(Operation::RateObserve))
    ));
    assert_eq!(calls.get(), 1);
    frame.finish().unwrap();
}

#[test]
fn expansion_equality_clears_only_expansion_not_unrelated_latched_loss() {
    let mut tracker = Tracker::new();
    let mut frame = tracker.begin(tick(1)).unwrap();
    frame
        .rate(|| {
            Ok(counted(
                1,
                rate::Coverage::WindowExpanded {
                    complete_after: tick(10),
                },
            ))
        })
        .unwrap();
    assert!(frame.anomaly(|| Err(Unavailable::Budget)).is_err());
    frame.finish().unwrap();
    assert_eq!(tracker.view(tick(9)).expansion_until, Some(tick(10)));
    let at_boundary = tracker.view(tick(10));
    assert_eq!(at_boundary.expansion_until, None);
    assert_eq!(
        at_boundary.losses[Domain::Anomaly.index()].unwrap().reason,
        Gap::Provider(Unavailable::Budget)
    );
}

#[test]
fn interrupted_or_abandoned_progress_and_clock_regression_remain_explicit() {
    let mut tracker = Tracker::new();
    {
        let mut frame = tracker.begin(tick(1)).unwrap();
        frame
            .traffic_http(|| Ok(traffic::Association::Absent))
            .unwrap();
        // No finish: committed lower work cannot be rolled back or hidden.
    }
    assert!(tracker.view(tick(1)).abandoned);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut frame = tracker.begin(tick(2)).unwrap();
            let _ = frame.traffic_tls(|| panic!("injected lower-call interruption"));
        }))
        .is_err()
    );
    assert_eq!(
        tracker.view(tick(2)).losses[Domain::Traffic.index()]
            .unwrap()
            .reason,
        Gap::Interrupted
    );
    assert!(matches!(
        tracker.begin(tick(1)),
        Err(Error::ClockRegression)
    ));
    tracker.begin(tick(3)).unwrap().finish().unwrap();
    assert!(tracker.view(tick(3)).clock_fault);
}
