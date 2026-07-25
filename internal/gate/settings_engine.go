package gate

import (
	"github.com/modootoday/humanymous/internal/gate/settings"
	"github.com/modootoday/humanymous/internal/scoring"
)

// ConfigureEngineFromEffective maps SoT-39 Effective → scoring.Engine inputs (P2).
// scoring must not import settings (SRP / freeze purity).
func ConfigureEngineFromEffective(e *scoring.Engine, eff settings.Effective) {
	if e == nil {
		return
	}
	p := scoring.DefaultPolicy()
	p.ChallengeAt = eff.ChallengeAt
	p.DenyAt = eff.DenyAt
	p.LayerCap = eff.LayerCap
	// Keep policy version pin for empty overlay; bump only when non-default (P2 minimal).
	if !eff.EmptyOverlay {
		p.Version = "1.0.0+overlay"
	}
	var modes map[string]string
	if len(eff.HardRules) > 0 {
		modes = make(map[string]string, len(eff.HardRules))
		for k, m := range eff.HardRules {
			modes[k] = string(m)
		}
	}
	var net map[string]string
	if len(eff.NetPolicy) > 0 {
		net = make(map[string]string, len(eff.NetPolicy))
		for k, m := range eff.NetPolicy {
			net[k] = string(m)
		}
	}
	e.ConfigureFull(p, modes, eff.Weights, net)
}
