// Package livefeed provides an in-process broadcast hub that fans events out to
// SSE subscribers (SoT-30 §4, the Detection Observatory). Publish is
// NON-BLOCKING (drop-on-full) so a slow or wedged subscriber can never stall the
// caller — the detection serving path (/api/collect) stays unaffected. The hub
// carries detection TELEMETRY only: no bans, policy, audit-chain keys, RIT seeds,
// or bearer tokens ever pass through it.
package livefeed

import (
	"encoding/json"
	"sync"
	"sync/atomic"
)

// Event is one broadcast item. Data is the marshaled payload; ID is a monotonic
// per-hub counter used as the SSE `id:` so a reconnecting EventSource can replay
// from Last-Event-ID.
type Event struct {
	ID   uint64          `json:"id"`
	Type string          `json:"type"`
	Data json.RawMessage `json:"data"`
}

type subscriber struct {
	ch     chan Event
	lagged uint64 // count of events dropped because this subscriber was full
}

// Hub is a fan-out broadcaster with a bounded replay ring.
type Hub struct {
	mu      sync.Mutex
	nextID  uint64
	ring    []Event
	ringCap int
	subs    map[*subscriber]struct{}
	subBuf  int
}

// New builds a Hub with a replay ring of ringCap events and per-subscriber
// buffers of subBuf events. Non-positive values fall back to 256.
func New(ringCap, subBuf int) *Hub {
	if ringCap <= 0 {
		ringCap = 256
	}
	if subBuf <= 0 {
		subBuf = 256
	}
	return &Hub{ringCap: ringCap, subBuf: subBuf, subs: map[*subscriber]struct{}{}}
}

// Publish marshals payload, assigns the next id, appends to the replay ring, and
// non-blocking-sends to every subscriber (dropping + counting for any whose
// buffer is full). It NEVER blocks. Returns the published event.
func (h *Hub) Publish(typ string, payload any) Event {
	data, err := json.Marshal(payload)
	if err != nil {
		data = []byte("{}")
	}
	h.mu.Lock()
	defer h.mu.Unlock()
	h.nextID++
	e := Event{ID: h.nextID, Type: typ, Data: data}
	h.ring = append(h.ring, e)
	if len(h.ring) > h.ringCap {
		h.ring = h.ring[len(h.ring)-h.ringCap:]
	}
	for s := range h.subs {
		select {
		case s.ch <- e:
		default:
			atomic.AddUint64(&s.lagged, 1)
		}
	}
	return e
}

// Subscribe registers a subscriber and returns (a) the backlog of ring events
// newer than lastID for immediate replay, (b) a channel of future events, and
// (c) a cancel func the caller MUST call when done. Registration and the backlog
// snapshot happen under one lock, so no event is ever both in the backlog and the
// channel, and none is lost in the gap between them.
func (h *Hub) Subscribe(lastID uint64) (backlog []Event, ch <-chan Event, cancel func()) {
	h.mu.Lock()
	defer h.mu.Unlock()
	for _, e := range h.ring {
		if e.ID > lastID {
			backlog = append(backlog, e)
		}
	}
	s := &subscriber{ch: make(chan Event, h.subBuf)}
	h.subs[s] = struct{}{}
	return backlog, s.ch, func() {
		h.mu.Lock()
		if _, ok := h.subs[s]; ok {
			delete(h.subs, s)
			close(s.ch)
		}
		h.mu.Unlock()
	}
}

// Subscribers returns the current subscriber count.
func (h *Hub) Subscribers() int {
	h.mu.Lock()
	defer h.mu.Unlock()
	return len(h.subs)
}

// LastID returns the id of the most recently published event (0 if none).
func (h *Hub) LastID() uint64 {
	h.mu.Lock()
	defer h.mu.Unlock()
	return h.nextID
}
