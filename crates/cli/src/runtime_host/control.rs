//! Independent native watchdog: synchronous custody/flush cannot stall it.
use super::plan::{StopPlan, invalid};
use humanymous_runtime::service::{ExitReport, ShutdownRequest, StopBudget};
use std::{
    io,
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tokio::sync::watch;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Starting,
    Running,
    Draining,
    Stopped,
    Failed,
}
struct State {
    phase: Phase,
    deadline: Option<Instant>,
}
impl State {
    fn ready(&mut self) -> io::Result<bool> {
        match self.phase {
            Phase::Starting => {
                self.phase = Phase::Running;
                self.deadline = None;
                Ok(true)
            }
            Phase::Draining => Ok(false),
            _ => Err(invalid("unexpected host readiness transition")),
        }
    }
    fn stop(&mut self, deadline: Instant) -> bool {
        match self.phase {
            Phase::Starting | Phase::Running => {
                self.phase = Phase::Draining;
                self.deadline = Some(self.deadline.map_or(deadline, |old| old.min(deadline)));
                true
            }
            _ => false,
        }
    }
    fn stopped(&mut self, report: &ExitReport) -> io::Result<()> {
        if self.phase != Phase::Draining
            || !report.stopped
            || !report.drained
            || !report.audit_flushed
        {
            return Err(invalid("unproven runtime shutdown"));
        }
        self.phase = Phase::Stopped;
        self.deadline = None;
        Ok(())
    }
}
type Shared = Arc<(Mutex<State>, Condvar)>;

#[derive(Clone)]
pub(super) struct StopHandle {
    state: Shared,
    sender: watch::Sender<Option<ShutdownRequest>>,
    budget: StopBudget,
    watchdog: Duration,
}
impl StopHandle {
    pub(super) fn request(&self) -> io::Result<()> {
        let (mut state, wake) = (
            self.state
                .0
                .lock()
                .map_err(|_| invalid("supervisor poisoned"))?,
            &self.state.1,
        );
        if state.stop(Instant::now() + self.watchdog) {
            // Preserve the stop deadline even if the runtime has already returned
            // and dropped its receiver. Its typed failure must remain observable.
            self.sender.send_replace(Some(self.budget.request()));
            wake.notify_all();
        }
        Ok(())
    }
    pub(super) fn stopping(&self) -> io::Result<bool> {
        Ok(self
            .state
            .0
            .lock()
            .map_err(|_| invalid("supervisor poisoned"))?
            .phase
            == Phase::Draining)
    }
}

pub(super) struct Supervisor {
    state: Shared,
    worker: Option<JoinHandle<()>>,
    stop: StopHandle,
}
impl Supervisor {
    pub(super) fn preflight() -> io::Result<Self> {
        // No secret/custody exists yet. Bound even a synchronous metadata or
        // executable read before its plan-controlled runtime supervisor starts.
        let plan = StopPlan {
            drain_ms: 3000,
            supervisor_reserve_ms: 1000,
            manager_stop_ms: if cfg!(windows) { 6000 } else { 5000 },
        };
        let (supervisor, _) = Self::start(Duration::from_secs(5), plan)?;
        Ok(supervisor)
    }
    pub(super) fn start(
        startup: Duration,
        plan: StopPlan,
    ) -> io::Result<(Self, watch::Receiver<Option<ShutdownRequest>>)> {
        let budget = plan.budget()?;
        let watchdog = plan.watchdog()?;
        let state = Arc::new((
            Mutex::new(State {
                phase: Phase::Starting,
                deadline: Some(Instant::now() + startup),
            }),
            Condvar::new(),
        ));
        let (sender, receiver) = watch::channel(None);
        let stop = StopHandle {
            state: state.clone(),
            sender,
            budget,
            watchdog,
        };
        let monitor = state.clone();
        let worker = std::thread::Builder::new()
            .name("runtime-deadline".into())
            .spawn(move || {
                let mut state = match monitor.0.lock() {
                    Ok(state) => state,
                    Err(_) => std::process::exit(74),
                };
                loop {
                    if matches!(state.phase, Phase::Stopped | Phase::Failed) {
                        return;
                    }
                    if let Some(deadline) = state.deadline {
                        let Some(remaining) = deadline.checked_duration_since(Instant::now())
                        else {
                            eprintln!("runtime-host deadline expired; shutdown is unproven");
                            std::process::exit(74);
                        };
                        state = match monitor.1.wait_timeout(state, remaining) {
                            Ok((state, _)) => state,
                            Err(_) => std::process::exit(74),
                        };
                    } else {
                        state = match monitor.1.wait(state) {
                            Ok(state) => state,
                            Err(_) => std::process::exit(74),
                        };
                    }
                }
            })?;
        Ok((
            Self {
                state,
                worker: Some(worker),
                stop,
            },
            receiver,
        ))
    }
    pub(super) fn stop_handle(&self) -> StopHandle {
        self.stop.clone()
    }
    pub(super) fn ready(&self) -> io::Result<bool> {
        let result = self
            .state
            .0
            .lock()
            .map_err(|_| invalid("supervisor poisoned"))?
            .ready();
        self.state.1.notify_all();
        result
    }
    pub(super) fn stopped(&self, report: &ExitReport) -> io::Result<()> {
        self.state
            .0
            .lock()
            .map_err(|_| invalid("supervisor poisoned"))?
            .stopped(report)?;
        self.state.1.notify_all();
        Ok(())
    }
}
impl Drop for Supervisor {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.0.lock() {
            if state.phase != Phase::Stopped {
                state.phase = Phase::Failed;
            }
            state.deadline = None;
            self.state.1.notify_all();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pure_stop_state_never_extends_deadline_or_fabricates_flush() {
        let now = Instant::now();
        let mut state = State {
            phase: Phase::Starting,
            deadline: Some(now + Duration::from_secs(1)),
        };
        assert!(state.stop(now + Duration::from_secs(5)));
        assert_eq!(state.deadline, Some(now + Duration::from_secs(1)));
        assert!(!state.ready().unwrap());
        assert!(!state.stop(now + Duration::from_secs(20)));
        assert_eq!(state.deadline, Some(now + Duration::from_secs(1)));
        assert!(
            state
                .stopped(&ExitReport {
                    stopped: true,
                    drained: true,
                    audit_flushed: false
                })
                .is_err()
        );
        state
            .stopped(&ExitReport {
                stopped: true,
                drained: true,
                audit_flushed: true,
            })
            .unwrap();
        assert_eq!(state.phase, Phase::Stopped);
    }
    #[test]
    fn pure_readiness_before_stop_cannot_prove_stopped() {
        let mut state = State {
            phase: Phase::Starting,
            deadline: None,
        };
        assert!(state.ready().unwrap());
        assert!(
            state
                .stopped(&ExitReport {
                    stopped: true,
                    drained: true,
                    audit_flushed: true
                })
                .is_err()
        );
    }
}
