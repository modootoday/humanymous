//! Opt-in bounded asynchronous operational logging (legacy internal/logger
//! logger.go and handler.go at baseline 981d25c).
//!
//! The gateway keeps the synchronous [`OperationalLogger`]; the owner
//! accepted that difference on 2026-11-16. A composition that must not block
//! on a slow sink can move its logger into a [`QueuedLogger`] instead. Then,
//! as in Go:
//! - a producer never waits for a sink: when the bounded queue is full or the
//!   logger is closed the record is dropped and counted per level;
//! - one worker thread owns every sink and assigns sequence numbers in queue
//!   order;
//! - flush waits until every record accepted before it has reached every sink
//!   and file sinks are synced, close stops intake and drains the queue, and
//!   both give up at a caller-supplied deadline;
//! - [`LogHandle::with_attrs`] and [`LogHandle::with_group`] nest attributes
//!   exactly like the Go `slog.Handler`.
//!
//! Record time stays caller-supplied, and the deadline is an [`Instant`] the
//! caller chose. Dropping the [`QueuedLogger`] closes intake without waiting.
use super::record::{Attr, Record, sanitize_key};
use super::{Identity, Level, OperationalLogger, SinkStats};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Instant;

/// Queue capacity used when the caller passes zero, as Go did.
pub const DEFAULT_QUEUE_SIZE: usize = 1024;
/// Largest accepted queue capacity.
pub const MAX_QUEUE_SIZE: usize = 65_536;

/// Static refusal to start a queued logger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueRefusal {
    /// Capacity exceeds [`MAX_QUEUE_SIZE`].
    QueueSize,
    /// The worker thread could not be started.
    Worker,
}

impl QueueRefusal {
    /// Stable code for startup diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::QueueSize => "log_queue_size",
            Self::Worker => "log_queue_worker",
        }
    }
}

/// Flush or close did not complete before its deadline. Records already
/// queued stay queued; nothing is discarded by the timeout itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineExceeded;

/// Dropped records per level; no dynamic labels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LevelCounts {
    /// Debug records.
    pub debug: u64,
    /// Info records.
    pub info: u64,
    /// Warn records.
    pub warn: u64,
    /// Error records.
    pub error: u64,
}

/// Drops split into Go's two producer-side reasons.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DropStats {
    /// The queue was at capacity.
    pub queue_full: LevelCounts,
    /// The logger was already closed.
    pub closed: LevelCounts,
}

/// Race-safe bounded snapshot; no dynamic error text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueStats {
    /// Items waiting for the worker.
    pub queue_depth: usize,
    /// Producer-side drops.
    pub dropped: DropStats,
    /// Sum of the per-sink write errors.
    pub write_errors: u64,
    /// Per-sink snapshot as of the last item the worker finished.
    pub sinks: Vec<SinkStats>,
}

#[derive(Default)]
struct LevelCounters([AtomicU64; 4]);

impl LevelCounters {
    fn add(&self, level: Level) {
        let index = match level {
            Level::Debug => 0,
            Level::Info => 1,
            Level::Warn => 2,
            Level::Error => 3,
        };
        self.0[index].fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> LevelCounts {
        let [debug, info, warn, error] = &self.0;
        LevelCounts {
            debug: debug.load(Ordering::Relaxed),
            info: info.load(Ordering::Relaxed),
            warn: warn.load(Ordering::Relaxed),
            error: error.load(Ordering::Relaxed),
        }
    }
}

/// One-shot completion signal with a deadline-bounded wait.
#[derive(Default)]
struct Signal {
    set: Mutex<bool>,
    changed: Condvar,
}

impl Signal {
    fn set(&self) {
        *lock(&self.set) = true;
        self.changed.notify_all();
    }

