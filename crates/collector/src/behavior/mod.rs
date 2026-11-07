//! Bounded passive event-to-summary reduction; no scoring or raw text/key export.
mod cadence;
mod keyboard;
mod pointer;
mod statistics;
use humanymous_core::signals::report::{BehaviorSummary, EventFeatures};

/// Frozen first-sample/timeline cap, not a rolling window.
pub const MAX_SAMPLES: usize = 600;
const MAX_NUMBER: f64 = 9_007_199_254_740_991.0;

/// One pointer observation, retained only inside the collection epoch.
#[derive(Clone, Copy)]
pub struct PointerSample {
    /// Viewport x coordinate.
    pub x: f64,
    /// Viewport y coordinate.
    pub y: f64,
    /// Monotonic clock observation in milliseconds.
    pub time_ms: f64,
}
/// Key timing with an opaque per-instance identity, never a character/code string.
#[derive(Clone, Copy)]
pub struct KeySample {
    /// Nonexported pairing identity.
    pub token: u32,
    /// Monotonic clock observation in milliseconds.
    pub time_ms: f64,
}
/// Pure recorder owning bounded input buffers and aggregate counters.
pub struct Recorder {
    start: f64,
    moves: Vec<PointerSample>,
    downs: Vec<KeySample>,
    ups: Vec<KeySample>,
    times: Vec<f64>,
    events: i64,
    untrusted: i64,
    no_move: i64,
    pastes: i64,
    last_move: f64,
    coalesced_total: f64,
    coalesced_samples: f64,
    clicks: i64,
}
fn count(value: &mut i64) -> Result<(), &'static str> {
    if *value >= MAX_NUMBER as i64 {
        return Err("event counter budget");
    }
    *value += 1;
    Ok(())
}
fn bounded(value: f64) -> bool {
    value.is_finite() && value.abs() <= MAX_NUMBER
}
fn push<T>(values: &mut Vec<T>, value: T) {
    if values.len() < MAX_SAMPLES {
        values.push(value);
    }
}

impl Recorder {
    /// Start an epoch using the browser monotonic clock.
    pub fn new(start: f64) -> Result<Self, &'static str> {
        if !bounded(start) || start < 0.0 {
            return Err("invalid epoch clock");
        }
        Ok(Self {
            start,
            moves: Vec::new(),
            downs: Vec::new(),
            ups: Vec::new(),
            times: Vec::new(),
            events: 0,
            untrusted: 0,
            no_move: 0,
            pastes: 0,
            last_move: 0.0,
            coalesced_total: 0.0,
            coalesced_samples: 0.0,
            clicks: 0,
        })
    }
    fn time(&self, value: f64) -> Result<(), &'static str> {
        if !bounded(value) || value < self.start {
            Err("invalid event clock")
        } else {
            Ok(())
        }
    }
    fn mark(&mut self, time: f64) -> Result<(), &'static str> {
        self.time(time)?;
        push(&mut self.times, time);
        Ok(())
    }
    fn event(&mut self, trusted: bool) -> Result<(), &'static str> {
        count(&mut self.events)?;
        if !trusted {
            count(&mut self.untrusted)?;
        }
        Ok(())
    }
    /// Record a pointermove with the oracle's separate sample/last-move/mark reads.
    pub fn pointer_move(
        &mut self,
        sample: PointerSample,
        last: f64,
        mark: f64,
        coalesced: f64,
        trusted: bool,
    ) -> Result<(), &'static str> {
        self.time(sample.time_ms)?;
        self.time(last)?;
        self.time(mark)?;
        if !bounded(sample.x) || !bounded(sample.y) || !bounded(coalesced) || coalesced < 1.0 {
            return Err("invalid pointer observation");
        }
        self.event(trusted)?;
        self.coalesced_total += coalesced;
        self.coalesced_samples += 1.0;
        if !self.coalesced_total.is_finite() {
            return Err("coalesced counter budget");
        }
        push(&mut self.moves, sample);
        self.last_move = last;
        self.mark(mark)
    }
    /// Record pointerdown; equality at800ms does not increment the no-move count.
    pub fn pointer_down(
        &mut self,
        check: f64,
        mark: f64,
        trusted: bool,
    ) -> Result<(), &'static str> {
        self.time(check)?;
        self.time(mark)?;
        self.event(trusted)?;
        count(&mut self.clicks)?;
        if check - self.last_move > 800.0 || self.moves.is_empty() {
            count(&mut self.no_move)?;
        }
        self.mark(mark)
    }
    /// Record a keydown timing without storing a physical key string.
    pub fn key_down(
        &mut self,
        sample: KeySample,
        mark: f64,
        trusted: bool,
    ) -> Result<(), &'static str> {
        self.time(sample.time_ms)?;
        self.time(mark)?;
        self.event(trusted)?;
        push(&mut self.downs, sample);
        self.mark(mark)
    }
    /// Keyup contributes timing only, not totalEvents or trust/cadence counts.
    pub fn key_up(&mut self, sample: KeySample) -> Result<(), &'static str> {
        self.time(sample.time_ms)?;
        push(&mut self.ups, sample);
        Ok(())
    }
    /// Wheel contributes an event/timestamp but retains the frozen trust omission.
    pub fn wheel(&mut self, mark: f64) -> Result<(), &'static str> {
        self.time(mark)?;
        count(&mut self.events)?;
        self.mark(mark)
    }
    /// Paste contributes only a counter; clipboard contents are never observed.
    pub fn paste(&mut self) -> Result<(), &'static str> {
        count(&mut self.pastes)
    }
    /// Reduce the epoch to the shared wire schema, exporting no samples/identities.
    pub fn summary(&self, end: f64) -> Result<BehaviorSummary, &'static str> {
        self.time(end)?;
        let mut mouse = pointer::reduce(&self.moves);
        if self.coalesced_samples > 0.0 {
            mouse.coalesced_ratio = self.coalesced_total / self.coalesced_samples;
        }
        let (long_gap_count, burst_var_ms, min_reaction_ms) = cadence::reduce(&self.times);
        Ok(BehaviorSummary {
            mouse,
            key: keyboard::reduce(&self.downs, &self.ups, self.pastes),
            events: EventFeatures {
                total_events: self.events,
                untrusted_frac: if self.events > 0 {
                    self.untrusted as f64 / self.events as f64
                } else {
                    0.0
                },
                no_move_clicks: self.no_move,
                synthetic_flags: 0,
                click_count: self.clicks,
                long_gap_count,
                burst_var_ms,
                min_reaction_ms,
            },
            duration_s: (end - self.start) / 1000.0,
        })
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod oracle_tests;
#[cfg(test)]
mod tests;
