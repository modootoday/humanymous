package gate

import (
	"sync"
	"time"
)

// recon.go defends the decision surface against model-extraction (SoT-21 §8,
// HR-30): a sweep detector that flags one client fingerprint spinning up many
// near-identical sessions to binary-search which signal trips a verdict, plus a
// constant-floor response delay so allow/challenge/deny cannot be told apart by
// timing.

// SweepDetector counts distinct sessions seen per client-fingerprint binding in
// a sliding window. One binding fanning out to many sessions rapidly is a
// decision-probing / signal-sweep recon pattern.
type SweepDetector struct {
	mu      sync.Mutex
	byBind  map[string]*bindWindow
	window  time.Duration
	maxSess int
}

type bindWindow struct {
	sessions map[string]time.Time
	first    time.Time
}

// NewSweepDetector flags a binding that spawns > maxSess sessions within window.
func NewSweepDetector(window time.Duration, maxSess int) *SweepDetector {
	return &SweepDetector{byBind: map[string]*bindWindow{}, window: window, maxSess: maxSess}
}

// GC evicts binding windows that have fully aged out. byBind is keyed by client
// binding (fingerprint/IP), so without eviction a churn of bindings grows it without
// bound (PLAN-08 deployment-review ship-blocker).
func (d *SweepDetector) GC(now time.Time) {
	d.mu.Lock()
	defer d.mu.Unlock()
	for k, bw := range d.byBind {
		if now.Sub(bw.first) > 2*d.window {
			delete(d.byBind, k)
		}
	}
}

// Observe records (bind, sid) and returns true if the binding is now sweeping.
func (d *SweepDetector) Observe(bind, sid string, now time.Time) bool {
	if bind == "" {
		return false
	}
	d.mu.Lock()
	defer d.mu.Unlock()
	bw := d.byBind[bind]
	if bw == nil {
		bw = &bindWindow{sessions: map[string]time.Time{}, first: now}
		d.byBind[bind] = bw
	}
	// True sliding window (deep-review): expire each session by its OWN last-seen age
	// against the window, so entries never accumulate across what used to be a tumbling
	// reset — the count reflects only the trailing `window`, matching the documented
	// semantics. The scan is bounded by the maxSess-scale fan-out. `first` tracks last
	// activity so an idle binding still ages out of GC.
	cut := now.Add(-d.window)
	for s, seen := range bw.sessions {
		if seen.Before(cut) {
			delete(bw.sessions, s)
		}
	}
	bw.sessions[sid] = now
	bw.first = now
	return len(bw.sessions) > d.maxSess
}

// constantFloor blocks until at least `floor` has elapsed since `start`, so the
// visible latency of allow/challenge/deny converges to a fixed floor and does
// not leak which decision path (extra crosschecks / PoW issuance) ran. This is
// coarse jitter defense, not a full constant-time guarantee (SoT-21 §8).
func constantFloor(start time.Time, floor time.Duration, now func() time.Time) {
	if d := floor - now().Sub(start); d > 0 {
		time.Sleep(d)
	}
}
