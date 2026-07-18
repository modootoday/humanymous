package scoring

import (
	"testing"

	"github.com/modootoday/humanymous/internal/signals"
)

// sig is a test helper to build a scored client signal via the registry.
func sig(id string, v signals.Verdict, conf float64) signals.Signal {
	return signals.New(id, v == signals.VerdictBot, v, conf, signals.SourceWASM, "")
}

func TestScore_HumanChrome_Allow(t *testing.T) {
	r := &signals.SessionReport{
		Client: signals.ClientReport{
			UserAgent:     "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36",
			UAClientHints: signals.UAClientHints{Platform: "Windows", Brands: []signals.UABrand{{Brand: "Chromium", Version: "126"}}},
			Signals: []signals.Signal{
				sig("l1.navigator.webdriver", signals.VerdictOK, 1),
			},
			Environment: signals.Environment{Probed: true, AdBlock: true},
		},
		Network: signals.NetworkReport{
			JA4Engine: "chrome", H2Engine: "chrome",
			SecFetchPresent: true, SecCHUAPresent: true,
		},
	}
	res := NewEngine().Score(r)
	if res.Verdict != VerdictAllow {
		t.Fatalf("expected ALLOW, got %s (risk=%.1f rule=%s)", res.Verdict, res.RiskScore, res.HardRuleFired)
	}
}

func TestScore_Selenium_DenyHR1(t *testing.T) {
	r := &signals.SessionReport{
		Client: signals.ClientReport{
			UserAgent: "Mozilla/5.0 Chrome/126.0",
			Signals: []signals.Signal{
				sig("l1.artifact.selenium", signals.VerdictBot, 1),
				sig("l1.navigator.webdriver", signals.VerdictBot, 1),
			},
		},
		Network: signals.NetworkReport{JA4Engine: "chrome", H2Engine: "chrome", SecFetchPresent: true, SecCHUAPresent: true},
	}
	res := NewEngine().Score(r)
	if res.Verdict != VerdictDeny || res.HardRuleFired != "HR-1" {
		t.Fatalf("expected DENY/HR-1, got %s/%s", res.Verdict, res.HardRuleFired)
	}
}

func TestScore_UAvsTLSandH2_DenyHR2(t *testing.T) {
	// UA claims Chrome, but TLS and HTTP/2 both resolve to Go.
	r := &signals.SessionReport{
		Client: signals.ClientReport{
			UserAgent: "Mozilla/5.0 (Windows NT 10.0) Chrome/126.0 Safari/537.36",
			Signals:   []signals.Signal{},
		},
		Network: signals.NetworkReport{JA4Engine: "go", H2Engine: "go", SecFetchPresent: false, SecCHUAPresent: false},
	}
	res := NewEngine().Score(r)
	if res.Verdict != VerdictDeny || res.HardRuleFired != "HR-2" {
		t.Fatalf("expected DENY/HR-2, got %s/%s (risk=%.1f)", res.Verdict, res.HardRuleFired, res.RiskScore)
	}
}

func TestScore_NonBrowserHTTPClient(t *testing.T) {
	// A Go http client: non-browser UA, no client report.
	r := &signals.SessionReport{
		Client:  signals.ClientReport{UserAgent: "Go-http-client/2.0"},
		Network: signals.NetworkReport{JA4Engine: "go", H2Engine: "go"},
	}
	res := NewEngine().Score(r)
	if res.Verdict == VerdictAllow {
		t.Fatalf("expected non-ALLOW for Go http client, got ALLOW (risk=%.1f)", res.RiskScore)
	}
}

func TestScore_SyntheticEventsWithWebdriver_DenyHR3(t *testing.T) {
	r := &signals.SessionReport{
		Client: signals.ClientReport{
			UserAgent: "Mozilla/5.0 Chrome/126.0",
			Signals:   []signals.Signal{sig("l1.navigator.webdriver", signals.VerdictBot, 1)},
			Behavior: signals.BehaviorSummary{
				Events: signals.EventFeatures{TotalEvents: 10, UntrustedFrac: 0.5},
			},
		},
		Network: signals.NetworkReport{JA4Engine: "chrome", H2Engine: "chrome", SecFetchPresent: true, SecCHUAPresent: true},
	}
	res := NewEngine().Score(r)
	if res.Verdict != VerdictDeny {
		t.Fatalf("expected DENY, got %s/%s", res.Verdict, res.HardRuleFired)
	}
}

func TestScore_NoClient_ChallengeHR10(t *testing.T) {
	r := &signals.SessionReport{
		Client:  signals.ClientReport{}, // nothing collected
		Network: signals.NetworkReport{JA4Engine: "chrome", H2Engine: "chrome"},
	}
	res := NewEngine().Score(r)
	if res.Verdict != VerdictChallenge || res.HardRuleFired != "HR-10" {
		t.Fatalf("expected CHALLENGE/HR-10, got %s/%s", res.Verdict, res.HardRuleFired)
	}
}

func TestScore_Deterministic(t *testing.T) {
	build := func() *signals.SessionReport {
		return &signals.SessionReport{
			Client:  signals.ClientReport{UserAgent: "Mozilla/5.0 Chrome/126.0", Signals: []signals.Signal{sig("l2.webgl.swiftshader", signals.VerdictBot, 1)}},
			Network: signals.NetworkReport{JA4Engine: "chrome", H2Engine: "chrome", SecFetchPresent: true, SecCHUAPresent: true},
		}
	}
	a := NewEngine().Score(build())
	b := NewEngine().Score(build())
	if a.RiskScore != b.RiskScore || a.Verdict != b.Verdict {
		t.Fatalf("non-deterministic: %+v vs %+v", a, b)
	}
}
