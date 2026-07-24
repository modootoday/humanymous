// Package correlation links sessions that rotate IP/UA but share a stable
// fingerprint, unmasking residential-proxy rotation and coordinated botnets
// (SoT-15). A single browser fingerprint (canvas/audio/webgl/fonts/timezone +
// JA4) appearing across MANY distinct /24 subnets is one bot behind a rotating
// proxy pool — the pattern IP reputation alone cannot catch.
package correlation

import (
	"sync"
	"time"

	"github.com/modootoday/humanymous/internal/signals"
)

// Thresholds for flagging shared fingerprints (SoT-15 §2).
const (
	subnetsForProxyRotation = 3 // distinct /24 subnets sharing one fingerprint
	sessionsForShared       = 5 // distinct sessions sharing one fingerprint
)

type entry struct {
	subnets  map[string]struct{}
	sessions map[string]struct{}
	updated  time.Time
}

// Registry tracks fingerprint -> {subnets, sessions}.
type Registry struct {
	mu  sync.Mutex
	m   map[string]*entry
	ttl time.Duration
}

// New returns a correlation registry with the given TTL.
func New(ttl time.Duration) *Registry {
	return &Registry{m: map[string]*entry{}, ttl: ttl}
}

// Observe records that (subnet, sessionID) presented the given correlation key
// (fingerprintId|ja4) and returns any correlation signals it triggers. An empty
// key (no client fingerprint) records nothing.
func (r *Registry) Observe(key, subnet, sessionID string, now time.Time) []signals.Signal {
	if key == "" || key == "|" {
		return nil
	}
	r.mu.Lock()
	e, ok := r.m[key]
	if !ok {
		e = &entry{subnets: map[string]struct{}{}, sessions: map[string]struct{}{}}
		r.m[key] = e
	}
	if subnet != "" {
		e.subnets[subnet] = struct{}{}
	}
	e.sessions[sessionID] = struct{}{}
	e.updated = now
	nSubnets, nSessions := len(e.subnets), len(e.sessions)
	r.mu.Unlock()

	var out []signals.Signal
	if nSubnets >= subnetsForProxyRotation {
		out = append(out, signals.New("l5.correlation.proxy_rotation", nSubnets,
			signals.VerdictBot, 1.0, signals.SourceServer,
			"one fingerprint across many subnets (residential-proxy rotation)"))
	} else if nSessions >= sessionsForShared {
		out = append(out, signals.New("l5.correlation.shared_fingerprint", nSessions,
			signals.VerdictSuspicious, 1.0, signals.SourceServer,
			"one fingerprint shared by many sessions (coordinated botnet)"))
	}
	return out
}

// GC drops fingerprint entries older than the TTL.
func (r *Registry) GC(now time.Time) {
	r.mu.Lock()
	defer r.mu.Unlock()
	for k, e := range r.m {
		if now.Sub(e.updated) > r.ttl {
			delete(r.m, k)
		}
	}
}

// Stats returns (subnets, sessions) currently associated with a key. It is the
// production read for the Gate's ceiling-guard #2 per-credential /24 fan-out cap
// (internal/gate webauthnGate) — NOT a test-only helper — so removing it silently
// disables that cap.
func (r *Registry) Stats(key string) (int, int) {
	r.mu.Lock()
	defer r.mu.Unlock()
	if e, ok := r.m[key]; ok {
		return len(e.subnets), len(e.sessions)
	}
	return 0, 0
}

// Reset clears all cross-session correlation state (SoT-30 §10 run isolation).
func (r *Registry) Reset() {
	r.mu.Lock()
	r.m = map[string]*entry{}
	r.mu.Unlock()
}
