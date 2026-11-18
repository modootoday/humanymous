//! In-process broadcast hub for the observatory live feed (legacy
//! internal/livefeed/hub.go at baseline 981d25c).
//!
//! Publish never blocks: a full subscriber drops the event and counts it, so
//! a slow or wedged reader can never stall the detection path. The hub carries
//! detection telemetry only; bans, policy secrets, audit keys, RIT seeds and
//! bearer tokens never pass through it.
use serde::Serialize;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel},
    },
    time::Duration,
};

/// Fallback ring and per-subscriber buffer size (Go `New` with non-positive input).
pub const DEFAULT_CAPACITY: usize = 256;

/// One broadcast item; `id` is the SSE `id:` so a reconnecting reader can replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Event {
    /// Monotonic per-hub id, starting at 1.
    pub id: u64,
    /// SSE event name.
    #[serde(rename = "type")]
    pub kind: String,
    /// Marshalled JSON payload.
    #[serde(serialize_with = "raw_json")]
    pub data: String,
}

fn raw_json<S: serde::Serializer>(data: &str, serializer: S) -> Result<S::Ok, S::Error> {
    let value: serde_json::Value = serde_json::from_str(data).map_err(serde::ser::Error::custom)?;
    value.serialize(serializer)
}

struct Subscriber {
    sender: SyncSender<Event>,
    lagged: Arc<AtomicU64>,
}

struct State {
    next_id: u64,
    ring: VecDeque<Event>,
    subscribers: BTreeMap<u64, Subscriber>,
    next_subscriber: u64,
}

/// Fan-out broadcaster with a bounded replay ring.
pub struct Hub {
    state: Mutex<State>,
    ring_capacity: usize,
    buffer: usize,
}

impl Hub {
    /// Hub replaying up to `ring_capacity` events with per-subscriber buffers of
    /// `buffer` events; zero falls back to [`DEFAULT_CAPACITY`].
    pub fn new(ring_capacity: usize, buffer: usize) -> Arc<Self> {
        let fallback = |value: usize| if value == 0 { DEFAULT_CAPACITY } else { value };
        Arc::new(Self {
            state: Mutex::new(State {
                next_id: 0,
                ring: VecDeque::new(),
                subscribers: BTreeMap::new(),
                next_subscriber: 0,
            }),
            ring_capacity: fallback(ring_capacity),
            buffer: fallback(buffer),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Marshal `payload`, assign the next id, keep it in the ring and offer it
    /// to every subscriber without blocking. A payload that cannot be encoded
    /// is published as `{}` (Go parity).
    pub fn publish(&self, kind: &str, payload: &impl Serialize) -> Event {
        let data = serde_json::to_string(payload).unwrap_or_else(|_| "{}".into());
        let mut state = self.lock();
        state.next_id += 1;
        let event = Event {
            id: state.next_id,
            kind: kind.into(),
            data,
        };
        state.ring.push_back(event.clone());
        while state.ring.len() > self.ring_capacity {
            state.ring.pop_front();
        }
        state.subscribers.retain(
            |_, subscriber| match subscriber.sender.try_send(event.clone()) {
                Ok(()) => true,
                Err(TrySendError::Full(_)) => {
                    subscriber.lagged.fetch_add(1, Ordering::Relaxed);
                    true
                }
                Err(TrySendError::Disconnected(_)) => false,
            },
        );
        event
    }

    /// Register a subscriber. The backlog of ring events newer than `last_id`
    /// is taken under the same lock as registration, so no event is both in the
    /// backlog and the stream, and none falls between them.
    pub fn subscribe(self: &Arc<Self>, last_id: u64) -> (Vec<Event>, Subscription) {
        let mut state = self.lock();
        let backlog = state
            .ring
            .iter()
            .filter(|event| event.id > last_id)
            .cloned()
            .collect();
        let (sender, receiver) = sync_channel(self.buffer);
        let lagged = Arc::new(AtomicU64::new(0));
        let key = state.next_subscriber;
        state.next_subscriber += 1;
        state.subscribers.insert(
            key,
            Subscriber {
                sender,
                lagged: lagged.clone(),
            },
        );
        (
            backlog,
            Subscription {
                hub: self.clone(),
                key,
                receiver,
                lagged,
            },
        )
    }

    /// Current subscriber count.
    pub fn subscribers(&self) -> usize {
        self.lock().subscribers.len()
    }

    /// Id of the most recently published event, zero when none.
    pub fn last_id(&self) -> u64 {
        self.lock().next_id
    }
}

/// A live subscription; dropping or cancelling it unregisters the reader.
pub struct Subscription {
    hub: Arc<Hub>,
    key: u64,
    receiver: Receiver<Event>,
    lagged: Arc<AtomicU64>,
}

impl Subscription {
    /// Next event, `None` once cancelled and drained.
    pub fn recv(&self) -> Option<Event> {
        self.receiver.recv().ok()
    }
    /// Next event within `timeout`; `Err(true)` when cancelled, `Err(false)` on timeout.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Event, bool> {
        self.receiver
            .recv_timeout(timeout)
            .map_err(|error| matches!(error, RecvTimeoutError::Disconnected))
    }
    /// A pending event, if any.
    pub fn try_recv(&self) -> Option<Event> {
        self.receiver.try_recv().ok()
    }
    /// Events dropped because this reader's buffer was full.
    pub fn lagged(&self) -> u64 {
        self.lagged.load(Ordering::Relaxed)
    }
    /// Unregister; buffered events stay readable, then the stream ends.
    pub fn cancel(&self) {
        self.hub.lock().subscribers.remove(&self.key);
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.cancel();
    }
}
