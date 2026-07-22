// Package anomaly is a dependency-free streaming outlier detector (PLAN-08 R5). It
// keeps a bounded, per-key sliding window and flags a value that deviates from the
// window's median by more than k robust deviations (MAD — median absolute deviation),
// which unlike mean/stddev is not dragged around by the very outliers it should
// catch. It is deliberately SHADOW-first: it observes and scores, it does not decide.
// The engine's frozen rule table remains the authority; an anomaly score is, at most,
// a log-only signal until empirically validated against real human traffic (the
// reason the blueprint deferred giving it any weight).
//
// Safety guards the blueprint called for are built in: a warmup (no flag until enough
// samples), a MAD floor (a near-constant series never over-flags on sub-noise wobble),
// and bounded cardinality (a churn of keys cannot grow memory without limit).
package anomaly

import (
	"sort"
	"sync"
	"time"
)

// Detector is a concurrent, bounded streaming MAD outlier detector.
type Detector struct {
	mu       sync.Mutex
	window   int     // samples retained per key
	k        float64 // deviation threshold in MADs
	warmup   int     // samples required before a key can flag
	madFloor float64 // minimum MAD (absolute units) to consider a spread meaningful
	maxKeys  int     // cardinality bound
	series   map[string]*series
}

type series struct {
	vals []float64
	last time.Time
}

// Config configures a Detector. Zero values fall back to sane defaults.
type Config struct {
	Window   int
	K        float64
	Warmup   int
	MADFloor float64
	MaxKeys  int
}

// New builds a Detector.
func New(c Config) *Detector {
	if c.Window <= 0 {
		c.Window = 64
	}
	if c.K <= 0 {
		c.K = 6
	}
	if c.Warmup <= 0 {
		c.Warmup = 12
	}
	if c.MaxKeys <= 0 {
		c.MaxKeys = 100000
	}
	return &Detector{window: c.Window, k: c.K, warmup: c.Warmup, madFloor: c.MADFloor, maxKeys: c.MaxKeys,
		series: map[string]*series{}}
}

// Result is the outcome of one observation.
type Result struct {
	Anomalous bool
	Score     float64 // robust deviation in MADs (0 during warmup)
	Median    float64
	MAD       float64
	N         int
}

// Observe records value for key at now and returns whether it is an outlier vs. the
// key's recent window. The value is appended AFTER scoring, so a point is judged
// against its own history, not itself.
func (d *Detector) Observe(key string, value float64, now time.Time) Result {
	d.mu.Lock()
	defer d.mu.Unlock()

	s := d.series[key]
	if s == nil {
		if len(d.series) >= d.maxKeys {
			d.evictOldestLocked()
		}
		s = &series{}
		d.series[key] = s
	}
	s.last = now

	res := Result{N: len(s.vals)}
	if len(s.vals) >= d.warmup {
		med := median(s.vals)
		mad := medianAbsDev(s.vals, med)
		if mad < d.madFloor {
			mad = d.madFloor
		}
		res.Median, res.MAD = med, mad
		if mad > 0 {
			dev := value - med
			if dev < 0 {
				dev = -dev
			}
			res.Score = dev / mad
			res.Anomalous = res.Score > d.k
		}
	}

	// Append into the ring (drop the oldest when full).
	if len(s.vals) >= d.window {
		copy(s.vals, s.vals[1:])
		s.vals[len(s.vals)-1] = value
	} else {
		s.vals = append(s.vals, value)
	}
	return res
}

// GC evicts keys idle longer than ttl.
func (d *Detector) GC(now time.Time, ttl time.Duration) {
	d.mu.Lock()
	defer d.mu.Unlock()
	for k, s := range d.series {
		if now.Sub(s.last) > ttl {
			delete(d.series, k)
		}
	}
}

func (d *Detector) evictOldestLocked() {
	var oldestKey string
	var oldest time.Time
	first := true
	for k, s := range d.series {
		if first || s.last.Before(oldest) {
			oldestKey, oldest, first = k, s.last, false
		}
	}
	if oldestKey != "" {
		delete(d.series, oldestKey)
	}
}

func median(xs []float64) float64 {
	c := append([]float64(nil), xs...)
	sort.Float64s(c)
	n := len(c)
	if n == 0 {
		return 0
	}
	if n%2 == 1 {
		return c[n/2]
	}
	return (c[n/2-1] + c[n/2]) / 2
}

func medianAbsDev(xs []float64, med float64) float64 {
	dev := make([]float64, len(xs))
	for i, x := range xs {
		d := x - med
		if d < 0 {
			d = -d
		}
		dev[i] = d
	}
	return median(dev)
}
