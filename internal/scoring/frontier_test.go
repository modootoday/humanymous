package scoring

import (
	"testing"

	"github.com/modootoday/humanymous/internal/signals"
)

// frontier_test.go pins the round-3 detections: AI-agent behavioral (HR-20) and
// cross-session correlation / PoW solve-time (HR-19), plus their FP safety.

func TestAgent_TeleportClickPlusCDP_DenyHR20(t *testing.T) {
	r := base("Mozilla/5.0 Chrome/126", []signals.Signal{
		signals.New("l1.cdp.proxy_leak", true, signals.VerdictBot, 1, signals.SourceWASM, ""),
	}, signals.BehaviorSummary{
		// clicks with almost no approach trajectory + machine typing.
		Mouse:     signals.MouseFeatures{Samples: 2},
		Key:       signals.KeyFeatures{Keystrokes: 8, MeanFlightMs: 5},
		Events:    signals.EventFeatures{TotalEvents: 12, ClickCount: 4},
		DurationS: 5,
	}, chromeNet)
	v := NewEngine().Score(r)
	if v.HardRuleFired != "HR-20" {
		t.Fatalf("ai-agent want HR-20 got %s (%s)", v.HardRuleFired, v.Verdict)
	}
}

func TestAgent_HumanTrajectory_NotFlagged(t *testing.T) {
	// A human: rich mouse trajectory, human-speed typing, no teleport clicks.
	r := base("Mozilla/5.0 (Windows NT 10.0) Chrome/126 Safari/537.36",
		[]signals.Signal{},
		signals.BehaviorSummary{
			Mouse:     signals.MouseFeatures{Samples: 40, VelocityStdDev: 0.5, CoalescedRatio: 3.2},
			Key:       signals.KeyFeatures{Keystrokes: 10, MeanFlightMs: 110, DwellStdDevMs: 30, FlightStdDev: 40},
			Events:    signals.EventFeatures{TotalEvents: 60, ClickCount: 1},
			DurationS: 5,
		}, chromeNet)
	if v := NewEngine().Score(r); v.Verdict != VerdictAllow {
		t.Fatalf("human want ALLOW got %s/%s (risk=%.1f)", v.Verdict, v.HardRuleFired, v.RiskScore)
	}
}

func TestCorrelation_ProxyRotation_DenyHR19(t *testing.T) {
	r := base("Mozilla/5.0 Chrome/126", []signals.Signal{
		signals.New("l5.correlation.proxy_rotation", 3, signals.VerdictBot, 1, signals.SourceServer, ""),
	}, humanBeh, chromeNet)
	if v := NewEngine().Score(r); v.HardRuleFired != "HR-19" {
		t.Fatalf("proxy_rotation want HR-19 got %s (%s)", v.HardRuleFired, v.Verdict)
	}
}

func TestPoW_TooFast_DenyHR19(t *testing.T) {
	r := base("Mozilla/5.0 Chrome/126", []signals.Signal{
		signals.New("l7.pow.too_fast", 3, signals.VerdictBot, 1, signals.SourceServer, ""),
	}, humanBeh, chromeNet)
	if v := NewEngine().Score(r); v.HardRuleFired != "HR-19" {
		t.Fatalf("pow.too_fast want HR-19 got %s (%s)", v.HardRuleFired, v.Verdict)
	}
}
