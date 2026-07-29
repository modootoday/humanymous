// Package correlation links sessions that rotate IP/UA but share a stable
// fingerprint, unmasking residential-proxy rotation and coordinated botnets
// (SoT-15). A single browser fingerprint (canvas/audio/webgl/fonts/timezone +
// JA4) appearing across MANY distinct /24 subnets is one bot behind a rotating
// proxy pool — the pattern IP reputation alone cannot catch.
package correlation

import (
	"sync"
	"time"

	"github.com/modootoday/humanymous/internal/anomaly"
	"github.com/modootoday/humanymous/internal/signals"
)

// Thresholds for flagging shared fingerprints (SoT-15 §2).
const (
	subnetsForProxyRotation = 3 // distinct /24 subnets sharing one fingerprint
	sessionsForShared       = 5 // distinct sessions sharing one fingerprint
	// Bot rotates fingerprintId per exit to dodge proxy_rotation while one JA4
	// still fans out across many subnets (anonymous-proxy / residential evasion).
	ja4SubnetsForFPChurn = 3
	ja4FPsForFPChurn     = 3

	// Fleet-velocity residual (NG-detection Pillar C1). proxy_rotation above fires on the ABSOLUTE
	// distinct-subnet count (>=3), which cannot tell an organic popular-fingerprint that accumulates
	// /24s slowly over days from a residential-proxy CAMPAIGN that lights up many /24s in minutes.
	// This tracks the DERIVATIVE: new /24s per key within velWindow, flagged by the streaming MAD
	// detector when a key's rate STEPS above its own recent baseline AND clears velFloor (so a step
	// alone on tiny numbers, or a slow organic ramp, does not fire). It is a WEIGHT-0 / score-exempt
	// residual (audit only), asymmetric by design: it corroborates proxy_rotation, never a lone DENY.
	velWindow  = 2 * time.Minute // trailing window over which new-subnet arrivals are counted
	velFloor   = 8               // min new /24s in the window before a step is even considered a fleet
	velRingCap = 512             // per-key arrival ring bound (memory)
	velPopKey  = "pop"           // single shared series: the population norm of per-fp burst-rate
)

type entry struct {
	subnets  map[string]struct{}
	sessions map[string]struct{}
	arrivals []time.Time // first-seen times of new /24s, pruned to velWindow (fleet-velocity)
	updated  time.Time
}

// ja4Fan tracks distinct client fingerprints + subnets under one server JA4.
type ja4Fan struct {
	fps     map[string]struct{}
	subnets map[string]struct{}
	updated time.Time
}

// Registry tracks fingerprint -> {subnets, sessions} and JA4 fan-out for fp-churn.
type Registry struct {
	mu   sync.Mutex
	m    map[string]*entry
	ja4m map[string]*ja4Fan
	ttl  time.Duration
	vel  *anomaly.Detector // fleet-velocity: per-key subnet-acquisition-rate MAD detector
}

// New returns a correlation registry with the given TTL.
func New(ttl time.Duration) *Registry {
	return &Registry{
		m: map[string]*entry{}, ja4m: map[string]*ja4Fan{}, ttl: ttl,
		// K=6 MADs and warmup=12 mirror the shadow anomaly detector already in tree; MADFloor 1
		// keeps a near-constant (organic) rate from over-flagging on integer wobble.
		vel: anomaly.New(anomaly.Config{Window: 256, K: 6, Warmup: 20, MADFloor: 1, MaxKeys: 16}),
	}
}

// velocity counts the new-/24 arrivals for a key within the trailing velWindow, pruning older ones.
// Caller holds r.mu.
func (e *entry) velocity(now time.Time) int {
	cut := now.Add(-velWindow)
	i := 0
	for i < len(e.arrivals) && e.arrivals[i].Before(cut) {
		i++
	}
	if i > 0 {
		e.arrivals = e.arrivals[i:]
	}
	return len(e.arrivals)
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
	newSubnet := false
	if subnet != "" {
		if _, seen := e.subnets[subnet]; !seen {
			e.subnets[subnet] = struct{}{}
			newSubnet = true
			if len(e.arrivals) < velRingCap {
				e.arrivals = append(e.arrivals, now)
			}
		}
	}
	e.sessions[sessionID] = struct{}{}
	e.updated = now
	nSubnets, nSessions := len(e.subnets), len(e.sessions)
	vel := e.velocity(now)
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

	// Fleet-velocity residual. On every new /24 we feed this key's windowed acquisition rate into ONE
	// shared "population" series, so the detector learns the NORMAL burst-rate of ordinary
	// fingerprints (≈1 new /24 per window). A residential-proxy campaign lighting up many /24s in
	// minutes is a MAD outlier against that norm — caught from birth, unlike a per-key baseline that a
	// fleet would poison on sample 1. We only EMIT when the rate also clears velFloor (a meaningful
	// fleet, not a handful) so a tiny outlier over a near-zero norm cannot fire. Weight-0 → audit only;
	// asymmetric — it corroborates proxy_rotation, never a lone DENY. (Honest limit: a campaign that
	// dominates the recent population poisons the norm; monitor-first + corroboration is why.)
	if r.vel != nil && newSubnet {
		res := r.vel.Observe(velPopKey, float64(vel), now)
		if vel >= velFloor && res.Anomalous {
			out = append(out, signals.New("l5.correlation.ip_velocity",
				map[string]any{"newSubnetsInWindow": vel, "madScore": res.Score},
				signals.VerdictSuspicious, 0.6, signals.SourceServer,
				"one fingerprint acquiring /24 subnets abnormally fast (fleet-velocity)"))
		}
	}
	return out
}

// ObserveJA4Fanout records that (fingerprintId, subnet) presented under a stable
// server-observed JA4. When one JA4 fans out across many subnets AND many client
// fingerprints, the bot is rotating fingerprintId to dodge proxy_rotation (HR-19
// class) while still riding a rotating anonymous/residential proxy pool.
func (r *Registry) ObserveJA4Fanout(ja4, fingerprintID, subnet string, now time.Time) []signals.Signal {
	if ja4 == "" || fingerprintID == "" {
		return nil
	}
	r.mu.Lock()
	f, ok := r.ja4m[ja4]
	if !ok {
		f = &ja4Fan{fps: map[string]struct{}{}, subnets: map[string]struct{}{}}
		r.ja4m[ja4] = f
	}
	f.fps[fingerprintID] = struct{}{}
	if subnet != "" {
		f.subnets[subnet] = struct{}{}
	}
	f.updated = now
	nFP, nSub := len(f.fps), len(f.subnets)
	r.mu.Unlock()

	if nFP >= ja4FPsForFPChurn && nSub >= ja4SubnetsForFPChurn {
		return []signals.Signal{signals.New("l5.correlation.fp_churn_proxy",
			map[string]int{"fingerprints": nFP, "subnets": nSub},
			signals.VerdictBot, 1.0, signals.SourceServer,
			"many fingerprints + many subnets under one JA4 (proxy fp-churn evasion)")}
	}
	return nil
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
	for k, f := range r.ja4m {
		if now.Sub(f.updated) > r.ttl {
			delete(r.ja4m, k)
		}
	}
	if r.vel != nil {
		r.vel.GC(now, r.ttl)
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
	r.ja4m = map[string]*ja4Fan{}
	r.vel = anomaly.New(anomaly.Config{Window: 256, K: 6, Warmup: 20, MADFloor: 1, MaxKeys: 16})
	r.mu.Unlock()
}
