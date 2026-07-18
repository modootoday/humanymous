package audit

import (
	"testing"
	"time"
)

// WS8: a registered node that stops heartbeating is flagged as suppression-
// suspected once it misses its window; a live node is not.
func TestNodeSuppressionDetected(t *testing.T) {
	reg := NewNodeRegistry()
	t0 := time.Unix(1000, 0)
	reg.Expect("proxy-1", time.Second, nil, t0)
	reg.Expect("proxy-2", time.Second, nil, t0)

	// Both heartbeat at t0+1s.
	reg.Heartbeat("proxy-1", t0.Add(time.Second))
	reg.Heartbeat("proxy-2", t0.Add(time.Second))
	if m := reg.Missing(t0.Add(time.Second)); len(m) != 0 {
		t.Fatalf("no node should be missing yet: %v", m)
	}

	// proxy-2 is silenced (stops heartbeating); proxy-1 keeps going.
	reg.Heartbeat("proxy-1", t0.Add(3*time.Second))
	missing := reg.Missing(t0.Add(3500 * time.Millisecond)) // > 2× interval since proxy-2's last
	if len(missing) != 1 || missing[0] != "proxy-2" {
		t.Fatalf("proxy-2 suppression should be detected, got %v", missing)
	}

	// A resumed heartbeat clears the alert.
	reg.Heartbeat("proxy-2", t0.Add(4*time.Second))
	if m := reg.Missing(t0.Add(4*time.Second + 500*time.Millisecond)); len(m) != 0 {
		t.Fatalf("resumed node must clear: %v", m)
	}
}
