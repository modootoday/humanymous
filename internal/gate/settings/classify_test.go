package settings

import "testing"

func TestClassifyHR12MonitorIsB(t *testing.T) {
	eff := Resolve(BootInput{HMACKey: []byte("k"), RateWindowSec: 10, RateSoft: 60, RateHard: 120}, nil)
	o := &Overlay{HardRules: map[string]Mode{"HR-12": ModeMonitor}}
	if c := Classify(eff, o); c != ClassB {
		t.Fatalf("got %s want B", c)
	}
}

func TestClassifyHR18MonitorIsC(t *testing.T) {
	eff := Resolve(BootInput{HMACKey: []byte("k"), RateWindowSec: 10, RateSoft: 60, RateHard: 120}, nil)
	o := &Overlay{HardRules: map[string]Mode{"HR-18": ModeMonitor}}
	if c := Classify(eff, o); c != ClassC {
		t.Fatalf("got %s want C", c)
	}
}

func TestClassifyChallengeAtBigRaiseIsC(t *testing.T) {
	eff := Resolve(BootInput{HMACKey: []byte("k"), RateWindowSec: 10, RateSoft: 60, RateHard: 120}, nil)
	ch := 50.0 // default 30 + 20
	o := &Overlay{Scoring: &ScoringPatch{ChallengeAt: &ch}}
	if c := Classify(eff, o); c != ClassC {
		t.Fatalf("got %s want C", c)
	}
}

func TestClassifyLowerChallengeIsA(t *testing.T) {
	eff := Resolve(BootInput{HMACKey: []byte("k"), RateWindowSec: 10, RateSoft: 60, RateHard: 120}, nil)
	ch := 20.0
	o := &Overlay{Scoring: &ScoringPatch{ChallengeAt: &ch}}
	if c := Classify(eff, o); c != ClassA {
		t.Fatalf("got %s want A", c)
	}
}

func TestClassifyNetCorrelationMonitorIsC(t *testing.T) {
	eff := Resolve(BootInput{HMACKey: []byte("k"), RateWindowSec: 10, RateSoft: 60, RateHard: 120}, nil)
	o := &Overlay{NetPolicy: map[string]Mode{"net.correlation": ModeMonitor}}
	if c := Classify(eff, o); c != ClassC {
		t.Fatalf("got %s want C", c)
	}
}

func TestClassifyRouteAttestedToMonitorIsD(t *testing.T) {
	eff := Resolve(BootInput{
		HMACKey: []byte("k"), RateWindowSec: 10, RateSoft: 60, RateHard: 120,
		Routes: map[string]string{"/checkout": "attested"},
	}, nil)
	o := &Overlay{Routes: map[string]string{"/checkout": "monitor"}}
	if c := Classify(eff, o); c != ClassD {
		t.Fatalf("got %s want D", c)
	}
}

func TestClassifyRateHardRaiseIsB(t *testing.T) {
	eff := Resolve(BootInput{HMACKey: []byte("k"), RateWindowSec: 10, RateSoft: 60, RateHard: 120}, nil)
	hard := 200
	o := &Overlay{RateLimit: &RateLimitPatch{Hard: &hard}}
	if c := Classify(eff, o); c != ClassB {
		t.Fatalf("got %s want B", c)
	}
}

func TestClassifyRateWindowShorterIsB(t *testing.T) {
	eff := Resolve(BootInput{HMACKey: []byte("k"), RateWindowSec: 10, RateSoft: 60, RateHard: 120}, nil)
	short := 5
	o := &Overlay{RateLimit: &RateLimitPatch{WindowSec: &short}}
	if got := Classify(eff, o); got != ClassB {
		t.Fatalf("shorter rate window weakens enforcement: want B got %s", got)
	}
}
