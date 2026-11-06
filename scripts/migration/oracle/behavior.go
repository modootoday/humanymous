package main

import (
	"encoding/json"
	"os"
	"reflect"

	"github.com/modootoday/humanymous/internal/behavior"
	"github.com/modootoday/humanymous/internal/signals"
)

func exportBehavior() {
	type vector struct {
		Input    signals.BehaviorSummary `json:"input"`
		Expected behavior.FeatureVector  `json:"expected"`
	}
	var populated signals.BehaviorSummary
	populate(reflect.ValueOf(&populated).Elem())
	inputs := []signals.BehaviorSummary{{}, populated, {Key: signals.KeyFeatures{Keystrokes: 20, MeanDwellMs: 90}, DurationS: 5}, {Mouse: signals.MouseFeatures{Samples: -1, PauseCount: -1, MaxTeleportPx: -1}, DurationS: -1}}
	for i := 1; i <= 64; i++ {
		value := populated
		value.Mouse.Samples = i
		value.Key.Keystrokes = i * 2
		value.Mouse.MaxTeleportPx = float64(i) / 10
		value.DurationS = float64(i) * 10000
		inputs = append(inputs, value)
	}
	var vectors []vector
	for _, input := range inputs {
		vectors = append(vectors, vector{input, behavior.Extract(input)})
	}
	encoder := json.NewEncoder(os.Stdout)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(struct {
		Names   []string `json:"names"`
		Hash    string   `json:"hash"`
		Vectors []vector `json:"vectors"`
	}{behavior.FeatureNames(), behavior.SchemaHash(), vectors}); err != nil {
		panic(err)
	}
}
