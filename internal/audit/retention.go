package audit

import "time"

// retention.go defines the tiered retention policy (SoT-28 WS8 / SoT-18 §6). The
// console must not display retention tiers the code does not enforce; this makes
// the tiers a real, queryable policy and classifies a record's age into a tier.
// Physical segment retirement (moving whole checkpoint-bounded prefixes to WORM
// and recording their terminal STH so surviving segments still verify) is the
// production step; the policy + classifier are the enforced contract.

// RetentionPolicy holds the tier windows.
type RetentionPolicy struct {
	Hot  time.Duration // queryable
	Warm time.Duration // compressed
	Cold time.Duration // WORM; expired after this
}

// DefaultRetention returns the default tiered policy: HOT 90d, WARM 1y, COLD 7y.
func DefaultRetention() RetentionPolicy {
	return RetentionPolicy{Hot: 90 * 24 * time.Hour, Warm: 365 * 24 * time.Hour, Cold: 7 * 365 * 24 * time.Hour}
}

// Tier classifies a record age into its retention tier.
func (p RetentionPolicy) Tier(age time.Duration) string {
	switch {
	case age <= p.Hot:
		return "HOT"
	case age <= p.Warm:
		return "WARM"
	case age <= p.Cold:
		return "COLD"
	default:
		return "EXPIRED"
	}
}

// Days renders the tier windows in days (for the console).
func (p RetentionPolicy) Days() map[string]int {
	d := func(t time.Duration) int { return int(t.Hours() / 24) }
	return map[string]int{"hot": d(p.Hot), "warm": d(p.Warm), "cold": d(p.Cold)}
}
