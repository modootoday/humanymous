// Package collector correlates client-collected signals (L1-L4) with
// server-captured network observations (L5) into a single SessionReport, and
// stores sessions for the request lifecycle (SoT-00 §5, plan/02 §2).
package collector

import (
	"sync"
	"time"

	"github.com/modootoday/humanymous/internal/signals"
)

// sessionState holds the accumulating record for one session plus its RIT
// counter. The first network observation is pinned as the session's L5 basis
// (plan/02 §2); later requests only re-validate.
type sessionState struct {
	report       signals.SessionReport
	ritN         uint64 // last RIT counter a seed was issued for
	netPinned    bool
	powSolved    bool      // SoT-13: session has solved a proof-of-work challenge
	powIssued    time.Time // when the current PoW challenge was issued (solve-time)
	ritBootstrap bool      // SoT-07: the one-shot tokenless RIT grace has been consumed
	updated      time.Time
}

// Store is an in-memory, TTL-expiring session store (sufficient for the sample;
// swap for Redis in production).
type Store struct {
	mu  sync.RWMutex
	m   map[string]*sessionState
	ttl time.Duration
}

// NewStore returns a Store with the given TTL and starts no background GC; call
// GC periodically from the server, or rely on lazy expiry on access.
func NewStore(ttl time.Duration) *Store {
	return &Store{m: map[string]*sessionState{}, ttl: ttl}
}

// Ensure returns (creating if needed) the state for a session id.
func (s *Store) Ensure(id string, now time.Time) *sessionState {
	s.mu.Lock()
	defer s.mu.Unlock()
	st, ok := s.m[id]
	if !ok {
		st = &sessionState{}
		st.report.SessionID = id
		st.report.Timestamp = now
		s.m[id] = st
	}
	st.updated = now
	return st
}

// Get returns a snapshot of a session report and whether it exists.
func (s *Store) Get(id string) (signals.SessionReport, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	st, ok := s.m[id]
	if !ok {
		return signals.SessionReport{}, false
	}
	return st.report, true
}

// RITCounter returns the last issued RIT counter for a session (0 if new).
func (s *Store) RITCounter(id string) uint64 {
	s.mu.RLock()
	defer s.mu.RUnlock()
	if st, ok := s.m[id]; ok {
		return st.ritN
	}
	return 0
}

// AdvanceRIT increments and returns the new RIT counter for a session.
func (s *Store) AdvanceRIT(id string, now time.Time) uint64 {
	s.mu.Lock()
	defer s.mu.Unlock()
	st, ok := s.m[id]
	if !ok {
		st = &sessionState{}
		st.report.SessionID = id
		s.m[id] = st
	}
	st.ritN++
	st.updated = now
	return st.ritN
}

// ClaimRITBootstrap atomically consumes the session's ONE-SHOT tokenless RIT grace:
// it returns true the first time (and marks it used), false on every later call. This
// makes the bootstrap grace one-shot so a bot cannot camp indefinitely in the tokenless
// grace state and permanently suppress l5.rit.absent (deep-review). An honest client
// fetches /api/session (a seed) before its first /api/collect, so it presents a token and
// never relies on the grace — no false-positive.
func (s *Store) ClaimRITBootstrap(id string, now time.Time) bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	st, ok := s.m[id]
	if !ok {
		st = &sessionState{}
		st.report.SessionID = id
		s.m[id] = st
	}
	st.updated = now
	if st.ritBootstrap {
		return false
	}
	st.ritBootstrap = true
	return true
}

// SetPowSolved marks a session as having solved a PoW challenge (SoT-13).
func (s *Store) SetPowSolved(id string, now time.Time) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if st, ok := s.m[id]; ok {
		st.powSolved = true
		st.powIssued = time.Time{} // reset the solve-time clock for any future challenge cycle
		st.updated = now
	}
}

// SetPowIssued records when a PoW challenge was FIRST issued for solve-time analysis.
// It is first-write-wins: re-issuing the same challenge on every subsequent CHALLENGE
// collect must NOT reset the clock, or the measured solve time shrinks to the gap between
// the last re-issue and the solve — understating elapsed and defeating the too-fast control
// (deep-review). Cleared on solve so a fresh challenge cycle measures anew.
func (s *Store) SetPowIssued(id string, now time.Time) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if st, ok := s.m[id]; ok && st.powIssued.IsZero() {
		st.powIssued = now
	}
}

// PowIssuedAt returns when the session's PoW challenge was issued (zero if none).
func (s *Store) PowIssuedAt(id string) time.Time {
	s.mu.RLock()
	defer s.mu.RUnlock()
	if st, ok := s.m[id]; ok {
		return st.powIssued
	}
	return time.Time{}
}

// PowSolved reports whether a session has solved a PoW challenge.
func (s *Store) PowSolved(id string) bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	if st, ok := s.m[id]; ok {
		return st.powSolved
	}
	return false
}

// List returns recent session reports (newest-ish; for the report endpoint).
func (s *Store) List(limit int) []signals.SessionReport {
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := make([]signals.SessionReport, 0, len(s.m))
	for _, st := range s.m {
		out = append(out, st.report)
		if len(out) >= limit {
			break
		}
	}
	return out
}

// GC removes sessions older than the TTL.
func (s *Store) GC(now time.Time) {
	s.mu.Lock()
	defer s.mu.Unlock()
	for id, st := range s.m {
		if now.Sub(st.updated) > s.ttl {
			delete(s.m, id)
		}
	}
}
