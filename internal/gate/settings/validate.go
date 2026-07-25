package settings

import (
	"fmt"
	"strings"
)

// Validate checks SoT-39 §5.0 bounds. Returns nil if ok.
// Does not compute mutation class (caller Classify).
func Validate(o *Overlay) error {
	if o == nil {
		return nil
	}
	if o.SchemaVersion != "" && o.SchemaVersion != "1.0.0" {
		return fmt.Errorf("unsupported schemaVersion %q", o.SchemaVersion)
	}
	for id, m := range o.Gates {
		if err := validateGateMode(id, m); err != nil {
			return err
		}
	}
	for id, m := range o.HardRules {
		if err := validateHRMode(id, m); err != nil {
			return err
		}
	}
	for id, m := range o.NetPolicy {
		if err := validateNetMode(id, m); err != nil {
			return err
		}
	}
	if o.Scoring != nil {
		if err := validateScoring(o.Scoring); err != nil {
			return err
		}
	}
	for id, m := range o.WeightMultipliers {
		if m < 0 || m > 2 {
			return fmt.Errorf("weight multiplier %s=%v out of [0,2]", id, m)
		}
		if id == "" || strings.ContainsAny(id, " \t\n") {
			return fmt.Errorf("invalid signal id %q", id)
		}
	}
	if o.RateLimit != nil {
		if o.RateLimit.WindowSec != nil && *o.RateLimit.WindowSec < 1 {
			return fmt.Errorf("rateLimit.windowSec must be >= 1")
		}
		if o.RateLimit.Soft != nil && *o.RateLimit.Soft < 1 {
			return fmt.Errorf("rateLimit.soft must be >= 1")
		}
		if o.RateLimit.Hard != nil && *o.RateLimit.Hard < 1 {
			return fmt.Errorf("rateLimit.hard must be >= 1")
		}
		if o.RateLimit.Hard != nil && o.RateLimit.Soft != nil && *o.RateLimit.Hard < *o.RateLimit.Soft {
			return fmt.Errorf("rateLimit.hard must be >= soft")
		}
	}
	if pref, ok := o.Routes["/"]; ok && pref == "attested" {
		return fmt.Errorf("attested refused on catch-all prefix /")
	}
	if pref, ok := o.Routes[""]; ok && pref == "attested" {
		return fmt.Errorf("attested refused on empty prefix")
	}
	return nil
}

func validateGateMode(id string, m Mode) error {
	known := false
	for _, g := range KnownGates {
		if g == id {
			known = true
			break
		}
	}
	if !known {
		return fmt.Errorf("unknown gate module %q", id)
	}
	switch m {
	case ModeEnforce, ModeMonitor, ModeShadow:
		// ok
	case ModeOff:
		if _, crit := IntegrityCriticalGates[id]; crit {
			return fmt.Errorf("%s cannot be off (integrity-critical)", id)
		}
		if id == "gate.smuggle" || id == "gate.spoof_header" {
			return fmt.Errorf("%s cannot be off", id)
		}
	default:
		return fmt.Errorf("invalid mode %q for gate %s", m, id)
	}
	return nil
}

func validateHRMode(id string, m Mode) error {
	if !strings.HasPrefix(id, "HR-") {
		return fmt.Errorf("invalid hard rule id %q", id)
	}
	switch m {
	case ModeEnforce, ModeMonitor:
		return nil
	case ModeOff:
		return fmt.Errorf("engine hard rule %s cannot be off (use monitor)", id)
	default:
		return fmt.Errorf("invalid mode %q for hard rule %s", m, id)
	}
}

func validateNetMode(id string, m Mode) error {
	known := false
	for _, c := range NetPolicyClasses {
		if c == id {
			known = true
			break
		}
	}
	if !known {
		return fmt.Errorf("unknown netPolicy class %q", id)
	}
	if id == "net.tcp" && m == ModeEnforce {
		return fmt.Errorf("net.tcp cannot be enforce (Audit+Ban only)")
	}
	switch m {
	case ModeEnforce, ModeMonitor, ModeShadow:
		return nil
	case ModeOff:
		return fmt.Errorf("netPolicy %s cannot be off (use monitor)", id)
	default:
		return fmt.Errorf("invalid mode %q for netPolicy %s", m, id)
	}
}

func validateScoring(s *ScoringPatch) error {
	ch, dn, lc := ScoringDefaults()
	if s.ChallengeAt != nil {
		ch = *s.ChallengeAt
		if ch < 10 || ch > 60 {
			return fmt.Errorf("challengeAt %.1f out of [10,60]", ch)
		}
	}
	if s.DenyAt != nil {
		dn = *s.DenyAt
		if dn < 40 || dn > 95 {
			return fmt.Errorf("denyAt %.1f out of [40,95]", dn)
		}
	}
	if s.LayerCap != nil {
		lc = *s.LayerCap
		if lc < 30 || lc > 80 {
			return fmt.Errorf("layerCap %.1f out of [30,80]", lc)
		}
	}
	if ch >= dn {
		return fmt.Errorf("challengeAt (%.1f) must be < denyAt (%.1f)", ch, dn)
	}
	_ = lc
	return nil
}
