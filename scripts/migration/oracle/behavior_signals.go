package main

import (
	"encoding/json"
	"os"

	"github.com/modootoday/humanymous/internal/behavior"
	"github.com/modootoday/humanymous/internal/mlserve"
	"github.com/modootoday/humanymous/internal/scoring"
	"github.com/modootoday/humanymous/internal/signals"
)

type fixedPrediction float32

func (p fixedPrediction) Predict(behavior.FeatureVector) mlserve.Prediction {
	return mlserve.Prediction{PBot: float32(p)}
}
func (p fixedPrediction) SchemaHash() string    { return behavior.SchemaHash() }
func (p fixedPrediction) BundleVersion() string { return "oracle" }

func exportBehaviorSignals() {
	type vector struct {
		Input       signals.BehaviorSummary `json:"input"`
		Probability *float32                `json:"probability"`
		Expected    []signals.Signal        `json:"expected"`
	}
	var vectors []vector
	defer mlserve.Set(nil)
	for sample := 0; sample < 80; sample++ {
		input := signals.BehaviorSummary{Mouse: signals.MouseFeatures{Samples: sample, MeanCurvature: 0.0005, VelocityStdDev: 0.01, StraightLineFrac: 0.9, PauseCount: 1, MaxTeleportPx: 401, CoalescedRatio: 1}, Key: signals.KeyFeatures{Keystrokes: sample / 2, MeanDwellMs: 5, DwellStdDevMs: 2, MeanFlightMs: 24, FlightStdDev: 4, ZeroDwellFrac: 0.6}, Events: signals.EventFeatures{TotalEvents: sample, UntrustedFrac: float64(sample % 2), NoMoveClicks: 1, SyntheticFlags: 1, ClickCount: 2, LongGapCount: 2, MinReactionMs: 149, BurstVarMs: 2499}, DurationS: float64(sample) / 10}
		for _, probability := range []float32{-1, 0.2, 0.5, 0.8} {
			var ptr *float32
			if probability < 0 {
				mlserve.Set(nil)
			} else {
				value := probability
				ptr = &value
				mlserve.Set(fixedPrediction(probability))
			}
			vectors = append(vectors, vector{input, ptr, scoring.BehaviorSignals(input)})
		}
	}
	encoder := json.NewEncoder(os.Stdout)
	if err := encoder.Encode(vectors); err != nil {
		panic(err)
	}
}
