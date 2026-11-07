use super::*;

impl Frame<'_> {
    pub(super) fn anomaly(
        &mut self,
        call: impl FnOnce() -> Result<anomaly::Measurement, Unavailable>,
    ) -> Result<anomaly::Measurement, Error> {
        self.apply(Operation::AnomalyObserve, call, |value| {
            (
                Effect::Committed,
                Detail::Anomaly {
                    warmed_up: value.warmed_up,
                    evicted: value.evicted_key.is_some(),
                },
            )
        })
    }
    pub(super) fn rate(
        &mut self,
        call: impl FnOnce() -> Result<rate::Observation, Unavailable>,
    ) -> Result<rate::Observation, Error> {
        self.apply(Operation::RateObserve, call, |value| match value {
            rate::Observation::IgnoredEmptyKey => (Effect::Noop, Detail::MissingIdentity),
            rate::Observation::Counted { coverage, .. } => {
                (Effect::Committed, Detail::Rate(*coverage))
            }
        })
    }
    pub(super) fn correlation(
        &mut self,
        fanout: bool,
        call: impl FnOnce() -> Result<correlation::Outcome, Unavailable>,
    ) -> Result<correlation::Outcome, Error> {
        self.apply(
            if fanout {
                Operation::CorrelationFanout
            } else {
                Operation::CorrelationObserve
            },
            call,
            |value| {
                if value.scope == correlation::Scope::MissingKey {
                    (Effect::Noop, Detail::MissingIdentity)
                } else {
                    (Effect::Committed, Detail::Correlation(value.scope))
                }
            },
        )
    }
    pub(super) fn traffic_tls(
        &mut self,
        call: impl FnOnce() -> Result<(), Unavailable>,
    ) -> Result<(), Error> {
        self.apply(Operation::TrafficTls, call, |_| {
            (Effect::Committed, Detail::Maintenance)
        })
    }
    pub(super) fn traffic_http(
        &mut self,
        call: impl FnOnce() -> Result<traffic::Association, Unavailable>,
    ) -> Result<traffic::Association, Error> {
        self.apply(Operation::TrafficHttp, call, |value| {
            (Effect::Committed, Detail::Association(*value))
        })
    }
    pub(super) fn traffic_check(
        &mut self,
        call: impl FnOnce() -> Result<traffic::Outcome, Unavailable>,
    ) -> Result<traffic::Outcome, Error> {
        self.apply(Operation::TrafficCheck, call, |value| {
            (Effect::Inspection, Detail::History(value.coverage))
        })
    }
    pub(super) fn rate_level(
        &mut self,
        call: impl FnOnce() -> rate::Level,
    ) -> Result<rate::Level, Error> {
        self.apply(
            Operation::RateLevel,
            || Ok(call()),
            |_| (Effect::Inspection, Detail::Inspection),
        )
    }
    pub(super) fn stats(
        &mut self,
        call: impl FnOnce() -> correlation::Stats,
    ) -> Result<correlation::Stats, Error> {
        self.apply(
            Operation::CorrelationStats,
            || Ok(call()),
            |value| (Effect::Inspection, Detail::Stats(*value)),
        )
    }
    pub(super) fn traffic_snapshot(
        &mut self,
        call: impl FnOnce() -> Vec<traffic::Record>,
    ) -> Result<Vec<traffic::Record>, Error> {
        self.apply(
            Operation::TrafficSnapshot,
            || Ok(call()),
            |_| (Effect::Inspection, Detail::Inspection),
        )
    }
    pub(super) fn maintenance<T>(
        &mut self,
        domain: Domain,
        reset: bool,
        call: impl FnOnce() -> Result<T, Unavailable>,
    ) -> Result<T, Error> {
        self.apply(
            if reset {
                Operation::Reset(domain)
            } else {
                Operation::Gc(domain)
            },
            call,
            |_| (Effect::Committed, Detail::Maintenance),
        )
    }
    pub(super) fn rate_configure(
        &mut self,
        call: impl FnOnce() -> Result<rate::ConfigurationEffect, Unavailable>,
    ) -> Result<rate::ConfigurationEffect, Error> {
        self.apply(Operation::RateConfigure, call, |_| {
            (Effect::Committed, Detail::Maintenance)
        })
    }
}
