package mlcorrect

import (
	"sort"
	"sync"

	"github.com/modootoday/humanymous/internal/behavior"
)

// features_monitor.go is the covariate-shift arm of the drift gate (SoT-42 Pillar A). It watches the
// live behavioral feature distribution and reports the maximum per-feature PSI (Population Stability
// Index) versus a frozen REFERENCE window — "the traffic mix changed", detected without any labels.
// It is the third, independent signal fused with STUDD (student vs frozen-engine) and the
// oracle-error stream in the 2-of-3 DriftMonitor, so no single label-free detector's false alarms
// can trip a retrain candidate alone.
//
// Design: the FIRST `window` feature vectors are captured as the reference; per feature its
// per-decile bucket edges are frozen and its reference bucket fractions computed. Every subsequent
// `window` observations form a current window whose per-feature fractions are compared to the
// reference by PSI; the max over features is the covariate signal. Per-feature quantile buckets (not
// one global grid) are required because Extract's features have heterogeneous ranges.

const (
	psiBuckets   = 8
	defaultWindow = 500
)

// FeatureMonitor accumulates a reference distribution then rolls a current window against it. Safe
// for concurrent use; the hot-path critical section is a handful of comparisons per feature.
type FeatureMonitor struct {
	mu     sync.Mutex
	dim    int
	window int

	refSamples [][]float32 // [dim][] reference values (only until the reference is frozen)
	frozen     bool
	edges      [][]float64 // [dim][psiBuckets+1] frozen bucket edges from reference deciles
	refFrac    [][]float64 // [dim][psiBuckets] reference bucket fractions

	curCounts [][]int // [dim][psiBuckets] current-window counts
	curN      int
	lastMax   float64
}

// NewFeatureMonitor builds a monitor. window ≤ 0 uses a sensible default.
func NewFeatureMonitor(window int) *FeatureMonitor {
	if window <= 0 {
		window = defaultWindow
	}
	d := behavior.FeatureDim
	m := &FeatureMonitor{dim: d, window: window}
	m.refSamples = make([][]float32, d)
	return m
}

// Observe feeds one live feature vector. It returns (maxPSI, true) exactly when a current window
// closes (a fresh covariate reading is available); otherwise (0, false). The caller pushes the
// reading into the drift gate's covariate arm.
func (m *FeatureMonitor) Observe(fv behavior.FeatureVector) (maxPSI float64, windowClosed bool) {
	if len(fv) != m.dim {
		return 0, false
	}
	m.mu.Lock()
	defer m.mu.Unlock()

	if !m.frozen {
		for i := 0; i < m.dim; i++ {
			m.refSamples[i] = append(m.refSamples[i], fv[i])
		}
		if len(m.refSamples[0]) >= m.window {
			m.freezeReference()
		}
		return 0, false
	}

	for i := 0; i < m.dim; i++ {
		b := bucketOf(m.edges[i], float64(fv[i]))
		m.curCounts[i][b]++
	}
	m.curN++
	if m.curN < m.window {
		return 0, false
	}
	// window closed — compute max per-feature PSI vs reference, then reset the current window.
	max := 0.0
	for i := 0; i < m.dim; i++ {
		cur := fracOf(m.curCounts[i], m.curN)
		if p := PSI(m.refFrac[i], cur); p > max {
			max = p
		}
		for b := range m.curCounts[i] {
			m.curCounts[i][b] = 0
		}
	}
	m.curN = 0
	m.lastMax = max
	return max, true
}

// LastMaxPSI returns the most recent closed-window covariate reading (0 before the first window).
func (m *FeatureMonitor) LastMaxPSI() float64 {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.lastMax
}

// Ready reports whether the reference window has been captured and frozen.
func (m *FeatureMonitor) Ready() bool {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.frozen
}

// freezeReference computes per-feature decile edges + reference fractions, then drops the raw
// samples. Caller holds the lock.
func (m *FeatureMonitor) freezeReference() {
	m.edges = make([][]float64, m.dim)
	m.refFrac = make([][]float64, m.dim)
	m.curCounts = make([][]int, m.dim)
	for i := 0; i < m.dim; i++ {
		vals := make([]float64, len(m.refSamples[i]))
		for k, v := range m.refSamples[i] {
			vals[k] = float64(v)
		}
		sort.Float64s(vals)
		m.edges[i] = quantileEdges(vals, psiBuckets)
		counts := make([]int, psiBuckets)
		for _, v := range vals {
			counts[bucketOf(m.edges[i], v)]++
		}
		m.refFrac[i] = fracOf(counts, len(vals))
		m.curCounts[i] = make([]int, psiBuckets)
	}
	m.frozen = true
	m.refSamples = nil // free the reference buffer
}

// quantileEdges returns psiBuckets+1 edges at even quantiles of a SORTED slice. Degenerate features
// (all values equal) collapse to a single bucket; PSI is then ~0 by construction.
func quantileEdges(sorted []float64, buckets int) []float64 {
	edges := make([]float64, buckets+1)
	n := len(sorted)
	if n == 0 {
		return edges
	}
	for b := 0; b <= buckets; b++ {
		idx := b * (n - 1) / buckets
		edges[b] = sorted[idx]
	}
	return edges
}

// bucketOf maps a value to a bucket index in [0, len(edges)-2] by the frozen edges (clamped).
func bucketOf(edges []float64, v float64) int {
	// edges has psiBuckets+1 entries; bucket b covers [edges[b], edges[b+1]).
	for b := len(edges) - 2; b >= 0; b-- {
		if v >= edges[b] {
			return b
		}
	}
	return 0
}

// fracOf converts counts to fractions summing to 1 (empty ⇒ zeros).
func fracOf(counts []int, n int) []float64 {
	out := make([]float64, len(counts))
	if n <= 0 {
		return out
	}
	for i, c := range counts {
		out[i] = float64(c) / float64(n)
	}
	return out
}
