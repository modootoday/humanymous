package gate

import (
	"sync"
	"time"

	"github.com/modootoday/humanymous/internal/abuse"
)

// ban.go implements rate-limit-triggered progressive bans and manual ban
// management (SoT-27). An abusive source (keyed by IP and/or stable fingerprint)
// that exceeds the hard rate threshold is auto-banned for an escalating duration
// (1h → 6h → 24h → permanent on repeat strikes). Bans are checked at the very
// front of the request path so an abuser is dropped before the scorer or an
// upstream connection is touched.

// BanEntry is one active or historical ban.
type BanEntry struct {
	Key      string // "ip:1.2.3.4" or "fp:<hash>"
	Reason   string
	Source   string // "auto" | "manual"
	By       string // operator pseudonym (manual)
	Incident string
	Created  time.Time
	Until    time.Time // zero value => permanent
	Strike   int
}

// Permanent reports whether the ban never expires.
func (b BanEntry) Permanent() bool { return b.Until.IsZero() }

// active reports whether the ban is in force at now.
func (b BanEntry) active(now time.Time) bool { return b.Permanent() || now.Before(b.Until) }

// escalation maps a strike count to an auto-ban duration (SoT-27 §2). A zero
// duration means permanent.
func escalation(strike int) time.Duration {
	switch {
	case strike <= 1:
		return time.Hour
	case strike == 2:
		return 6 * time.Hour
	case strike == 3:
		return 24 * time.Hour
	default:
		return 0 // permanent
	}
}

// strikeDecay is how long a strike is remembered after a ban expires, so a
// repeat abuser escalates quickly (SoT-27 §2).
const strikeDecay = 7 * 24 * time.Hour

// BanStore holds active bans + strike history + the IP/fingerprint rate limiter.
type BanStore struct {
	mu      sync.Mutex
	bans    map[string]BanEntry
	strikes map[string]strikeRec
	limiter RateLimiter // in-memory (abuse.Limiter) or a shared Redis counter (PLAN-08 R1)
	nowFn   func() time.Time
}

type strikeRec struct {
	count int
	last  time.Time
}

// NewBanStore builds a ban store with an in-memory (per-node) rate limiter.
// window/soft/hard configure the limiter that drives auto-bans (SoT-27 §2).
func NewBanStore(window time.Duration, soft, hard int) *BanStore {
	return NewBanStoreWithLimiter(abuse.NewLimiter(window, soft, hard))
}

// NewBanStoreWithLimiter builds a ban store over an injected rate limiter, so a
// shared (Redis) counter can drive aggregate-across-nodes auto-bans (PLAN-08 R1).
func NewBanStoreWithLimiter(rl RateLimiter) *BanStore {
	return &BanStore{
		bans:    map[string]BanEntry{},
		strikes: map[string]strikeRec{},
		limiter: rl,
		nowFn:   time.Now,
	}
}

// Check returns the active ban for a key, expiring it lazily. ok=false means the
// key is not (or no longer) banned.
func (s *BanStore) Check(key string) (BanEntry, bool) {
	now := s.nowFn()
	s.mu.Lock()
	defer s.mu.Unlock()
	b, ok := s.bans[key]
	if !ok {
		return BanEntry{}, false
	}
	if !b.active(now) {
		delete(s.bans, key) // lazy expire (caller audits ban.expired)
		return BanEntry{}, false
	}
	return b, true
}

// Observe records a request against a key's rate limiter and, on a HARD breach,
// applies an escalating auto-ban. It returns (entry, banned, level) where level
// is 0 ok / 1 soft / 2 hard. When banned is true the caller enforces + audits.
func (s *BanStore) Observe(key string) (BanEntry, bool, int) {
	now := s.nowFn()
	count := s.limiter.Observe(key, now)
	level := s.limiter.Level(count)
	if level < 2 {
		return BanEntry{}, false, level
	}
	// Hard breach -> escalating auto-ban.
	s.mu.Lock()
	defer s.mu.Unlock()
	if b, ok := s.bans[key]; ok && b.active(now) {
		return b, true, level // already banned
	}
	st := s.strikes[key]
	if now.Sub(st.last) > strikeDecay {
		st.count = 0
	}
	st.count++
	st.last = now
	s.strikes[key] = st
	entry := s.applyLocked(key, "rate limit exceeded (auto)", "auto", "", "", escalation(st.count), st.count, now)
	return entry, true, level
}

// Add applies a manual ban (SoT-27 §4). dur == 0 => permanent.
func (s *BanStore) Add(key, reason, by, incident string, dur time.Duration) BanEntry {
	now := s.nowFn()
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.applyLocked(key, reason, "manual", by, incident, dur, 0, now)
}

// applyLocked writes a ban entry. Caller holds s.mu.
func (s *BanStore) applyLocked(key, reason, source, by, incident string, dur time.Duration, strike int, now time.Time) BanEntry {
	e := BanEntry{Key: key, Reason: reason, Source: source, By: by, Incident: incident, Created: now, Strike: strike}
	if dur > 0 {
		e.Until = now.Add(dur)
	} // else permanent (zero Until)
	s.bans[key] = e
	return e
}

// Lift removes a ban (manual unblock, SoT-27 §4). Returns true if one existed.
func (s *BanStore) Lift(key string) bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	_, ok := s.bans[key]
	delete(s.bans, key)
	return ok
}

// remainingSecs returns seconds until a temporary ban expires (0 for permanent/past).
// It lives on the entry (not the store) so the enforcement path can compute a
// Retry-After without reaching through the store — keeping the BanLedger seam (R18)
// limited to the operations a shared implementation must actually provide.
func (b BanEntry) remainingSecs(now time.Time) float64 {
	if b.Permanent() {
		return 0
	}
	d := b.Until.Sub(now).Seconds()
	if d < 0 {
		return 0
	}
	return d
}

// GC evicts expired bans, stale strike records, and old rate-limiter windows. The
// strikes map in particular is fingerprint/IP-keyed and was reset-but-never-deleted,
// so a churn of fresh keys grew it without bound (PLAN-08 deployment-review
// ship-blocker).
func (s *BanStore) GC(now time.Time) {
	if l, ok := s.limiter.(interface{ GC(time.Time) }); ok {
		l.GC(now)
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	for k, b := range s.bans {
		if !b.active(now) {
			delete(s.bans, k)
		}
	}
	for k, st := range s.strikes {
		if now.Sub(st.last) > strikeDecay {
			delete(s.strikes, k)
		}
	}
}

// List returns all active bans (for the console, SoT-26 §4). Expired entries are
// swept.
func (s *BanStore) List() []BanEntry {
	now := s.nowFn()
	s.mu.Lock()
	defer s.mu.Unlock()
	out := make([]BanEntry, 0, len(s.bans))
	for k, b := range s.bans {
		if !b.active(now) {
			delete(s.bans, k)
			continue
		}
		out = append(out, b)
	}
	return out
}
