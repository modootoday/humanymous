package livefeed

import (
	"encoding/json"
	"sync"
	"testing"
)

// A subscriber receives events published after it subscribed, in order.
func TestPublishDelivers(t *testing.T) {
	h := New(16, 16)
	_, ch, cancel := h.Subscribe(0)
	defer cancel()
	h.Publish("session.scored", map[string]any{"n": 1})
	h.Publish("session.scored", map[string]any{"n": 2})

	got := []uint64{}
	for i := 0; i < 2; i++ {
		e := <-ch
		got = append(got, e.ID)
	}
	if got[0] != 1 || got[1] != 2 {
		t.Fatalf("out-of-order or wrong ids: %v", got)
	}
}

// Publish is non-blocking: a full subscriber buffer drops events and increments
// the lagged counter rather than stalling the publisher.
func TestPublishNonBlockingDrops(t *testing.T) {
	h := New(1024, 2) // tiny per-subscriber buffer
	var sub *subscriber
	_, _, cancel := h.Subscribe(0)
	defer cancel()
	// Grab the single subscriber to inspect its lagged counter.
	h.mu.Lock()
	for s := range h.subs {
		sub = s
	}
	h.mu.Unlock()

	// Publish far more than the buffer holds; must not block or panic.
	done := make(chan struct{})
	go func() {
		for i := 0; i < 100; i++ {
			h.Publish("x", map[string]int{"i": i})
		}
		close(done)
	}()
	<-done // if Publish blocked, the test would hang here

	if sub.lagged == 0 {
		t.Fatal("expected drops (lagged>0) with a full buffer")
	}
	if got := int(sub.lagged) + 2; got != 100 { // 2 buffered + dropped == 100
		t.Fatalf("accounting off: buffered 2 + lagged %d != 100 (got %d)", sub.lagged, got)
	}
}

// A late subscriber replays the ring backlog newer than its Last-Event-ID, and
// the ring is bounded to ringCap.
func TestBacklogReplayAndRingCap(t *testing.T) {
	h := New(4, 16) // ring holds only the last 4
	for i := 0; i < 10; i++ {
		h.Publish("x", map[string]int{"i": i})
	}
	// New subscriber from id 0 should only see the last 4 (ids 7,8,9,10).
	backlog, _, cancel := h.Subscribe(0)
	defer cancel()
	if len(backlog) != 4 {
		t.Fatalf("ring not capped: got %d backlog events, want 4", len(backlog))
	}
	if backlog[0].ID != 7 || backlog[3].ID != 10 {
		t.Fatalf("wrong backlog window: %d..%d", backlog[0].ID, backlog[3].ID)
	}
	// Last-Event-ID=8 should replay only 9 and 10.
	b2, _, c2 := h.Subscribe(8)
	defer c2()
	if len(b2) != 2 || b2[0].ID != 9 || b2[1].ID != 10 {
		t.Fatalf("Last-Event-ID replay wrong: %+v", b2)
	}
}

// Registration + backlog snapshot are atomic: an event is never both in the
// backlog and the channel (no duplication, no loss) under concurrency.
func TestNoDuplicationAcrossSubscribe(t *testing.T) {
	h := New(256, 256)
	var wg sync.WaitGroup
	// Concurrent publishers.
	for p := 0; p < 4; p++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for i := 0; i < 50; i++ {
				h.Publish("x", map[string]int{"i": i})
			}
		}()
	}
	// Subscribe mid-flight; collect backlog + channel and assert no id appears twice.
	backlog, ch, cancel := h.Subscribe(0)
	seen := map[uint64]bool{}
	for _, e := range backlog {
		if seen[e.ID] {
			t.Fatalf("duplicate id %d in backlog", e.ID)
		}
		seen[e.ID] = true
	}
	wg.Wait()
	cancel() // closes the channel so the drain loop terminates
	for e := range ch {
		if seen[e.ID] {
			t.Fatalf("id %d in BOTH backlog and channel", e.ID)
		}
		seen[e.ID] = true
	}
}

// cancel deregisters the subscriber so later publishes neither reach nor panic on
// the closed channel.
func TestCancelStopsDelivery(t *testing.T) {
	h := New(16, 16)
	_, ch, cancel := h.Subscribe(0)
	if h.Subscribers() != 1 {
		t.Fatalf("want 1 subscriber, got %d", h.Subscribers())
	}
	cancel()
	if h.Subscribers() != 0 {
		t.Fatalf("cancel did not deregister: %d left", h.Subscribers())
	}
	// Must not panic (no send on closed channel).
	h.Publish("x", map[string]int{"i": 1})
	// Channel is closed and drained.
	if _, ok := <-ch; ok {
		t.Fatal("expected closed channel after cancel")
	}
	// cancel is idempotent.
	cancel()
}

// The published event carries a valid JSON payload under its type.
func TestEventPayloadShape(t *testing.T) {
	h := New(8, 8)
	e := h.Publish("session.scored", map[string]any{"verdict": "DENY", "risk": 67})
	if e.Type != "session.scored" || e.ID != 1 {
		t.Fatalf("bad envelope: %+v", e)
	}
	var got map[string]any
	if err := json.Unmarshal(e.Data, &got); err != nil {
		t.Fatalf("payload not JSON: %v", err)
	}
	if got["verdict"] != "DENY" {
		t.Fatalf("payload lost fields: %v", got)
	}
}
