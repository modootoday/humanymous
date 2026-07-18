package scoring

import (
	"testing"

	"github.com/modootoday/humanymous/internal/signals"
)

func passSolved() signals.Signal {
	return signals.New("l7.pass.solved", true, signals.VerdictOK, 1, signals.SourceServer, "")
}

// passChallengeReport is a browser-claimed session with moderate suspicion — a
// SCORE-based CHALLENGE (no hard rule): a tampered-canvas tell + a datacenter IP.
func passChallengeReport() *signals.SessionReport {
	return &signals.SessionReport{
		Client: signals.ClientReport{
			UserAgent:     "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36",
			UAClientHints: signals.UAClientHints{Platform: "Windows", Brands: []signals.UABrand{{Brand: "Chromium", Version: "126"}}},
			Signals: []signals.Signal{
				sig("l2.webgl.swiftshader", signals.VerdictBot, 1),        // L2, weight 30
				sig("l2.webgl.param_consistency", signals.VerdictBot, 1),  // L2, weight 25
			},
			Environment: signals.Environment{Probed: true},
			Behavior:    signals.BehaviorSummary{Events: signals.EventFeatures{TotalEvents: 50, UntrustedFrac: 0}},
		},
		Network: signals.NetworkReport{
			JA4Engine: "chrome", H2Engine: "chrome", SecFetchPresent: true, SecCHUAPresent: true,
			Signals: []signals.Signal{
				signals.New("l5.header.order", true, signals.VerdictBot, 1, signals.SourceServer, ""), // L5, weight 20
			},
		},
	}
}

// TestPassUpgrade_ChallengeToAllow: clearing Pass upgrades a score-based CHALLENGE to
// ALLOW, exactly like the PoW upgrade (SoT-36 §3) — the point of the challenge.
func TestPassUpgrade_ChallengeToAllow(t *testing.T) {
	base := passChallengeReport()
	if v := NewEngine().Score(base); v.Verdict != VerdictChallenge || v.HardRuleFired != "" {
		t.Fatalf("baseline must be a score-based CHALLENGE, got %s/%s (risk %.1f)", v.Verdict, v.HardRuleFired, v.RiskScore)
	}
	withPass := passChallengeReport()
	withPass.Network.Signals = append(withPass.Network.Signals, passSolved())
	res := NewEngine().Score(withPass)
	if res.Verdict != VerdictAllow || res.HardRuleFired != "Pass-upgrade" {
		t.Fatalf("solving Pass must upgrade CHALLENGE->ALLOW, got %s/%s (risk %.1f)", res.Verdict, res.HardRuleFired, res.RiskScore)
	}
}

// TestPassUpgrade_NeverLaundersHardDeny: a session already proven a bot by a hard rule
// (selenium artifacts -> HR-1 DENY) stays DENY even after clearing Pass. Solving Pass
// is one signal among many; it never launders conclusive independent bot evidence.
func TestPassUpgrade_NeverLaundersHardDeny(t *testing.T) {
	r := &signals.SessionReport{
		Client: signals.ClientReport{
			UserAgent: "Mozilla/5.0 Chrome/126.0",
			Signals: []signals.Signal{
				sig("l1.artifact.selenium", signals.VerdictBot, 1),
				sig("l1.navigator.webdriver", signals.VerdictBot, 1),
			},
		},
		Network: signals.NetworkReport{
			JA4Engine: "chrome", H2Engine: "chrome", SecFetchPresent: true, SecCHUAPresent: true,
			Signals: []signals.Signal{passSolved()},
		},
	}
	res := NewEngine().Score(r)
	if res.Verdict != VerdictDeny {
		t.Fatalf("Pass must NEVER launder a hard-rule DENY, got %s/%s (risk %.1f)", res.Verdict, res.HardRuleFired, res.RiskScore)
	}
}
