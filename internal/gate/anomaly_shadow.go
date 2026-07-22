package gate

import (
	"log"
	"sync"
	"time"

	"github.com/modootoday/humanymous/internal/anomaly"
)

// anomaly_shadow.go is the PLAN-08 R5 shadow observer. It watches a per-request
// feature — the per-fingerprint request inter-arrival — through a streaming MAD
// outlier detector and LOGS outliers. It is strictly observational: it never touches
// the verdict, the ban ledger, or the enforcement path, and it exists only when
// -anomaly-shadow is set. This is the blueprint's shadow-first discipline: the
// engine's frozen rule table stays authoritative while evidence is collected, so an
// anomaly signal can be validated (FPR on real traffic) BEFORE it ever earns weight.
type anomalyShadow struct {
	det  *anomaly.Detector
	mu   sync.Mutex
	last map[string]time.Time
}

func newAnomalyShadow() *anomalyShadow {
	return &anomalyShadow{
		det:  anomaly.New(anomaly.Config{Window: 64, K: 6, Warmup: 12, MADFloor: 2}),
		last: map[string]time.Time{},
	}
}

// observe feeds the per-fingerprint inter-arrival to the detector and logs an outlier.
// Never blocks the request or changes the verdict.
func (a *anomalyShadow) observe(fp string, now time.Time) {
	if fp == "" {
		return
	}
	a.mu.Lock()
	prev, ok := a.last[fp]
	a.last[fp] = now
	if len(a.last) > 200000 { // bound the last-seen map alongside the detector's own bound
		for k, t := range a.last {
			if now.Sub(t) > 10*time.Minute {
				delete(a.last, k)
			}
		}
	}
	a.mu.Unlock()
	if !ok {
		return // first request from this fingerprint: no inter-arrival yet
	}
	gap := float64(now.Sub(prev).Milliseconds())
	if res := a.det.Observe(fp, gap, now); res.Anomalous {
		log.Printf("anomaly.shadow: inter-arrival outlier gap=%.0fms score=%.1fMAD median=%.0fms (SHADOW — verdict unaffected)",
			gap, res.Score, res.Median)
	}
}
