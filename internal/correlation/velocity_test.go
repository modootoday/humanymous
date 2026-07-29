package correlation

import (
	"fmt"
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/signals"
)

func findSig(sigs []signals.Signal, id string) (signals.Signal, bool) {
	for _, s := range sigs {
		if s.ID == id {
			return s, true
		}
	}
	return signals.Signal{}, false
}

// The residual must be score-exempt: registered weight 0 so it can never move a verdict.
func TestVelocity_WeightZeroInvariant(t *testing.T) {
	d, ok := signals.Lookup("l5.correlation.ip_velocity")
	if !ok {
		t.Fatal("l5.correlation.ip_velocity must be registered")
	}
	if d.Weight != 0 {
		t.Fatalf("ip_velocity must be weight-0 (audit only, never a verdict), got weight %v", d.Weight)
	}
}

// A residential-proxy campaign — one fingerprint acquiring many /24s in a burst — must fire against
// an organic population baseline, and the emitted signal must itself be weight-0.
func TestVelocity_FiresOnBurstAgainstPopulation(t *testing.T) {
	r := New(time.Hour)
	t0 := time.Unix(1_700_000_000, 0)

	// Organic norm: 40 distinct fingerprints each on ONE /24 (per-fp velocity 1).
	for i := 0; i < 40; i++ {
		r.Observe(fmt.Sprintf("org-%d|ja4", i), fmt.Sprintf("10.%d.0", i), fmt.Sprintf("s-%d", i), t0)
	}

	fired := false
	var got signals.Signal
	for i := 0; i < 30; i++ { // one fp lights up 30 /24s within the window
		sigs := r.Observe("bot-fp|ja4", fmt.Sprintf("172.16.%d", i), fmt.Sprintf("bs-%d", i),
			t0.Add(time.Duration(i)*time.Second))
		if s, ok := findSig(sigs, "l5.correlation.ip_velocity"); ok {
			fired, got = true, s
		}
	}
	if !fired {
		t.Fatal("a burst of /24s from one fingerprint vs an organic population must fire ip_velocity")
	}
	if got.Weight != 0 {
		t.Fatalf("the emitted ip_velocity signal must be weight-0, got weight %v", got.Weight)
	}
}

// Slow organic accumulation — one /24 every 30s so the windowed rate stays below velFloor — must
// never fire, even as the cumulative subnet count grows large (the popular-fingerprint FP guard).
func TestVelocity_SilentOnSlowOrganicAccumulation(t *testing.T) {
	r := New(time.Hour)
	t0 := time.Unix(1_700_000_000, 0)
	for i := 0; i < 100; i++ {
		now := t0.Add(time.Duration(i) * 30 * time.Second)
		sigs := r.Observe("slow-fp|ja4", fmt.Sprintf("192.168.%d", i), fmt.Sprintf("ss-%d", i), now)
		if _, ok := findSig(sigs, "l5.correlation.ip_velocity"); ok {
			t.Fatalf("slow organic accumulation (30s apart) must never fire ip_velocity (fired at i=%d)", i)
		}
	}
}
