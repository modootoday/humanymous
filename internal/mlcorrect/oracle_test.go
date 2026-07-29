package mlcorrect

import "testing"

func TestConfirm_LabelMapping(t *testing.T) {
	cases := []struct {
		o    Outcome
		want Label
		auto bool
	}{
		{OutcomePassSolved, LabelHuman, true},
		{OutcomeCatalogBot, LabelBot, true},
		{OutcomeChallengeFailed, LabelBot, true},
		{OutcomeChallengeAbandoned, LabelBot, true},
		{OutcomeUnknown, LabelAmbiguous, false},
	}
	for _, c := range cases {
		l, a := Confirm(c.o)
		if l != c.want || a != c.auto {
			t.Errorf("Confirm(%v) = (%v,%v), want (%v,%v)", c.o, l, a, c.want, c.auto)
		}
	}
}

func TestPassGuard_CapsFraction(t *testing.T) {
	g := NewPassGuard(0.4, 0.6) // Pass may be at most 40% of a batch
	// with 600 gold (catalog/CHALLENGE) labels, Pass admitted <= 600 * 0.4/0.6 = 400,
	// so pass/(pass+other) = 400/1000 = 0.4.
	adm := g.AdmitPassFraction(600)
	if adm < 380 || adm > 400 {
		t.Fatalf("Pass admission cap off: got %d want ~400", adm)
	}
	frac := float64(adm) / float64(adm+600)
	if frac > 0.41 {
		t.Fatalf("admitted Pass fraction %.3f exceeds cap 0.40", frac)
	}
}

func TestPassGuard_AnomalyStopsIngestion(t *testing.T) {
	g := NewPassGuard(0.4, 0.6)
	if g.SolveRateAnomalous() {
		t.Fatal("baseline solve-rate must not be anomalous")
	}
	// adversaries solving Pass at ~100% → solve-rate climbs far above baseline.
	for i := 0; i < 5000; i++ {
		g.ObservePassAttempt(true)
	}
	if !g.SolveRateAnomalous() {
		t.Fatalf("a sustained solve-rate spike must be anomalous (got %.3f)", g.SolveRate())
	}
	// when anomalous, NO Pass-human labels are admitted (poisoning defense).
	if adm := g.AdmitPassFraction(600); adm != 0 {
		t.Fatalf("anomalous solve-rate must stop Pass ingestion, admitted %d", adm)
	}
}
