package collector

import (
	"strings"
	"time"

	"github.com/modootoday/humanymous/internal/signals"
)

// merge.go combines a client report and a network report into the session's
// SessionReport. The first network observation is pinned (plan/02 §2); the
// client report is replaced on each /api/collect.

// MergeClient stores/updates the client-collected portion of a session. The client
// report is sanitized first: a browser only ever reports L1–L4 evidence (fingerprint,
// guard, integrity, behavior), so any client-supplied signal that claims a
// server-authoritative identity — Collected==server, or an id in the server-minted
// L5/L6/L7 namespace (RIT, cross-checks, PoW/Pass upgrades) — is a forgery attempt and
// is dropped at ingest. This is the provenance boundary that stops a client from laundering
// a CHALLENGE to ALLOW by asserting its own trust-upgrade (deep-review). No weight change.
func (s *Store) MergeClient(id string, client signals.ClientReport, now time.Time) {
	client.Signals = sanitizeClientSignals(client.Signals)
	s.mu.Lock()
	defer s.mu.Unlock()
	st := s.ensureLocked(id, now)
	st.report.Client = client
	st.updated = now
}

// sanitizeClientSignals drops any client-reported signal that claims server provenance:
// a server Collected source, or a reserved server-authoritative id (L5/L6/L7). Returns a
// new slice; a nil/empty input yields nil.
func sanitizeClientSignals(in []signals.Signal) []signals.Signal {
	if len(in) == 0 {
		return in
	}
	out := in[:0:0]
	for _, sig := range in {
		if sig.Collected == signals.SourceServer || reservedServerID(sig.ID) {
			continue // forged server-provenance claim
		}
		out = append(out, sig)
	}
	return out
}

// reservedServerID reports whether id belongs to a server-minted layer (L5 RIT/traffic/
// correlation/abuse/h2dos/resource, L6 cross-checks, L7 PoW/Pass) that a client must
// never author.
func reservedServerID(id string) bool {
	return strings.HasPrefix(id, "l5.") || strings.HasPrefix(id, "l6.") ||
		strings.HasPrefix(id, "l7.") || strings.HasPrefix(id, "x.")
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
