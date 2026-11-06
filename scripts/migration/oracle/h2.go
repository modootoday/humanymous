package main

import (
	"encoding/json"
	"github.com/modootoday/humanymous/internal/network"
	"os"
)

func exportH2() {
	type vector struct {
		Input  any    `json:"input"`
		Akamai string `json:"akamai"`
		Engine string `json:"engine"`
		Sorted []int  `json:"sorted"`
	}
	var vectors []vector
	for _, order := range [][]string{{}, {"m", "a", "s", "p"}, {"m", "p", "a", "s"}, {"m", "s", "p", "a"}, {"m", "a", "p", "s"}, {"m", "m"}} {
		for _, window := range []uint32{0, 15663105, 0xffffffff} {
			for _, settings := range [][]network.H2Setting{{}, {{ID: 3, Value: 100}, {ID: 4, Value: 65535}}, {{ID: 4, Value: 65535}, {ID: 4, Value: 1}, {ID: 3, Value: 100}}, {{ID: 1, Value: 65536}, {ID: 2, Value: 0}, {ID: 4, Value: 6291456}, {ID: 6, Value: 262144}}} {
				for _, priorities := range [][]string{{}, {"3:0:0:200", "5:1:3:100"}} {
					f := network.H2Fingerprint{Settings: settings, WindowUpdate: window, Priorities: priorities, HeadersPrio: "0:0:15", PseudoOrder: order, HeaderOrder: []string{"user-agent", "accept"}}
					pairs := []map[string]any{}
					for _, s := range settings {
						pairs = append(pairs, map[string]any{"id": s.ID, "value": s.Value})
					}
					input := map[string]any{"settings": pairs, "window_update": window, "priorities": priorities, "headers_priority": f.HeadersPrio, "pseudo_order": order, "header_order": f.HeaderOrder}
					vectors = append(vectors, vector{input, f.Akamai(), network.EngineFromH2(f), f.SortedSettingIDs()})
				}
			}
		}
	}
	if err := json.NewEncoder(os.Stdout).Encode(vectors); err != nil {
		panic(err)
	}
}
