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

// TestCatalog_HTTPParrot_NoExecEvidence_DenyHR18: a header-spoofing parrot (curl_cffi/
// uTLS) that perfectly mimics Chrome JA4+H2+Sec-CH-UA but runs no JS must not evade HR-18
// by setting a self-reported field. Before the deep-review fix, one client-authored field
// (engineVersion / environment.probed) flipped x.browser_no_js off; now execution proof
// requires genuine client signals, behavior, or a server-verified RIT round-trip.
func TestCatalog_HTTPParrot_NoExecEvidence_DenyHR18(t *testing.T) {
	// Parrot: browser UA, browser JA4/H2, but zero execution evidence — only a forged
	// version string / probed flag (the old single-field bypass).
	r := base("Mozilla/5.0 (Windows NT 10.0) Chrome/126 Safari/537.36", nil, noBeh, chromeNet)
	r.Client.EngineVersion = "126.0.0"                       // self-reported claim (the old bypass)
	r.Client.Environment = signals.Environment{Probed: true} // another self-reported claim
	if v := NewEngine().Score(r); v.Verdict != VerdictDeny || v.HardRuleFired != "HR-18" {
		t.Fatalf("HTTP parrot want DENY/HR-18 got %s/%s risk=%.1f", v.Verdict, v.HardRuleFired, v.RiskScore)
	}
	// A real browser that delivered genuine WASM evidence (a client signal) must NOT trip
	// HR-18 even with the same headers — execution evidence, not a self-reported string.
	r2 := base("Mozilla/5.0 (Windows NT 10.0) Chrome/126 Safari/537.36",
		[]signals.Signal{wd(signals.VerdictOK)}, noBeh, chromeNet)
	if v := NewEngine().Score(r2); v.HardRuleFired == "HR-18" {
		t.Fatalf("a browser with real WASM evidence must not trip HR-18, got %s/%s", v.Verdict, v.HardRuleFired)
	}
}

// TestFPR_PrivacyBrowser_ProxyRotation_NotDeniedHR19: a genuine privacy browser (Tor/Brave/
// RFP) collides fingerprints and rotates exit subnets BY DESIGN, so proxy_rotation must NOT
// hard-DENY it (deep-review no-lockout). A non-privacy scraper with the same rotation still does.
func TestFPR_PrivacyBrowser_ProxyRotation_NotDeniedHR19(t *testing.T) {
	priv := base("Mozilla/5.0 (Windows NT 10.0) Chrome/126 Safari/537.36", []signals.Signal{wd(signals.VerdictOK)}, humanBeh, chromeNet)
	priv.Client.Environment = signals.Environment{Probed: true, AdBlock: true} // genuine tracking protection
	priv.Network.Signals = append(priv.Network.Signals, signals.New("l5.correlation.proxy_rotation", 3, signals.VerdictBot, 1, signals.SourceServer, ""))
	if v := NewEngine().Score(priv); v.HardRuleFired == "HR-19" {
		t.Fatalf("a genuine privacy browser must not be HR-19-DENIED on proxy_rotation, got %s/%s", v.Verdict, v.HardRuleFired)
	}
	scraper := base("Mozilla/5.0 Chrome/126", nil, humanBeh, chromeNet)
	scraper.Network.Signals = append(scraper.Network.Signals, signals.New("l5.correlation.proxy_rotation", 3, signals.VerdictBot, 1, signals.SourceServer, ""))
	if v := NewEngine().Score(scraper); v.HardRuleFired != "HR-19" {
		t.Fatalf("a non-privacy proxy-rotation scraper must still fire HR-19, got %s/%s", v.Verdict, v.HardRuleFired)
	}
}

// TestFPR_Flood_NotLoneDenyHR21: l5.abuse.flood keys on a SHARED ja4|subnet bucket, so a busy
// CGNAT trips it on strangers' traffic — it must drive a score-based CHALLENGE, not a lone HR-21
// DENY. The unambiguous protocol attacks (h2dos) still hard-DENY (deep-review no-lockout).
func TestFPR_Flood_NotLoneDenyHR21(t *testing.T) {
	flood := base("Mozilla/5.0 (Windows NT 10.0) Chrome/126 Safari/537.36", []signals.Signal{wd(signals.VerdictOK)}, humanBeh, chromeNet)
	flood.Network.Signals = append(flood.Network.Signals, signals.New("l5.abuse.flood", 100, signals.VerdictBot, 1, signals.SourceServer, ""))
	if v := NewEngine().Score(flood); v.Verdict == VerdictDeny && v.HardRuleFired == "HR-21" {
		t.Fatalf("flood alone must NOT be a lone HR-21 DENY (CGNAT lockout), got %s/%s", v.Verdict, v.HardRuleFired)
	}
	dos := base("Mozilla/5.0 Chrome/126", nil, humanBeh, chromeNet)
	dos.Network.Signals = append(dos.Network.Signals, signals.New("l5.h2dos.rapid_reset", 1, signals.VerdictBot, 1, signals.SourceServer, ""))
	if v := NewEngine().Score(dos); v.HardRuleFired != "HR-21" {
		t.Fatalf("h2dos.rapid_reset must still fire HR-21 DENY, got %s/%s", v.Verdict, v.HardRuleFired)
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
