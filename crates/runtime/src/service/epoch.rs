//! Private, test-compiled S1 gate. This is not a native observer authority.
use std::{
    io,
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone)]
struct Gate(Arc<Mutex<State>>);
struct State {
    epoch: [u8; 16],
    phase: Phase,
    loss: Option<Loss>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Starting,
    Ready,
    ManagerStopping,
    Revoked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cause {
    ObserverLost,
    EpochExpired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Loss {
    at: Instant,
    cause: Cause,
}
impl Loss {
    fn deadline(self, original: Instant, drain: std::time::Duration) -> Result<Instant, Refusal> {
        if drain.is_zero() {
            return Err(Refusal::Closed);
        }
        self.at
            .checked_add(drain)
            .map(|until| original.min(until))
            .ok_or(Refusal::Closed)
    }
}
#[derive(Debug)]
enum Refusal {
    Closed,
    ForeignEpoch,
    Poisoned,
    AlreadyAttempted,
    InvalidWrite,
    Io(io::Error),
}
struct Work {
    owner: Arc<Mutex<State>>,
    epoch: [u8; 16],
    attempted: bool,
    accepted: usize,
    unknown: bool,
}
impl Gate {
    /// Caller-chosen epoch bytes cannot open this initially closed gate.
    fn closed(epoch: [u8; 16]) -> Self {
        Self(Arc::new(Mutex::new(State {
            epoch,
            phase: Phase::Starting,
            loss: None,
        })))
    }
    // Test-private activation models a future accepted producer. There is no
    // production constructor, boolean, callback or deserialized Ready grant.
    fn activate_for_test(&self) -> Result<(), Refusal> {
        let mut state = self.0.lock().map_err(|_| Refusal::Poisoned)?;
        if state.phase != Phase::Starting || state.epoch == [0; 16] {
            return Err(Refusal::Closed);
        }
        state.phase = Phase::Ready;
        Ok(())
    }
    fn live(&self) -> bool {
        self.0.lock().is_ok_and(|state| state.phase == Phase::Ready)
    }
    fn admit(&self, epoch: [u8; 16]) -> Result<Work, Refusal> {
        let state = self.0.lock().map_err(|_| Refusal::Poisoned)?;
        check(&state, epoch)?;
        Ok(Work {
            owner: self.0.clone(),
            epoch,
            attempted: false,
            accepted: 0,
            unknown: false,
        })
    }
    fn revoke(&self, epoch: [u8; 16], cause: Cause) -> Result<Loss, Refusal> {
        let mut state = self.0.lock().map_err(|_| Refusal::Poisoned)?;
        if state.epoch != epoch {
            return Err(Refusal::ForeignEpoch);
        }
        let loss = *state.loss.get_or_insert_with(|| Loss {
            at: Instant::now(),
            cause,
        });
        state.phase = Phase::Revoked;
        Ok(loss)
    }
    fn manager_stop(&self) -> Result<(), Refusal> {
        let mut state = self.0.lock().map_err(|_| Refusal::Poisoned)?;
        if state.phase != Phase::Revoked {
            state.phase = Phase::ManagerStopping;
        }
        Ok(())
    }
    /// S1 models exactly one bounded nonblocking connect initiation. An accepted
    /// transport attempt is not a first application byte or completed effect.
    fn connect(
        &self,
        work: &mut Work,
        operation: impl FnOnce() -> io::Result<()>,
    ) -> Result<(), Refusal> {
        let state = self.0.lock().map_err(|_| Refusal::Poisoned)?;
        self.check_work(&state, work)?;
        if work.attempted {
            return Err(Refusal::AlreadyAttempted);
        }
        work.attempted = true;
        operation().map_err(Refusal::Io)
    }
    /// A private model closure stands for one kernel try_write, never async I/O,
    /// flush, token query, callback authority or durable audit under the mutex.
    fn write(
        &self,
        work: &mut Work,
        bytes: &[u8],
        operation: impl FnOnce(&[u8]) -> io::Result<usize>,
    ) -> Result<usize, Refusal> {
        let state = self.0.lock().map_err(|_| Refusal::Poisoned)?;
        self.check_work(&state, work)?;
        if !work.attempted || work.unknown || work.accepted.checked_add(bytes.len()).is_none() {
            return Err(Refusal::InvalidWrite);
        }
        let count = operation(bytes).map_err(Refusal::Io)?;
        if count > bytes.len() {
            // An invalid adapter result cannot justify a zero-effect claim.
            work.unknown = true;
            return Err(Refusal::InvalidWrite);
        }
        work.accepted += count;
        Ok(count)
    }
    fn check_work(&self, state: &State, work: &Work) -> Result<(), Refusal> {
        if !Arc::ptr_eq(&self.0, &work.owner) {
            return Err(Refusal::ForeignEpoch);
        }
        check(state, work.epoch)
    }
}
fn check(state: &State, epoch: [u8; 16]) -> Result<(), Refusal> {
    if state.epoch != epoch {
        return Err(Refusal::ForeignEpoch);
    }
    if state.phase != Phase::Ready {
        return Err(Refusal::Closed);
    }
    Ok(())
}

mod tests;
