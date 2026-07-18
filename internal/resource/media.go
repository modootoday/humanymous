package resource

import (
	"sync"
	"time"

	"github.com/modootoday/humanymous/internal/signals"
)

// media.go detects media bandwidth-abuse patterns (SoT-10 §3.2): parallel Range
// storms, hotlinking, and RIT-less media fetches. A per-session sliding window
// counts recent range requests; thresholds are conservative to avoid flagging
// normal seek/preload (SoT-10 §4 FP note).

// MediaEvent is one media request observed by the server.
type MediaEvent struct {
	SessionID   string
	IsRange     bool
	ExternalRef bool // Referer host != our host
	HasRIT      bool
	At          time.Time
}

// MediaTracker counts recent range requests per session.
type MediaTracker struct {
	mu     sync.Mutex
	window time.Duration
	storm  int // range requests in window to call it a storm
	hist   map[string][]time.Time
}

// NewMediaTracker returns a tracker (default: 8 range reqs / 3s = storm).
func NewMediaTracker() *MediaTracker {
	return &MediaTracker{window: 3 * time.Second, storm: 8, hist: map[string][]time.Time{}}
}

// Observe records an event and returns the media signals it triggers.
func (m *MediaTracker) Observe(e MediaEvent) []signals.Signal {
	var out []signals.Signal
	add := func(id string, val any, v signals.Verdict, notes string) {
		out = append(out, signals.New(id, val, v, 1.0, signals.SourceServer, notes))
	}

	if e.IsRange {
		if n := m.recordRange(e.SessionID, e.At); n >= m.storm {
			add("l5.media.range_storm", n, signals.VerdictBot, "parallel Range request storm")
		}
	}
	if e.ExternalRef {
		add("l5.media.hotlink", true, signals.VerdictSuspicious, "external referer media fetch")
	}
	if !e.HasRIT {
		add("l5.resource.rit_absent", true, signals.VerdictBot, "media fetch without RIT token")
	}
	return out
}

// recordRange appends a timestamp and returns the count within the window.
func (m *MediaTracker) recordRange(sid string, at time.Time) int {
	m.mu.Lock()
	defer m.mu.Unlock()
	cut := at.Add(-m.window)
	h := m.hist[sid]
	kept := h[:0]
	for _, t := range h {
		if t.After(cut) {
			kept = append(kept, t)
		}
	}
	kept = append(kept, at)
	m.hist[sid] = kept
	return len(kept)
}
