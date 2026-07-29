package mlcorrect

import (
	"math"
	"sync"
)

// drift.go is the label-free drift MONITOR (SoT-42 Pillar A). Ground truth is never available at
// inference — you cannot know a session was a bot. The research converges on three independent
// label-free families, each catching a different failure; this file implements all three and a
// 2-of-3 gate that suppresses the false alarms pure distance detectors are notorious for:
//
//  1. STUDD (student–teacher disagreement): the model is the "student"; the FROZEN scoring engine
//     verdict is the "teacher" it must not diverge from. A rising mimic-loss = the feature→verdict
//     relationship is decaying = the residual is going stale — no human label needed. Monitored by
//     Page-Hinkley.
//  2. Covariate shift: PSI per feature vs a frozen reference — "the traffic mix changed."
//  3. Oracle-error stream: Page-Hinkley on the residual's error against ORACLE-confirmed labels
//     (solved Pass ⇒ human, catalog/failed-CHALLENGE ⇒ bot) — the delayed partial-label signal.
//
// The monitor is read-only: it emits a DriftEvent (a retrain-candidate proposal). It never retrains
// or mutates a verdict — that stays offline + human-gated.

// PageHinkley is a one-sided change detector for a rising mean in a stream (Page 1954). It alarms
// when the cumulative positive deviation from the running mean exceeds Lambda, tolerating drift of
// magnitude Delta. Cheap (O(1) state) and label-free.
type PageHinkley struct {
	Delta  float64 // allowed magnitude of change before contributing
	Lambda float64 // alarm threshold
	n      int
	mean   float64
	cumSum float64 // cumulative (x - mean - delta)
	minSum float64
	alarm  bool
}

// NewPageHinkley builds a detector. Sensible defaults applied to non-positive args.
func NewPageHinkley(delta, lambda float64) *PageHinkley {
	if delta <= 0 {
		delta = 0.005
	}
	if lambda <= 0 {
		lambda = 5
	}
	return &PageHinkley{Delta: delta, Lambda: lambda}
}

// Update feeds one value; returns true if a rising-mean change is detected THIS step.
func (p *PageHinkley) Update(x float64) bool {
	p.n++
	p.mean += (x - p.mean) / float64(p.n)
	p.cumSum += x - p.mean - p.Delta
	if p.cumSum < p.minSum {
		p.minSum = p.cumSum
	}
	p.alarm = (p.cumSum - p.minSum) > p.Lambda
	return p.alarm
}

// Reset clears the detector after a confirmed change / retrain.
func (p *PageHinkley) Reset() { *p = PageHinkley{Delta: p.Delta, Lambda: p.Lambda} }

// PSI is the Population Stability Index between a reference and a current distribution over the
// SAME bucket edges: Σ (cur_i − ref_i)·ln(cur_i/ref_i). Rule of thumb: <0.1 stable, 0.1–0.25
// moderate shift, >0.25 significant shift. Inputs are fractions per bucket; zeros are floored so
// the log is finite.
func PSI(ref, cur []float64) float64 {
	if len(ref) != len(cur) || len(ref) == 0 {
		return 0
	}
	const eps = 1e-6
	var psi float64
	for i := range ref {
		r, c := ref[i], cur[i]
		if r < eps {
			r = eps
		}
		if c < eps {
			c = eps
		}
		psi += (c - r) * math.Log(c/r)
	}
	return psi
}

// DriftMonitor fuses the three families into a 2-of-3 gate. It is safe for concurrent use.
type DriftMonitor struct {
	mu        sync.Mutex
	mimic     *PageHinkley // STUDD: |student p(bot) − teacher(engine) label| stream
	oracle    *PageHinkley // error on oracle-confirmed labels
	psiThresh float64      // covariate-shift threshold (e.g. 0.25)
	lastPSI   float64
	events    uint64
}

// NewDriftMonitor builds the fused monitor with reasonable defaults.
func NewDriftMonitor() *DriftMonitor {
	return &DriftMonitor{
		mimic:     NewPageHinkley(0.01, 6),
		oracle:    NewPageHinkley(0.01, 6),
		psiThresh: 0.25,
	}
}

// ObserveMimic feeds one STUDD residual: the absolute disagreement between the model's p(bot) and
// the engine's own verdict (teacher), in [0,1]. Returns the mimic detector's alarm state.
func (d *DriftMonitor) ObserveMimic(disagreement float64) bool {
	d.mu.Lock()
	defer d.mu.Unlock()
	return d.mimic.Update(disagreement)
}

// ObserveOracleError feeds one error against an oracle-confirmed label (1 if the model was wrong,
// 0 if right). Returns the oracle detector's alarm state.
func (d *DriftMonitor) ObserveOracleError(errIndicator float64) bool {
	d.mu.Lock()
	defer d.mu.Unlock()
	return d.oracle.Update(errIndicator)
}

// UpdatePSI records the latest per-feature-max PSI vs the frozen reference.
func (d *DriftMonitor) UpdatePSI(maxFeaturePSI float64) {
	d.mu.Lock()
	defer d.mu.Unlock()
	d.lastPSI = maxFeaturePSI
}

// DriftEvent describes a retrain-candidate proposal (never an action).
type DriftEvent struct {
	Fired      bool
	MimicAlarm bool
	OracleAlarm bool
	CovariateShift bool
	MaxPSI     float64
}

// Gate evaluates the 2-of-3 rule: fire a retrain-candidate only when at least two of {STUDD mimic,
// oracle-error, covariate PSI} agree — this suppresses the harmless-covariate false alarms while
// still catching genuine concept drift. Read-only; the caller opens a retrain ticket, never
// retrains inline.
func (d *DriftMonitor) Gate() DriftEvent {
	d.mu.Lock()
	defer d.mu.Unlock()
	e := DriftEvent{
		MimicAlarm:     d.mimic.alarm,
		OracleAlarm:    d.oracle.alarm,
		CovariateShift: d.lastPSI > d.psiThresh,
		MaxPSI:         d.lastPSI,
	}
	n := 0
	if e.MimicAlarm {
		n++
	}
	if e.OracleAlarm {
		n++
	}
	if e.CovariateShift {
		n++
	}
	e.Fired = n >= 2
	if e.Fired {
		d.events++
	}
	return e
}
