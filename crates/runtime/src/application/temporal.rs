//! Private test-compiled L1 bookkeeping, never serving/capture/fleet authority.
mod operations;
mod progress;
mod tests;
use humanymous_network::{
    correlation,
    history::{Unavailable, anomaly, rate},
    traffic,
};
use progress::{Frame, Tracker};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Domain {
    Anomaly,
    Rate,
    Correlation,
    Traffic,
}
impl Domain {
    fn index(self) -> usize {
        self as usize
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    AnomalyObserve,
    RateObserve,
    CorrelationObserve,
    CorrelationFanout,
    TrafficTls,
    TrafficHttp,
    TrafficCheck,
    RateLevel,
    CorrelationStats,
    TrafficSnapshot,
    Gc(Domain),
    Reset(Domain),
    RateConfigure,
}
const SLOTS: usize = 19;
impl Operation {
    fn index(self) -> usize {
        match self {
            Self::AnomalyObserve => 0,
            Self::RateObserve => 1,
            Self::CorrelationObserve => 2,
            Self::CorrelationFanout => 3,
            Self::TrafficTls => 4,
            Self::TrafficHttp => 5,
            Self::TrafficCheck => 6,
            Self::RateLevel => 7,
            Self::CorrelationStats => 8,
            Self::TrafficSnapshot => 9,
            Self::Gc(domain) => 10 + domain.index(),
            Self::Reset(domain) => 14 + domain.index(),
            Self::RateConfigure => 18,
        }
    }
    fn domain(self) -> Domain {
        match self {
            Self::AnomalyObserve => Domain::Anomaly,
            Self::RateObserve | Self::RateLevel | Self::RateConfigure => Domain::Rate,
            Self::CorrelationObserve | Self::CorrelationFanout | Self::CorrelationStats => {
                Domain::Correlation
            }
            Self::TrafficTls | Self::TrafficHttp | Self::TrafficCheck | Self::TrafficSnapshot => {
                Domain::Traffic
            }
            Self::Gc(domain) | Self::Reset(domain) => domain,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Effect {
    Pending,
    Committed,
    Noop,
    Inspection,
    Refused,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Detail {
    Pending,
    MissingIdentity,
    Anomaly { warmed_up: bool, evicted: bool },
    Rate(rate::Coverage),
    Correlation(correlation::Scope),
    Association(traffic::Association),
    History(traffic::Coverage),
    Stats(correlation::Stats),
    Inspection,
    Maintenance,
    Refused(Unavailable),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Record {
    operation: Operation,
    effect: Effect,
    detail: Detail,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gap {
    Provider(Unavailable),
    MissingIdentity,
    Evicted,
    Interrupted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Loss {
    at: Duration,
    reason: Gap,
}
#[derive(Debug, PartialEq, Eq)]
enum Error {
    Duplicate(Operation),
    ClockRegression,
    Provider(Unavailable),
}
