package anomaly

import (
	"testing"
	"time"
)

// detector_test.go verifies the streaming MAD detector's core guarantees: a stable
// stream never flags, a clear outlier does, warmup suppresses early flags, a
// near-constant stream does not over-flag (MAD floor), and cardinality is bounded.

var t0 = time.Unix(1_700_000_000, 0)

func TestStableStreamNoFlag(t *testing.T) {
	d := New(Config{Window: 32, K: 6, Warmup: 8})
	vals := []float64{100, 102, 98, 101, 99, 103, 97, 100, 101, 99, 100, 102, 98, 101, 99, 100}
	flagged := 0
	for i, v := range vals {
		if d.Observe("k", v, t0.Add(time.Duration(i)*time.Second)).Anomalous {
			flagged++
		}
	}
	if flagged != 0 {
		t.Fatalf("a stable stream must not flag, got %d flags", flagged)
	}
}

func TestClearOutlierFlags(t *testing.T) {
	d := New(Config{Window: 32, K: 6, Warmup: 8})
	for i := 0; i < 20; i++ { // warm up around ~100
		d.Observe("k", 100+float64(i%3), t0.Add(time.Duration(i)*time.Second))
	}
	res := d.Observe("k", 5000, t0.Add(21*time.Second)) // a gross outlier
	if !res.Anomalous {
		t.Fatalf("a 50x outlier must flag; score=%.1f median=%.1f mad=%.1f", res.Score, res.Median, res.MAD)
	}
}

func TestWarmupSuppressesEarlyFlags(t *testing.T) {
	d := New(Config{Window: 32, K: 6, Warmup: 10})
	// Even a wild value cannot flag before warmup samples are collected.
	for i := 0; i < 9; i++ {
		if d.Observe("k", 1e9, t0.Add(time.Duration(i)*time.Second)).Anomalous {
			t.Fatal("must not flag before warmup")
		}
	}
}

func TestMADFloorPreventsOverFlagging(t *testing.T) {
	d := New(Config{Window: 32, K: 6, Warmup: 8, MADFloor: 1.0})
	// A perfectly constant stream has MAD 0; the floor keeps a tiny wobble from
	// reading as an infinite-sigma outlier.
	for i := 0; i < 20; i++ {
		d.Observe("k", 50, t0.Add(time.Duration(i)*time.Second))
	}
	if d.Observe("k", 50.5, t0.Add(21*time.Second)).Anomalous {
		t.Fatal("a sub-floor wobble on a constant stream must not flag")
	}
}

func TestBoundedCardinality(t *testing.T) {
	d := New(Config{Window: 8, MaxKeys: 100})
	for i := 0; i < 1000; i++ {
		d.Observe(string(rune(i))+"-key", 1, t0.Add(time.Duration(i)*time.Millisecond))
	}
	d.mu.Lock()
	n := len(d.series)
	d.mu.Unlock()
	if n > 100 {
		t.Fatalf("cardinality must stay bounded at MaxKeys=100, got %d", n)
	}
}
