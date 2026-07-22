package gate

import (
	"sync"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
)

// enforce.go holds the sticky-verdict store and the verdict→edge-action mapping
// (SoT-21 §1, §2). Every decision is emitted into the audit sink BEFORE the
// action takes effect (SoT-18 §7, SoT-19 step 13).
//
// This is the in-process reference of the shared verdict state; production
// externalizes it to Redis for fleet coherence (SoT-22 §1). The interface — a
// keyed sticky verdict with a coherence TTL — is identical.

// Verdict enumerates the edge decision.
type Verdict string

const (
	VerdictAllow     Verdict = "allow"
	VerdictChallenge Verdict = "challenge"
	VerdictDeny      Verdict = "deny"
	VerdictUnknown   Verdict = "none" // no evidence yet (fail-open default on public routes)
)

// stickyVerdict is a cached scoring outcome for a session (SoT-21 §2).
type stickyVerdict struct {
	verdict Verdict
	risk    float64
	rule    string
	top     []audit.Signal
	updated time.Time
}

// VerdictStore keeps sticky verdicts keyed by session id.
type VerdictStore struct {
	mu  sync.RWMutex
	m   map[string]stickyVerdict
	ttl time.Duration
}

// NewVerdictStore builds a sticky-verdict store with a coherence TTL.
func NewVerdictStore(ttl time.Duration) *VerdictStore {
	return &VerdictStore{m: map[string]stickyVerdict{}, ttl: ttl}
}

// Set records/updates the sticky verdict for a session (called by the control
// plane on each /collect beacon, SoT-19 step 5).
func (s *VerdictStore) Set(sid string, v stickyVerdict) {
	s.mu.Lock()
	s.m[sid] = v
	s.mu.Unlock()
}

// Get returns the sticky verdict (VerdictUnknown if absent/stale).
func (s *VerdictStore) Get(sid string, now time.Time) stickyVerdict {
	s.mu.RLock()
	v, ok := s.m[sid]
	s.mu.RUnlock()
	if !ok || now.Sub(v.updated) > s.ttl {
		return stickyVerdict{verdict: VerdictUnknown}
	}
	return v
}

// GC evicts sticky verdicts past their TTL. Without it the map grows once per
// distinct session forever — an attacker-controllable unbounded-memory vector, since
// Set runs on every /collect beacon (PLAN-08 deployment-review ship-blocker).
func (s *VerdictStore) GC(now time.Time) {
	s.mu.Lock()
	defer s.mu.Unlock()
	for k, v := range s.m {
		if now.Sub(v.updated) > s.ttl {
			delete(s.m, k)
		}
	}
}

// mapVerdict converts an engine verdict string (ALLOW/CHALLENGE/DENY) to the
// edge Verdict.
func mapVerdict(engine string) Verdict {
	switch engine {
	case "DENY":
		return VerdictDeny
	case "CHALLENGE":
		return VerdictChallenge
	case "ALLOW":
		return VerdictAllow
	}
	return VerdictUnknown
}

// action names the enforcement side effect for the audit record (SoT-25 §4).
// unsafe is true for state-changing HTTP methods (SoT-28 WS5).
func (v Verdict) action(route routePolicy, unsafe bool) (eventType, actionName string) {
	switch v {
	case VerdictDeny:
		return audit.EventEnfDeny, "block"
	case VerdictChallenge:
		return audit.EventEnfChallengeIssued, "challenge_pow"
	case VerdictAllow:
		return audit.EventEnfAllow, "pass"
	default: // Unknown — no verdict yet.
		// Fail CLOSED on sensitive routes AND on any state-changing (unsafe)
		// method or sync-score route: a mutation must never reach the origin
		// with zero detection evidence (SoT-28 WS5, closes the bare-POST-/checkout
		// path). Public SAFE-method routes fail OPEN for availability — no-JS GET
		// scraping of balanced routes is a documented accepted residual until the
		// JA4-corroborated beacon lands (SoT-28 WS5/WS6), covered meanwhile by the
		// fingerprint/subnet rate metering (SoT-27/WS7).
		if route.failClosed || route.syncScore || unsafe {
			return audit.EventEnfChallengeIssued, "challenge_pow"
		}
		return audit.EventEnfAllow, "pass"
	}
}

// isUnsafeMethod reports whether the HTTP method can change server state and so
// must fail closed on an unknown verdict (SoT-28 WS5).
func isUnsafeMethod(m string) bool {
	switch m {
	case "POST", "PUT", "PATCH", "DELETE":
		return true
	}
	return false
}
