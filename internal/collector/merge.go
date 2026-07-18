package collector

import (
	"time"

	"github.com/modootoday/humanymous/internal/signals"
)

// merge.go combines a client report and a network report into the session's
// SessionReport. The first network observation is pinned (plan/02 §2); the
// client report is replaced on each /api/collect.

// MergeClient stores/updates the client-collected portion of a session.
func (s *Store) MergeClient(id string, client signals.ClientReport, now time.Time) {
	s.mu.Lock()
	defer s.mu.Unlock()
	st := s.ensureLocked(id, now)
	st.report.Client = client
	st.updated = now
}

// MergeNetwork attaches the network observation. Only the first observation is
// pinned as the session basis; later ones are ignored here (re-validation is a
// separate concern).
func (s *Store) MergeNetwork(id string, net signals.NetworkReport, now time.Time) {
	s.mu.Lock()
	defer s.mu.Unlock()
	st := s.ensureLocked(id, now)
	if !st.netPinned {
		st.report.Network = net
		st.netPinned = true
	}
	st.updated = now
}

// AppendNetworkSignals adds extra L5 signals (e.g. RIT/resource) to the pinned
// network report without replacing it.
func (s *Store) AppendNetworkSignals(id string, extra []signals.Signal, now time.Time) {
	s.mu.Lock()
	defer s.mu.Unlock()
	st := s.ensureLocked(id, now)
	st.report.Network.Signals = append(st.report.Network.Signals, extra...)
	st.updated = now
}

// SetLabel records ground-truth for e2e evaluation.
func (s *Store) SetLabel(id, label string, now time.Time) {
	s.mu.Lock()
	defer s.mu.Unlock()
	st := s.ensureLocked(id, now)
	st.report.Label = label
	st.updated = now
}

// StoreScored persists the final scored report (after the engine runs).
func (s *Store) StoreScored(id string, r signals.SessionReport, now time.Time) {
	s.mu.Lock()
	defer s.mu.Unlock()
	st := s.ensureLocked(id, now)
	st.report = r
	st.updated = now
}

// ensureLocked is Ensure without locking (caller holds s.mu).
func (s *Store) ensureLocked(id string, now time.Time) *sessionState {
	st, ok := s.m[id]
	if !ok {
		st = &sessionState{}
		st.report.SessionID = id
		st.report.Timestamp = now
		s.m[id] = st
	}
	return st
}
