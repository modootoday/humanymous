package mlcorrect

import "sync"

// shadow.go quantifies how a STAGED candidate model would differ from the live model if promoted,
// from real traffic, without ever serving the candidate (SELF-CORRECTION.md ⑦ SHADOW). It is the
// safety gate between the offline eval gates and a dual-control promotion: an operator loads a
// candidate as a shadow, watches the divergence on production traffic, and only then promotes.
//
// It compares the two models on the quantities that matter for a residual whose only power is to
// nudge toward CHALLENGE: how far the calibrated p(bot) moved, and — at the live fire threshold θ —
// how often the candidate would fire DIFFERENTLY, split by direction (would it CHALLENGE more humans,
// or catch more bots). Everything is aggregate, no PII.

// ShadowComparator accumulates active-vs-shadow agreement statistics. Safe for concurrent use.
type ShadowComparator struct {
	mu             sync.Mutex
	n              uint64
	sumAbsDelta    float64 // Σ |active.PBot − shadow.PBot| over non-abstaining pairs
	bothScored     uint64
	shadowAbstain  uint64 // candidate abstained where active did not
	activeAbstain  uint64
	fireAgree      uint64 // both fire, or neither fires, at θ
	shadowFiresMore uint64 // shadow fires where active does not (candidate more aggressive)
	activeFiresMore uint64 // active fires where shadow does not (candidate more lenient)
}

// observe records one (active, shadow) pair evaluated at the live fire threshold theta.
func (c *ShadowComparator) observe(activePBot, shadowPBot float32, activeAbstain, shadowAbstain bool, theta float32) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.n++
	if activeAbstain && !shadowAbstain {
		c.activeAbstain++
	}
	if shadowAbstain && !activeAbstain {
		c.shadowAbstain++
	}
	if activeAbstain || shadowAbstain {
		return // no comparable pair
	}
	c.bothScored++
	d := activePBot - shadowPBot
	if d < 0 {
		d = -d
	}
	c.sumAbsDelta += float64(d)
	af := activePBot >= theta
	sf := shadowPBot >= theta
	switch {
	case af == sf:
		c.fireAgree++
	case sf && !af:
		c.shadowFiresMore++
	default:
		c.activeFiresMore++
	}
}

// ShadowStats is the observability view of an in-progress shadow evaluation (no PII).
type ShadowStats struct {
	N               uint64  `json:"n"`               // sessions seen with a shadow installed
	BothScored      uint64  `json:"bothScored"`      // sessions both models scored (comparable)
	MeanAbsDelta    float64 `json:"meanAbsDelta"`    // average |Δ p(bot)| over comparable pairs
	FireAgreement   float64 `json:"fireAgreement"`   // fraction of comparable pairs that fire the same at θ
	ShadowFiresMore uint64  `json:"shadowFiresMore"` // candidate would CHALLENGE where active does not
	ActiveFiresMore uint64  `json:"activeFiresMore"` // candidate would pass where active challenges
	ShadowAbstained uint64  `json:"shadowAbstained"` // candidate abstained where active scored
}

// Snapshot returns the current comparison. Fields are safe to expose on the ops surface.
func (c *ShadowComparator) Snapshot() ShadowStats {
	c.mu.Lock()
	defer c.mu.Unlock()
	s := ShadowStats{
		N:               c.n,
		BothScored:      c.bothScored,
		ShadowFiresMore: c.shadowFiresMore,
		ActiveFiresMore: c.activeFiresMore,
		ShadowAbstained: c.shadowAbstain,
	}
	if c.bothScored > 0 {
		s.MeanAbsDelta = c.sumAbsDelta / float64(c.bothScored)
		s.FireAgreement = float64(c.fireAgree) / float64(c.bothScored)
	}
	return s
}
