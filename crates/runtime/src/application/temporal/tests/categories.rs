//! Private consumer bookkeeping tests; no provider authority.
use super::*;

#[test]
fn missing_key_noops_never_become_committed_normal_observations() {
    let mut tracker = Tracker::new();
    let mut frame = tracker.begin(tick(1)).unwrap();
    frame
        .rate(|| Ok(rate::Observation::IgnoredEmptyKey))
        .unwrap();
    frame
        .correlation(false, || Ok(correlation(correlation::Scope::MissingKey)))
        .unwrap();
    frame
        .correlation(true, || Ok(correlation(correlation::Scope::MissingKey)))
        .unwrap();
    let receipt = frame.finish();
    assert!(
        receipt
            .records
            .iter()
            .flatten()
            .all(|record| record.effect == Effect::Noop)
    );
    let before = tracker.view(tick(1)).losses;
    let mut frame = tracker.begin(tick(2)).unwrap();
    frame
        .rate(|| Ok(counted(1, rate::Coverage::Retained)))
        .unwrap();
    frame.finish();
    assert_eq!(tracker.view(tick(2)).losses, before);
}

#[test]
fn pure_inspections_preserve_absence_and_unqualified_history_without_mutation() {
    let mut tracker = Tracker::new();
    let mut frame = tracker.begin(tick(1)).unwrap();
    frame.rate_level(|| rate::Level::Normal).unwrap();
    frame.stats(|| correlation::Stats::Absent).unwrap();
    frame.traffic_snapshot(Vec::new).unwrap();
    frame
        .traffic_check(|| {
            Ok(traffic::Outcome {
                signals: Vec::new(),
                coverage: traffic::Coverage::UnqualifiedHistory,
            })
        })
        .unwrap();
    let receipt = frame.finish();
    assert!(
        receipt
            .records
            .iter()
            .flatten()
            .all(|record| record.effect == Effect::Inspection)
    );
    assert_eq!(
        receipt.records[Operation::CorrelationStats.index()]
            .unwrap()
            .detail,
        Detail::Stats(correlation::Stats::Absent)
    );
    assert_eq!(
        receipt.records[Operation::TrafficCheck.index()]
            .unwrap()
            .detail,
        Detail::History(traffic::Coverage::UnqualifiedHistory)
    );
}

#[test]
fn maintenance_and_warmup_commit_but_never_upgrade_scope_or_erase_loss() {
    let mut tracker = Tracker::new();
    let mut frame = tracker.begin(tick(1)).unwrap();
    frame
        .anomaly(|| {
            Ok(anomaly::Measurement {
                anomalous: false,
                score: 0.0,
                median: 0.0,
                mad: 0.0,
                samples: 0,
                warmed_up: false,
                evicted_key: Some("private-key".into()),
            })
        })
        .unwrap();
    frame
        .correlation(false, || Ok(correlation(correlation::Scope::LocalDegraded)))
        .unwrap();
    frame
        .correlation(true, || {
            Ok(correlation(
                correlation::Scope::CoordinatorReportedUnqualified,
            ))
        })
        .unwrap();
    frame.traffic_tls(|| Ok(())).unwrap();
    frame
        .traffic_http(|| Ok(traffic::Association::AddressOnly))
        .unwrap();
    for domain in [
        Domain::Anomaly,
        Domain::Rate,
        Domain::Correlation,
        Domain::Traffic,
    ] {
        assert_eq!(frame.maintenance(domain, false, || Ok(0usize)).unwrap(), 0);
        frame.maintenance(domain, true, || Ok(())).unwrap();
    }
    frame
        .rate_configure(|| {
            Ok(rate::ConfigurationEffect {
                window_expanded: false,
                history_incomplete: false,
            })
        })
        .unwrap();
    let receipt = frame.finish();
    assert!(
        receipt
            .records
            .iter()
            .flatten()
            .all(|record| record.effect == Effect::Committed)
    );
    assert_eq!(
        receipt.records[Operation::CorrelationFanout.index()]
            .unwrap()
            .detail,
        Detail::Correlation(correlation::Scope::CoordinatorReportedUnqualified)
    );
    assert_eq!(
        receipt.records[Operation::TrafficHttp.index()]
            .unwrap()
            .detail,
        Detail::Association(traffic::Association::AddressOnly)
    );
    assert_eq!(
        tracker.view(tick(2)).losses[Domain::Anomaly.index()]
            .unwrap()
            .reason,
        Gap::Evicted
    );
}
