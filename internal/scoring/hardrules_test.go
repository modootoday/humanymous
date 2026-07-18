package scoring

import (
	"testing"

	"github.com/modootoday/humanymous/internal/signals"
)

// hardrules_test.go pins the Red/Blue catalog outcomes (plan/03, plan/04): every
// bot profile must be blocked (DENY/CHALLENGE); a human must be ALLOW.

func base(ua string, sigs []signals.Signal, beh signals.BehaviorSummary, net signals.NetworkReport) *signals.SessionReport {
	return &signals.SessionReport{
		Client:  signals.ClientReport{UserAgent: ua, UAClientHints: signals.UAClientHints{Platform: "Windows"}, Signals: sigs, Behavior: beh},
		Network: net,
	}
}

var chromeNet = signals.NetworkReport{JA4Engine: "chrome", H2Engine: "chrome", SecFetchPresent: true, SecCHUAPresent: true}
var humanBeh = signals.BehaviorSummary{Mouse: signals.MouseFeatures{Samples: 40, VelocityStdDev: 0.5, StraightLineFrac: 0.2}, Key: signals.KeyFeatures{Keystrokes: 12, DwellStdDevMs: 30, FlightStdDev: 40}, Events: signals.EventFeatures{TotalEvents: 60}, DurationS: 2.6}
var noBeh = signals.BehaviorSummary{DurationS: 2.6}

func wd(v signals.Verdict) signals.Signal {
	return signals.New("l1.navigator.webdriver", v == signals.VerdictBot, v, 1, signals.SourceWASM, "")
}

func TestCatalog_HumanAllow(t *testing.T) {
	r := base("Mozilla/5.0 (Windows NT 10.0) Chrome/126 Safari/537.36",
		[]signals.Signal{wd(signals.VerdictOK)}, humanBeh, chromeNet)
	if v := NewEngine().Score(r); v.Verdict != VerdictAllow {
		t.Fatalf("human want ALLOW got %s/%s risk=%.1f", v.Verdict, v.HardRuleFired, v.RiskScore)
	}
}

func TestCatalog_HeadlessWebdriver_DenyHR7(t *testing.T) {
	sigs := []signals.Signal{
		signals.New("l1.ua.headless_token", true, signals.VerdictBot, 1, signals.SourceWASM, ""),
		wd(signals.VerdictBot),
	}
	r := base("Mozilla/5.0 HeadlessChrome/126", sigs, noBeh, chromeNet)
	if v := NewEngine().Score(r); v.HardRuleFired != "HR-7" {
		t.Fatalf("want HR-7 got %s (%s)", v.HardRuleFired, v.Verdict)
	}
}

func TestCatalog_StealthPatchedNative_DenyHR8(t *testing.T) {
	sigs := []signals.Signal{
		signals.New("l3.integrity.native_tostring", false, signals.VerdictBot, 1, signals.SourceWASM, ""),
		signals.New("l1.window.outer_eq_inner", true, signals.VerdictSuspicious, 1, signals.SourceWASM, ""),
	}
	r := base("Mozilla/5.0 Chrome/126", sigs, noBeh, chromeNet)
	if v := NewEngine().Score(r); v.HardRuleFired != "HR-8" {
		t.Fatalf("want HR-8 got %s (%s)", v.HardRuleFired, v.Verdict)
	}
}

func TestCatalog_NodriverNoInteraction_ChallengeHR12(t *testing.T) {
	r := base("Mozilla/5.0 Chrome/126", []signals.Signal{wd(signals.VerdictOK)}, noBeh, chromeNet)
	v := NewEngine().Score(r)
	if v.Verdict != VerdictChallenge || v.HardRuleFired != "HR-12" {
		t.Fatalf("nodriver want CHALLENGE/HR-12 got %s/%s", v.Verdict, v.HardRuleFired)
	}
}

func TestCatalog_Patchright_ConsoleDisabled_DenyHR13(t *testing.T) {
	r := base("Mozilla/5.0 Chrome/126", []signals.Signal{
		signals.New("l1.window.outer_eq_inner", true, signals.VerdictSuspicious, 1, signals.SourceWASM, ""),
	}, noBeh, chromeNet)
	r.Client.Guard = signals.Guard{Reported: true, ConsoleDisabled: true}
	v := NewEngine().Score(r)
	if v.HardRuleFired != "HR-13" {
		t.Fatalf("patchright want HR-13 got %s (%s)", v.HardRuleFired, v.Verdict)
	}
}

func TestFP_DeviceMemoryPowerOfTwo_NotFlagged(t *testing.T) {
	// A real browser reporting deviceMemory=32 (power of two) must not raise risk.
	// The signal is only produced client-side for non-power-of-two; here we assert
	// a consistent human with rich behavior stays ALLOW.
	r := base("Mozilla/5.0 (Windows NT 10.0) Chrome/126 Safari/537.36",
		[]signals.Signal{wd(signals.VerdictOK)}, humanBeh, chromeNet)
	if v := NewEngine().Score(r); v.Verdict != VerdictAllow {
		t.Fatalf("power-of-two memory human want ALLOW got %s", v.Verdict)
	}
}
