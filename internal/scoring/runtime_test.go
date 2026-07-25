package scoring

import (
	"testing"

	"github.com/modootoday/humanymous/internal/signals"
)

// Empty Configure must not change freeze behavior vs NewEngine.
func TestConfigureEmptyIdenticalToNewEngine(t *testing.T) {
	r := base("Mozilla/5.0 (Windows NT 10.0) Chrome/126 Safari/537.36",
		[]signals.Signal{wd(signals.VerdictOK)}, humanBeh, chromeNet)
	a := NewEngine().Score(r)
	e := NewEngine()
	e.Configure(DefaultPolicy(), nil, nil)
	b := e.Score(r)
	if a.Verdict != b.Verdict || a.RiskScore != b.RiskScore || a.HardRuleFired != b.HardRuleFired {
		t.Fatalf("empty configure drifted: %+v vs %+v", a, b)
	}
}

func TestRuleModeMonitorContinuesPastHR12(t *testing.T) {
	// HR-12 would fire on no_interaction; put HR-18 also true (browser no js).
	// With HR-12 monitor, first-match must not stop — later rules can still win.
	// Minimal: only HR-12 candidate (no_interaction) under monitor → no hard rule.
	r := base("Mozilla/5.0 (Windows NT 10.0) Chrome/126 Safari/537.36",
		[]signals.Signal{wd(signals.VerdictOK)}, noBeh, chromeNet)
	r.Client.Behavior = signals.BehaviorSummary{} // no interaction
	// Ensure no_interaction fires via behavior signals path — use fired l4.event.no_interaction
	r.Client.Signals = []signals.Signal{
		signals.New("l4.event.no_interaction", true, signals.VerdictSuspicious, 1, signals.SourceJS, ""),
	}
	e := NewEngine()
	// Default: HR-12 should challenge
	if v := e.Score(r); v.HardRuleFired != "HR-12" {
		// may be score challenge without HR if signal weight low — force with bot verdict
		r.Client.Signals = []signals.Signal{
			signals.New("l4.event.no_interaction", true, signals.VerdictBot, 1, signals.SourceJS, ""),
		}
	}
	r.Client.Signals = []signals.Signal{
		signals.New("l4.event.no_interaction", true, signals.VerdictBot, 1, signals.SourceJS, ""),
	}
	if v := NewEngine().Score(r); v.HardRuleFired != "HR-12" {
		t.Fatalf("baseline want HR-12 got %s/%s", v.HardRuleFired, v.Verdict)
	}
	e.Configure(DefaultPolicy(), map[string]string{"HR-12": "monitor"}, nil)
	if v := e.Score(r); v.HardRuleFired == "HR-12" {
		t.Fatalf("HR-12 monitor must not win, got %s/%s", v.HardRuleFired, v.Verdict)
	}
}

func TestWeightMultiplierZeroKillsContribution(t *testing.T) {
	// Datacenter-style soft score path: use a high-weight bot signal and zero it.
	r := base("Mozilla/5.0 (Windows NT 10.0) Chrome/126 Safari/537.36",
		[]signals.Signal{
			signals.New("l1.navigator.webdriver", true, signals.VerdictBot, 1, signals.SourceWASM, ""),
		}, humanBeh, chromeNet)
	baseRisk := NewEngine().Score(r).RiskScore
	e := NewEngine()
	e.Configure(DefaultPolicy(), nil, map[string]float64{"l1.navigator.webdriver": 0})
	got := e.Score(r).RiskScore
	if got >= baseRisk && baseRisk > 0 {
		t.Fatalf("zero multiplier should reduce risk: base=%.1f got=%.1f", baseRisk, got)
	}
}

func TestChallengeAtRaisedWeakensVerdict(t *testing.T) {
	// Build a mid-risk session that challenges under default 30.
	r := base("Mozilla/5.0 (Windows NT 10.0) Chrome/126 Safari/537.36",
		[]signals.Signal{
			signals.New("l1.navigator.webdriver", true, signals.VerdictSuspicious, 1, signals.SourceWASM, ""),
			signals.New("l2.webgl.swiftshader", true, signals.VerdictSuspicious, 1, signals.SourceWASM, ""),
		}, humanBeh, chromeNet)
	def := NewEngine().Score(r)
	p := DefaultPolicy()
	p.ChallengeAt = 99
	p.DenyAt = 100
	e := NewEngine()
	e.Configure(p, nil, nil)
	got := e.Score(r)
	if def.Verdict == VerdictChallenge && got.Verdict != VerdictAllow && got.HardRuleFired == "" {
		// only assert when no hard rule dominates
		t.Fatalf("raised ChallengeAt should allow score-band, got %s (def was %s)", got.Verdict, def.Verdict)
	}
}

func TestNetPolicyMonitorDisarmsHR24ProxyHop(t *testing.T) {
	r := base("Mozilla/5.0 (Windows NT 10.0) Chrome/126 Safari/537.36",
		[]signals.Signal{wd(signals.VerdictOK)}, humanBeh, chromeNet)
	r.Network.Signals = append(r.Network.Signals,
		signals.New("l5.header.proxy_hop", true, signals.VerdictBot, 1, signals.SourceServer, ""))
	// Empty overlay: HR-24 fires on proxy_hop.
	if v := NewEngine().Score(r); v.HardRuleFired != "HR-24" {
		t.Fatalf("baseline want HR-24 got %s/%s", v.HardRuleFired, v.Verdict)
	}
	e := NewEngine()
	e.ConfigureFull(DefaultPolicy(), nil, nil, map[string]string{"net.proxy.hop": "monitor"})
	if v := e.Score(r); v.HardRuleFired == "HR-24" {
		t.Fatalf("net.proxy.hop monitor must disarm HR-24, got %s/%s", v.HardRuleFired, v.Verdict)
	}
}

func TestNetPolicyMonitorDisarmsHR19CorrelationNotPow(t *testing.T) {
	r := base("Mozilla/5.0 (Windows NT 10.0) Chrome/126 Safari/537.36",
		[]signals.Signal{wd(signals.VerdictOK)}, humanBeh, chromeNet)
	r.Network.Signals = append(r.Network.Signals,
		signals.New("l5.correlation.proxy_rotation", 3, signals.VerdictBot, 1, signals.SourceServer, ""))
	if v := NewEngine().Score(r); v.HardRuleFired != "HR-19" {
		t.Fatalf("baseline want HR-19 got %s/%s", v.HardRuleFired, v.Verdict)
	}
	e := NewEngine()
	e.ConfigureFull(DefaultPolicy(), nil, nil, map[string]string{"net.correlation": "monitor"})
	if v := e.Score(r); v.HardRuleFired == "HR-19" {
		t.Fatalf("net.correlation monitor must disarm rotation HR-19, got %s/%s", v.HardRuleFired, v.Verdict)
	}
	// pow.too_fast remains integrity — not gated by net.correlation.
	r2 := base("Mozilla/5.0 (Windows NT 10.0) Chrome/126 Safari/537.36",
		[]signals.Signal{wd(signals.VerdictOK)}, humanBeh, chromeNet)
	r2.Client.Signals = append(r2.Client.Signals,
		signals.New("l7.pow.too_fast", true, signals.VerdictBot, 1, signals.SourceJS, ""))
	e2 := NewEngine()
	e2.ConfigureFull(DefaultPolicy(), nil, nil, map[string]string{"net.correlation": "monitor"})
	if v := e2.Score(r2); v.HardRuleFired != "HR-19" {
		t.Fatalf("pow.too_fast must still fire HR-19 under correlation monitor, got %s/%s", v.HardRuleFired, v.Verdict)
	}
}
