// Package abuse implements fingerprint-keyed rate limiting and flood detection
// (SoT-17). Destructive attacks — request floods, credential stuffing, media
// range-storms, distributed low-and-slow — must be metered by a STABLE
// fingerprint (JA4 + device fingerprint), not by IP, because residential-proxy
// pools rotate IPs while the fingerprint stays fixed (SoT-15). A per-IP limiter
// is trivially defeated; a per-fingerprint limiter is not.
package abuse

import (
	"sync"
	"time"
)

// Limiter is a concurrent sliding-window counter keyed by an arbitrary string
// (the caller passes fingerprint|subnet). It reports the current rate and
// whether a soft/hard threshold is exceeded.
type Limiter struct {
	mu     sync.Mutex
	window time.Duration
	soft   int // requests/window -> SUSPICIOUS
	hard   int // requests/window -> BOT/flood
	hits   map[string][]int64 // key -> unix-nano timestamps in window
	ttl    time.Duration
	lastGC time.Time
}

// NewLimiter returns a limiter. Defaults suit a demo: 60 req / 10s soft,
// 120 req / 10s hard. Production tunes per endpoint.
func NewLimiter(window time.Duration, soft, hard int) *Limiter {
	return &Limiter{
		window: window,
		soft:   soft,
		hard:   hard,
		hits:   map[string][]int64{},
		ttl:    window * 6,
	}
}

// Observe records one request for key at time now and returns the rolling count
// within the window.
func (l *Limiter) Observe(key string, now time.Time) int {
	if key == "" {
		return 0
	}
	cut := now.Add(-l.window).UnixNano()
	l.mu.Lock()
	defer l.mu.Unlock()
	h := l.hits[key]
	kept := h[:0]
	for _, t := range h {
		if t > cut {
			kept = append(kept, t)
		}
	}
	kept = append(kept, now.UnixNano())
	l.hits[key] = kept
	return len(kept)
}

// Level classifies a rolling count: 0 ok, 1 soft (suspicious), 2 hard (flood).
func (l *Limiter) Level(count int) int {
	switch {
	case count >= l.hard:
		return 2
	case count >= l.soft:
		return 1
	default:
		return 0
	}
}

// GC evicts keys with no recent hits.
func (l *Limiter) GC(now time.Time) {
	cut := now.Add(-l.ttl).UnixNano()
	l.mu.Lock()
	defer l.mu.Unlock()
	for k, h := range l.hits {
		if len(h) == 0 || h[len(h)-1] < cut {
			delete(l.hits, k)
		}
	}
}

// Reset clears all rate-limit state. Used by the Detection Observatory to
// isolate a launched run so a flood cannot poison the shared fingerprint-keyed
// counters that an organic baseline would then inherit (SoT-30 §10).
func (l *Limiter) Reset() {
	l.mu.Lock()
	l.hits = map[string][]int64{}
	l.mu.Unlock()
}
