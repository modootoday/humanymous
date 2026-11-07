//! Opt-in bounded numeric observations; handler/submission time is not display time.
use serde::Serialize;
use std::{cell::RefCell, collections::VecDeque, rc::Rc, time::Instant};

const CAPACITY: usize = 512;
const MAX_BYTES: usize = 512 * 1024;
pub type Shared = Rc<RefCell<Trace>>;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    MeariResizeHandler,
    MeariRedrawHandler,
    TabProjection,
}
#[derive(Clone, Serialize)]
pub struct Record {
    sequence: u64,
    stage: Stage,
    stage_started_ns: u64,
    duration_ns: u64,
    source_commits: Option<u64>,
    last_submitted_frame_generation: Option<u64>,
    succeeded: Option<bool>,
}
#[derive(Clone, Serialize)]
pub struct Snapshot {
    schema: u32,
    records: VecDeque<Record>,
    dropped: u64,
    saturated: bool,
    raster_stage_ns: Option<u64>,
    present_call_ns: Option<u64>,
    os_displayed_ns: Option<u64>,
}
pub struct Trace {
    origin: Instant,
    enabled: bool,
    sequence: u64,
    data: Snapshot,
}
impl Trace {
    pub fn shared(enabled: bool) -> Shared {
        Rc::new(RefCell::new(Self {
            origin: Instant::now(),
            enabled,
            sequence: 0,
            data: Snapshot {
                schema: 1,
                records: VecDeque::new(),
                dropped: 0,
                saturated: false,
                raster_stage_ns: None,
                present_call_ns: None,
                os_displayed_ns: None,
            },
        }))
    }
    pub fn start(&self) -> Option<Instant> {
        self.enabled.then(Instant::now)
    }
    pub fn finish(
        &mut self,
        stage: Stage,
        start: Option<Instant>,
        commits: Option<u64>,
        frame: Option<u64>,
        succeeded: Option<bool>,
    ) {
        let Some(start) = start.filter(|_| self.enabled) else {
            return;
        };
        let received = start.saturating_duration_since(self.origin).as_nanos();
        let duration = start.elapsed().as_nanos();
        self.data.saturated |= received > u128::from(u64::MAX) || duration > u128::from(u64::MAX);
        let Some(sequence) = self.sequence.checked_add(1) else {
            self.data.saturated = true;
            self.enabled = false;
            return;
        };
        self.sequence = sequence;
        if self.data.records.len() == CAPACITY {
            self.data.records.pop_front();
            self.data.dropped = self.data.dropped.saturating_add(1);
        }
        self.data.records.push_back(Record {
            sequence,
            stage,
            stage_started_ns: received.min(u128::from(u64::MAX)) as u64,
            duration_ns: duration.min(u128::from(u64::MAX)) as u64,
            source_commits: commits,
            last_submitted_frame_generation: frame,
            succeeded,
        });
    }
    pub fn snapshot(&self) -> Result<Option<Snapshot>, std::io::Error> {
        if self.sequence == 0 && !self.enabled {
            return Ok(None);
        }
        let mut data = self.data.clone();
        while serde_json::to_vec_pretty(&data)?.len() > MAX_BYTES {
            if data.records.pop_front().is_none() {
                return Err(std::io::Error::other("timing metadata exceeds byte budget"));
            }
            data.dropped = data.dropped.saturating_add(1);
        }
        Ok(Some(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_ring_and_encoded_bytes_are_bounded_without_fabricating_display_time() {
        let trace = Trace::shared(true);
        for _ in 0..CAPACITY + 7 {
            let start = trace.borrow().start();
            trace.borrow_mut().finish(
                Stage::TabProjection,
                start,
                Some(u64::MAX),
                Some(u64::MAX),
                Some(true),
            );
        }
        let data = trace.borrow().snapshot().unwrap().unwrap();
        assert_eq!(data.records.len(), CAPACITY);
        assert_eq!(data.dropped, 7);
        assert_eq!(data.records.front().unwrap().sequence, 8);
        assert!(serde_json::to_vec_pretty(&data).unwrap().len() <= MAX_BYTES);
        assert!(
            data.raster_stage_ns.is_none()
                && data.present_call_ns.is_none()
                && data.os_displayed_ns.is_none()
        );
        let disabled = Trace::shared(false);
        assert!(disabled.borrow().start().is_none());
        assert!(disabled.borrow().snapshot().unwrap().is_none());
    }
}
