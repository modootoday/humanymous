// Package scoring implements L6 cross-checks and the L7 risk-scoring engine.
//
// The scorer is a pure function of a SessionReport: same input -> same output
// (SoT-05 §8). No randomness, no wall-clock. This keeps golden tests stable.
package scoring

// Policy holds the tunable knobs of the L7 decision (SoT-05). Versioned so
// weight/threshold changes are traceable against golden fixtures.
type Policy struct {
	Version string

	// Verdict thresholds on the 0..100 risk score (SoT-05 §5).
	ChallengeAt float64 // >= this -> at least CHALLENGE
	DenyAt      float64 // >= this -> DENY

	// LayerCap is the maximum risk any single layer may contribute before
	// combination, to stop one noisy layer from dominating (SoT-05 §2).
	LayerCap float64
}

// DefaultPolicy is policyVersion 1.0.0 (SoT-05).
func DefaultPolicy() Policy {
	return Policy{
		Version:     "1.0.0",
		ChallengeAt: 30,
		DenyAt:      70,
		LayerCap:    60,
	}
}

// Verdict labels (SoT-00 §6).
const (
	VerdictAllow     = "ALLOW"
	VerdictChallenge = "CHALLENGE"
	VerdictDeny      = "DENY"
)

// verdictFor maps a risk score to a verdict using the policy thresholds.
func (p Policy) verdictFor(score float64) string {
	switch {
	case score >= p.DenyAt:
		return VerdictDeny
	case score >= p.ChallengeAt:
		return VerdictChallenge
	default:
		return VerdictAllow
	}
}
