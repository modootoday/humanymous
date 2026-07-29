package main

import (
	"strings"
	"testing"

	"github.com/modootoday/humanymous/internal/behavior"
	"github.com/modootoday/humanymous/internal/mltrain"
)

// mkSample builds one labeled sample. Feature 0 is the discriminant (bots high, humans low); the
// rest is deterministic jitter so standardization has non-zero variance.
func mkSample(label int, cohort string, f0 float32, i int) mltrain.Sample {
	x := make(behavior.FeatureVector, behavior.FeatureDim)
	x[0] = f0 + float32(i%3)*0.02
	for j := 1; j < len(x); j++ {
		x[j] = float32((i+j)%5) * 0.01
	}
	return mltrain.Sample{X: x, Y: float32(label), Human: label == 0, TS: int64(i), Cohort: cohort}
}

// mkSamples produces a time-interleaved gold set: 3 catalog bots : 2 "typical" humans : 1 "edge"
// human per 6 records, so every TESSERACT window carries all three. When poisonEdge is set, the
// "edge" (accessibility-minority) humans are made bot-like — a model will over-flag them, which the
// per-cohort gate must catch even though the aggregate looks fine.
func mkSamples(poisonEdge bool) []mltrain.Sample {
	var s []mltrain.Sample
	for i := 0; i < 1200; i++ {
		switch i % 6 {
		case 0, 2, 4:
			s = append(s, mkSample(1, "", 3.0, i)) // bot
		case 1, 5:
			s = append(s, mkSample(0, "typical", 0.0, i)) // typical human
		default: // 3
			f0 := float32(0.0)
			if poisonEdge {
				f0 = 3.0 // edge humans look bot-like
			}
			s = append(s, mkSample(0, "edge", f0, i))
		}
	}
	return s
}

func testGateConfig() gateConfig {
	return gateConfig{
		Version: "test", Hidden: 8, Epochs: 40, LR: 0.1, Seed: 2, TestFrac: 0.25,
		MaxHumanFPR: 0.005, MinBotTPR: 0.80, MinCohortHumans: 20,
	}
}

func TestRunGate_CleanDataEmitsCandidate(t *testing.T) {
	res, err := runGate(mkSamples(false), nil, testGateConfig())
	if err != nil {
		t.Fatal(err)
	}
	if res.Candidate == nil || len(res.Failures) != 0 {
		t.Fatalf("clean data must pass every gate and emit a candidate; failures=%v", res.Failures)
	}
	if res.Cohorts["edge"].Humans < 20 || res.Cohorts["typical"].Humans < 20 {
		t.Fatalf("both cohorts must be represented in the test window: %+v", res.Cohorts)
	}
	if res.BotTPR < 0.80 {
		t.Fatalf("bot TPR must clear the no-regression floor, got %.3f", res.BotTPR)
	}
}

func TestRunGate_PoisonedCohortBlocksCandidate_FailClosed(t *testing.T) {
	res, err := runGate(mkSamples(true), nil, testGateConfig())
	if err != nil {
		t.Fatal(err)
	}
	// The aggregate can look acceptable while a minority cohort is wrecked — the per-cohort gate is
	// exactly what must catch it. No candidate may be emitted.
	if res.Candidate != nil {
		t.Fatalf("a poisoned accessibility cohort must block the candidate (aggregate FPR was %.3f)", res.AggHumanFPR)
	}
	joined := strings.Join(res.Failures, " | ")
	if !strings.Contains(joined, `cohort "edge"`) {
		t.Fatalf("the failure must name the regressed edge cohort, got: %s", joined)
	}
	if res.Cohorts["edge"].FPR <= res.Cohorts["typical"].FPR {
		t.Fatalf("the edge cohort must show the higher FPR: typical=%.3f edge=%.3f",
			res.Cohorts["typical"].FPR, res.Cohorts["edge"].FPR)
	}
}

func TestRunGate_TooFewSamplesIsAPreconditionError(t *testing.T) {
	if _, err := runGate(mkSamples(false)[:50], nil, testGateConfig()); err == nil {
		t.Fatal("fewer than 100 samples must be a precondition error, not a silent pass")
	}
}
