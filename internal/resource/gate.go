package resource

// gate.go decides how to serve a resource given the session's trust (SoT-10 §2).
// Pure function: (tier, verdict, ritOK, liveness) -> Decision. Heavy/embed tiers
// require ALLOW + valid RIT + behavioral liveness; otherwise they are downgraded
// or denied to save bandwidth and block scrapers.

// Decision is the serving outcome.
type Decision int

const (
	Serve     Decision = iota // full resource
	Downgrade                 // preview / poster / lower quality
	DenyServe                 // placeholder / blocked (traffic saved)
	Challenge                 // require verification before serving heavy
)

func (d Decision) String() string {
	switch d {
	case Serve:
		return "serve"
	case Downgrade:
		return "downgrade"
	case DenyServe:
		return "deny"
	case Challenge:
		return "challenge"
	default:
		return "unknown"
	}
}

// Verdict is the L7 session verdict (subset used here).
const (
	VerdictAllow     = "ALLOW"
	VerdictChallenge = "CHALLENGE"
	VerdictDeny      = "DENY"
)

// Gate returns the serving decision (SoT-10 §2 matrix).
func Gate(tier Tier, verdict string, ritOK, liveness bool) Decision {
	// Essential and light resources are always served (blocking them breaks
	// signal collection / basic UX). RIT/liveness gate only the heavy tiers.
	if tier == TierEssential {
		return Serve
	}

	switch verdict {
	case VerdictAllow:
		switch tier {
		case TierHeavy:
			// Full heavy resource requires valid RIT + human liveness.
			if ritOK && liveness {
				return Serve
			}
			return Challenge
		case TierEmbed:
			return Serve // embed inspection handled separately
		default:
			return Serve
		}
	case VerdictChallenge:
		switch tier {
		case TierHeavy:
			return Downgrade // poster/preview only
		case TierEmbed:
			return Challenge
		case TierMedia:
			return Downgrade
		default:
			return Serve
		}
	case VerdictDeny:
		switch tier {
		case TierLight:
			return Serve // cheap; not worth a placeholder
		case TierMedia:
			return DenyServe
		case TierHeavy, TierEmbed:
			return DenyServe // save bandwidth: no video to bots
		default:
			return DenyServe
		}
	default:
		// Unknown verdict: be conservative on heavy tiers only.
		if tier == TierHeavy || tier == TierEmbed {
			return Challenge
		}
		return Serve
	}
}
