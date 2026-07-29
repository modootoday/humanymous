package mlcorrect

import (
	"math/rand"
	"testing"

	"github.com/modootoday/humanymous/internal/behavior"
)

// fvFrom builds a feature vector whose feature 0 is drawn from a shiftable distribution and the rest
// are mild noise, so we can drive a controlled covariate shift.
func fvFrom(rng *rand.Rand, center float64) behavior.FeatureVector {
	fv := make(behavior.FeatureVector, behavior.FeatureDim)
	for i := range fv {
		fv[i] = float32(rng.NormFloat64() * 0.1)
	}
	fv[0] = float32(center + rng.NormFloat64()*0.3)
	return fv
}

func TestFeatureMonitor_FreezesReferenceThenEvaluates(t *testing.T) {
	rng := rand.New(rand.NewSource(1))
	m := NewFeatureMonitor(200)
	if m.Ready() {
		t.Fatal("monitor must not be ready before the reference window fills")
	}
	// fill the reference window — no reading yet.
	for i := 0; i < 200; i++ {
		if _, closed := m.Observe(fvFrom(rng, 0)); closed {
			t.Fatal("no window should close while capturing the reference")
		}
	}
	if !m.Ready() {
		t.Fatal("monitor must be ready after the reference window fills")
	}
}

func TestFeatureMonitor_StableTrafficBelowShiftThreshold(t *testing.T) {
	rng := rand.New(rand.NewSource(2))
	m := NewFeatureMonitor(600)
	for i := 0; i < 600; i++ {
		m.Observe(fvFrom(rng, 0)) // reference
	}
	var got float64
	var closed bool
	for i := 0; i < 600; i++ { // a current window from the SAME distribution
		if p, c := m.Observe(fvFrom(rng, 0)); c {
			got, closed = p, true
		}
	}
	if !closed {
		t.Fatal("a full current window must close and report a reading")
	}
	// Same-distribution traffic must stay UNDER the drift gate's 0.25 significant-shift threshold, so
	// the covariate arm does not false-alarm. (max-PSI over many features has finite-sample upward
	// bias; the gate fuses this with two other arms precisely so this arm alone can't trip it.)
	if got >= 0.25 {
		t.Fatalf("same-distribution traffic must stay below the 0.25 shift threshold, got %.3f", got)
	}
}

func TestFeatureMonitor_ShiftedTrafficHighPSI(t *testing.T) {
	rng := rand.New(rand.NewSource(3))
	m := NewFeatureMonitor(300)
	for i := 0; i < 300; i++ {
		m.Observe(fvFrom(rng, 0)) // reference centered at 0
	}
	var got float64
	for i := 0; i < 300; i++ {
		if p, c := m.Observe(fvFrom(rng, 4)); c { // current window shifted far
			got = p
		}
	}
	if got < 0.25 {
		t.Fatalf("a large covariate shift must exceed the 0.25 significant-shift threshold, got %.3f", got)
	}
	if m.LastMaxPSI() != got {
		t.Fatalf("LastMaxPSI %.3f must match the last closed-window reading %.3f", m.LastMaxPSI(), got)
	}
}

func TestController_FeatureObserverDrivesCovariateArm(t *testing.T) {
	rng := rand.New(rand.NewSource(4))
	c := NewController(0.01, 0.5, 0.02)
	// reference then a shifted current window via the mlserve.FeatureObserver entry point.
	for i := 0; i < defaultWindow; i++ {
		c.ObserveFeatures(fvFrom(rng, 0))
	}
	for i := 0; i < defaultWindow; i++ {
		c.ObserveFeatures(fvFrom(rng, 5))
	}
	ev := c.Snapshot().Drift
	if !ev.CovariateShift {
		t.Fatalf("a shifted feature window must register covariate shift in the drift gate (maxPSI=%.3f)", ev.MaxPSI)
	}
}
