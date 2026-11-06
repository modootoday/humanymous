package main

import (
	"encoding/json"
	"os"

	"github.com/modootoday/humanymous/internal/scoring"
	"github.com/modootoday/humanymous/internal/signals"
)

func exportDerivation() {
	type vector struct {
		Client      signals.ClientReport `json:"client"`
		HasClient   bool                 `json:"has_client"`
		Environment []signals.Signal     `json:"environment"`
		Privacy     bool                 `json:"privacy"`
		Guard       []signals.Signal     `json:"guard"`
		Advanced    []signals.Signal     `json:"advanced"`
	}
	var vectors []vector
	for flags := 0; flags < 256; flags++ {
		bit := func(position uint) bool { return flags&(1<<position) != 0 }
		count := func(position uint) int {
			if bit(position) {
				return 1
			}
			return 0
		}
		dnt := "0"
		if bit(4) {
			dnt = "1"
		}
		client := signals.ClientReport{UserAgent: "Mozilla Chrome", Environment: signals.Environment{Probed: bit(0), AdBlock: bit(1), TrackingProtection: bit(2), GPC: bit(3), DoNotTrack: dnt, BraveShields: bit(5), CollectorBlocked: bit(7)}, Guard: signals.Guard{Reported: bit(0), NativeHooked: bit(1), ConsoleDisabled: bit(2), ProtoPolluted: bit(3), ScriptInjected: count(4), EvalUsed: count(5), FuncCtor: count(6)}, Advanced: signals.Advanced{Probed: bit(0), VoiceCount: count(1), MediaDeviceCount: count(2), WidevineSupported: bit(3), ConnectionPresent: bit(4), MeasureMemThrows: bit(6), WebGPUPresent: bit(7), WebGLVendor: "Intel", WebGPUVendor: "NVIDIA", MobileUA: bit(1), PointerCoarse: bit(2), MaxTouchPoints: count(3)}}
		if bit(5) {
			client.Advanced.AudioSampleRate = 24000
		} else {
			client.Advanced.AudioSampleRate = 48000
		}
		environment := scoring.EnvironmentSignals(client.Environment, bit(6))
		vectors = append(vectors, vector{client, bit(6), environment.Signals, environment.Privacy, scoring.GuardSignals(client.Guard), scoring.AdvancedSignals(client.Advanced, client.UserAgent)})
	}
	for _, vendor := range []string{"Intel", "NVIDIA", "unknown", ""} {
		client := signals.ClientReport{UserAgent: "Firefox", Advanced: signals.Advanced{Probed: true, VoiceCount: 1, MediaDeviceCount: 1, WebGPUPresent: true, WebGLVendor: "Intel", WebGPUVendor: vendor, AudioSampleRate: 24000.1, ConnectionRTT: 1}}
		vectors = append(vectors, vector{Client: client, Advanced: scoring.AdvancedSignals(client.Advanced, client.UserAgent)})
	}
	encoder := json.NewEncoder(os.Stdout)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(vectors); err != nil {
		panic(err)
	}
}
