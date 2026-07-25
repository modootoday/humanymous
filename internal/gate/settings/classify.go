package settings

// Classify computes mutation class from a proposed overlay relative to code
// defaults + current effective (SoT-39 §5.1). Never trust client class.
//
// Surface: hardRules, gates, scoring, weights, netPolicy, routes, rate, globalMonitor.
func Classify(eff Effective, o *Overlay) MutationClass {
	if o == nil {
		return ClassA
	}
	class := ClassA
	raise := func(c MutationClass) {
		// C > B > D > A
		order := map[MutationClass]int{ClassA: 1, ClassD: 2, ClassB: 3, ClassC: 4}
		if order[c] > order[class] {
			class = c
		}
	}

	defCh, defDn, defLc := ScoringDefaults()
	if o.Scoring != nil {
		if o.Scoring.ChallengeAt != nil {
			v := *o.Scoring.ChallengeAt
			if v > eff.ChallengeAt || v > defCh {
				// weaker enforcement
				if v > defCh+15 {
					raise(ClassC)
				} else {
					raise(ClassB)
				}
			}
			if v < eff.ChallengeAt {
				raise(ClassA) // tighten
			}
		}
		if o.Scoring.DenyAt != nil {
			v := *o.Scoring.DenyAt
			if v > eff.DenyAt || v > defDn {
				if v > defDn+15 {
					raise(ClassC)
				} else {
					raise(ClassB)
				}
			}
			if v < eff.DenyAt {
				raise(ClassA)
			}
		}
		if o.Scoring.LayerCap != nil {
			v := *o.Scoring.LayerCap
			if v < eff.LayerCap || v < defLc {
				raise(ClassB) // lowering layerCap weakens
			}
			if v > eff.LayerCap {
				raise(ClassA)
			}
		}
	}

	zeroCount := 0
	for _, m := range o.WeightMultipliers {
		if m == 0 {
			zeroCount++
			raise(ClassB)
		} else if m < 1 {
			raise(ClassB)
		} else if m > 1 {
			raise(ClassA)
		}
	}
	if zeroCount >= 5 {
		raise(ClassC)
	}

	for id, mode := range o.HardRules {
		cur := eff.HRMode(id)
		if mode == ModeMonitor && cur == ModeEnforce {
			if _, crit := IntegrityCriticalEngineHRs[id]; crit {
				raise(ClassC)
			} else {
				raise(ClassB)
			}
		}
		if mode == ModeEnforce && cur == ModeMonitor {
			raise(ClassA)
		}
	}

	for id, mode := range o.Gates {
		cur := eff.GateMode(id)
		if mode != ModeEnforce && cur == ModeEnforce {
			if _, crit := IntegrityCriticalGates[id]; crit {
				raise(ClassC)
			} else {
				raise(ClassB)
			}
		}
	}

	if o.GlobalMonitor != nil && *o.GlobalMonitor && !eff.GlobalMonitor {
		raise(ClassC)
	}

	// netPolicy demote enforce→monitor = B; net.correlation demote = C (HR-19 family)
	for id, mode := range o.NetPolicy {
		cur := ModeEnforce
		if m, ok := eff.NetPolicy[id]; ok {
			cur = m
		} else if id == "net.correlation" {
			cur = ModeEnforce // code default when empty overlay
		} else {
			cur = ModeMonitor
		}
		if mode == ModeMonitor && cur == ModeEnforce {
			if id == "net.correlation" {
				raise(ClassC)
			} else {
				raise(ClassB)
			}
		}
	}

	// routes: weaker preset / monitor demotion
	for prefix, preset := range o.Routes {
		cur := eff.Routes[prefix]
		if preset == "monitor" || preset == "off" {
			if cur == "strict" || cur == "attested" || cur == "balanced" {
				if cur == "attested" || cur == "strict" {
					raise(ClassD)
				} else {
					raise(ClassB)
				}
			}
		}
		if preset == "attested" || preset == "strict" {
			if cur == "monitor" || cur == "off" || cur == "" {
				raise(ClassA)
			}
		}
	}

	// rate loosen
	if o.RateLimit != nil {
		if o.RateLimit.WindowSec != nil && *o.RateLimit.WindowSec < eff.RateWindowSec {
			raise(ClassB)
		}
		if o.RateLimit.WindowSec != nil && *o.RateLimit.WindowSec > eff.RateWindowSec {
			raise(ClassA)
		}
		if o.RateLimit.Hard != nil && *o.RateLimit.Hard > eff.RateHard {
			raise(ClassB)
		}
		if o.RateLimit.Soft != nil && *o.RateLimit.Soft > eff.RateSoft {
			raise(ClassB)
		}
		if o.RateLimit.Hard != nil && *o.RateLimit.Hard < eff.RateHard {
			raise(ClassA)
		}
		if o.RateLimit.Soft != nil && *o.RateLimit.Soft < eff.RateSoft {
			raise(ClassA)
		}
	}

	return class
}
