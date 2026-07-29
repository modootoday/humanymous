package main

import (
	"encoding/json"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/behavior"
	"github.com/modootoday/humanymous/internal/mlcorrect"
	"github.com/modootoday/humanymous/internal/mlserve"
	"github.com/modootoday/humanymous/internal/mltrain"
	"github.com/modootoday/humanymous/internal/scoring"
	"github.com/modootoday/humanymous/internal/signals"
)

// stubPredictor makes mlserve.Score return a fixed p(bot) with a matching schema so feedPassOutcome
// does not abstain.
type stubPredictor struct{ p float32 }

func (s stubPredictor) Predict(behavior.FeatureVector) mlserve.Prediction {
	return mlserve.Prediction{PBot: s.p}
}
func (s stubPredictor) SchemaHash() string    { return behavior.SchemaHash() }
func (s stubPredictor) BundleVersion() string { return "stub" }

func TestFeedPassOutcome_SolvedRaisesThresholdOnOverFlaggedHumans(t *testing.T) {
	mlserve.Set(stubPredictor{p: 0.9}) // the model over-flags these confirmed humans
	defer mlserve.Set(nil)
	a := &app{ctrl: mlcorrect.NewController(0.01, 0.5, 0.05)}
	var rep signals.SessionReport
	rep.Scoring.Verdict = scoring.VerdictAllow

	start := a.ctrl.FireThreshold()
	for i := 0; i < 300; i++ {
		a.feedPassOutcome(rep, true)
	}
	if a.ctrl.FireThreshold() <= start {
		t.Fatalf("solved-Pass humans the model over-flags must raise θ: start=%v now=%v", start, a.ctrl.FireThreshold())
	}
	if s := a.ctrl.Snapshot(); s.Calibration.HumansSeen != 300 {
		t.Fatalf("300 solved Passes must calibrate as 300 humans, got %d", s.Calibration.HumansSeen)
	}
}

func TestFeedPassOutcome_FailedIsNotABotLabel(t *testing.T) {
	mlserve.Set(stubPredictor{p: 0.9})
	defer mlserve.Set(nil)
	a := &app{ctrl: mlcorrect.NewController(0.01, 0.5, 0.05)}
	for i := 0; i < 100; i++ {
		a.feedPassOutcome(signals.SessionReport{}, false)
	}
	// failures feed only the solve-rate guard — never a human/bot oracle label (ACC-1).
	if s := a.ctrl.Snapshot(); s.Calibration.HumansSeen != 0 || s.Calibration.BotsSeen != 0 {
		t.Fatalf("failed Pass must not label human/bot: %+v", s.Calibration)
	}
}

func TestFeedPassOutcome_NilControllerIsNoop(t *testing.T) {
	a := &app{} // no bundle → ctrl nil, no sink
	a.feedPassOutcome(signals.SessionReport{}, true)
	a.feedPassOutcome(signals.SessionReport{}, false) // must not panic
}

func TestFeedPassOutcome_SolvedWritesHumanTrace(t *testing.T) {
	path := filepath.Join(t.TempDir(), "oracle.jsonl")
	sink, err := mltrain.NewTraceSink(path)
	if err != nil {
		t.Fatal(err)
	}
	a := &app{traceSink: sink} // sink on, no controller
	var rep signals.SessionReport
	rep.Timestamp = time.Unix(4242, 0)
	// a pointer-using session → the "pointer" cohort.
	rep.Client.Behavior = signals.BehaviorSummary{DurationS: 7, Mouse: signals.MouseFeatures{Samples: 40}}

	a.feedPassOutcome(rep, true)  // confirmed human → one trace
	a.feedPassOutcome(rep, false) // failure → NOT a trace (only the solve-rate guard, absent here)
	if got := sink.Count(); got != 1 {
		t.Fatalf("only the solved Pass must be persisted as a trace, got %d", got)
	}

	sink.Close()
	data, _ := os.ReadFile(path)
	var r mltrain.Record
	if err := json.Unmarshal([]byte(strings.TrimSpace(string(data))), &r); err != nil {
		t.Fatalf("trace parse: %v", err)
	}
	if r.Label != 0 || r.Source != "pass" || r.TS != 4242 || r.Cohort != "pointer" {
		t.Fatalf("trace must be a labeled, cohort-tagged human Pass record, got %+v", r)
	}
}

func TestCohortOf_InputModality(t *testing.T) {
	cases := []struct {
		name string
		b    signals.BehaviorSummary
		want string
	}{
		{"pointer", signals.BehaviorSummary{Mouse: signals.MouseFeatures{Samples: 30}}, "pointer"},
		{"keyboard", signals.BehaviorSummary{Key: signals.KeyFeatures{Keystrokes: 12}}, "keyboard"},
		{"keyboard-few-mouse", signals.BehaviorSummary{Mouse: signals.MouseFeatures{Samples: 2}, Key: signals.KeyFeatures{Keystrokes: 5}}, "keyboard"},
		{"default", signals.BehaviorSummary{}, "default"},
	}
	for _, c := range cases {
		if got := cohortOf(c.b); got != c.want {
			t.Errorf("%s: cohortOf = %q, want %q", c.name, got, c.want)
		}
	}
}

func TestHandleMLCorrect_GateAndShape(t *testing.T) {
	// unauthorized → 404 (non-discoverable)
	a := &app{opsToken: "secret", ctrl: mlcorrect.NewController(0.01, 0.5, 0.01)}
	w := httptest.NewRecorder()
	a.handleMLCorrect(w, httptest.NewRequest("GET", "/api/mlcorrect", nil))
	if w.Code != 404 {
		t.Fatalf("missing bearer must 404, got %d", w.Code)
	}

	// authorized + controller → enabled snapshot
	w = httptest.NewRecorder()
	r := httptest.NewRequest("GET", "/api/mlcorrect", nil)
	r.Header.Set("Authorization", "Bearer secret")
	a.handleMLCorrect(w, r)
	if w.Code != 200 || !strings.Contains(w.Body.String(), `"enabled":true`) || !strings.Contains(w.Body.String(), "fireThreshold") {
		t.Fatalf("authorized snapshot must be enabled with a threshold: code=%d body=%s", w.Code, w.Body.String())
	}

	// authorized but no controller → disabled marker, not 404
	a2 := &app{opsToken: "secret"}
	w = httptest.NewRecorder()
	r = httptest.NewRequest("GET", "/api/mlcorrect", nil)
	r.Header.Set("Authorization", "Bearer secret")
	a2.handleMLCorrect(w, r)
	if w.Code != 200 || !strings.Contains(w.Body.String(), `"enabled":false`) {
		t.Fatalf("no controller must report enabled:false, got code=%d body=%s", w.Code, w.Body.String())
	}
}
