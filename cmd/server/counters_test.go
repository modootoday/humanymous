package main

import (
	"encoding/json"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/correlation"
	"github.com/modootoday/humanymous/internal/mlcorrect"
)

func getCounters(t *testing.T, a *app) map[string]any {
	t.Helper()
	w := httptest.NewRecorder()
	r := httptest.NewRequest("GET", "/api/counters", nil)
	r.Header.Set("Authorization", "Bearer secret")
	a.handleCounters(w, r)
	if w.Code != 200 {
		t.Fatalf("handleCounters code = %d, want 200; body=%s", w.Code, w.Body.String())
	}
	var body map[string]any
	if err := json.Unmarshal(w.Body.Bytes(), &body); err != nil {
		t.Fatalf("decode counters: %v", err)
	}
	return body
}

// A tripped canary must be visible in the /api/counters rollup — that is the on-call tell for a
// behavioral-model self-disable (whose rollback is otherwise only logged).
func TestHandleCounters_ShowsTrippedCanary(t *testing.T) {
	c := mlcorrect.NewController(0.005, 0.5, 0.0) // gamma 0 → θ frozen so the model keeps over-flagging
	tripped := false
	c.ArmCanary(mlcorrect.CanaryBudget{MaxHumanFP: 0.10, ProbationHumans: 100000}, func(string) { tripped = true })
	for i := 0; i < 400; i++ {
		c.ObserveOutcome(mlcorrect.OutcomePassSolved, 0.9, false) // confirmed humans the model over-flags
	}
	if !tripped {
		t.Fatal("precondition: sustained over-flagging should trip the canary")
	}

	a := &app{opsToken: "secret", ctrl: c, corr: correlation.New(time.Hour), started: time.Now()}
	body := getCounters(t, a)
	ml, ok := body["ml"].(map[string]any)
	if !ok || ml["enabled"] != true {
		t.Fatalf("ml block must be enabled, got %v", body["ml"])
	}
	if ml["canaryState"] != "tripped" {
		t.Fatalf("a tripped canary must surface at /api/counters, got canaryState=%v", ml["canaryState"])
	}
	if _, ok := body["fleet"].(map[string]any); !ok {
		t.Fatalf("fleet block must always be present, got %v", body["fleet"])
	}
}

// With no model and no trace sink loaded, the rollup must be nil-safe and honest (enabled:false),
// and the pre-existing counters must be unchanged.
func TestHandleCounters_NilSafeWithoutModel(t *testing.T) {
	a := &app{opsToken: "secret", corr: correlation.New(time.Hour), started: time.Now()}
	body := getCounters(t, a) // must not panic
	ml, ok := body["ml"].(map[string]any)
	if !ok || ml["enabled"] != false {
		t.Fatalf("ml.enabled must be false with no model, got %v", body["ml"])
	}
	if _, ok := body["version"]; !ok {
		t.Fatal("pre-existing counters (version, …) must be preserved")
	}
	fleet, ok := body["fleet"].(map[string]any)
	if !ok {
		t.Fatalf("fleet block must be present, got %v", body["fleet"])
	}
	if fleet["redisFallbackTotal"].(float64) != 0 {
		t.Fatalf("per-process registry must report 0 fleet fallbacks, got %v", fleet["redisFallbackTotal"])
	}
}
