package main

import (
	"encoding/json"
	"os"

	"github.com/modootoday/humanymous/internal/scoring"
	"github.com/modootoday/humanymous/internal/signals"
)

func exportCrossChecks() {
	type vector struct {
		Report   map[string]any       `json:"report"`
		Expected []signals.CrossCheck `json:"expected"`
	}
	var vectors []vector
	engines := []string{"", "unknown", "chrome", "firefox", "go"}
	for i, userAgent := range []string{"", "Mozilla Chrome Windows", "Mozilla Firefox Linux", "Mozilla Safari Macintosh", "Mozilla Chrome Android", "Mozilla Safari iPhone", "Mozilla Chrome CrOS", "python-requests/2", "library\t\"한국어\"\u200b"} {
		for j, platform := range []string{"", "\"Windows\"", "Android", "iOS", "macOS", "Linux", "Chrome OS"} {
			for flags := 0; flags < 8; flags++ {
				bit := func(position uint) bool { return flags&(1<<position) != 0 }
				var observations []signals.Signal
				if bit(0) {
					observations = []signals.Signal{signals.New("l1.navigator.webdriver", false, signals.VerdictOK, 1, signals.SourceWASM, "")}
				}
				samples := 0
				if bit(2) {
					samples = 1
				}
				input := map[string]any{"client": map[string]any{"userAgent": userAgent, "signals": observations, "advanced": map[string]any{"probed": bit(1)}, "environment": map[string]any{"probed": true}, "engineVersion": "120", "behavior": map[string]any{"mouse": map[string]any{"samples": samples}, "key": map[string]any{"keystrokes": 1}}, "uaClientHints": map[string]any{"brands": []signals.UABrand{{Brand: "sample", Version: "1"}}, "platform": platform}}, "network": map[string]any{"ja4Engine": engines[j%len(engines)], "h2Engine": engines[i%len(engines)], "secChUaPresent": bit(0), "secFetchPresent": bit(1)}}
				raw, err := json.Marshal(input)
				if err != nil {
					panic(err)
				}
				var report signals.SessionReport
				if err := json.Unmarshal(raw, &report); err != nil {
					panic(err)
				}
				vectors = append(vectors, vector{input, scoring.CrossChecks(&report)})
			}
		}
	}
	encoder := json.NewEncoder(os.Stdout)
	if err := encoder.Encode(vectors); err != nil {
		panic(err)
	}
}