    fn wait_until(&self, deadline: Instant) -> bool {
        let mut set = lock(&self.set);
        while !*set {
            let Some(remaining) = deadline
                .checked_duration_since(Instant::now())
                .filter(|left| !left.is_zero())
            else {
                return false;
            };
            set = self
                .changed
                .wait_timeout(set, remaining)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        true
    }
}

enum Item {
    Record(Box<Record>),
    Flush(Arc<Signal>),
}

#[derive(Default)]
struct Queue {
    items: VecDeque<Item>,
    closed: bool,
}

struct Shared {
    identity: Identity,
    threshold: Option<Level>,
    capacity: usize,
    queue: Mutex<Queue>,
    /// Worker waits here for an item or for close.
    ready: Condvar,
    /// Flush waits here for queue room.
    room: Condvar,
    queue_full: LevelCounters,
    closed: LevelCounters,
    sinks: Mutex<Vec<SinkStats>>,
    done: Signal,
}

impl Shared {
    fn close(&self) {
        lock(&self.queue).closed = true;
        self.ready.notify_all();
        self.room.notify_all();
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Marks the worker finished even if a caller-supplied writer panics, so
/// producers count closed drops and close/flush stop waiting.
struct WorkerExit(Arc<Shared>);

impl Drop for WorkerExit {
    fn drop(&mut self) {
        self.0.close();
        self.0.done.set();
    }
}

fn run(shared: &Shared, mut logger: OperationalLogger) {
    loop {
        let item = {
            let mut queue = lock(&shared.queue);
            loop {
                if let Some(item) = queue.items.pop_front() {
                    break Some(item);
                }
                if queue.closed {
                    break None;
                }
                queue = shared
                    .ready
                    .wait(queue)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        };
        shared.room.notify_all();
        match item {
            None => break,
            Some(Item::Record(record)) => {
                logger.write_record(*record);
                *lock(&shared.sinks) = logger.stats();
            }
            Some(Item::Flush(ack)) => {
                logger.flush();
                *lock(&shared.sinks) = logger.stats();
                ack.set();
            }
        }
    }
    logger.flush();
    *lock(&shared.sinks) = logger.stats();
    // Dropping the logger closes its file sinks.
}

/// Owner of the worker: flush, close and stats.
pub struct QueuedLogger {
    handle: LogHandle,
}

impl QueuedLogger {
    /// Move `logger` onto one worker thread behind a queue of `queue_size`
    /// items (zero selects [`DEFAULT_QUEUE_SIZE`]).
    pub fn start(logger: OperationalLogger, queue_size: usize) -> Result<Self, QueueRefusal> {
        let capacity = match queue_size {
            0 => DEFAULT_QUEUE_SIZE,
            size if size > MAX_QUEUE_SIZE => return Err(QueueRefusal::QueueSize),
            size => size,
        };
        let shared = Arc::new(Shared {
            identity: logger.identity.clone(),
            threshold: logger.threshold,
            capacity,
            queue: Mutex::new(Queue::default()),
            ready: Condvar::new(),
            room: Condvar::new(),
            queue_full: LevelCounters::default(),
            closed: LevelCounters::default(),
            sinks: Mutex::new(logger.stats()),
            done: Signal::default(),
        });
        let exit = WorkerExit(Arc::clone(&shared));
        thread::Builder::new()
            .name("hmn-oplog".to_owned())
            .spawn(move || {
                let exit = exit;
                run(&exit.0, logger);
            })
            .map_err(|_| QueueRefusal::Worker)?;
        Ok(Self {
            handle: LogHandle {
                shared,
                attrs: Vec::new(),
                groups: Vec::new(),
            },
        })
    }

    /// Producer handle without bound attributes or groups.
    pub fn handle(&self) -> LogHandle {
        self.handle.clone()
    }

    /// Wait until every record accepted before this call has reached every
    /// sink and file sinks are synced. After close, wait for the drain.
    pub fn flush(&self, deadline: Instant) -> Result<(), DeadlineExceeded> {
        let shared = &self.handle.shared;
        let ack = Arc::new(Signal::default());
        {
            let mut queue = lock(&shared.queue);
            loop {
                if queue.closed {
                    drop(queue);
                    return self.wait_done(deadline);
                }
                if queue.items.len() < shared.capacity {
                    queue.items.push_back(Item::Flush(Arc::clone(&ack)));
                    break;
                }
                let Some(remaining) = deadline
                    .checked_duration_since(Instant::now())
                    .filter(|left| !left.is_zero())
                else {
                    return Err(DeadlineExceeded);
                };
                queue = shared
                    .room
                    .wait_timeout(queue, remaining)
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
            }
        }
        shared.ready.notify_one();
        if ack.wait_until(deadline) {
            Ok(())
        } else {
            Err(DeadlineExceeded)
        }
    }

    /// Stop accepting records and wait for the worker to drain the queue,
    /// sync and close every sink. Idempotent.
    pub fn close(&self, deadline: Instant) -> Result<(), DeadlineExceeded> {
        self.handle.shared.close();
        self.wait_done(deadline)
    }

    fn wait_done(&self, deadline: Instant) -> Result<(), DeadlineExceeded> {
        if self.handle.shared.done.wait_until(deadline) {
            Ok(())
        } else {
            Err(DeadlineExceeded)
        }
    }

    /// Race-safe bounded snapshot.
    pub fn stats(&self) -> QueueStats {
        let shared = &self.handle.shared;
        let queue_depth = lock(&shared.queue).items.len();
        let sinks = lock(&shared.sinks).clone();
        QueueStats {
            queue_depth,
            dropped: DropStats {
                queue_full: shared.queue_full.snapshot(),
                closed: shared.closed.snapshot(),
            },
            write_errors: sinks
                .iter()
                .fold(0_u64, |sum, sink| sum.saturating_add(sink.write_errors)),
            sinks,
        }
    }
}

impl Drop for QueuedLogger {
    fn drop(&mut self) {
        self.handle.shared.close();
    }
}

/// Cloneable producer, the counterpart of Go's `slog.Handler`.
#[derive(Clone)]
pub struct LogHandle {
    shared: Arc<Shared>,
    /// Bound attributes, keys already prefixed with the groups open when
    /// they were bound.
    attrs: Vec<Attr>,
    /// Sanitized, non-empty group names.
    groups: Vec<String>,
}

impl LogHandle {
    /// Whether a record at `level` passes the threshold.
    pub fn enabled(&self, level: Level) -> bool {
        self.shared
            .threshold
            .is_some_and(|threshold| level >= threshold)
    }

    /// Handle that adds `attrs` to every record, under the open groups.
    pub fn with_attrs(&self, attrs: &[Attr]) -> Self {
        let prefix = self.groups.join(".");
        let mut clone = self.clone();
        clone.attrs.extend(attrs.iter().map(|attr| {
            let mut attr = attr.clone();
            if !prefix.is_empty() {
                attr.key = format!("{prefix}.{}", attr.key);
            }
            attr
        }));
        clone
    }

    /// Handle whose record attributes nest under `name`; a name that
    /// sanitizes to empty opens no group.
    pub fn with_group(&self, name: &str) -> Self {
        let mut clone = self.clone();
        let name = sanitize_key(name);
        if !name.is_empty() {
            clone.groups.push(name);
        }
        clone
    }

    /// Build the record on this thread and enqueue it without waiting. A full
    /// queue or a closed logger drops it and counts the drop by level.
    pub fn emit(&self, at: std::time::Duration, level: Level, message: &str, attrs: &[Attr]) {
        if !self.enabled(level) {
            return;
        }
        let shared = &self.shared;
        let mut record = Record::envelope(at, level, shared.identity.parts(), message);
        for attr in &self.attrs {
            record.add_attr("", attr);
        }
        let prefix = self.groups.join(".");
        for attr in attrs {
            record.add_attr(&prefix, attr);
        }
        {
            let mut queue = lock(&shared.queue);
            if queue.closed {
                shared.closed.add(level);
                return;
            }
            if queue.items.len() >= shared.capacity {
                shared.queue_full.add(level);
                return;
            }
            queue.items.push_back(Item::Record(Box::new(record)));
        }
        shared.ready.notify_one();
    }
}

#[cfg(test)]
mod tests;
