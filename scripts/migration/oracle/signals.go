package main

import (
	"encoding/json"
	"os"
	"sort"

	"github.com/modootoday/humanymous/internal/signals"
)

func exportSignals() {
	definitions := signals.All()
	sort.Slice(definitions, func(i, j int) bool { return definitions[i].ID < definitions[j].ID })
	verdicts := []signals.Verdict{signals.VerdictOK, signals.VerdictSuspicious, signals.VerdictBot, signals.VerdictUnknown, "future"}
	confidences := []float64{-1, 0, 0.25, 1, 2}
	var vectors []signals.Signal
	for i, definition := range definitions {
		vectors = append(vectors, signals.New(definition.ID, map[string]any{"observed": true}, verdicts[i%5], confidences[(i/5)%5], signals.SourceServer, "oracle"))
	}
	for _, id := range []string{"l9.migration.unknown", "l1.navigator.webdriver"} {
		for _, verdict := range verdicts {
			for _, confidence := range confidences {
				vectors = append(vectors, signals.New(id, nil, verdict, confidence, signals.SourceWASM, ""))
			}
		}
	}
	encoder := json.NewEncoder(os.Stdout)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(vectors); err != nil {
		panic(err)
	}
}
