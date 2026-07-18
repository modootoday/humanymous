package signals

// Severity maps a Verdict to its risk multiplier (SoT-05 §1.1).
//
// score = weight * severity(verdict) * confidence
func Severity(v Verdict) float64 {
	switch v {
	case VerdictBot:
		return 1.0
	case VerdictSuspicious:
		return 0.5
	case VerdictOK, VerdictUnknown:
		return 0.0
	default:
		return 0.0
	}
}

// ComputeScore returns the risk contribution of a signal given its verdict,
// weight and confidence. It never exceeds weight and is clamped to [0, weight].
func ComputeScore(weight, confidence float64, v Verdict) float64 {
	s := weight * Severity(v) * clamp01(confidence)
	if s < 0 {
		return 0
	}
	if s > weight {
		return weight
	}
	return s
}

func clamp01(f float64) float64 {
	if f < 0 {
		return 0
	}
	if f > 1 {
		return 1
	}
	return f
}
