package gate

import (
	"log"
	"sync"
	"time"

	"github.com/modootoday/humanymous/internal/signals"
)

// cohort_shadow.go is ceiling-guard #3 from the Blue-team ceiling-guard meeting: a
// POPULATION/COHORT behavioral observer. A coherent per-session spoof samples a
// plausible human behavior joint-distribution and passes the per-session engine — by
// construction it is undetectable one session at a time. But a FARM driving many
// unlinked sessions from ONE synthetic behavior generator leaves a different
// fingerprint: the COHORT's behavioral moments cluster improbably tightly, far
// tighter than any real population of distinct humans. A clean single session still
// passes; the shared generator betrays itself only in the aggregate.
//
// It is STRICTLY LOG-ONLY (the PLAN-08 R5 shadow-first discipline): it never touches
// the verdict, the ban ledger, or the enforcement path — so it adds ZERO live FPR by
// construction, and raises ~0 live cost against a coherent spoof (which can resample
// its distribution per session). Its value is farm-review intel and FPR evidence for
// a POSSIBLE future low-weight signal, whose promotion is gated on FIRST proving that
// natural shared-egress cohorts (CGNAT / corporate golden-image / Tor / assistive
// tech) show HIGH inter-user entropy — a bar they will likely fail, so the honest
// expectation is that this stays a diagnostic and never earns weight.
type cohortShadow struct {
	mu        sync.Mutex
	cohorts   map[string]*cohortEntry // cohortKey (server-observed fp|subnet) -> signature tallies
	minCohort int                     // distinct sessions sharing one signature before it is improbable
}

type cohortEntry struct {
	sigs    map[string]map[string]struct{} // behaviorSig -> set of distinct session ids
	flagged map[string]struct{}            // signatures already logged (log once per cohort+sig)
	updated time.Time
}

func newCohortShadow() *cohortShadow {
	return &cohortShadow{cohorts: map[string]*cohortEntry{}, minCohort: 6}
}

// cohortSigMax bounds the per-process cohort map so a churn of attacker-controlled
// fingerprints cannot grow it without limit between GC ticks (deep-review discipline).
const cohortSigMax = 100000

// observe records one session's coarse behavioral signature under its cohort and logs
// (once) when a cohort accumulates an improbably tight cluster: many DISTINCT sessions
// presenting the SAME coarse motor signature. Never blocks, never scores.
func (cs *cohortShadow) observe(cohortKey string, b signals.BehaviorSummary, sid string, now time.Time) {
	if cohortKey == "" || cohortKey == "|" || sid == "" {
		return
	}
	sig := behaviorSignature(b)
	if sig == "" {
		return // no behavioral content to cluster on (e.g. a no-JS/beacon-only hit)
	}
	cs.mu.Lock()
	defer cs.mu.Unlock()
	e := cs.cohorts[cohortKey]
	if e == nil {
		if len(cs.cohorts) >= cohortSigMax {
			// Saturated: sweep idle cohorts, then FAIL-SAFE — if still full, refuse the new
			// cohort rather than grow unbounded (mirrors NonceCache.Use). Under a fresh-cohort
			// flood the sweep frees nothing, so this hard cap is what actually bounds memory.
			for k, ce := range cs.cohorts {
				if now.Sub(ce.updated) > 10*time.Minute {
					delete(cs.cohorts, k)
				}
			}
			if len(cs.cohorts) >= cohortSigMax {
				return
			}
		}
		e = &cohortEntry{sigs: map[string]map[string]struct{}{}, flagged: map[string]struct{}{}}
		cs.cohorts[cohortKey] = e
	}
	e.updated = now
	// Per-(cohort,sig) sid set cap: once the improbable-cluster signal has been captured at
	// minCohort and flagged, STOP adding sids. The sid is a client-chosen cookie, so a fixed
	// fp+subnet+signature with rotating sids would otherwise grow this set without bound; and
	// further sids add zero detection value beyond the flag. This bounds the sharper vector.
	if _, done := e.flagged[sig]; done {
		return
	}
	set := e.sigs[sig]
	if set == nil {
		set = map[string]struct{}{}
		e.sigs[sig] = set
	}
	set[sid] = struct{}{}
	if len(set) >= cs.minCohort {
		e.flagged[sig] = struct{}{}
		log.Printf("cohort.shadow: %d unlinked sessions in one cohort share an identical motor signature (suspected shared behavior generator — SHADOW, verdict unaffected)", len(set))
		delete(e.sigs, sig) // signal captured; release the sid set (the flag is the durable record)
	}
}

// gc age-evicts idle cohorts.
func (cs *cohortShadow) gc(now time.Time) {
	cs.mu.Lock()
	defer cs.mu.Unlock()
	for k, e := range cs.cohorts {
		if now.Sub(e.updated) > 10*time.Minute {
			delete(cs.cohorts, k)
		}
	}
}

// behaviorSignature reduces the motor moments to a COARSE bucketed signature. Real
// humans scatter across these buckets; a shared synthetic generator collapses many
// sessions onto one. Returns "" when there is no interaction to characterise.
func behaviorSignature(b signals.BehaviorSummary) string {
	if b.Mouse.Samples == 0 && b.Key.Keystrokes == 0 && b.Events.TotalEvents == 0 {
		return ""
	}
	// Coarse buckets: quantise each moment so near-identical (not bit-identical)
	// synthetic vectors still collide, while genuine human spread does not.
	q := func(v float64, step float64) int {
		if step <= 0 {
			return 0
		}
		return int(v / step)
	}
	parts := []int{
		q(b.Mouse.MeanCurvature, 0.05),
		q(b.Mouse.VelocityStdDev, 0.5),
		q(b.Mouse.CoalescedRatio, 0.5),
		q(b.Mouse.StraightLineFrac, 0.1),
		q(b.Key.MeanFlightMs, 10),
		q(b.Key.FlightStdDev, 5),
	}
	buf := make([]byte, 0, 24)
	for _, p := range parts {
		buf = append(buf, byte(p), byte(p>>8), '|')
	}
	return string(buf)
}
