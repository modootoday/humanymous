//! Media bandwidth-abuse detection (SoT-10 §3.2): parallel Range storms,
//! hotlinking and RIT-less media fetches.
//!
//! A per-session sliding window counts recent Range requests; the frozen
//! threshold (8 within 3 s) is conservative to avoid flagging normal seek and
//! preload (SoT-10 §4 false-positive note).
use crate::signals::observation::{Signal, Source, Verdict};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use time::{Duration, OffsetDateTime};

/// Sliding-window length for Range counting.
const WINDOW: Duration = Duration::seconds(3);
/// Range requests within the window that constitute a storm.
const STORM: usize = 8;

/// One media request observed by the server, clocked by the caller.
#[derive(Debug, Clone, Copy)]
pub struct MediaEvent<'a> {
    /// Session identity.
    pub session_id: &'a str,
    /// Whether the request carried a `Range` header.
    pub is_range: bool,
    /// Whether the Referer host differs from this deployment's host.
    pub external_ref: bool,
    /// Whether a RIT token accompanied the request.
    pub has_rit: bool,
    /// Observation instant.
    pub at: OffsetDateTime,
}

/// Per-session Range request history. Safe to share across request threads.
///
/// Like the legacy tracker, session histories are only pruned when that
/// session issues another Range request; there is no global eviction.
#[derive(Debug, Default)]
pub struct MediaTracker {
    history: Mutex<HashMap<String, Vec<OffsetDateTime>>>,
}

impl MediaTracker {
    /// Tracker with the frozen storm threshold.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an event and return the frozen L5 signals it triggers, in order:
    /// `l5.media.range_storm`, `l5.media.hotlink`, `l5.resource.rit_absent`.
    pub fn observe(&self, event: &MediaEvent<'_>) -> Vec<Signal> {
        let signal =
            |id, value, verdict, notes| Signal::new(id, value, verdict, 1.0, Source::Server, notes);
        let mut result = Vec::new();
        if event.is_range {
            let count = self.record_range(event.session_id, event.at);
            if count >= STORM {
                result.push(signal(
                    "l5.media.range_storm",
                    json!(count),
                    Verdict::Bot,
                    "parallel Range request storm",
                ));
            }
        }
        if event.external_ref {
            result.push(signal(
                "l5.media.hotlink",
                json!(true),
                Verdict::Suspicious,
                "external referer media fetch",
            ));
        }
        if !event.has_rit {
            result.push(signal(
                "l5.resource.rit_absent",
                json!(true),
                Verdict::Bot,
                "media fetch without RIT token",
            ));
        }
        result
    }

    /// Append `at` and return how many retained instants are strictly after
    /// `at - WINDOW`, including `at` itself.
    fn record_range(&self, session_id: &str, at: OffsetDateTime) -> usize {
        // The map holds only plain timestamps, so a poisoned lock is still consistent.
        let mut history = self.history.lock().unwrap_or_else(PoisonError::into_inner);
        let entries = history.entry(session_id.to_owned()).or_default();
        // Without a representable cut every retained instant is inside the window.
        if let Some(cut) = at.checked_sub(WINDOW) {
            entries.retain(|instant| *instant > cut);
        }
        entries.push(at);
        entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(session_id: &str, at: OffsetDateTime) -> MediaEvent<'_> {
        MediaEvent {
            session_id,
            is_range: true,
            external_ref: false,
            has_rit: true,
            at,
        }
    }

    fn base() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1000).unwrap()
    }

    #[test]
    fn storm_fires_at_eighth_range_inside_window_and_window_is_exclusive() {
        let tracker = MediaTracker::new();
        for index in 0..7 {
            assert!(
                tracker
                    .observe(&event("s", base() + Duration::milliseconds(index * 100)))
                    .is_empty()
            );
        }
        let signals = tracker.observe(&event("s", base() + Duration::milliseconds(700)));
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].id, "l5.media.range_storm");
        assert_eq!(signals[0].value, json!(8));
        assert_eq!(signals[0].verdict, Verdict::Bot);

        // Exactly WINDOW after the first instant drops it (strictly-after rule).
        let other = MediaTracker::new();
        for index in 0..7 {
            other.observe(&event("s", base() + Duration::milliseconds(index)));
        }
        assert!(other.observe(&event("s", base() + WINDOW)).is_empty());
    }

    #[test]
    fn sessions_are_isolated_and_non_range_events_do_not_count() {
        let tracker = MediaTracker::new();
        for index in 0..7 {
            tracker.observe(&event("a", base() + Duration::milliseconds(index)));
            tracker.observe(&event("b", base() + Duration::milliseconds(index)));
            let mut plain = event("a", base());
            plain.is_range = false;
            assert!(tracker.observe(&plain).is_empty());
        }
        assert_eq!(
            tracker.observe(&event("a", base() + Duration::milliseconds(8)))[0].value,
            json!(8)
        );
    }

    #[test]
    fn hotlink_and_rit_absent_follow_storm_in_order() {
        let tracker = MediaTracker::new();
        let mut last = Vec::new();
        for index in 0..8 {
            last = tracker.observe(&MediaEvent {
                session_id: "s",
                is_range: true,
                external_ref: true,
                has_rit: false,
                at: base() + Duration::milliseconds(index),
            });
        }
        let ids: Vec<_> = last.iter().map(|signal| signal.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "l5.media.range_storm",
                "l5.media.hotlink",
                "l5.resource.rit_absent"
            ]
        );
        assert_eq!(last[1].verdict, Verdict::Suspicious);
        assert_eq!(last[2].verdict, Verdict::Bot);
        for signal in &last {
            assert!(crate::signals::observation::lookup(&signal.id).is_some());
            assert_eq!(signal.collected, Source::Server);
        }
    }

    #[test]
    fn earliest_instant_does_not_overflow() {
        let tracker = MediaTracker::new();
        for _ in 0..8 {
            tracker.observe(&event(
                "s",
                OffsetDateTime::new_utc(time::Date::MIN, time::Time::MIDNIGHT),
            ));
        }
        assert_eq!(
            tracker.observe(&event(
                "s",
                OffsetDateTime::new_utc(time::Date::MIN, time::Time::MIDNIGHT)
            ))[0]
                .value,
            json!(9)
        );
    }
}
